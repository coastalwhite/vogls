use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::InputEdge;
use vogls_ir::{
    BasicBlockBuilder, Bits, ProcessBuilder, ProcessKind, SCALAR_VSIZE, SignalSlice, VariableKey,
    VectorSize,
};
use vogls_utils::OrderedSet;

use crate::ast::constant_expr::{ConstantBitSlice, ConstantExpr};
use crate::ast::expr::Expr;
use crate::ast::module::{ContinousAssign, NetAssignment};
use crate::ast::statement::Delay3;
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::VSymbol;
use crate::lower::addressing::{ConstantAddressingContext, lower_addressing};
use crate::lower::assign::{assign_net_lvalue, net_lvalue_size};
use crate::lower::expression::{coerce_to, get_expr_type, get_used_signals, lower_expr};
use crate::lower::fuse::try_lower_fuse_driver_expr;
use crate::lower::statement::procedural_timing_control::lower_min_typ_max_delay;
use crate::lower::{Diagnostics, LowerContext, MutLowerContext, VType, try_resolve_hident};

use super::gate_instantiation::{GateDelayFn, lower_delay_value, lower_delay3, lower_gate_drive};

/// Lower a Verilog `assign` construct to Vogls IR.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    assign: AstId<'a, ContinousAssign<'a>>,
) -> Result<(), ()> {
    let ContinousAssign {
        drive_strength,
        delay3,
        list_of_net_assignments,
    } = &*assign;

    if let Some(drive_strength) = drive_strength {
        mctx.diagnostics.warn_not_yet_implemented(
            ctx.arenas.get_item_span(*drive_strength),
            "Drive strength is ignored.",
        );
    }

    for ast_net_assignment in list_of_net_assignments.iter() {
        let net_assignment = &*ast_net_assignment;

        // Optimization: Try to alias the expression into the assignee. If this is successful,
        // future analysis may be able to completely remove this process.
        if delay3.is_none() && try_fuse_assign(ctx, mctx, scope, ast_net_assignment)? {
            continue;
        }

        // See which signals are used in the expression.
        let mut watch_list = OrderedSet::new();
        get_used_signals(ctx, mctx, scope, &mut watch_list, net_assignment.expression)?;
        let watch_list = watch_list.items;

        // Create a process that
        // 1. Computes the expression output                       (RValue)
        // 2. Drives the net(s)                                    (LValue)
        // 3. Watches for updates to any used signal in the RValue
        let (proc_builder, mut bb_builder) =
            ProcessBuilder::new(mctx.gl(), ProcessKind::Assign, ctx.arenas.get_span(assign));
        let entry_tr = proc_builder.entry();

        let context_width = net_lvalue_size(ctx, mctx, scope, net_assignment.net_lvalue)?;
        let (rvalue, rvalue_ty) = lower_expr(
            ctx,
            mctx,
            scope,
            &mut bb_builder,
            net_assignment.expression,
            Some(context_width),
        )?;

        match delay3 {
            None => assign_net_lvalue(
                ctx,
                mctx,
                scope,
                &mut bb_builder,
                net_assignment.net_lvalue,
                rvalue,
                rvalue_ty,
            )?,
            Some(delay3) => {
                let delay3 = *delay3;
                let value = coerce_to(
                    mctx.gl(),
                    &mut bb_builder,
                    rvalue,
                    rvalue_ty,
                    VType::UnsignedNet(context_width),
                );
                let delay = move |mctx: &mut MutLowerContext,
                                  builder: &mut BasicBlockBuilder,
                                  new_value: VariableKey,
                                  driven_value: VariableKey| {
                    lower_assign_delay3(
                        ctx,
                        mctx,
                        scope,
                        builder,
                        delay3,
                        new_value,
                        driven_value,
                        context_width,
                    )
                };
                lower_gate_drive(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    ctx.arenas.get_span(ast_net_assignment),
                    Some(&delay as &GateDelayFn),
                    net_assignment.net_lvalue,
                    value,
                    context_width,
                )?;
            }
        }

        bb_builder.watch_to(mctx.gl(), watch_list, entry_tr);
        proc_builder.finalize(mctx.gl());
    }

    Ok(())
}

pub(crate) fn lower_assign_delay3<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    delay3: AstId<'a, Delay3<'a>>,
    new_value: VariableKey,
    driven_value: VariableKey,
    width: VectorSize,
) -> Result<VariableKey, ()> {
    if width == SCALAR_VSIZE {
        return lower_delay3(ctx, mctx, scope, builder, delay3, new_value, width);
    }

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 71
    //
    // """
    // If the left-hand references a scalar net, then the delay shall be treated in the same way as for
    // gate delays [..]
    //
    // If the left-hand references a vector net, then up to three delays can be applied. The following
    // rules determine which delay controls the assignment:
    // - If the right-hand side makes a transition from nonzero to zero, then the falling delay shall
    //   be used.
    // - If the right-hand side makes a transition to z, then the turn-off delay shall be used.
    // - For all other cases, the rising delay shall be used.
    // """
    let (rise, fall, turn_off) = match &*delay3 {
        Delay3::Value(value) => return lower_delay_value(ctx, mctx, scope, builder, *value),
        Delay3::Single(delay) => return lower_min_typ_max_delay(ctx, mctx, scope, builder, *delay),
        Delay3::Double(rise, fall) => (*rise, *fall, None),
        Delay3::Triple(rise, fall, turn_off) => (*rise, *fall, Some(*turn_off)),
    };

    let rise = lower_min_typ_max_delay(ctx, mctx, scope, builder, rise)?;
    let fall = lower_min_typ_max_delay(ctx, mctx, scope, builder, fall)?;

    // Without a turn-off delay, a transition to z takes the smaller of the rise and fall delay.
    let turn_off = match turn_off {
        Some(turn_off) => lower_min_typ_max_delay(ctx, mctx, scope, builder, turn_off)?,
        None => builder.min(mctx.gl(), rise, fall),
    };

    let is_z = builder.case_equals_constant(mctx.gl(), new_value, Bits::new_high_impedance(width));
    let is_zero = builder.case_equals_constant(mctx.gl(), new_value, Bits::new_zeroed(width));
    let was_zero = builder.case_equals_constant(mctx.gl(), driven_value, Bits::new_zeroed(width));
    let was_nonzero = builder.binary_not(mctx.gl(), was_zero);
    let is_falling = builder.and(mctx.gl(), is_zero, was_nonzero);

    let delay = builder.select(mctx.gl(), is_z, turn_off, rise);
    Ok(builder.select(mctx.gl(), is_falling, fall, delay))
}

fn try_fuse_assign<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    net_assignment: AstId<'a, NetAssignment<'a>>,
) -> Result<bool, ()> {
    // @TODO: Support concatenation
    if net_assignment.net_lvalue.0.len() != 1 {
        return Ok(false);
    }

    let lvalue = net_assignment.net_lvalue.0.get(0);
    let to_net = try_resolve_hident(
        scope,
        &ctx.table,
        ctx.arenas,
        lvalue.ident,
        &mut mctx.diagnostics,
    )?;

    try_fuse_flat_assign(
        ctx,
        mctx,
        scope,
        to_net,
        net_assignment.expression,
        net_assignment.loc,
        lvalue.constant_exprs,
        lvalue.constant_range_expression,
    )
}

pub fn try_fuse_flat_assign<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    net: SymbolId,
    rvalue: AstId<'a, Expr<'a>>,
    loc: usize,
    constant_exprs: AstIdRange<'a, ConstantExpr<'a>>,
    constant_range_expression: Option<AstId<'a, ConstantBitSlice<'a>>>,
) -> Result<bool, ()> {
    mctx.fuse_scratch.clear();
    if !try_lower_fuse_driver_expr(ctx, mctx, scope, rvalue)? {
        return Ok(false);
    }

    let VSymbol::Net(n) = &ctx.table[net].content else {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.spans[loc], "cannot be used as net");
        return Err(());
    };

    // Fusing does not deal with sign-extension or conversion to/from reals. Therefore, we stop
    // fusing them here.
    if get_expr_type(
        &mctx.gl,
        ctx.arenas,
        &ctx.table,
        scope,
        &mut Diagnostics::default(),
        rvalue,
    )
    .map_or(true, |v| {
        v.bit_length() != n.ty.bit_length() && (!v.is_unsigned_net() || !n.ty.is_unsigned_net())
    }) {
        return Ok(false);
    }

    let mut actx = ConstantAddressingContext {
        gl: &mctx.gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut Diagnostics::default(),
        loc,
        _pd: std::marker::PhantomData,
    };

    let Ok(address) = lower_addressing(
        &mut actx,
        n.ty.bit_length(),
        &n.dims,
        n.transform,
        constant_exprs.iter(),
        constant_range_expression.map(|r| (*r).into()),
    ) else {
        return Ok(false);
    };

    let Some(offset) = address.signal_offset_as_u32() else {
        return Ok(false);
    };
    let drivee = n.net.blocking_drive_signal();

    // @TODO: sum(driver.size()) > output_width
    let drivee_signal_width = mctx.gl.signals[n.net.blocking_drive_signal()].size;
    let mut offset = offset;
    for driver in &mctx.fuse_scratch {
        let width = driver.size(&mctx.gl.signals);
        let Some(width) = VectorSize::new((drivee_signal_width.get() - offset).min(width.get()))
        else {
            break;
        };

        // If we are fusing things that don't exist. Just cancel the fuse.
        if drivee_signal_width.get() < offset + width.get() {
            return Ok(false);
        }

        mctx.connections.push(InputEdge {
            driver: driver.clone(),
            drivee,
            drivee_slice: Some(SignalSlice::from_width(offset, width).unwrap()),
        });
        offset += width.get();
    }
    Ok(true)
}
