use std::fmt;
use std::num::NonZeroU16;

use vogls_bits::slice::tv_ll_slice;
use vogls_codegen::{HeapAlignment, HeapOffset, SixBitSize};
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use crate::write_padded_mnemonic;

use super::reg::{Reg, RegInfo, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineAddrOffset, InlineNBitSize, Schedule,
};

pub struct TvLoadRelAligned(pub LoadRelArgs);
pub struct FvLoadAligned(pub LoadRelArgs);
pub struct LoadHeapAligned {
    pub rd: Reg,
    pub rs: Reg,
    pub num_words: Option<NonZeroU16>,
}
pub struct LoadHeapUnaligned {
    pub rd: Reg,
    pub rs: Reg,
    pub imm8: InlineAddrOffset<8>,
    pub size: InlineNBitSize<8>,
}

pub struct TvLoadAligned(pub LoadArgs);

pub struct LoadUnaligned(pub LoadArgs);
pub struct LoadRelUnaligned(pub LoadRelArgs);

pub struct LoadRelArgs {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,
    imm10: InlineAddrOffset<10>,
}
pub struct LoadArgs {
    rd: Reg,
    size: SixBitSize,
    imm14: u16,
}

impl LoadRelArgs {
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
                | (self.size.encode() << 16)
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

impl LoadArgs {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            size: SixBitSize::new_masked(v >> 12),
            imm14: (v >> 18) as u16,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | (self.size.encode() << 12)
                | ((self.imm14 as u32) << 18),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, size, imm14 } = self;
        write!(f, "{rd}, {imm14}, |{size}|")
    }
}

macro_rules! impl_load_rel_args {
    ($variant:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(LoadRelArgs::extract(v))
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

impl BytecodeInstruction for TvLoadRelAligned {
    impl_load_rel_args!(TvLoadRelAligned, "tv.load_rel_aligned");

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::heap(
            "rs",
            self.0.rs,
            LogicMode::TwoValue,
            self.0.size.into(),
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(LoadRelArgs {
            rd,
            rs,
            imm10,
            size,
        }) = self;

        let offset = imm10.get(regs[rs]);
        debug_assert!(HeapAlignment::new(size.into(), LogicMode::TwoValue).is_aligned(offset));

        let word = offset / 64;
        let boff = offset % 64;

        let heap = &state.heap.0;
        regs[rd] = size.mask(heap[word as usize] >> boff);
    }
}

impl BytecodeInstruction for TvLoadAligned {
    impl_load_args!(TvLoadAligned, "tv.load_aligned");

    fn num_additional_slots(&self) -> u8 {
        1
    }

    #[inline(always)]
    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(LoadArgs { rd, size, imm14 }) = self;

        let code_offset = code[*pc as usize].0;
        *pc += 1;
        let offset = ((code_offset as u64) << 14) | (imm14 as u64);
        debug_assert!(HeapAlignment::new(size.into(), LogicMode::TwoValue).is_aligned(offset));

        let word = offset / 64;
        let boff = offset % 64;

        let heap = &state.heap.0;
        regs[rd] = size.mask(heap[word as usize] >> boff);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
    }
}

impl BytecodeInstruction for FvLoadAligned {
    impl_load_rel_args!(FvLoadAligned, "fv.load_aligned");

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(LoadRelArgs {
            rd,
            rs,
            imm10,
            size,
        }) = self;

        let alignment = HeapAlignment::new(size.into(), LogicMode::FourValue);
        let spc_offset = imm10.get(regs[rs]);
        debug_assert!(alignment.is_aligned(spc_offset));
        let val_offset = HeapAlignment::spc_offset_to_val_offset(size.into(), spc_offset);

        let heap = &state.heap.0;
        let mask = size.mask(u64::MAX);

        let spc_word = spc_offset / 64;
        let spc_boff = spc_offset % 64;
        let val_word = val_offset / 64;
        let val_boff = val_offset % 64;

        let (spc, val) = rd.to_spc_and_val();
        regs[spc] = mask & (heap[spc_word as usize] >> spc_boff);
        regs[val] = mask & (heap[val_word as usize] >> val_boff);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::heap(
            "rs",
            self.0.rs,
            LogicMode::FourValue,
            self.0.size.into(),
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::FourValue,
            Some(self.0.size.into()),
        ));
    }
}

impl BytecodeInstruction for LoadRelUnaligned {
    impl_load_rel_args!(LoadRelUnaligned, "load_rel_unaligned");

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(LoadRelArgs {
            rd,
            rs,
            imm10,
            size,
        }) = self;

        let offset = imm10.get(regs[rs]);
        let end_offset = offset + size as u64 - 1;

        let word = (offset / 64) as usize;
        let boff = offset % 64;
        let endword = (end_offset / 64) as usize;

        let heap = &state.heap.0;
        if word == endword {
            regs[rd] = size.mask(heap[word] >> boff);
            return;
        }

        assert!(heap.is_empty() && word < heap.len() - 1);
        let w1 = heap[word];
        let w2 = heap[word + 1];
        let w = (w1 >> boff) | (w2 << (64 - boff));
        regs[rd] = size.mask(w);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::heap(
            "rs",
            self.0.rs,
            LogicMode::TwoValue,
            self.0.size.into(),
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
    }
}

impl BytecodeInstruction for LoadUnaligned {
    impl_load_args!(LoadUnaligned, "load_unaligned");

    fn num_additional_slots(&self) -> u8 {
        1
    }

    #[inline(always)]
    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(LoadArgs { rd, imm14, size }) = self;

        let code_offset = code[*pc as usize].0;
        *pc += 1;
        let offset = ((code_offset as u64) << 14) | (imm14 as u64);
        let end_offset = offset + size as u64 - 1;

        let word = (offset / 64) as usize;
        let boff = offset % 64;
        let endword = (end_offset / 64) as usize;

        let heap = &state.heap.0;
        if word == endword {
            regs[rd] = size.mask(heap[word] >> boff);
            return;
        }

        assert!(!heap.is_empty() && word < heap.len() - 1);
        let w1 = heap[word];
        let w2 = heap[word + 1];
        let w = (w1 >> boff) | (w2 << (64 - boff));
        regs[rd] = size.mask(w);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
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
            num_words: NonZeroU16::new((v >> 16) as u16),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::LoadHeapAligned as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.num_words.map_or(0, |v| (v.get() as u32) << 16)),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, num_words } = self;
        write_padded_mnemonic(f, "load_heap_aligned")?;
        write!(f, "{rd}, {rs}, ")?;
        match num_words {
            Some(n) => fmt::Display::fmt(n, f),
            None => f.write_str("..."),
        }
    }

    fn num_additional_slots(&self) -> u8 {
        match self.num_words {
            Some(_) => 0,
            None => 1,
        }
    }

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let num_words = match self.num_words {
            Some(n) => n.get() as u32,
            None => code[pc as usize + 1].0,
        };
        operands.push(RegInfo::heap(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            VectorSize::new(num_words * 64).unwrap(),
        ));
    }
    fn dest_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let num_words = match self.num_words {
            Some(n) => n.get() as u32,
            None => code[pc as usize + 1].0,
        };
        operands.push(RegInfo::heap(
            "rd",
            self.rd,
            LogicMode::TwoValue,
            VectorSize::new(num_words * 64).unwrap(),
        ));
    }

    #[inline(always)]
    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd, rs, num_words } = self;
        let dst_offset = regs[rd];
        let src_offset = regs[rs];

        debug_assert!(HeapAlignment::B64.is_aligned(dst_offset));
        debug_assert!(HeapAlignment::B64.is_aligned(src_offset));

        let num_words = match num_words {
            None => {
                let num_words = code[*pc as usize].0;
                *pc += 1;
                num_words
            }
            Some(n) => n.get() as u32,
        };

        let num_words = num_words as usize;
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

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc + 1;
        let size = self.size.get(&mut pc, code);
        operands.push(RegInfo::heap("rs", self.rs, LogicMode::TwoValue, size));
    }
    fn dest_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc + 1;
        let size = self.size.get(&mut pc, code);
        operands.push(RegInfo::heap("rd", self.rd, LogicMode::TwoValue, size));
    }

    #[inline(always)]
    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rd, rs, imm8, size } = self;
        let dst_offset = regs[rd];
        let src_offset = imm8.get(regs[rs]);

        debug_assert!(HeapAlignment::B64.is_aligned(dst_offset));

        let size = size.get(pc, code);
        let dst_num_words = size.get().div_ceil(64) as usize;
        let src_start = src_offset - src_offset % 64;
        let src_end = (src_offset + size.get() as u64).next_multiple_of(64);
        let src_num_words = ((src_end - src_start) / 64) as usize;

        let src = state.heap.get_u64_slice(
            HeapOffset {
                bit_offset: src_offset as usize,
            },
            src_num_words,
        );

        cldctx.heap_scratch.clear();
        cldctx.heap_scratch.resize(dst_num_words, 0u64);

        tv_ll_slice(
            &mut cldctx.heap_scratch,
            src,
            (src_offset % 64) as u32,
            size,
            VectorSize::new((src_num_words * 64) as u32).unwrap(),
            false,
        );
        state
            .heap
            .get_mut_u64_slice(
                HeapOffset {
                    bit_offset: dst_offset as usize,
                },
                dst_num_words,
            )
            .copy_from_slice(&cldctx.heap_scratch);
    }
}

impl BytecodeEncoder {
    pub fn tv_load_aligned(&mut self, rd: Reg, at: u64, size: SixBitSize) {
        if at < (1u64 << (14 + 32)) {
            let imm14 = (at & 0x3FFF) as u16;
            let imm46_15 = (at >> 14) as u32;
            self.data
                .push(TvLoadAligned(LoadArgs { rd, size, imm14 }).encode());
            self.data.push(Bytecode(imm46_15));
            return;
        }

        self.load_u64(rd, at);
        self.tv_load_rel_aligned(rd, rd, InlineAddrOffset::ZERO, size);
    }
    pub fn tv_load_rel_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        imm10: InlineAddrOffset<10>,
        size: SixBitSize,
    ) {
        self.data.push(
            TvLoadRelAligned(LoadRelArgs {
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
            FvLoadAligned(LoadRelArgs {
                rd,
                rs,
                size,
                imm10,
            })
            .encode(),
        )
    }
    pub fn load_heap_aligned(&mut self, rd: Reg, rs: Reg, num_words: u32) {
        match u16::try_from(num_words).ok().and_then(NonZeroU16::new) {
            Some(num_words) => self.data.push(
                LoadHeapAligned {
                    rd,
                    rs,
                    num_words: Some(num_words),
                }
                .encode(),
            ),
            None => {
                self.data.push(
                    LoadHeapAligned {
                        rd,
                        rs,
                        num_words: None,
                    }
                    .encode(),
                );
                self.data.push(Bytecode(num_words));
            }
        }
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

    pub fn load_unaligned(&mut self, rd: Reg, at: u64, size: SixBitSize) {
        if at < (1u64 << (14 + 32)) {
            let imm14 = (at & 0x3FFF) as u16;
            let imm46_15 = (at >> 14) as u32;
            self.data
                .push(LoadUnaligned(LoadArgs { rd, size, imm14 }).encode());
            self.data.push(Bytecode(imm46_15));
            return;
        }

        self.load_u64(rd, at);
        self.load_rel_unaligned(rd, rd, InlineAddrOffset::ZERO, size);
    }
    pub fn load_rel_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        imm10: InlineAddrOffset<10>,
        size: SixBitSize,
    ) {
        self.data.push(
            LoadRelUnaligned(LoadRelArgs {
                rd,
                rs,
                size,
                imm10,
            })
            .encode(),
        )
    }
}
