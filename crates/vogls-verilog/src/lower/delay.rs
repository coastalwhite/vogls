//! Conversion of delays in time units of a module to simulation ticks.
//!
//! Rules for reals:
//! - NaNs, infinities and negative numbers become 0.
//!
//! Rules for integers:
//! - Signed integers sign extend to i64 and are reinterpreted as u64.
//! - Integers containing `x` and `z` become zero.

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::time::TimeResolution;
use vogls_ir::{BasicBlockBuilder, Bits, LogicMode, TIME_VSIZE, Time, VariableKey};

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::TimeScale;
use crate::ast::statement::{DelayValue, MinTypMaxExpression};
use crate::lower::delay_pointer::{DelayPtr, DelayPtrVariant};
use crate::lower::expression::truncate_or_extend;
use crate::lower::statement::procedural_timing_control::evaluate_delay_value_as_ticks;
use crate::lower::{Diagnostics, LowerContext, MutLowerContext, VType, VValue, eval_constant_expr};

impl VValue {
    /// Convert a constant delay to simulation ticks.
    ///
    /// Delays larger than the largest time saturate.
    pub fn delay_to_ticks(&self, time_scale: TimeScale, time_resolution: TimeResolution) -> u64 {
        let delay = match self {
            VValue::Real(value) => {
                return f64_delay_to_ticks(value.as_f64(), time_scale, time_resolution);
            }

            // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) 9.7.1
            //
            // """
            // If the delay expression evaluates to an unknown or high-impedance value, it shall be
            // interpreted as zero delay. If the delay expression evaluates to a negative value, it shall
            // be interpreted as a two's-complement unsigned integer of the same size as a time variable.
            // """
            //
            // This is handled by the `unwrap_or(0)` guards below.
            VValue::SignedNet(bits) if bits.select_bit(bits.size().get() - 1) => {
                if bits.size() <= TIME_VSIZE {
                    bits.as_signed_i64().unwrap_or(0) as u64
                } else {
                    u64::MAX
                }
            }
            VValue::SignedNet(bits) | VValue::UnsignedNet(bits)
                if bits.size() <= TIME_VSIZE
                    || bits.contains_special()
                    || bits.size().get() - bits.leading_zeroes() <= TIME_VSIZE.get() =>
            {
                bits.extract_least_significant_u64().unwrap_or(0)
            }

            // Overflow saturates to the maximum.
            VValue::SignedNet(_) | VValue::UnsignedNet(_) => u64::MAX,
        };
        u64_delay_to_ticks(delay, time_scale, time_resolution)
    }
}

/// Convert a delay variable of type `delay_ty` to simulation ticks.
pub fn typed_var_delay_to_ticks<'a>(
    mctx: &mut MutLowerContext<'a>,
    builder: &mut BasicBlockBuilder,
    delay: VariableKey,
    delay_ty: VType,
    time_scale: TimeScale,
    time_resolution: TimeResolution,
) -> VariableKey {
    if delay_ty.is_real() {
        return var_real_delay_to_ticks(mctx, builder, delay, time_scale, time_resolution);
    }

    // Negative delays become two's-complement unsigned integers of the size of a time variable.
    let delay = truncate_or_extend(mctx.gl(), builder, delay, delay_ty, TIME_VSIZE);
    var_int_delay_to_ticks(mctx, builder, delay, time_scale, time_resolution)
}

/// Convert an integer delay to simulation ticks, saturating at the largest time.
pub fn u64_delay_to_ticks(
    delay: u64,
    time_scale: TimeScale,
    time_resolution: TimeResolution,
) -> u64 {
    let ticks_per_unit = time_scale.unit.truncate_or_multiply_to(1, time_resolution);
    delay.saturating_mul(ticks_per_unit)
}

/// Convert a real delay to simulation ticks, rounded to the time precision.
pub fn f64_delay_to_ticks(
    delay: f64,
    time_scale: TimeScale,
    time_resolution: TimeResolution,
) -> u64 {
    time_scale
        .unit
        .real_to_ticks(delay, time_scale.precision, time_resolution)
        .unwrap_or(u64::MAX)
}

/// Convert an integer delay variable of `TIME_VSIZE` bits to simulation ticks.
pub fn var_int_delay_to_ticks<'a>(
    mctx: &mut MutLowerContext<'a>,
    builder: &mut BasicBlockBuilder,
    delay: VariableKey,
    time_scale: TimeScale,
    time_resolution: TimeResolution,
) -> VariableKey {
    let ticks_per_unit = time_scale.unit.truncate_or_multiply_to(1, time_resolution);
    let delay = builder.convert_mode(mctx.gl(), delay, LogicMode::TwoValue);
    builder.multiply_constant(mctx.gl(), delay, Bits::from_u64(TIME_VSIZE, ticks_per_unit))
}

/// Convert a real delay variable to simulation ticks, rounded to the time precision.
pub fn var_real_delay_to_ticks<'a>(
    mctx: &mut MutLowerContext<'a>,
    builder: &mut BasicBlockBuilder,
    delay: VariableKey,
    time_scale: TimeScale,
    time_resolution: TimeResolution,
) -> VariableKey {
    let (unit_to_prec, prec_to_reso) = time_scale
        .unit
        .real_to_ticks_factors(time_scale.precision, time_resolution);

    // @TODO: This ignores NaNs, Infinity and negative numbers; these should all go to zero.

    let unit_to_prec = builder.constant_u64(mctx.gl(), unit_to_prec.to_bits());
    let delay = builder.real_mul(mctx.gl(), delay, unit_to_prec);

    // Rounds to the nearest integer and saturates.
    let delay = builder.real_to_u64(mctx.gl(), delay);
    builder.multiply_constant(mctx.gl(), delay, Bits::from_u64(TIME_VSIZE, prec_to_reso))
}

/// One of the rise, fall and turn-off delays of a gate or net assignment.
#[derive(Clone, Copy)]
pub(crate) enum DelayExpr<'a> {
    Value(AstId<'a, DelayValue>),
    MinTypMax(AstId<'a, MinTypMaxExpression<'a>>),
}

/// Evaluates `exprs`, the rise, fall and turn-off delays of a gate or net assignment in `scope`,
/// to ticks in the delays of the driver resolution, where a `min:typ:max` becomes a triple.
///
/// Returns `None`, and leaves the delays of the driver resolution as they are, if an expression is
/// not constant. Such a delay is evaluated by the process of the delay instead.
pub(crate) fn try_push_const_delays<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    exprs: &[DelayExpr<'a>],
) -> Result<Option<DelayPtr>, ()> {
    let variant = match exprs.len() {
        1 => DelayPtrVariant::One,
        2 => DelayPtrVariant::Two,
        3 => DelayPtrVariant::Three,
        n => unreachable!("{n} gate or net assignment delays"),
    };

    let offset = mctx.driver_resolution.delays.len();
    let mut triple_mask = 0u16;
    for (i, expr) in exprs.iter().enumerate() {
        let is_pushed = match *expr {
            // A delay value is a number or a parameter, so it is always constant.
            DelayExpr::Value(value) => {
                let ticks = evaluate_delay_value_as_ticks(ctx, mctx, scope, value)?;
                mctx.driver_resolution.delays.push(Time(ticks));
                true
            }
            DelayExpr::MinTypMax(min_typ_max) => match min_typ_max.min_max {
                None => try_push_const_delay(ctx, mctx, scope, min_typ_max.typical),
                Some((min, max)) => {
                    triple_mask |= 1 << i;
                    try_push_const_delay(ctx, mctx, scope, min)
                        && try_push_const_delay(ctx, mctx, scope, min_typ_max.typical)
                        && try_push_const_delay(ctx, mctx, scope, max)
                }
            },
        };
        if !is_pushed {
            mctx.driver_resolution.delays.truncate(offset);
            return Ok(None);
        }
    }
    Ok(Some(DelayPtr::new(offset as u64, variant, triple_mask)))
}

/// Pushes the ticks of the delay `expr` in `scope` to the delays of the driver resolution, if it
/// is constant.
fn try_push_const_delay<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
) -> bool {
    // The expression is allowed not to be constant, so its errors are not reported.
    let mut diagnostics = Diagnostics::default();
    let Ok(value) = eval_constant_expr(
        &mctx.gl,
        ctx.arenas,
        &ctx.table,
        scope,
        &mut diagnostics,
        expr.into_constant(),
        Some(TIME_VSIZE),
    ) else {
        return false;
    };
    let ticks = value.delay_to_ticks(ctx.time_scale, ctx.time_resolution);
    mctx.driver_resolution.delays.push(Time(ticks));
    true
}
