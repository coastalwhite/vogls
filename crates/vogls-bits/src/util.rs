use std::cell::Cell;
use std::ops::Rem;

use crate::VectorSize;

pub fn saturating_rem<T: Default + Copy + Eq + Rem<T, Output = T>>(a: T, b: T) -> T {
    let rem = a.rem(b);
    if rem == T::default() { b } else { rem }
}

pub fn wrapping_u64_pow(l: u64, r: u64) -> u64 {
    // X**Y
    //      = X**LowerWord(Y) * X**(2**32 * UpperWord(Y))
    //      = X**LowerWord(Y) * (X**UpperWord(Y))**(2**32)
    //      = X**LowerWord(Y) * (X**UpperWord(Y))**(2**16)**2
    let a = l.wrapping_pow((r & 0xFFFF_FFFF) as u32);
    if r < (1 << 32) {
        return a;
    }
    let b = l
        .wrapping_pow((r >> 32) as u32)
        .wrapping_pow(1 << 16)
        .wrapping_pow(1 << 16);
    a.wrapping_mul(b)
}

pub trait CellSlice {
    fn copy_from_slice(&self, other: &Self);
    fn fill(&self, value: u64);
}

impl CellSlice for [Cell<u64>] {
    fn copy_from_slice(&self, other: &Self) {
        assert_eq!(self.len(), other.len());
        self.iter().zip(other).for_each(|(d, s)| d.set(s.get()));
    }
    fn fill(&self, value: u64) {
        self.iter().for_each(|d| d.set(value));
    }
}

pub fn mask_size_1to64(size: u32) -> u64 {
    debug_assert!(size > 0);
    debug_assert!(size <= 64);
    u64::MAX.unbounded_shr(64u32.wrapping_sub(size))
}
pub fn last_word_mask(size: VectorSize) -> u64 {
    mask_size_1to64(saturating_rem(size.get(), 64))
}
pub fn mask_size_0to63(size: u32) -> u64 {
    debug_assert!(size < 64);
    1u64.wrapping_shl(size).wrapping_sub(1)
}
