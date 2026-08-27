use std::fmt::{self, Write};
use std::ops::{Index, IndexMut};

use vogls_codegen::HeapOffset;
use vogls_ir::{LogicMode, VectorSize};

/// The register bank used by the bytecode interpreter for temporary results.
pub struct Regs {
    value: [u64; 16],
    pub stack_offset: u64,
}
impl Regs {
    #[inline(always)]
    pub fn new(stack_offset: u64) -> Self {
        Self {
            value: [0u64; 16],
            stack_offset,
        }
    }
    #[inline(always)]
    pub fn get_as_addr(&self, reg: Reg) -> HeapOffset {
        HeapOffset {
            bit_offset: self[reg] as usize,
        }
    }
}

/// Bytecode register
///
/// Points to a slot in the [`Regs`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reg {
    #[default]
    X0,
    X1,
    X2,
    X3,
    X4,
    X5,
    X6,
    X7,
    X8,
    X9,
    X10,
    X11,
    X12,
    X13,
    X14,
    X15,
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('x')?;
        (*self as u32).fmt(f)
    }
}
impl Index<Reg> for Regs {
    type Output = u64;

    fn index(&self, index: Reg) -> &Self::Output {
        &self.value[index as usize]
    }
}
impl IndexMut<Reg> for Regs {
    fn index_mut(&mut self, index: Reg) -> &mut Self::Output {
        &mut self.value[index as usize]
    }
}

impl Reg {
    #[inline(always)]
    pub fn new_masked(v: u32) -> Self {
        match v & 0xF {
            0 => Self::X0,
            1 => Self::X1,
            2 => Self::X2,
            3 => Self::X3,
            4 => Self::X4,
            5 => Self::X5,
            6 => Self::X6,
            7 => Self::X7,
            8 => Self::X8,
            9 => Self::X9,
            10 => Self::X10,
            11 => Self::X11,
            12 => Self::X12,
            13 => Self::X13,
            14 => Self::X14,
            15 => Self::X15,
            _ => unreachable!(),
        }
    }

    /// Get the two registers used to store Four-Value Logic.
    ///
    /// This splits the value into the _Special_ (`spc`) and the _Value_ (`val`).
    ///
    /// |           | special=0 | special=1 |
    /// | value = 0 |         x |         0 |
    /// | value = 1 |         z |         1 |
    #[inline(always)]
    pub fn to_spc_and_val(self) -> (Self, Self) {
        debug_assert_ne!(self, Self::X15);
        (self, Self::new_masked(self as u32 + 1))
    }
}

#[derive(Debug)]
pub struct RegInfo {
    name: &'static str,
    reg: Reg,
    mode: LogicMode,
    /// The size of the operand in the register, if it is known.
    size: Option<VectorSize>,

    /// Whether the register is stored in the register or whether it is stored on the heap.
    /// - If it is stored in a register, the size must be <= 64.
    /// - If it is stored on the heap, the register contains an address into the heap.
    storage: RegStorage,
}

#[derive(Debug, Clone, Copy)]
pub enum RegStorage {
    Register,
    Heap,
}

impl RegInfo {
    #[inline]
    pub fn new(
        name: &'static str,
        reg: Reg,
        mode: LogicMode,
        size: Option<VectorSize>,
        storage: RegStorage,
    ) -> Self {
        Self {
            name,
            reg,
            mode,
            size,
            storage,
        }
    }

    #[inline]
    pub fn register(
        name: &'static str,
        reg: Reg,
        mode: LogicMode,
        size: Option<VectorSize>,
    ) -> Self {
        Self::new(name, reg, mode, size, RegStorage::Register)
    }

    #[inline]
    pub fn heap(name: &'static str, reg: Reg, mode: LogicMode, size: VectorSize) -> Self {
        Self::new(name, reg, mode, Some(size), RegStorage::Heap)
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    pub fn reg(&self) -> Reg {
        self.reg
    }

    #[inline]
    pub fn mode(&self) -> LogicMode {
        self.mode
    }

    #[inline]
    pub fn size(&self) -> Option<VectorSize> {
        self.size
    }

    #[inline]
    pub fn storage(&self) -> RegStorage {
        self.storage
    }
}
