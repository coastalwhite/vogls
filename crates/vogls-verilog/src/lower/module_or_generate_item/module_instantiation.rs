use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::{Driver, InputEdge};
use vogls_ir::{ConnectionDirection, ProcessBuilder, ProcessKind, SignalSlice, VariableKey};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::{
    ListOfPortConnections, ModuleInstance, ModuleInstantiation, NamedPortConnection,
};
use crate::elaborate::VSymbol;
use crate::lower::addressing::{
    Address, ConstantAddressingContext, LValueAddressingContext, lower_addressing,
};
use crate::lower::expression::{self, get_expr_type};
use crate::lower::fuse::try_lower_fuse_driver_expr;
use crate::lower::{
    Diagnostics, LowerContext, MutLowerContext, VType, resolve_symbol_id, try_resolve_hident,
    try_resolve_net, unwrap_get_net,
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
                        .and_then(|sid| ctx.table[sid].content.as_net())
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
        .as_net()
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
        &mut mctx.gl,
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
    let port = unwrap_get_net(&ctx.table, port);

    let start_connections_length = mctx.connections.len();

    mctx.fuse_scratch.clear();
    'try_fuse: {
        // @TODO: Implement expr_slice.is_some()
        if expr_slice.is_none() && try_lower_fuse_driver_expr(ctx, mctx, scope, expr)? {
            let drivee = port.net.blocking_drive_signal();

            let mut offset = 0;
            let drivee_width = port.ty.bit_length();
            for driver in &mctx.fuse_scratch {
                let driver_width = driver.size(&mctx.gl.signals);

                if driver_width.get() > drivee_width.get() - offset {
                    mctx.connections.truncate(start_connections_length);
                    break 'try_fuse;
                }

                mctx.connections.push(InputEdge {
                    driver: driver.clone(),
                    drivee,
                    drivee_slice: Some(SignalSlice::from_width(offset, driver_width).unwrap()),
                });
                offset += driver_width.get();
            }

            if offset != drivee_width.get() {
                mctx.connections.truncate(start_connections_length);
                break 'try_fuse;
            }

            return Ok(());
        }
    }

    let mut sensitivity_list = OrderedSet::new();
    expression::get_used_signals(ctx, mctx, scope, &mut sensitivity_list, expr)?;
    let sensitivity_list = sensitivity_list.items;

    let port_bit_length = port.ty.bit_length();

    let (process, mut bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Port, ctx.arenas.get_span(expr));
    let entry_key = bb_builder.key();
    let (v, v_ty) = expression::lower_expr(
        ctx,
        mctx,
        scope,
        &mut bb_builder,
        expr,
        Some(port_bit_length),
    )?;
    let v = match expr_slice {
        None => {
            expression::sign_or_zero_extend(mctx.gl(), &mut bb_builder, v, v_ty, port_bit_length)
        }
        Some(slice) => bb_builder.slice_constant(mctx.gl(), v, slice.lsb(), slice.width()),
    };
    port.net.drive_blocking(mctx.gl(), &mut bb_builder, v, None);

    bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_key);
    process.finalize(mctx.gl());
    Ok(())
}

fn assign_port_output<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    output_net: SymbolId,
    expr_slice: Option<SignalSlice>,
) -> Result<(), ()> {
    let output = unwrap_get_net(&ctx.table, output_net);

    if let Expr::Ident(ident, exprs, range) = &*expr {
        let to_signal =
            try_resolve_net(scope, &ctx.table, ctx.arenas, *ident, &mut mctx.diagnostics)?;
        let driver = output.net.probe_signal();
        let drivee = to_signal.net.blocking_drive_signal();

        let mut actx = ConstantAddressingContext {
            gl: &mctx.gl,
            arenas: ctx.arenas,
            table: &ctx.table,
            scope,
            diagnostics: &mut Diagnostics::default(),
            loc: expr.loc,
            _pd: std::marker::PhantomData,
        };

        let range = range.map(|r| r.into());

        if let Ok(address) = lower_addressing(
                &mut actx,
                to_signal.ty.bit_length(),
                &to_signal.dims,
                to_signal.transform,
                exprs.iter().map(|e| e.into_constant()),
                range,
            ) && let Some(offset) = address.signal_offset_as_u32()
            // Don't fuse if the widths don't match.
            && expr_slice.map_or(address.output_width, |s| s.width()) == output.ty.bit_length()
        {
            mctx.connections.push(InputEdge {
                driver: Driver::Signal(driver, None),
                drivee,
                drivee_slice: Some(
                    SignalSlice::from_width(
                        expr_slice.map_or(0, |v| v.lsb()) + offset,
                        expr_slice.map_or(address.output_width, |v| v.width()),
                    )
                    .unwrap(),
                ),
            });
            return Ok(());
        }
    }

    if expr_slice.is_some() {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_span(expr),
            "range specification with concat or repetition output port.",
        );
        return Err(());
    }

    let (process, mut bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Port, ctx.arenas.get_span(expr));
    let bb_key = bb_builder.key();

    let output_net = &output.net;
    let probed = output.net.probe(mctx.gl(), &mut bb_builder);

    let mut driving: Vec<(VariableKey, VType, AstId<Expr>)> = Vec::new();
    driving.push((probed, output.ty, expr));

    let mut sensitivity_list = OrderedSet::new();
    sensitivity_list.insert(output_net.probe_signal());

    let mut error = false;
    while let Some((var, var_ty, expr)) = driving.pop() {
        match &*expr {
            Expr::Concatenation(exprs) => {
                let mut shift = 0;
                for e in exprs.iter().rev() {
                    let e_ty = get_expr_type(
                        &mctx.gl,
                        ctx.arenas,
                        &ctx.table,
                        scope,
                        &mut mctx.diagnostics,
                        e,
                    )?;
                    let e_width = e_ty.bit_length();
                    let subvar = bb_builder.slice_constant(mctx.gl(), var, shift, e_width);
                    driving.push((subvar, e_ty, e));
                    shift += e_width.get();
                }
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                let symbol_key = try_resolve_hident(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    *ast_ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Net(s) = &ctx.table[symbol_key].content else {
                    mctx.diagnostics
                        .output_expr_not_allowed(ctx.arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let mut actx = LValueAddressingContext {
                    ctx,
                    mctx,
                    builder: &mut bb_builder,
                    loc: expr.loc,
                    scope,
                };

                let Address {
                    elem_offset,
                    output_width,
                    array,
                    is_unsigned: _,
                } = lower_addressing(
                    &mut actx,
                    s.ty.bit_length(),
                    &s.dims,
                    s.transform,
                    exprs.iter(),
                    range_expression.map(|r| r.into()),
                )?;

                // @TODO: Use array overflow.
                let partial = match (elem_offset, array) {
                    (Some(elem_offset), Some((array_offset, _array_overflow))) => {
                        Some(bb_builder.plus(mctx.gl(), elem_offset, array_offset))
                    }
                    (Some(elem_offset), None) => Some(elem_offset),
                    (None, Some((array_offset, _array_overflow))) => Some(array_offset),
                    (None, None) => None,
                };
                let variable = expression::truncate_or_extend(
                    mctx.gl(),
                    &mut bb_builder,
                    var,
                    var_ty,
                    output_width,
                );
                s.net
                    .drive_blocking(mctx.gl(), &mut bb_builder, variable, partial);
            }

            Expr::Replication(_) => {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(expr), "repetition in net assign");
                error = true;
            }

            Expr::FunctionCall(..)
            | Expr::SystemFunctionCall(..)
            | Expr::Decimal(..)
            | Expr::Sized(..)
            | Expr::Ternary(..)
            | Expr::String(..)
            | Expr::Unary(..)
            | Expr::Binary(..) => {
                mctx.diagnostics
                    .output_expr_not_allowed(ctx.arenas.get_span(expr));
                error = true;
            }
        }
    }

    let sensitivity_list = sensitivity_list.items;
    bb_builder.watch_to(mctx.gl(), sensitivity_list, bb_key);
    process.finalize(mctx.gl());

    if error {
        return Err(());
    }

    Ok(())
}
