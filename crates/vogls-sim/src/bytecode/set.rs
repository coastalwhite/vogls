use std::fmt;

use vogls_codegen::HeapOffset;
use vogls_ir::VectorSize;
use vogls_runtime::RuntimeState;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    MNEMONIC_ALIGN, Schedule, SixBitSize,
};

pub struct SetArgs {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: SixBitSize,
    imm6: i8,
}
pub struct SetHeapArgs {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: Option<VectorSize>,
}

impl SetArgs {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
            imm6: ((v as i32) >> 26) as i8,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | ((self.size.0 as u32) << 20)
                | ((self.imm6 as u16 as u32) << 26),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            imm6,
            size,
        } = self;
        write!(f, "{rd}, {rs}, {roff}, {imm6}, |{size}|")
    }
}
impl SetHeapArgs {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            size: VectorSize::new(v >> 20),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.size.map_or(0, |v| v.get()) << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            size: _,
        } = self;
        write!(f, "{rd}, {rs}, {roff}")
    }
}

pub struct TvSetAligned(pub SetArgs);
pub struct FvSetAligned(pub SetArgs);
pub struct TvSetHeapAligned(pub SetHeapArgs);
pub struct FvSetHeapAligned(pub SetHeapArgs);

macro_rules! impl_set_args {
    ($variant:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(SetArgs::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:<1$}", $mnemonic, MNEMONIC_ALIGN)?;
            self.0.fmt(f)
        }
    };
}

macro_rules! impl_set_heap_args {
    ($variant:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(SetHeapArgs::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:<1$}", $mnemonic, MNEMONIC_ALIGN)?;
            self.0.fmt(f)
        }
    };
}

impl BytecodeInstruction for TvSetAligned {
    impl_set_args!(TvSetAligned, "tv.set_aligned");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(SetArgs {
            rd,
            rs,
            roff,
            size,
            imm6,
        }) = self;
        // @Performance: We can likely make a better specialized implementation for this load.
        let size = VectorSize::from(size);
        let factor = i64::from(size.get().next_power_of_two());
        let offset = i64::from(imm6) * factor;
        let offset = regs[roff].wrapping_add_signed(i64::from(offset));
        let at = HeapOffset {
            bit_offset: offset as usize,
        };
        let at = at.to_ref(size);
        let value = regs[rs];
        let prev_value = state.heap.set_tv_u64(at, value);
        let updated = value != prev_value;
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for FvSetAligned {
    impl_set_args!(FvSetAligned, "fv.set_aligned");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(SetArgs {
            rd,
            rs,
            roff,
            size,
            imm6,
        }) = self;
        // @Performance: We can likely make a better specialized implementation for this load.
        let size = VectorSize::from(size);
        let factor = i64::from((size.get() * 2).next_power_of_two().min(64));
        let offset = i64::from(imm6) * factor;
        let offset = regs[roff].wrapping_add_signed(i64::from(offset));
        let at = HeapOffset {
            bit_offset: offset as usize,
        };
        let at = at.to_ref(size);
        let (spc, val) = rs.to_spc_and_val();
        let spc = regs[spc];
        let val = regs[val];
        let (prev_spc, prev_val) = state.heap.set_fv_u64(at, spc, val);
        let updated = (prev_spc != spc) | (prev_val != val);
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for TvSetHeapAligned {
    impl_set_heap_args!(TvSetHeapAligned, "tv.set_heap_aligned");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(SetHeapArgs { rd, rs, roff, size }) = self;
        let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
        let roff_offset = regs[roff];
        let src_offset = regs[rs];
        let num_words = size.get().div_ceil(64) as usize;
        let [roff, src] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: roff_offset as usize,
                },
                num_words,
            ),
            (
                HeapOffset {
                    bit_offset: src_offset as usize,
                },
                num_words,
            ),
        ]);
        let mut updated = false;
        for (d, s) in roff.iter().zip(src) {
            let value = s.get();
            let prev_value = d.replace(value);
            updated |= value != prev_value;
        }
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for FvSetHeapAligned {
    impl_set_heap_args!(FvSetHeapAligned, "fv.set_heap_aligned");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(SetHeapArgs { rd, rs, roff, size }) = self;
        let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
        let roff_offset = regs[roff];
        let src_offset = regs[rs];
        let num_words = size.get().div_ceil(64) as usize * 2;
        let [roff, src] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: roff_offset as usize,
                },
                num_words,
            ),
            (
                HeapOffset {
                    bit_offset: src_offset as usize,
                },
                num_words,
            ),
        ]);
        let mut updated = false;
        for (d, s) in roff.iter().zip(src) {
            let value = s.get();
            let prev_value = d.replace(value);
            updated |= value != prev_value;
        }
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeEncoder {
    pub fn tv_set_aligned(&mut self, rd: Reg, rs: Reg, roff: Reg, imm6: i8, size: SixBitSize) {
        self.data.push(
            TvSetAligned(SetArgs {
                rd,
                rs,
                roff,
                size,
                imm6,
            })
            .encode(),
        )
    }
    pub fn fv_set_aligned(&mut self, rd: Reg, rs: Reg, roff: Reg, imm6: i8, size: SixBitSize) {
        self.data.push(
            FvSetAligned(SetArgs {
                rd,
                rs,
                roff,
                size,
                imm6,
            })
            .encode(),
        )
    }
    pub fn tv_set_heap_aligned(&mut self, rd: Reg, rs: Reg, roff: Reg, size: Option<VectorSize>) {
        self.data
            .push(TvSetHeapAligned(SetHeapArgs { rd, rs, roff, size }).encode())
    }
    pub fn fv_set_heap_aligned(&mut self, rd: Reg, rs: Reg, roff: Reg, size: Option<VectorSize>) {
        self.data
            .push(FvSetHeapAligned(SetHeapArgs { rd, rs, roff, size }).encode())
    }
}
