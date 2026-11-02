use std::cmp::Ordering;

use crate::VectorSize;
use crate::arithmetic::{FvLogicValue, fv_contains_special, fv_unpack_u64};
use crate::load::load_partial_u64;
use crate::select::tv_gtu64_select_bit;
use crate::util::last_word_mask;

pub fn tv_unsigned_leq(lhs: &[u8], rhs: &[u8], size: VectorSize) -> bool {
    for i in (0..size.get().div_ceil(8) as usize).rev() {
        let value = match lhs[i].cmp(&rhs[i]) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => continue,
        };
        return value;
    }
    true
}

pub fn tv_gtu64_unsigned_leq(lhs: &[u64], rhs: &[u64], size: VectorSize) -> bool {
    for i in (0..size.get().div_ceil(64) as usize).rev() {
        let value = match lhs[i].cmp(&rhs[i]) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => continue,
        };
        return value;
    }
    true
}
pub fn tv_gtu64_signed_leq(lhs: &[u64], rhs: &[u64], size: VectorSize) -> bool {
    let lhs_neg = tv_gtu64_select_bit(lhs, size.get() - 1, size);
    let rhs_neg = tv_gtu64_select_bit(rhs, size.get() - 1, size);

    // If the signs differ, the negative sign is the smaller one.
    if lhs_neg != rhs_neg {
        return lhs_neg;
    }

    tv_gtu64_unsigned_leq(lhs, rhs, size)
}

pub fn fv_l_unsigned_leq(lhs: &[u64], rhs: &[u64], size: VectorSize) -> FvLogicValue {
    if fv_contains_special(lhs, size) || fv_contains_special(rhs, size) {
        return FvLogicValue::X;
    }

    let nwords = lhs.len() / 2;
    for i in (0..nwords).rev() {
        let value = match lhs[nwords + i].cmp(&rhs[nwords + i]) {
            Ordering::Less => FvLogicValue::L1,
            Ordering::Greater => FvLogicValue::L0,
            Ordering::Equal => continue,
        };
        return value;
    }
    FvLogicValue::L1
}
pub fn fv_s_unsigned_leq(lhs: &[u8], rhs: &[u8], size: VectorSize) -> FvLogicValue {
    let dsize = VectorSize::new(2 * size.get()).unwrap();
    let lhs = load_partial_u64(lhs, dsize);
    let rhs = load_partial_u64(rhs, dsize);

    let (lspc, lval) = fv_unpack_u64(lhs, size);
    let (rspc, rval) = fv_unpack_u64(rhs, size);

    let mask = (1u64 << size.get()) - 1;
    if lspc != mask || rspc != mask {
        return FvLogicValue::X;
    }

    FvLogicValue::from_bool(lval <= rval)
}

pub fn bitwise_ceq(lspc: u64, lval: u64, rspc: u64, rval: u64) -> u64 {
    (lspc ^ !rspc) & (lval ^ !rval)
}

/// The index of the most significant set bit of a non-zero word.
#[inline(always)]
fn msb(word: u64) -> u32 {
    debug_assert!(word != 0);
    63 - word.leading_zeros()
}

impl crate::Bits {
    /// The most significant bit.
    fn sign_bit(&self) -> FvLogicValue {
        self.select_value(self.size().get() - 1)
    }

    /// Compare two operands of the same width, word by word from the top.
    fn cmp_same_size(&self, other: &Self) -> Option<Ordering> {
        debug_assert_eq!(self.size(), other.size());

        // The inline representations are borrowed out of the `BitsDataRef`, so it
        // has to outlive the slices.
        let ldata = self.as_data_ref();
        let rdata = other.as_data_ref();
        let (lval, lspc) = ldata.to_u64_slices();
        let (rval, rspc) = rdata.to_u64_slices();

        let nwords = lval.len();
        debug_assert_eq!(nwords, rval.len());

        for i in (0..nwords).rev() {
            let valid = if i + 1 == nwords {
                last_word_mask(self.size())
            } else {
                u64::MAX
            };

            // A bit is usable only where both operands know it; a `None` plane is
            // two-value, and so known throughout. Bits past the vector's size are
            // not normalised in `spc`, which is why every word is masked.
            let known = lspc.map_or(u64::MAX, |p| p[i]) & rspc.map_or(u64::MAX, |p| p[i]) & valid;
            let unknown = valid & !known;
            // A difference at an unknown position says nothing, so `diff` and
            // `unknown` end up disjoint.
            let diff = (lval[i] ^ rval[i]) & known;

            if diff == 0 && unknown == 0 {
                continue;
            }

            // Whichever of the two comes first scanning down from the top decides:
            // a difference settles the ordering, an unknown destroys it.
            if diff != 0 && (unknown == 0 || msb(diff) > msb(unknown)) {
                // `diff` only carries bits both operands know and disagree on, so
                // reading one side is enough. Going through `select_value` keeps
                // the plane layout out of this function.
                let at = i as u32 * 64 + msb(diff);
                return Some(if self.select_value(at) == FvLogicValue::L1 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                });
            }
            return None;
        }

        Some(Ordering::Equal)
    }

    /// Compare two bit vectors as unsigned magnitudes.
    pub fn unsigned_cmp(&self, other: &Self) -> Option<Ordering> {
        // Equal widths are the common case; otherwise pay for one extension.
        match self.size().cmp(&other.size()) {
            Ordering::Equal => self.cmp_same_size(other),
            Ordering::Less => self.zero_extend(other.size()).cmp_same_size(other),
            Ordering::Greater => self.cmp_same_size(&other.zero_extend(self.size())),
        }
    }

    /// Compare two bit vectors as two's-complement signed values.
    pub fn signed_cmp(&self, other: &Self) -> Option<Ordering> {
        let lhs_neg = self.sign_bit();
        let rhs_neg = other.sign_bit();

        if lhs_neg.is_special() | rhs_neg.is_special() {
            return None;
        }

        if lhs_neg != rhs_neg {
            // Exactly one is negative, and that is the smaller one.
            return Some(if lhs_neg == FvLogicValue::L1 {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }

        // Same sign, so sign-extension leaves the padded top bits equal and the
        // scan falls through to the magnitudes.
        match self.size().cmp(&other.size()) {
            Ordering::Equal => self.cmp_same_size(other),
            Ordering::Less => self.sign_extend(other.size()).cmp_same_size(other),
            Ordering::Greater => self.cmp_same_size(&other.sign_extend(self.size())),
        }
    }
}
