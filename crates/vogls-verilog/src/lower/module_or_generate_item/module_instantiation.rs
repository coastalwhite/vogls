use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ConnectionDirection, SignalSlice, VectorSize};

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::{
    ListOfPortConnections, ModuleInstance, ModuleInstantiation, NamedPortConnection,
};
use crate::elaborate::{SignalSymbol, VSymbol};
use crate::lower::expression::get_expr_type;
use crate::lower::fuse::{
    DestinationSite, DestinationSiteContent, RequiredSignalKind, SiteTerminal, SiteType,
    SourceSite, SourceSiteContent, convert_src_sites_to_dsts, expr_to_src_sites, slice_src_sites,
};
use crate::lower::{LowerContext, MutLowerContext, VType, resolve_symbol_id, unwrap_get_signal};

/// Lower a Verilog module instantiation to Vogls IR.
///
/// This creates marshalling processes for port assignments.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, ModuleInstantiation<'a>>,
) -> Result<(), ()> {
    for instance in id.module_instances.iter() {
        let ModuleInstance {
            name_of_module_instance,
            range: _,
            list_of_port_connections,
        } = &*instance;

        let instance_sid =
            resolve_symbol_id(scope, &ctx.table, name_of_module_instance.item.0).unwrap();

        let instance_symbol = &ctx.table[instance_sid];
        let (module, module_sid, range_spec) = match &instance_symbol.content {
            VSymbol::Module(module) => (module, instance_sid, None),
            VSymbol::ModuleRange(_) => {
                let module_sid = instance_symbol.children()[0];
                let module = ctx.table[module_sid]
                    .content
                    .as_module()
                    .expect("A module range should only have module children");
                (module, module_sid, Some(instance_sid))
            }
            _ => unreachable!(),
        };

        match &**list_of_port_connections {
            ListOfPortConnections::Ordered(assignments) => {
                // Icarus Verilog has a good perspective on how to deal with unequal port
                // lengths. We should always warn here, but allow less ports.
                //
                // https://steveicarus.github.io/iverilog/developer/guide/misc/ieee1364-notes.html.
                if module.ports.len() != assignments.len() {
                    mctx.diagnostics.warn_not_yet_implemented(
                        ctx.arenas.get_range_span(*assignments),
                        "unequal number of ports",
                    );
                }

                // This will take the shortests of the assignments and the ports, which is what we
                // want.
                for (expr, &(port, direction)) in assignments.iter().zip(&module.ports) {
                    assign_port(ctx, mctx, scope, range_spec, direction, expr, port)?;
                }
            }
            ListOfPortConnections::Named(assignments) => {
                let mut error = false;
                // Keep track of which ports have already been assigned, so you can error on double
                // assignment.
                let mut ports_assigned = vec![false; module.ports.len()];

                for assignment in assignments.iter() {
                    let NamedPortConnection {
                        port_identifier,
                        expression,
                    } = *assignment;

                    // Find the assigned port and confirm it is actually a port.
                    // @TODO: Give a better error if it is not a port.
                    let Some(port_idx) = ctx
                        .table
                        .resolve(module_sid, port_identifier.item.0)
                        .and_then(|sid| ctx.table[sid].content.as_signal())
                        .and_then(|net| net.port_idx)
                    else {
                        mctx.diagnostics
                            .port_not_found(ctx.arenas, module, port_identifier);
                        error = true;
                        continue;
                    };
                    let (port, direction) = module.ports[port_idx];

                    // Error: port assigned twice.
                    if std::mem::replace(&mut ports_assigned[port_idx], true) {
                        mctx.diagnostics
                            .duplicate_definition(ctx.arenas, port_identifier);
                        error = true;
                        continue;
                    }

                    // Expression can be done, meaning "don't assign anything"
                    if let Some(expr) = expression {
                        assign_port(ctx, mctx, scope, range_spec, direction, expr, port)?;
                    }
                }

                if error {
                    return Err(());
                }
            }
        };
    }
    Ok(())
}

/// Assign a port to an expression.
fn assign_port<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    range_specification: Option<SymbolId>,
    direction: ConnectionDirection,
    value: AstId<'a, Expr<'a>>,
    port: SymbolId,
) -> Result<(), ()> {
    let port_ident = ctx.table[port].name();
    let port_symbol = ctx.table[port]
        .content
        .as_signal()
        .expect("Port should always be nets");
    let Some(modules_sid) = range_specification else {
        return match direction {
            ConnectionDirection::In => {
                assign_input_port(ctx, mctx, scope, value, port_symbol, None)
            }
            ConnectionDirection::Out => {
                assign_port_output(ctx, mctx, scope, value, port_symbol, None)
            }
            ConnectionDirection::Both => assign_inout(ctx, mctx, scope, value, port_symbol, None),
        };
    };

    let assigner_bit_length = get_expr_type(
        &mctx.gl,
        ctx.arenas,
        &ctx.table,
        scope,
        &mut mctx.diagnostics,
        value,
    )?
    .bit_length();
    let assigned_bit_length = port_symbol.ty.bit_length();

    let module_instances = ctx.table[modules_sid].children();

    let mut split_width = 0u32;
    if assigner_bit_length != assigned_bit_length {
        let expected_bit_length = u32::try_from(module_instances.len())
            .ok()
            .and_then(|num_instances| num_instances.checked_mul(assigned_bit_length.get()));

        if expected_bit_length != Some(assigner_bit_length.get()) {
            mctx.diagnostics
                .not_yet_implemented(ctx.arenas.get_span(value), "unexpected bit length");
            return Err(());
        }
        split_width = assigned_bit_length.get();
    }

    // Get the SymbolId of the actual module instance.
    for (i, &module_sid) in module_instances.iter().enumerate() {
        let port = ctx.table.resolve(module_sid, port_ident).unwrap();
        let port_symbol = unwrap_get_signal(&ctx.table, port);
        let lsb = i as u32 * split_width;
        let slice = SignalSlice::from_width(lsb, assigned_bit_length).unwrap();
        match direction {
            ConnectionDirection::In => {
                assign_input_port(ctx, mctx, scope, value, port_symbol, Some(slice))?
            }
            ConnectionDirection::Out => {
                assign_port_output(ctx, mctx, scope, value, port_symbol, Some(slice))?
            }
            ConnectionDirection::Both => {
                assign_inout(ctx, mctx, scope, value, port_symbol, Some(slice))?
            }
        }
    }

    Ok(())
}

/// Connects an inout port bidirectionally to the nets of `expr`.
///
/// The port and the nets are connected bit for bit from the least significant bit. If their widths
/// differ, the excess bits are left unconnected.
fn assign_inout<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    port: &SignalSymbol,
    expr_slice: Option<SignalSlice>,
) -> Result<(), ()> {
    mctx.fuse_srcs_scratch.clear();
    expr_to_src_sites(
        ctx,
        &mctx.gl,
        &mut mctx.fuse_srcs_scratch,
        scope,
        &mut mctx.diagnostics,
        expr,
        SiteTerminal::Probe,
        RequiredSignalKind::Net,
    )?;
    if let Some(expr_slice) = expr_slice
        && slice_src_sites(&mut mctx.fuse_srcs_scratch, expr_slice).is_err()
    {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(expr), "part-select on real");
        return Err(());
    }

    let Some(outer) = mctx
        .fuse_srcs_scratch
        .drain(..)
        .map(|site| match site.content {
            SourceSiteContent::Signal { signal, slice, .. } => Some((signal, slice)),
            SourceSiteContent::Constant(_)
            | SourceSiteContent::OutOfBounds { .. }
            | SourceSiteContent::Expr { .. } => None,
        })
        .collect::<Option<Vec<_>>>()
    else {
        mctx.diagnostics
            .output_expr_not_allowed(ctx.arenas.get_span(expr));
        return Err(());
    };

    let port_bit_length = port.ty.bit_length().get();
    let outer_bit_length = outer
        .iter()
        .try_fold(0u32, |acc, (_, slice)| acc.checked_add(slice.width().get()))
        .and_then(VectorSize::new);
    let Some(outer_bit_length) = outer_bit_length else {
        mctx.diagnostics
            .net_width_overflow(ctx.arenas.get_span(expr));
        return Err(());
    };
    if outer_bit_length.get() != port_bit_length {
        mctx.diagnostics.warn_assign_type_mismatch(
            ctx.arenas.get_span(expr),
            port.ty,
            VType::UnsignedNet(outer_bit_length),
        );
    }

    // Only the common least significant bits are connected.
    let bit_length = outer_bit_length.get().min(port_bit_length);
    let mut outer_connected = Vec::with_capacity(outer.len());
    let mut remaining = bit_length;
    for (signal, slice) in outer {
        let Some(width) = VectorSize::new(slice.width().get().min(remaining)) else {
            break;
        };
        outer_connected.push((signal, SignalSlice::from_width(slice.lsb(), width).unwrap()));
        remaining -= width.get();
    }

    let inner = [(
        port.terminals.probe_signal(),
        SignalSlice::from_width(0, VectorSize::new(bit_length).unwrap()).unwrap(),
    )];
    mctx.driver_resolution
        .push_bidirectional_sites(&outer_connected, &inner);
    Ok(())
}

fn assign_input_port<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    port: &SignalSymbol,
    expr_slice: Option<SignalSlice>,
) -> Result<(), ()> {
    let MutLowerContext {
        gl,
        diagnostics,
        fuse_srcs_scratch: srcs,
        fuse_dsts_scratch: dsts,
        driver_resolution,
        ..
    } = mctx;
    srcs.clear();
    dsts.clear();

    // Destinations, from least to most significant.
    dsts.push(DestinationSite {
        content: DestinationSiteContent::Signal {
            signal: port.terminals.blocking_drive_signal(),
            slice: SignalSlice::with_end(port.ty.bit_length()),
            kind: port.kind,
        },
        ty: port.ty.into(),
    });

    // Sources, from least to most significant.
    expr_to_src_sites(
        ctx,
        gl,
        srcs,
        scope,
        diagnostics,
        expr,
        SiteTerminal::Probe,
        RequiredSignalKind::None,
    )?;
    if let Some(expr_slice) = expr_slice
        && slice_src_sites(srcs, expr_slice).is_err()
    {
        diagnostics.not_yet_implemented(ctx.arenas.get_span(expr), "part-select on real");
        return Err(());
    }

    // The right-hand side is context-determined by the destinations, unless it is a concatenation
    // or only a part of it is assigned.
    let is_context_determined =
        expr_slice.is_none() && !matches!(&*expr, Expr::Concatenation(_) | Expr::Replication(_));
    let context = is_context_determined.then_some((expr, scope, port.ty));
    if driver_resolution
        .push_sites(gl, srcs, dsts, context)
        .is_err()
    {
        diagnostics.not_yet_implemented(
            ctx.arenas.get_span(expr),
            "assigning a real as part of a concatenation",
        );
        return Err(());
    }
    Ok(())
}

fn assign_port_output<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    output: &SignalSymbol,
    expr_slice: Option<SignalSlice>,
) -> Result<(), ()> {
    mctx.fuse_srcs_scratch.clear();
    mctx.fuse_dsts_scratch.clear();

    // Destinations
    expr_to_src_sites(
        ctx,
        &mctx.gl,
        &mut mctx.fuse_srcs_scratch,
        scope,
        &mut mctx.diagnostics,
        expr,
        SiteTerminal::BlockingDrive,
        RequiredSignalKind::Net,
    )?;
    if let Some(expr_slice) = expr_slice {
        // If we only include a part of the expression, we slice fuse destinations afterwards.
        if slice_src_sites(&mut mctx.fuse_srcs_scratch, expr_slice).is_err() {
            mctx.diagnostics
                .output_expr_not_allowed(ctx.arenas.get_span(expr));
            return Err(());
        }
    }
    if convert_src_sites_to_dsts(0, &mut mctx.fuse_dsts_scratch, &mut mctx.fuse_srcs_scratch)
        .is_err()
    {
        mctx.diagnostics
            .output_expr_not_allowed(ctx.arenas.get_span(expr));
        return Err(());
    }

    mctx.fuse_srcs_scratch.push(SourceSite {
        content: SourceSiteContent::Signal {
            signal: output.terminals.probe_signal(),
            slice: SignalSlice::with_end(output.ty.bit_length()),
            kind: output.kind,
        },
        ty: SiteType::from(output.ty),
    });

    if mctx
        .driver_resolution
        .push_sites(
            &mctx.gl,
            &mctx.fuse_srcs_scratch,
            &mctx.fuse_dsts_scratch,
            None,
        )
        .is_err()
    {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_span(expr),
            "connecting a real output port to a net",
        );
        return Err(());
    }

    Ok(())
}
