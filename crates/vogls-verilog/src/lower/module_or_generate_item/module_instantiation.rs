use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ConnectionDirection, ProcessBuilder, ProcessKind, SignalSlice, VectorSize};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::{
    ListOfPortConnections, ModuleInstance, ModuleInstantiation, NamedPortConnection,
};
use crate::elaborate::{SignalSymbol, VSymbol};
use crate::lower::expression::{self, get_expr_type};
use crate::lower::fuse::{
    DestinationSite, DestinationSiteContent, FuseFailure, FuseResult, RequiredSignalKind,
    SiteTerminal, SiteType, SourceSite, SourceSiteContent, convert_src_sites_to_dsts,
    expr_to_src_sites, try_fuse_srcs_to_dsts,
};
use crate::lower::multidriver::MultiDriverResult;
use crate::lower::{
    Diagnostics, LowerContext, MutLowerContext, VType, resolve_symbol_id, unwrap_get_signal,
};

/// Lower a Verilog module instantiation to Vogls IR.
///
/// This creates marshalling processes for port assignments.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
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
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    range_specification: Option<SymbolId>,
    direction: ConnectionDirection,
    value: AstId<'a, Expr<'a>>,
    port: SymbolId,
) -> Result<(), ()> {
    let port_ident = ctx.table[port].name();
    let port_net = ctx.table[port]
        .content
        .as_signal()
        .expect("Port should always be nets");
    let is_input = matches!(
        direction,
        ConnectionDirection::In | ConnectionDirection::Both
    );
    let is_output = matches!(
        direction,
        ConnectionDirection::Out | ConnectionDirection::Both
    );

    let Some(modules_sid) = range_specification else {
        // Base case is that you have no range specification for your port.
        if is_input {
            assign_input_port(ctx, mctx, scope, value, port, None)?;
        }
        if is_output {
            assign_port_output(ctx, mctx, scope, value, port, None)?;
        }
        return Ok(());
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
    let assigned_bit_length = port_net.ty.bit_length();

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
        let lsb = i as u32 * split_width;
        let slice = SignalSlice::from_width(lsb, assigned_bit_length).unwrap();
        if is_input {
            assign_input_port(ctx, mctx, scope, value, port, Some(slice))?;
        }
        if is_output {
            assign_port_output(ctx, mctx, scope, value, port, Some(slice))?;
        }
    }

    Ok(())
}

fn assign_input_port<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    port: SymbolId,
    expr_slice: Option<SignalSlice>,
) -> Result<(), ()> {
    // @NOTE: Input ports cannot have multiple drivers, so they don't have be tracked.
    let port = unwrap_get_signal(&ctx.table, port);

    if try_fuse_input_port(ctx, mctx, scope, expr, port, expr_slice).is_ok() {
        return Ok(());
    }

    let mut sensitivity_list = OrderedSet::new();
    expression::get_used_signals(ctx, mctx, scope, &mut sensitivity_list, expr)?;
    let sensitivity_list = sensitivity_list.items;

    let port_bit_length = port.ty.bit_length();

    let (proc_builder, mut bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Port, ctx.arenas.get_span(expr));
    let entry_tr = proc_builder.entry();
    let (v, v_ty) = expression::lower_expr(
        ctx,
        mctx,
        scope,
        &mut bb_builder,
        expr,
        Some(port_bit_length),
    )?;
    let v = match expr_slice {
        None => expression::coerce_to(mctx.gl(), &mut bb_builder, v, v_ty, port.ty),
        Some(slice) => bb_builder.slice_constant(mctx.gl(), v, slice.lsb(), slice.width()),
    };
    port.terminals
        .drive_blocking(mctx.gl(), &mut bb_builder, v, None);

    bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);
    proc_builder.finalize(mctx.gl());
    Ok(())
}

fn try_fuse_input_port<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    port: &SignalSymbol,
    expr_slice: Option<SignalSlice>,
) -> FuseResult {
    mctx.fuse_srcs_scratch.clear();
    mctx.fuse_dsts_scratch.clear();

    // Destinations
    mctx.fuse_dsts_scratch.push(DestinationSite {
        content: DestinationSiteContent::Signal {
            signal: port.terminals.blocking_drive_signal(),
            slice: SignalSlice::with_end(port.ty.bit_length()),
        },
        ty: port.ty.into(),
    });

    // Sources
    if expr_to_src_sites(
        ctx,
        &mctx.gl,
        &mut mctx.fuse_srcs_scratch,
        scope,
        &mut Diagnostics::default(),
        expr,
        SiteTerminal::Probe,
        RequiredSignalKind::None,
    )
    .is_err()
    {
        return FuseResult::Err(FuseFailure);
    };
    if let Some(expr_slice) = expr_slice {
        // If we only include a part of the expression, we slice fuse sources afterwards.
        if slice_src_sites(&mut mctx.fuse_srcs_scratch, expr_slice).is_err() {
            return Err(FuseFailure);
        }
    }

    // @NOTE: Input ports cannot have multiple drivers, so they don't have be tracked.
    try_fuse_srcs_to_dsts(
        &mctx.gl,
        &mut mctx.connections,
        &mctx.fuse_srcs_scratch,
        &mctx.fuse_dsts_scratch,
    )
}

fn assign_port_output<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    output_net: SymbolId,
    expr_slice: Option<SignalSlice>,
) -> Result<(), ()> {
    let output = unwrap_get_signal(&ctx.table, output_net);

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
        },
        ty: SiteType::from(output.ty),
    });

    for dst in mctx.fuse_dsts_scratch.iter_mut() {
        if let DestinationSiteContent::Signal { signal, slice } = &mut dst.content {
            match mctx
                .multidriver
                .push_new_driver(&mut mctx.gl, *signal, *slice)
            {
                MultiDriverResult::Keep => {}
                MultiDriverResult::NeedsRemap(proxy) => *signal = proxy,
            };
        }
    }

    if try_fuse_srcs_to_dsts(
        &mctx.gl,
        &mut mctx.connections,
        &mctx.fuse_srcs_scratch,
        &mctx.fuse_dsts_scratch,
    )
    .is_ok()
    {
        return Ok(());
    }

    let (proc_builder, mut bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Port, ctx.arenas.get_span(expr));
    let entry_tr = proc_builder.entry();

    let destination_size = mctx
        .fuse_dsts_scratch
        .iter()
        .map(|d| d.bit_length())
        .try_fold(0u32, |l, r| l.checked_add(r.get()))
        .unwrap();
    let destination_size = VectorSize::new(destination_size).unwrap();

    let output_signal = output.terminals.probe_signal();
    let mut probed = bb_builder.probe(mctx.gl(), output_signal);
    let source_size = mctx.gl.vars.size(probed);
    if source_size != destination_size {
        probed = match output.ty {
            VType::SignedNet(_) | VType::UnsignedNet(_) if destination_size < source_size => {
                bb_builder.truncate(mctx.gl(), probed, destination_size)
            }
            VType::SignedNet(_) => bb_builder.sign_extend(mctx.gl(), probed, destination_size),
            VType::UnsignedNet(_) => bb_builder.zero_extend(mctx.gl(), probed, destination_size),
            VType::Real => {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(expr), "part-select on real");
                return Err(());
            }
        };
    }

    let mut offset = 0;
    for dst in &mctx.fuse_dsts_scratch {
        let bitlength = dst.bit_length();
        if let DestinationSiteContent::Signal { signal, slice } = dst.content {
            let probe_v = bb_builder.slice_constant(&mut mctx.gl, probed, offset, bitlength);
            bb_builder.drive_partial_constant(&mut mctx.gl, signal, probe_v, slice.lsb());
        }
        offset += bitlength.get();
    }

    bb_builder.watch_to(mctx.gl(), [output_signal].into(), entry_tr);
    proc_builder.finalize(mctx.gl());

    Ok(())
}

fn slice_src_sites(srcs: &mut Vec<SourceSite>, slice: SignalSlice) -> Result<(), ()> {
    // Expression slice is [lo, hi)
    let (lo, hi) = (slice.lsb(), slice.msb() + 1);

    let mut failed = false;
    let mut bit_offset = 0u32;

    srcs.retain_mut(|src| {
        let src_offset = bit_offset;
        bit_offset += src.bit_length().get();

        // Entire source is included.
        if lo <= src_offset && hi >= bit_offset {
            return true;
        }

        // Source slice is [src_lo, src_hi)
        let src_lo = lo.max(src_offset);
        let src_hi = hi.min(bit_offset);

        // Entire source is excluded.
        if src_lo >= src_hi {
            return false;
        }

        // We cannot partially slice a real.
        failed |= src.ty == SiteType::Real;

        let sub = SignalSlice::from_width(
            src_lo - src_offset,
            VectorSize::new(src_hi - src_lo).unwrap(),
        )
        .unwrap();
        src.content = match &src.content {
            SourceSiteContent::Constant(bits) => {
                SourceSiteContent::Constant(bits.slicez(sub.lsb(), sub.width()))
            }
            SourceSiteContent::Signal { signal, slice } => SourceSiteContent::Signal {
                signal: *signal,
                slice: slice.subslice(sub).unwrap(),
            },
            SourceSiteContent::OutOfBounds {
                signal,
                bit_length: _,
            } => SourceSiteContent::OutOfBounds {
                signal: *signal,
                bit_length: sub.width(),
            },
        };
        true
    });

    if failed {
        return Err(());
    }

    Ok(())
}
