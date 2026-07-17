use std::fmt;

use vogls_ir::VectorSize;

/// A size between 1 - 64 which can be represented by six bits.
///
/// This is generally used to represent the six of an operand within a register.
#[repr(u8)]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SixBitSize {
    // @NOTE: This is an enum to allow to compiler to infer the range of this value clearly.
    #[default]
    N1 = 1,
    N2 = 2,
    N3 = 3,
    N4 = 4,
    N5 = 5,
    N6 = 6,
    N7 = 7,
    N8 = 8,
    N9 = 9,
    N10 = 10,
    N11 = 11,
    N12 = 12,
    N13 = 13,
    N14 = 14,
    N15 = 15,
    N16 = 16,
    N17 = 17,
    N18 = 18,
    N19 = 19,
    N20 = 20,
    N21 = 21,
    N22 = 22,
    N23 = 23,
    N24 = 24,
    N25 = 25,
    N26 = 26,
    N27 = 27,
    N28 = 28,
    N29 = 29,
    N30 = 30,
    N31 = 31,
    N32 = 32,
    N33 = 33,
    N34 = 34,
    N35 = 35,
    N36 = 36,
    N37 = 37,
    N38 = 38,
    N39 = 39,
    N40 = 40,
    N41 = 41,
    N42 = 42,
    N43 = 43,
    N44 = 44,
    N45 = 45,
    N46 = 46,
    N47 = 47,
    N48 = 48,
    N49 = 49,
    N50 = 50,
    N51 = 51,
    N52 = 52,
    N53 = 53,
    N54 = 54,
    N55 = 55,
    N56 = 56,
    N57 = 57,
    N58 = 58,
    N59 = 59,
    N60 = 60,
    N61 = 61,
    N62 = 62,
    N63 = 63,
    N64 = 64,
}

impl From<SixBitSize> for VectorSize {
    #[inline(always)]
    fn from(value: SixBitSize) -> Self {
        VectorSize::new(value as u32).unwrap()
    }
}

impl SixBitSize {
    #[inline(always)]
    pub fn from_vector_size(size: VectorSize) -> Option<Self> {
        if size.get() > 64 {
            return None;
        }

        Some(Self::new_masked(size.get() - 1))
    }

    #[inline(always)]
    pub fn encode(self) -> u32 {
        self as u32 - 1
    }

    /// Create a value from six bits.
    ///
    /// Note that logical 0 corresponds to size 1.
    #[inline(always)]
    pub fn new_masked(v: u32) -> Self {
        let v = ((v & 0x3F) + 1) as u8;
        debug_assert!((1..=64).contains(&v));
        // @NOTE: I observed this not being optimized, when it was a match statement. Therefore,
        // this unsafe here.
        //
        // SAFETY: SixBitSize is defined between 1 - 64 and that is the only range that `v` can be.
        unsafe { std::mem::transmute::<u8, Self>(v) }
    }

    /// Mask a value to only keep the lower N bits.
    #[inline(always)]
    pub fn mask(self, v: u64) -> u64 {
        // @NOTE: This unbounded_shr generates better code than a normal shift for some reason.
        v & u64::MAX.unbounded_shr(64 - self as u32)
    }
}

impl fmt::Display for SixBitSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&(*self as u32), f)
    }
}
