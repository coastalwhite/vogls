use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};

// @Performance: This is double storing the length. In theory, this should be 3-words instead of 4.
#[derive(Default, Clone)]
pub struct Bitset {
    data: Vec<u64>,
    num_bits: usize,
}

impl PartialEq for Bitset {
    fn eq(&self, other: &Self) -> bool {
        self.num_bits == other.num_bits && self.data == other.data
    }
}
impl Eq for Bitset {}

impl Bitset {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            num_bits: 0,
        }
    }

    pub fn zeroed(num_bits: usize) -> Self {
        Self {
            data: vec![0u64; num_bits.div_ceil(64)],
            num_bits,
        }
    }

    pub fn with_capacity(num_bits: usize) -> Self {
        Self {
            data: Vec::with_capacity(num_bits.div_ceil(64)),
            num_bits: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.num_bits
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn extend_zeroed(&mut self, n: usize) {
        self.data.resize((self.len() + n).div_ceil(64), 0u64);
    }

    pub fn push(&mut self, value: bool) {
        if self.num_bits * 64 == self.data.capacity() {
            self.data.reserve(1);
        }
        let boffset = self.num_bits % 64;
        if boffset == 0 {
            self.data.push(u64::from(value));
        } else {
            *self.data.last_mut().unwrap() |= u64::from(value) << boffset;
        }
    }

    pub fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.num_bits);
        let boffset = index % 64;
        self.data[index / 64] &= !(1u64 << boffset);
        self.data[index / 64] |= u64::from(value) << boffset;
    }

    pub fn get(&self, index: usize) -> bool {
        assert!(index < self.num_bits);
        ((self.data[index / 64] >> (index % 64)) & 1) != 0
    }

    pub fn fill(&mut self, value: bool) {
        if value == false {
            self.data.fill(0u64);
        } else {
            self.data.fill(u64::MAX);
            let boffset = self.num_bits % 64;
            if boffset != 0 {
                *self.data.last_mut().unwrap() &= (1u64 << boffset) - 1;
            }
        }
    }

    pub fn andnot_mut(&mut self, other: &Self) {
        assert_eq!(self.num_bits, other.num_bits);
        for (l, r) in self.data.iter_mut().zip(other.data.iter()) {
            *l &= !*r;
        }
    }

    pub fn true_idx_iter<'a>(&'a self) -> TrueIdxIter<'a> {
        TrueIdxIter {
            bitset: self,
            word: self.data.get(0).copied().unwrap_or_default(),
            word_idx: 0,
        }
    }

    pub fn find_n_contiguous_zeros(&self, n: usize) -> Result<usize, usize> {
        if self.len() == 0 {
            return Err(0);
        }

        // @Performance: Specialized implementation
        let mut prev = 0;
        dbg!(self.len());
        for i in self.true_idx_iter() {
            dbg!(i);
            if i - prev > n {
                return Ok(prev);
            }
            prev = i;
        }
        Err(dbg!(prev + 1))
    }

    pub fn set_slice_constant(&mut self, offset: usize, num_words: usize, value: bool) {
        // @Performance: This should get a specialized method.
        for i in 0..num_words {
            self.set(offset + i, value);
        }
    }
}

pub struct TrueIdxIter<'a> {
    bitset: &'a Bitset,
    word: u64,
    word_idx: usize,
}

impl<'a> Iterator for TrueIdxIter<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.word_idx >= self.bitset.data.len() {
            return None;
        }

        if self.word == 0u64 {
            for (i, w) in self.bitset.data.iter().enumerate().skip(self.word_idx + 1) {
                if *w != 0 {
                    self.word_idx = i;
                    self.word = *w;
                }
            }

            if self.word == 0 {
                self.word_idx = self.bitset.data.len();
                return None;
            }
        }

        let tz = self.word.trailing_zeros();
        self.word ^= 1 << tz;
        Some(self.word_idx * 64 + tz as usize)
    }
}

macro_rules! impl_bitwise_ops {
    (
        |$l:ident, $r:ident|
        $(($op:ident, $opn:ident, $assign_op:ident, $assign_opn:ident, $f:expr)),+
        $(,)?
    ) => {
        $(
        impl $op<&Bitset> for Bitset {
            type Output = Bitset;
            fn $opn(self, rhs: &Bitset) -> Self::Output {
                assert_eq!(self.num_bits, rhs.num_bits);
                let data = self
                    .data
                    .iter()
                    .zip(rhs.data.iter())
                    .map(|($l, $r)| $f)
                    .collect();
                Self {
                    data,
                    num_bits: self.num_bits,
                }
            }
        }
        impl $assign_op<&Bitset> for Bitset {
            fn $assign_opn(&mut self, rhs: &Bitset) {
                assert_eq!(self.num_bits, rhs.num_bits);
                for ($l, $r) in self.data.iter_mut().zip(rhs.data.iter()) {
                    *$l = $f;
                }
            }
        }
        )+
    };
}

impl_bitwise_ops! {
    |l, r|
    (BitOr, bitor, BitOrAssign, bitor_assign, *l | *r),
    (BitAnd, bitand, BitAndAssign, bitand_assign, *l & *r),
    (BitXor, bitxor, BitXorAssign, bitxor_assign, *l | *r),
}
