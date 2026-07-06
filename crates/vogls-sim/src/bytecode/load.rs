use std::fmt;

use vogls_bits::slice::tv_cell_slice;
use vogls_codegen::HeapOffset;
use vogls_ir::VectorSize;
use vogls_runtime::RuntimeState;

use crate::bytecode::write_padded_mnemonic;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineAddrOffset, InlineNBitSize, Schedule, SixBitSize,
};

pub struct LoadArgs {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,
    imm10: InlineAddrOffset<10>,
}

impl LoadArgs {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: InlineAddrOffset::new_shifted(v, 22),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.size.0 as u32) << 16)
                | (self.imm10.encode() << 22),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            size,
            imm10,
        } = self;
        let imm10 = imm10.0;
        write!(f, "{rd}, {rs}, {imm10}, |{size}|")
    }
}

pub struct TvLoadAligned(pub LoadArgs);
pub struct FvLoadAligned(pub LoadArgs);
pub struct LoadUnaligned(pub LoadArgs);
pub struct LoadHeapAligned {
    pub rd: Reg,
    pub rs: Reg,
    pub num_words: u16,
}
pub struct LoadHeapUnaligned {
    pub rd: Reg,
    pub rs: Reg,
    pub imm8: InlineAddrOffset<8>,
    pub size: InlineNBitSize<8>,
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
        let offset = imm10.get_tv_aligned(regs[rs], size);
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
        let offset = imm10.get_fv_aligned(regs[rs], size);
        let at = HeapOffset {
            bit_offset: offset as usize,
        };
        let at = at.to_ref(size);
        let (spc, val) = rd.to_spc_and_val();
        (regs[spc], regs[val]) = state.heap.get_fv_u64(at);
    }
}
impl BytecodeInstruction for LoadUnaligned {
    impl_load_args!(LoadUnaligned, "load_unaligned");

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

        let offset = imm10.get(regs[rs]);
        let w = state.heap.load_unaligned_u64(offset);
        let w = size.mask(w);
        regs[rd] = w;
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

impl BytecodeInstruction for LoadHeapUnaligned {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::LoadHeapUnaligned as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: InlineNBitSize::new_masked(v >> 16),
            imm8: InlineAddrOffset::new_shifted(v, 24),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::LoadHeapUnaligned as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.size.encode() << 16)
                | (self.imm8.encode() << 24),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            size: _,
            imm8,
        } = self;
        write_padded_mnemonic(f, "load_heap_unaligned")?;
        let imm8 = imm8.0;
        write!(f, "{rd}, {rs}, {imm8}")
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
        let Self { rd, rs, imm8, size } = self;
        let dst_offset = regs[rd];
        let src_offset = imm8.get(regs[rs]);

        let size = size.get(regs);
        let dst_num_words = size.get().div_ceil(64) as usize;
        let src_start = src_offset - src_offset % 64;
        let src_end = (src_offset + size.get() as u64).next_multiple_of(64);
        let src_num_words = ((src_end - src_start) / 64) as usize;

        let [dst, src] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: dst_offset as usize,
                },
                dst_num_words,
            ),
            (
                HeapOffset {
                    bit_offset: src_offset as usize,
                },
                src_num_words,
            ),
        ]);

        tv_cell_slice(
            dst,
            src,
            (src_offset % 64) as u32,
            size,
            VectorSize::new((src_num_words * 64) as u32).unwrap(),
            false,
        );
    }
}

impl BytecodeEncoder {
    pub fn tv_load_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        imm10: InlineAddrOffset<10>,
        size: SixBitSize,
    ) {
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
    pub fn fv_load_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        imm10: InlineAddrOffset<10>,
        size: SixBitSize,
    ) {
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
    pub fn load_heap_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        imm8: InlineAddrOffset<8>,
        size: InlineNBitSize<8>,
    ) {
        self.data
            .push(LoadHeapUnaligned { rd, rs, imm8, size }.encode())
    }
    pub fn load_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        imm10: InlineAddrOffset<10>,
        size: SixBitSize,
    ) {
        self.data.push(
            LoadUnaligned(LoadArgs {
                rd,
                rs,
                size,
                imm10,
            })
            .encode(),
        )
    }
}
