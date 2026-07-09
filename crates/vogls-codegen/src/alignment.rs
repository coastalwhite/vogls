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
    pub fn new(size: VectorSize, mode: LogicMode) -> Self {
        let mut num_bits = size.get();
        match mode {
            LogicMode::TwoValue => {}
            LogicMode::FourValue => num_bits = num_bits.strict_mul(2),
        }
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

    pub fn is_aligned(self, value: u64) -> bool {
        value.unbounded_shl(64 - self as u32) == 0
    }

    pub fn from_elem_offset(self, elem: u64) -> u64 {
        debug_assert_eq!(elem >> (64 - self as u32), 0);
        elem << self as u32
    }

    pub fn next_aligned(self, value: u64) -> u64 {
        value.next_multiple_of(1u64 << self as u32)
    }

    pub fn spc_offset_to_val_offset(size: VectorSize, spc_offset: u64) -> u64 {
        if size.get() >= 32 {
            spc_offset + (size.get() as u64).next_multiple_of(64)
        } else {
            spc_offset + size.get() as u64
        }
    }
}
