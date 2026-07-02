use std::fmt;

use vogls_codegen::HeapOffset;
use vogls_ir::VectorSize;
use vogls_runtime::RuntimeState;

use crate::bytecode::write_padded_mnemonic;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule, SixBitSize,
};

pub struct LoadArgs {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,
    imm10: i16,
}

impl LoadArgs {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: ((v as i32) >> 22) as i16,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.size.0 as u32) << 16)
                | ((self.imm10 as u16 as u32) << 22),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            size,
            imm10,
        } = self;
        write!(f, "{rd}, {rs}, {imm10}, |{size}|")
    }
}

pub struct TvLoadAligned(pub LoadArgs);
pub struct FvLoadAligned(pub LoadArgs);
pub struct LoadHeapAligned {
    pub rd: Reg,
    pub rs: Reg,
    pub num_words: u16,
}

macro_rules! impl_load_args {
    ($variant:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(LoadArgs::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}

impl BytecodeInstruction for TvLoadAligned {
    impl_load_args!(TvLoadAligned, "tv.load_aligned");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(LoadArgs {
            rd,
            rs,
            imm10,
            size,
        }) = self;
        // @Performance: We can likely make a better specialized implementation for this load.
        let size = VectorSize::from(size);
        let factor = i64::from(size.get().next_power_of_two().min(64));
        let offset = i64::from(imm10) * factor;
        let offset = regs[rs].wrapping_add_signed(i64::from(offset));
        let at = HeapOffset {
            bit_offset: offset as usize,
        };
        let at = at.to_ref(size);
        regs[rd] = state.heap.get_tv_u64(at);
    }
}

impl BytecodeInstruction for FvLoadAligned {
    impl_load_args!(FvLoadAligned, "fv.load_aligned");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(LoadArgs {
            rd,
            rs,
            imm10,
            size,
        }) = self;
        // @Performance: We can likely make a better specialized implementation for this load.
        let size = VectorSize::from(size);
        let factor = i64::from((2 * size.get()).next_power_of_two().min(64));
        let offset = i64::from(imm10) * factor;
        let offset = regs[rs].wrapping_add_signed(i64::from(offset));
        let at = HeapOffset {
            bit_offset: offset as usize,
        };
        let at = at.to_ref(size);
        let (spc, val) = rd.to_spc_and_val();
        (regs[spc], regs[val]) = state.heap.get_fv_u64(at);
    }
}

impl BytecodeInstruction for LoadHeapAligned {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::LoadHeapAligned as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            num_words: (v >> 16) as u16,
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::LoadHeapAligned as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.num_words as u32) << 16),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, num_words } = self;
        write_padded_mnemonic(f, "load_heap_aligned")?;
        write!(f, "{rd}, {rs}, {num_words}")
    }

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd, rs, num_words } = self;
        let dst_offset = regs[rd];
        let src_offset = regs[rs];
        let num_words = usize::from(num_words);
        let [dst, src] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: dst_offset as usize,
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
        for (d, s) in dst.iter().zip(src) {
            d.set(s.get());
        }
    }
}

impl BytecodeEncoder {
    pub fn tv_load_aligned(&mut self, rd: Reg, rs: Reg, imm10: i16, size: SixBitSize) {
        self.data.push(
            TvLoadAligned(LoadArgs {
                rd,
                rs,
                size,
                imm10,
            })
            .encode(),
        )
    }
    pub fn fv_load_aligned(&mut self, rd: Reg, rs: Reg, imm10: i16, size: SixBitSize) {
        self.data.push(
            FvLoadAligned(LoadArgs {
                rd,
                rs,
                size,
                imm10,
            })
            .encode(),
        )
    }
    pub fn load_heap_aligned(&mut self, rd: Reg, rs: Reg, num_words: u16) {
        self.data
            .push(LoadHeapAligned { rd, rs, num_words }.encode())
    }
}
