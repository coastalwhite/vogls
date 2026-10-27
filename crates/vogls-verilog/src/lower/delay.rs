//! Conversion of delays in time units of a module to simulation ticks.
//!
//! Rules for reals:
//! - NaNs, infinities and negative numbers become 0.
//!
//! Rules for integers:
//! - Signed integers sign extend to i64 and are reinterpreted as u64.
//! - Integers containing `x` and `z` become zero.

use vogls_ir::time::TimeResolution;
use vogls_ir::{BasicBlockBuilder, Bits, LogicMode, TIME_VSIZE, VariableKey};

use crate::ast::module::TimeScale;
use crate::lower::expression::truncate_or_extend;
use crate::lower::{MutLowerContext, VType, VValue};

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
pub fn typed_var_delay_to_ticks(
    mctx: &mut MutLowerContext,
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
pub fn var_int_delay_to_ticks(
    mctx: &mut MutLowerContext,
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
pub fn var_real_delay_to_ticks(
    mctx: &mut MutLowerContext,
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
