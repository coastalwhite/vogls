use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, Bits, SCALAR_VSIZE, SignalSlice, Time, VariableKey, VectorSize};

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::{ContinousAssign, NetAssignment};
use crate::ast::statement::Delay3;
use crate::elaborate::{NetResolution, SignalKind};
use crate::lower::delay_pointer::DelayPtr;
use crate::lower::drivers::DelayKind;
use crate::lower::fuse::{
    DestinationSite, DestinationSiteContent, RequiredSignalKind, SiteTerminal, SiteType,
    expr_to_src_sites, net_flat_lvalue_to_dsts,
};
use crate::lower::statement::procedural_timing_control::lower_min_typ_max_delay;
use crate::lower::{LowerContext, MutLowerContext, VType};

use super::gate_instantiation::{
    lower_const_gate_delay, lower_delay_value, lower_delay3, try_push_delay3,
};

/// Lower a Verilog `assign` construct to Vogls IR.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
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

    for net_assignment in list_of_net_assignments.iter() {
        assign_net_lvalue(ctx, mctx, scope, net_assignment, *delay3)?;
    }

    Ok(())
}

fn assign_net_lvalue<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    net_assignment: AstId<'a, NetAssignment<'a>>,
    delay3: Option<AstId<'a, Delay3<'a>>>,
) -> Result<(), ()> {
    let NetAssignment {
        net_lvalue,
        expression: rvalue,
    } = *net_assignment;

    mctx.fuse_dsts_scratch.clear();
    mctx.fuse_srcs_scratch.clear();

    // Destinations, from least to most significant.
    for net_flat_lvalue in net_lvalue.0.iter().rev() {
        net_flat_lvalue_to_dsts(
            ctx,
            &mctx.gl,
            &mut mctx.fuse_dsts_scratch,
            &mut mctx.fuse_srcs_scratch,
            &mut mctx.diagnostics,
            scope,
            net_flat_lvalue,
        )?;
    }

    // The type of the destinations as a whole. A concatenation is unsigned.
    let dsts_ty = match mctx.fuse_dsts_scratch.as_slice() {
        [dst] => match dst.ty {
            SiteType::Real => VType::Real,
            SiteType::Signed => VType::SignedNet(dst.bit_length()),
            SiteType::Unsigned => VType::UnsignedNet(dst.bit_length()),
        },
        dsts => {
            let Some(bit_length) = dsts
                .iter()
                .try_fold(0u32, |acc, d| acc.checked_add(d.bit_length().get()))
                .and_then(VectorSize::new)
            else {
                mctx.diagnostics
                    .net_width_overflow(ctx.arenas.get_span(rvalue));
                return Err(());
            };
            VType::UnsignedNet(bit_length)
        }
    };

    // A delayed assignment assigns the input of a delay in front of the destinations.
    if let Some(delay3) = delay3 {
        let bit_length = dsts_ty.bit_length();
        let kind = assign_delay_kind(ctx, mctx, scope, delay3)?;
        let input = mctx.driver_resolution.push_delay(
            &mut mctx.gl,
            ctx.logic_mode,
            kind,
            &mctx.fuse_dsts_scratch,
            bit_length,
        );
        mctx.fuse_dsts_scratch.clear();
        mctx.fuse_dsts_scratch.push(DestinationSite {
            content: DestinationSiteContent::Signal {
                signal: input,
                slice: SignalSlice::with_end(bit_length),
                // The input of a delay is only driven by the assignment.
                kind: SignalKind::Net(NetResolution::Wire),
            },
            ty: SiteType::from(dsts_ty),
        });
    }

    // Sources, from least to most significant.
    expr_to_src_sites(
        ctx,
        &mctx.gl,
        &mut mctx.fuse_srcs_scratch,
        scope,
        &mut mctx.diagnostics,
        rvalue,
        SiteTerminal::Probe,
        RequiredSignalKind::None,
    )?;

    // The right-hand side is context-determined by the destinations, unless it is a concatenation.
    let is_context_determined = !matches!(&*rvalue, Expr::Concatenation(_) | Expr::Replication(_));
    let context = is_context_determined.then_some((rvalue, scope, dsts_ty));
    if mctx
        .driver_resolution
        .push_sites(
            &mctx.gl,
            &mctx.fuse_srcs_scratch,
            &mctx.fuse_dsts_scratch,
            context,
        )
        .is_err()
    {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_span(rvalue),
            "assigning a real as part of a concatenation",
        );
        return Err(());
    }
    Ok(())
}

pub(crate) fn lower_assign_delay3<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
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
    let turn_off = turn_off
        .map(|turn_off| lower_min_typ_max_delay(ctx, mctx, scope, builder, turn_off))
        .transpose()?;
    Ok(select_vector_assign_delay(
        mctx,
        builder,
        rise,
        fall,
        turn_off,
        new_value,
        driven_value,
        width,
    ))
}

/// The delay of the transition of a vector net assignment of `width` bits from `driven_value` to
/// `new_value`, given its `rise`, `fall` and `turn_off` delays.
#[allow(clippy::too_many_arguments)]
fn select_vector_assign_delay(
    mctx: &mut MutLowerContext<'_>,
    builder: &mut BasicBlockBuilder,
    rise: VariableKey,
    fall: VariableKey,
    turn_off: Option<VariableKey>,
    new_value: VariableKey,
    driven_value: VariableKey,
    width: VectorSize,
) -> VariableKey {
    // Without a turn-off delay, a transition to z takes the smaller of the rise and fall delay.
    let turn_off = turn_off.unwrap_or_else(|| builder.min(mctx.gl(), rise, fall));

    let is_z = builder.case_equals_constant(mctx.gl(), new_value, Bits::new_high_impedance(width));
    let is_zero = builder.case_equals_constant(mctx.gl(), new_value, Bits::new_zeroed(width));
    let was_zero = builder.case_equals_constant(mctx.gl(), driven_value, Bits::new_zeroed(width));
    let was_nonzero = builder.binary_not(mctx.gl(), was_zero);
    let is_falling = builder.and(mctx.gl(), is_zero, was_nonzero);

    let delay = builder.select(mctx.gl(), is_z, turn_off, rise);
    builder.select(mctx.gl(), is_falling, fall, delay)
}

/// The delay `delay3` of a net assignment in `scope`: constant if all of its delays are (see
/// [`try_push_delay3`]).
pub(crate) fn assign_delay_kind<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    delay3: AstId<'a, Delay3<'a>>,
) -> Result<DelayKind<'a>, ()> {
    Ok(match try_push_delay3(ctx, mctx, scope, delay3)? {
        Some(delays) => DelayKind::ConstAssign {
            delays,
            span: ctx.arenas.get_span(delay3),
        },
        None => DelayKind::Assign {
            delay3,
            scope,
            time_scale: ctx.time_scale,
        },
    })
}

/// Lowers the constant delay `ptr` into `delays` of a net assignment of `width` bits for its
/// transition from `driven_value` to `new_value`. A single delay applies to every transition.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_const_assign_delay(
    mctx: &mut MutLowerContext<'_>,
    builder: &mut BasicBlockBuilder,
    delays: &[Time],
    ptr: DelayPtr,
    new_value: VariableKey,
    driven_value: VariableKey,
    width: VectorSize,
) -> VariableKey {
    // A scalar net assignment is delayed like a gate.
    if width == SCALAR_VSIZE {
        return lower_const_gate_delay(mctx, builder, delays, ptr, new_value);
    }

    let mut reader = ptr.reader(delays);
    let mut next = |mctx: &mut MutLowerContext<'_>, builder: &mut BasicBlockBuilder| {
        builder.constant_u64(mctx.gl(), reader.get().0)
    };
    let rise = next(mctx, builder);
    if ptr.num_delays() == 1 {
        return rise;
    }
    let fall = next(mctx, builder);
    let turn_off = (ptr.num_delays() == 3).then(|| next(mctx, builder));
    select_vector_assign_delay(
        mctx,
        builder,
        rise,
        fall,
        turn_off,
        new_value,
        driven_value,
        width,
    )
}
