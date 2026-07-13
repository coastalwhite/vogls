use std::fmt;

use vogls_bits::set_subslice::tv_cell_set;
use vogls_codegen::{HeapAlignment, HeapOffset};
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, InlineAddrOffset, InlineNBitSize, Schedule, SixBitSize,
    write_padded_mnemonic, write_register,
};

pub struct SetArgs {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: SixBitSize,
    imm6: InlineAddrOffset<6>,
}
pub struct SetHeapArgs {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: InlineNBitSize<8>,
    imm4: InlineAddrOffset<4>,
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
            imm6: InlineAddrOffset::new_shifted(v, 26),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.size.encode() << 20)
                | (self.imm6.encode() << 26),
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
            size: InlineNBitSize::new_masked(v >> 20),
            imm4: InlineAddrOffset::new_shifted(v, 28),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.size.encode() << 20)
                | (self.imm4.encode() << 28),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            size,
            imm4,
        } = self;
        write!(f, "{rd}, {rs}, {roff}, {imm4}, {size}")
    }
}

pub struct TvSetAligned(pub SetArgs);
pub struct FvSetAligned(pub SetArgs);
pub struct SetUnaligned(pub SetArgs);
pub struct TvSetHeapAligned(pub SetHeapArgs);
pub struct FvSetHeapAligned(pub SetHeapArgs);
pub struct SetHeapUnaligned(pub SetHeapArgs);

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
            write_padded_mnemonic(f, $mnemonic)?;
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
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}

impl BytecodeInstruction for TvSetAligned {
    impl_set_args!(TvSetAligned, "tv.set_aligned");

    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rs", self.0.rs, LogicMode::TwoValue)?;
        f.write_str(", ")?;
        write_register(f, regs, "roff", self.0.roff, LogicMode::TwoValue)?;
        writeln!(f)
    }
    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rd", self.0.rd, LogicMode::TwoValue)?;
        writeln!(f)
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
        let Self(SetArgs {
            rd,
            rs,
            roff,
            size,
            imm6,
        }) = self;
        let offset = imm6.get(regs[roff]);
        debug_assert!(HeapAlignment::new(size.into(), LogicMode::TwoValue).is_aligned(offset));
        let value = regs[rs];
        let mask = size.mask(u64::MAX);
        let word = &mut state.heap.0[(offset / 64) as usize];
        let boff = offset % 64;
        let prev_value = mask & (*word >> boff);
        *word &= !(mask << boff);
        *word |= value << boff;
        let updated = value != prev_value;
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for FvSetAligned {
    impl_set_args!(FvSetAligned, "fv.set_aligned");

    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rs", self.0.rs, LogicMode::FourValue)?;
        f.write_str(", ")?;
        write_register(f, regs, "roff", self.0.roff, LogicMode::TwoValue)?;
        writeln!(f)
    }
    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rd", self.0.rd, LogicMode::TwoValue)?;
        writeln!(f)
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
        let Self(SetArgs {
            rd,
            rs,
            roff,
            size,
            imm6,
        }) = self;
        let spc_offset = imm6.get(regs[roff]);
        debug_assert!(HeapAlignment::new(size.into(), LogicMode::FourValue).is_aligned(spc_offset));
        let val_offset = HeapAlignment::spc_offset_to_val_offset(size.into(), spc_offset);

        let (spc, val) = rs.to_spc_and_val();
        let spc = regs[spc];
        let val = regs[val];

        let mask = size.mask(u64::MAX);

        let spc_boff = spc_offset % 64;
        let heap_spc_word = &mut state.heap.0[(spc_offset / 64) as usize];
        let prev_spc = mask & (*heap_spc_word >> spc_boff);
        *heap_spc_word &= !(mask << spc_boff);
        *heap_spc_word |= spc << spc_boff;

        let val_boff = val_offset % 64;
        let heap_val_word = &mut state.heap.0[(val_offset / 64) as usize];
        let prev_val = mask & (*heap_val_word >> val_boff);
        *heap_val_word &= !(mask << val_boff);
        *heap_val_word |= val << val_boff;

        let updated = (prev_spc != spc) | (prev_val != val);
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for SetUnaligned {
    impl_set_args!(SetUnaligned, "set_unaligned");

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
        let mask = size.mask(u64::MAX);
        let offset = imm6.get(regs[roff]);
        let end_offset = offset + size as u64 - 1;

        let val = regs[rs];
        let word = (offset / 64) as usize;
        let boff = offset % 64;
        let endword = (end_offset / 64) as usize;

        let heap = &mut state.heap.0;
        if word == endword {
            let word = &mut heap[word];
            let prev = mask & (*word >> boff);
            *word &= !(mask << boff);
            *word |= val << boff;
            let updated = prev != val;
            regs[rd] = u64::from(updated);
            return;
        }

        assert!(heap.len() > 0 && word < heap.len() - 1);
        let prev = mask & ((heap[word] >> boff) | (heap[word + 1] << (64 - boff)));
        heap[word] &= !(mask << boff);
        heap[word] |= val << boff;
        heap[word + 1] &= !(mask >> (64 - boff));
        heap[word + 1] |= val >> (64 - boff);
        let updated = prev != val;
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for SetHeapUnaligned {
    impl_set_heap_args!(SetHeapUnaligned, "set_heap_unaligned");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(SetHeapArgs {
            rd,
            rs,
            roff,
            size,
            imm4,
        }) = self;
        let size = size.get(regs);
        let bit_offset = imm4.get(regs[roff]);

        let dst_start_offset = bit_offset - bit_offset % 64;
        let dst_end_offset = (bit_offset + size.get() as u64).next_multiple_of(64);
        let dst_num_words = ((dst_end_offset - dst_start_offset) / 64) as usize;
        let src_num_words = size.get().div_ceil(64) as usize;

        let [dst, src] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: dst_start_offset as usize,
                },
                dst_num_words,
            ),
            (
                HeapOffset {
                    bit_offset: regs[rs] as usize,
                },
                src_num_words,
            ),
        ]);
        // @Incorrect. Note this can write into neighbours. That is probably not what we want.
        let updated = tv_cell_set(
            dst,
            src,
            VectorSize::new((dst_num_words * 64) as u32).unwrap(),
            (bit_offset % 64) as u32,
            size,
        );
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
        let Self(SetHeapArgs {
            rd,
            rs,
            roff,
            size,
            imm4,
        }) = self;
        let size = size.get(regs);
        let roff_offset = imm4.get(regs[roff]);
        let src_offset = regs[rs];

        debug_assert!(HeapAlignment::B64.is_aligned(roff_offset));
        debug_assert!(HeapAlignment::B64.is_aligned(src_offset));

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
        let Self(SetHeapArgs {
            rd,
            rs,
            roff,
            size,
            imm4,
        }) = self;
        let size = size.get(regs);
        let roff_offset = imm4.get(regs[roff]);
        let src_offset = regs[rs];

        debug_assert!(HeapAlignment::B64.is_aligned(roff_offset));
        debug_assert!(HeapAlignment::B64.is_aligned(src_offset));

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
    pub fn tv_set_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        imm6: InlineAddrOffset<6>,
        size: SixBitSize,
    ) {
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
    pub fn fv_set_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        imm6: InlineAddrOffset<6>,
        size: SixBitSize,
    ) {
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
    pub fn set_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        imm6: InlineAddrOffset<6>,
        size: SixBitSize,
    ) {
        self.data.push(
            SetUnaligned(SetArgs {
                rd,
                rs,
                roff,
                size,
                imm6,
            })
            .encode(),
        )
    }
    pub fn tv_set_heap_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        size: InlineNBitSize<8>,
        imm4: InlineAddrOffset<4>,
    ) {
        self.data.push(
            TvSetHeapAligned(SetHeapArgs {
                rd,
                rs,
                roff,
                size,
                imm4,
            })
            .encode(),
        )
    }
    pub fn fv_set_heap_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        size: InlineNBitSize<8>,
        imm4: InlineAddrOffset<4>,
    ) {
        self.data.push(
            FvSetHeapAligned(SetHeapArgs {
                rd,
                rs,
                roff,
                size,
                imm4,
            })
            .encode(),
        )
    }
    pub fn set_heap_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        size: InlineNBitSize<8>,
        imm4: InlineAddrOffset<4>,
    ) {
        self.data.push(
            SetHeapUnaligned(SetHeapArgs {
                rd,
                rs,
                roff,
                size,
                imm4,
            })
            .encode(),
        )
    }
}
