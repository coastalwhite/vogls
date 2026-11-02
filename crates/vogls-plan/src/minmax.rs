//! `min` / `max` / `minmax` over buffers and arrays.

use std::cmp::Ordering;
use std::num::NonZeroU32;

use vogls::Bits;

use crate::array::{Array, UIntArray, UIntElement};
use crate::buffer::Buffer;
use crate::value::{UIntValue, Value};

/// The smallest and largest item of `items`, or `None` if it is empty.
pub fn minmax<T: Copy + PartialOrd>(items: impl Iterator<Item = T>) -> Option<(T, T)> {
    let mut items = items;
    let first = items.next()?;
    Some(items.fold((first, first), |(lo, hi), v| {
        (if v < lo { v } else { lo }, if v > hi { v } else { hi })
    }))
}

fn fold_min<T: Copy + PartialOrd>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let first = items.next()?;
    Some(items.fold(first, |lo, v| if v < lo { v } else { lo }))
}

fn fold_max<T: Copy + PartialOrd>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let first = items.next()?;
    Some(items.fold(first, |hi, v| if v > hi { v } else { hi }))
}

impl<T: Copy + PartialOrd> Buffer<T> {
    pub fn min(&self) -> Option<T> {
        fold_min(self.as_slice().iter().copied())
    }

    pub fn max(&self) -> Option<T> {
        fold_max(self.as_slice().iter().copied())
    }

    /// Both extremes in a single pass.
    pub fn minmax(&self) -> Option<(T, T)> {
        minmax(self.as_slice().iter().copied())
    }
}

impl UIntArray {
    pub fn min(&self) -> Option<UIntValue> {
        with_match_uint_type!(self.ty(), |$T| {
            fold_min(self.as_slice::<$T>().iter().copied()).map(UIntElement::into_uint_value)
        })
    }

    pub fn max(&self) -> Option<UIntValue> {
        with_match_uint_type!(self.ty(), |$T| {
            fold_max(self.as_slice::<$T>().iter().copied()).map(UIntElement::into_uint_value)
        })
    }

    /// Both extremes in a single pass, at the array's own width.
    pub fn minmax(&self) -> Option<(UIntValue, UIntValue)> {
        with_match_uint_type!(self.ty(), |$T| {
            minmax(self.as_slice::<$T>().iter().copied())
                .map(|(lo, hi)| (lo.into_uint_value(), hi.into_uint_value()))
        })
    }
}

/// The extremes of a packed bit-vector array, compared as unsigned magnitudes.
///
/// Elements containing `x` or `z` are skipped: a value that is not fully known
/// has no place in a min/max, and skipping them keeps the result fully known, so
/// callers can convert it to a number. `None` when no element qualifies.
fn bits_extremes(bits: &Bits, stride: NonZeroU32, len: usize) -> Option<(Bits, Bits)> {
    let mut extremes: Option<(Bits, Bits)> = None;

    for i in 0..len {
        let item = bits.slicex(i as u32 * stride.get(), stride);
        if item.contains_special() {
            continue;
        }

        extremes = Some(match extremes {
            None => (item.clone(), item),
            Some((lo, hi)) => {
                // Both operands are fully known, so neither comparison is ever
                // undetermined; `None` is treated as "keep what we have".
                let lo = match item.unsigned_cmp(&lo) {
                    Some(Ordering::Less) => item.clone(),
                    _ => lo,
                };
                let hi = match item.unsigned_cmp(&hi) {
                    Some(Ordering::Greater) => item,
                    _ => hi,
                };
                (lo, hi)
            }
        });
    }

    extremes
}

impl Array {
    /// The smallest value, or `None` when the array is empty.
    pub fn min(&self) -> Option<Value> {
        match self {
            Array::Floats(items) => items.min().map(Value::Float),
            Array::Ints(items) => items.min().map(Value::Int),
            Array::UInts(items) => items.min().map(Value::UInt),
            // Delegating costs one extra comparison per element, which is not
            // worth a second hand-rolled fold for a type this cold.
            Array::Bits(..) => self.minmax().map(|(lo, _hi)| lo),
        }
    }

    /// The largest value, or `None` when the array is empty.
    pub fn max(&self) -> Option<Value> {
        match self {
            Array::Floats(items) => items.max().map(Value::Float),
            Array::Ints(items) => items.max().map(Value::Int),
            Array::UInts(items) => items.max().map(Value::UInt),
            // Delegating costs one extra comparison per element, which is not
            // worth a second hand-rolled fold for a type this cold.
            Array::Bits(..) => self.minmax().map(|(_lo, hi)| hi),
        }
    }

    /// Both extremes in a single pass.
    pub fn minmax(&self) -> Option<(Value, Value)> {
        match self {
            Array::Floats(items) => items
                .minmax()
                .map(|(lo, hi)| (Value::Float(lo), Value::Float(hi))),
            Array::Ints(items) => items
                .minmax()
                .map(|(lo, hi)| (Value::Int(lo), Value::Int(hi))),
            Array::UInts(items) => items
                .minmax()
                .map(|(lo, hi)| (Value::UInt(lo), Value::UInt(hi))),
            Array::Bits(bits, stride) => bits_extremes(bits, *stride, self.len())
                .map(|(lo, hi)| (Value::Bits(lo), Value::Bits(hi))),
        }
    }
}
