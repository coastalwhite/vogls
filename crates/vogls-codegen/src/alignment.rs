use vogls_ir::{LogicMode, VectorSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeapAlignment {
    B1,
    B2,
    B4,
    B8,
    B16,
    B32,
    B64,
}

impl HeapAlignment {
    #[inline(always)]
    pub fn new(size: VectorSize, mode: LogicMode) -> Self {
        debug_assert!(mode.is_two_value() || size.get().checked_mul(2).is_some());
        let shift = match mode {
            LogicMode::TwoValue => 0,
            LogicMode::FourValue => 1,
        };
        let num_bits = size.get().min(64) << shift;
        match num_bits.min(64).next_power_of_two().trailing_zeros() {
            0 => Self::B1,
            1 => Self::B2,
            2 => Self::B4,
            3 => Self::B8,
            4 => Self::B16,
            5 => Self::B32,
            6 => Self::B64,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    pub fn is_aligned(self, value: u64) -> bool {
        value.unbounded_shl(64 - self as u32) == 0
    }

    #[inline(always)]
    pub fn from_elem_offset(self, elem: u64) -> u64 {
        debug_assert_eq!(elem >> (64 - self as u32), 0);
        elem << self as u32
    }

    #[inline(always)]
    pub fn to_elem_offset(self, elem: u64) -> u64 {
        debug_assert!(self.is_aligned(elem));
        elem >> self as u32
    }

    #[inline(always)]
    pub fn next_aligned(self, value: u64) -> u64 {
        value.next_multiple_of(1u64 << self as u32)
    }

    #[inline(always)]
    pub fn spc_offset_to_val_offset(size: VectorSize, spc_offset: u64) -> u64 {
        if size.get() > 32 {
            spc_offset + (size.get() as u64).next_multiple_of(64)
        } else {
            spc_offset + size.get() as u64
        }
    }
}
