//! Linear quantization of a numeric array onto a narrower unsigned width.

use std::cmp::Ordering;
use std::fmt;
use std::num::NonZeroU32;

use vogls::{Bits, VectorSize};

use crate::array::{Array, UIntArray, UIntElement};
use crate::buffer::Buffer;
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::map::Map;
use crate::run_vector::RunVector;
use crate::typing::{ArrayType, DataType, UIntType};
use crate::value::Value;

/// Linearly map an array between two bounds onto `output_dtype`:
///
/// ```text
/// quantize(x) = round((clamp(x, lb, ub) - lb) / (ub - lb) * uXX::MAX)
/// ```
#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Quantize {
    pub lower: Option<Value>,
    pub upper: Option<Value>,
    pub output_dtype: UIntType,
}

#[derive(Default)]
pub struct QuantizeScratch {
    resolved: Option<Bounds>,
}

/// Resolved bounds, in the numeric domain of the input.
#[derive(Clone)]
enum Bounds {
    /// `UInt(_)`, `Int`, and `Bits` up to 64 bits.
    Int { lo: i128, hi: i128 },
    /// `Float` inputs.
    Float { lo: f64, hi: f64 },
    /// `Bits` wider than 64 bits, which no primitive can hold.
    WideBits { lo: Bits, hi: Bits },
}

#[derive(Clone, Copy)]
struct IntKernel {
    lo: i128,
    hi: i128,
    /// `hi - lo`, known to be non-zero: the caller handles `den == 0` separately.
    den: u128,
    max: u128,
}

impl IntKernel {
    /// Round-half-up
    /// - `x == lo` gives `0`
    /// - `x == hi` gives `max`
    #[inline(always)]
    fn apply(&self, x: i128) -> u64 {
        let offset = (x.clamp(self.lo, self.hi) - self.lo) as u128;
        ((offset * self.max + self.den / 2) / self.den) as u64
    }
}

#[derive(Clone, Copy)]
struct FloatKernel {
    lo: f64,
    hi: f64,
    /// `hi - lo`, non-zero and not NaN: the caller handles both separately.
    den: f64,
    max: f64,
    max_int: u64,
}

impl FloatKernel {
    /// `NaN` maps to `0`, the same answer an unknown bit-vector element gets.
    #[inline(always)]
    fn apply(&self, x: f64) -> u64 {
        if x.is_nan() {
            return 0;
        }
        let scaled = (x.clamp(self.lo, self.hi) - self.lo) / self.den * self.max;
        // A float-to-int cast saturates, but `max` can round *up* past the true
        // maximum (`u64::MAX` is not representable), so clamp explicitly too.
        (scaled.round() as u64).min(self.max_int)
    }
}

/// An element's unsigned value, or `None` if any of its bits is `x` or `z`.
fn bits_to_u64(bits: &Bits) -> Option<u64> {
    debug_assert!(bits.size().get() <= 64);
    if let Some(value) = bits.as_u64() {
        return Some(value);
    }
    if bits.contains_special() {
        return None;
    }
    // Known, but too wide for the inline representation `as_u64` covers.
    Some(
        bits.value_iter()
            .enumerate()
            .filter(|(_, v)| *v == vogls::bits::arithmetic::FvLogicValue::L1)
            .fold(0u64, |acc, (i, _)| acc | (1 << i)),
    )
}

struct WideBitsKernel {
    /// At the input's own stride.
    lo: Bits,
    hi: Bits,
    /// At [`WideKernel::width`].
    den: Bits,
    half_den: Bits,
    max: Bits,
    width: VectorSize,
}

impl WideBitsKernel {
    fn new(lo: Bits, hi: Bits, max: u64) -> ComputeResult<Self> {
        let stride = lo.size().get();
        let width = stride
            .checked_add(64)
            .and_then(VectorSize::new)
            .ok_or_else(|| {
                ComputeError::InvalidUsage(format!(
                    "quantize cannot widen a {stride}-bit vector far enough for its \
                     intermediate product"
                ))
            })?;

        let den = Bits::subtract(&hi, &lo).zero_extend(width);
        // `den >> 1`, as a slice rather than a division.
        let half_den = den
            .slicex(1, VectorSize::new(width.get() - 1).unwrap())
            .zero_extend(width);
        let max = Bits::from_u64(VectorSize::new(64).unwrap(), max).zero_extend(width);

        Ok(Self {
            lo,
            hi,
            den,
            half_den,
            max,
            width,
        })
    }

    /// `None` for an element containing `x`/`z`, which has no numeric value.
    fn apply(&self, x: &Bits) -> Option<u64> {
        if x.contains_special() {
            return None;
        }

        // `x` is known, so neither comparison is ever undetermined.
        let clamped = if x.unsigned_cmp(&self.lo) == Some(Ordering::Less) {
            &self.lo
        } else if x.unsigned_cmp(&self.hi) == Some(Ordering::Greater) {
            &self.hi
        } else {
            x
        };

        let offset = Bits::subtract(clamped, &self.lo).zero_extend(self.width);
        let numerator = Bits::add(&Bits::multiply(&offset, &self.max), &self.half_den);
        // `den` is non-zero: the caller handles `hi == lo` separately.
        let quotient = Bits::divide_0(&numerator, &self.den);

        // The quotient is at most `max`, so its low 64 bits hold the whole answer.
        bits_to_u64(&quotient.truncate(VectorSize::new(64).unwrap()))
    }
}

fn quantize_floats<O: UIntElement>(data: &[f64], kernel: &FloatKernel) -> UIntArray {
    O::into_uint_array(
        data.iter()
            .map(|&v| O::from_u64(kernel.apply(v)))
            .collect::<Buffer<O>>(),
    )
}

enum BitsKernel<'a> {
    /// Elements up to 64 bits.
    Narrow(&'a IntKernel),
    Wide(&'a WideBitsKernel),
}

fn quantize_bits<O: UIntElement>(
    bits: &Bits,
    stride: NonZeroU32,
    len: usize,
    kernel: BitsKernel<'_>,
) -> UIntArray {
    O::into_uint_array(
        (0..len)
            .map(|i| {
                let item = bits.slicex(i as u32 * stride.get(), stride);
                let value = match kernel {
                    BitsKernel::Narrow(k) => bits_to_u64(&item).map(|v| k.apply(v as i128)),
                    BitsKernel::Wide(k) => k.apply(&item),
                };
                O::from_u64(value.unwrap_or(0))
            })
            .collect::<Buffer<O>>(),
    )
}

fn quantize_uints<O: UIntElement>(data: &UIntArray, kernel: &IntKernel) -> UIntArray {
    with_match_uint_type!(data.ty(), |$T| {
        O::into_uint_array(
            data.as_slice::<$T>()
                .iter()
                .map(|&v| O::from_u64(kernel.apply(v.to_u64() as i128)))
                .collect::<Buffer<O>>(),
        )
    })
}

fn quantize_ints<O: UIntElement>(data: &[i64], kernel: &IntKernel) -> UIntArray {
    O::into_uint_array(
        data.iter()
            .map(|&v| O::from_u64(kernel.apply(v as i128)))
            .collect::<Buffer<O>>(),
    )
}

fn int_bound(value: &Value) -> ComputeResult<i128> {
    match value {
        Value::UInt(v) => Ok(v.to_u64() as i128),
        Value::Int(v) => Ok(*v as i128),
        Value::Bits(b) => bits_to_u64(b).map(|v| v as i128).ok_or_else(|| {
            ComputeError::InvalidUsage(
                "quantize bound is a bit-vector containing x or z".to_string(),
            )
        }),
        Value::Float(_) => Err(ComputeError::InvalidTypes),
    }
}

fn bits_bound(value: &Value) -> ComputeResult<Bits> {
    match value {
        Value::Bits(b) if b.contains_special() => Err(ComputeError::InvalidUsage(
            "quantize bound is a bit-vector containing x or z".to_string(),
        )),
        Value::Bits(b) => Ok(b.clone()),
        Value::UInt(_) | Value::Int(_) | Value::Float(_) => Err(ComputeError::InvalidTypes),
    }
}

fn float_bound(value: &Value) -> ComputeResult<f64> {
    match value {
        Value::Float(v) if v.is_nan() => Err(ComputeError::InvalidUsage(
            "quantize bound is NaN".to_string(),
        )),
        Value::Float(v) => Ok(*v),
        Value::UInt(_) | Value::Int(_) | Value::Bits(_) => Err(ComputeError::InvalidTypes),
    }
}

impl Quantize {
    fn resolve(&self, data: &Array) -> ComputeResult<Bounds> {
        let bounds = match data {
            Array::Floats(_) => {
                let (lo, hi) = self.resolve_with(data, float_bound, 0.0)?;
                Bounds::Float { lo, hi }
            }
            // Wider than any primitive, so the bounds stay as `Bits`.
            Array::Bits(_, stride) if stride.get() > 64 => {
                // `from_u64` is capped at 64 bits; this stride is wider.
                let zero = Bits::new_zeroed(*stride);
                let (lo, hi) = self.resolve_with(data, bits_bound, zero)?;
                Bounds::WideBits { lo, hi }
            }
            Array::UInts(_) | Array::Ints(_) | Array::Bits(..) => {
                let (lo, hi) = self.resolve_with(data, int_bound, 0)?;
                Bounds::Int { lo, hi }
            }
        };

        let inverted = match &bounds {
            Bounds::Int { lo, hi } => {
                (hi < lo).then(|| format!("({hi}) is below lowerbound ({lo})"))
            }
            Bounds::Float { lo, hi } => {
                (hi < lo).then(|| format!("({hi}) is below lowerbound ({lo})"))
            }
            Bounds::WideBits { lo, hi } => (hi.unsigned_cmp(lo) == Some(Ordering::Less))
                .then(|| "is below lowerbound".to_string()),
        };
        if let Some(detail) = inverted {
            return Err(ComputeError::InvalidUsage(format!(
                "quantize upperbound {detail}"
            )));
        }
        Ok(bounds)
    }

    fn resolve_with<T: Clone>(
        &self,
        data: &Array,
        convert: fn(&Value) -> ComputeResult<T>,
        empty: T,
    ) -> ComputeResult<(T, T)> {
        let lower = self.lower.as_ref().map(convert).transpose()?;
        let upper = self.upper.as_ref().map(convert).transpose()?;

        // Both given: no scan at all.
        if let (Some(lo), Some(hi)) = (&lower, &upper) {
            return Ok((lo.clone(), hi.clone()));
        }

        let Some((data_lo, data_hi)) = data.minmax() else {
            // Nothing comparable in the data, so the bounds cannot be derived;
            // for an empty array the output is empty regardless.
            return Ok((empty.clone(), empty));
        };

        Ok((
            match lower {
                Some(lo) => lo,
                None => convert(&data_lo)?,
            },
            match upper {
                Some(hi) => hi,
                None => convert(&data_hi)?,
            },
        ))
    }
}

impl Map for Quantize {
    type Inputs<Input>
        = [Input; 1]
    where
        Input: Send + Sync;

    type Scratches = QuantizeScratch;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bound = |b: &Option<Value>| match b {
            Some(v) => format!("{v:?}"),
            None => "data".to_string(),
        };
        write!(
            f,
            "Quantize {{ dtype: {:?}, lower: {}, upper: {} }}",
            self.output_dtype,
            bound(&self.lower),
            bound(&self.upper)
        )
    }

    fn output_type(&self, inputs: Self::Inputs<ArrayType>) -> ComputeResult<ArrayType> {
        let [input] = inputs;

        for bound in [&self.lower, &self.upper] {
            if let Some(bound) = bound
                && bound.data_type() != input.data
            {
                return Err(ComputeError::InvalidTypes);
            }
        }

        Ok(ArrayType {
            data: DataType::UInt(self.output_dtype),
            length: input.length,
        })
    }

    fn prepare_scratches_array(
        &self,
        inputs: &Self::Inputs<Array>,
        _ctx: &ComputeContext,
        scratch: &mut Self::Scratches,
    ) -> ComputeResult<()> {
        let [input] = inputs;
        scratch.resolved = Some(self.resolve(input)?);
        Ok(())
    }

    fn prepare_scratches_run_vector(
        &self,
        inputs: &Self::Inputs<RunVector>,
        _ctx: &ComputeContext,
        scratch: &mut Self::Scratches,
    ) -> ComputeResult<()> {
        let [input] = inputs;
        scratch.resolved = Some(self.resolve(&input.data)?);
        Ok(())
    }

    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        scratch: &mut Self::Scratches,
    ) -> ComputeResult<Array> {
        let [input] = inputs;
        let Some(bounds) = scratch.resolved.as_ref() else {
            return Err(ComputeError::InvalidUsage(
                "quantize bounds were never resolved; prepare_scratches did not run".to_string(),
            ));
        };

        let degenerate = match bounds {
            Bounds::Int { lo, hi } => hi == lo,
            Bounds::Float { lo, hi } => hi == lo,
            Bounds::WideBits { lo, hi } => hi.unsigned_cmp(lo) == Some(Ordering::Equal),
        };
        if degenerate {
            // `ub == lb` so all values are quantized to the same value.
            return Ok(Array::UInts(UIntArray::zeros(
                self.output_dtype,
                input.len(),
            )));
        }

        let data = match bounds {
            Bounds::Int { lo, hi } => {
                let kernel = IntKernel {
                    lo: *lo,
                    hi: *hi,
                    den: (hi - lo) as u128,
                    max: self.output_dtype.max_value() as u128,
                };
                with_match_uint_type!(self.output_dtype, |$O| match &input {
                    Array::UInts(d) => quantize_uints::<$O>(d, &kernel),
                    Array::Ints(d) => quantize_ints::<$O>(d, &kernel),
                    Array::Bits(bits, stride) => quantize_bits::<$O>(
                        bits,
                        *stride,
                        input.len(),
                        BitsKernel::Narrow(&kernel)
                    ),
                    Array::Floats(_) => return Err(ComputeError::InvalidTypes),
                })
            }
            Bounds::Float { lo, hi } => {
                let max = self.output_dtype.max_value();
                let kernel = FloatKernel {
                    lo: *lo,
                    hi: *hi,
                    den: hi - lo,
                    max: max as f64,
                    max_int: max,
                };
                with_match_uint_type!(self.output_dtype, |$O| match &input {
                    Array::Floats(d) => quantize_floats::<$O>(d, &kernel),
                    Array::UInts(_) | Array::Ints(_) | Array::Bits(..) => {
                        return Err(ComputeError::InvalidTypes);
                    }
                })
            }
            Bounds::WideBits { lo, hi } => {
                let kernel =
                    WideBitsKernel::new(lo.clone(), hi.clone(), self.output_dtype.max_value())?;
                with_match_uint_type!(self.output_dtype, |$O| match &input {
                    Array::Bits(bits, stride) => quantize_bits::<$O>(
                        bits,
                        *stride,
                        input.len(),
                        BitsKernel::Wide(&kernel)
                    ),
                    Array::Floats(_) | Array::UInts(_) | Array::Ints(_) => {
                        return Err(ComputeError::InvalidTypes);
                    }
                })
            }
        };

        Ok(Array::UInts(data))
    }
}
