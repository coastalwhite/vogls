use std::fmt;

use vogls_ir::VectorSize;

/// A size between 1 - 64 which can be represented by six bits.
///
/// This is generally used to represent the six of an operand within a register.
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

    pub fn encode(self) -> u32 {
        self as u32 - 1
    }

    /// Create a value from six bits.
    ///
    /// Note that logical 0 corresponds to size 1.
    #[inline(always)]
    pub fn new_masked(v: u32) -> Self {
        match (v & 0x3F) + 1 {
            1 => Self::N1,
            2 => Self::N2,
            3 => Self::N3,
            4 => Self::N4,
            5 => Self::N5,
            6 => Self::N6,
            7 => Self::N7,
            8 => Self::N8,
            9 => Self::N9,
            10 => Self::N10,
            11 => Self::N11,
            12 => Self::N12,
            13 => Self::N13,
            14 => Self::N14,
            15 => Self::N15,
            16 => Self::N16,
            17 => Self::N17,
            18 => Self::N18,
            19 => Self::N19,
            20 => Self::N20,
            21 => Self::N21,
            22 => Self::N22,
            23 => Self::N23,
            24 => Self::N24,
            25 => Self::N25,
            26 => Self::N26,
            27 => Self::N27,
            28 => Self::N28,
            29 => Self::N29,
            30 => Self::N30,
            31 => Self::N31,
            32 => Self::N32,
            33 => Self::N33,
            34 => Self::N34,
            35 => Self::N35,
            36 => Self::N36,
            37 => Self::N37,
            38 => Self::N38,
            39 => Self::N39,
            40 => Self::N40,
            41 => Self::N41,
            42 => Self::N42,
            43 => Self::N43,
            44 => Self::N44,
            45 => Self::N45,
            46 => Self::N46,
            47 => Self::N47,
            48 => Self::N48,
            49 => Self::N49,
            50 => Self::N50,
            51 => Self::N51,
            52 => Self::N52,
            53 => Self::N53,
            54 => Self::N54,
            55 => Self::N55,
            56 => Self::N56,
            57 => Self::N57,
            58 => Self::N58,
            59 => Self::N59,
            60 => Self::N60,
            61 => Self::N61,
            62 => Self::N62,
            63 => Self::N63,
            64 => Self::N64,
            _ => unreachable!(),
        }
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
