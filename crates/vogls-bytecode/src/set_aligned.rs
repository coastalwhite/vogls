use std::fmt;

use vogls_codegen::{HeapAlignment, HeapOffset, SixBitSize};
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use super::reg::{Reg, RegInfo, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineAddrOffset, InlineNBitSize, Schedule, write_padded_mnemonic,
};

pub struct SetArgs {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,
    imm10: u16,
}
pub struct SetRelArgs {
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
            size: SixBitSize::new_masked(v >> 16),
            imm10: (v >> 22) as u16,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.size.encode() << 16)
                | ((self.imm10 as u32) << 22),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            imm10,
            size,
        } = self;
        write!(f, "{rd}, {rs}, {imm10}, |{size}|")
    }
}
impl SetRelArgs {
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

pub struct TvRelSetAligned(pub SetRelArgs);
pub struct FvRelSetAligned(pub SetRelArgs);
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
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}

macro_rules! impl_set_rel_args {
    ($variant:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(SetRelArgs::extract(v))
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

#[inline(always)]
fn tv_set_aligned(heap: &mut [u64], offset: u64, value: u64, size: SixBitSize) -> u64 {
    debug_assert!(HeapAlignment::new(size.into(), LogicMode::TwoValue).is_aligned(offset));
    let mask = size.mask(u64::MAX);
    let word = &mut heap[(offset / 64) as usize];
    let boff = offset % 64;
    let prev_value = mask & (*word >> boff);
    *word &= !(mask << boff);
    *word |= value << boff;
    prev_value ^ value
}
#[inline(always)]
fn fv_set_aligned(heap: &mut [u64], offset: u64, spc: u64, val: u64, size: SixBitSize) -> u64 {
    debug_assert!(HeapAlignment::new(size.into(), LogicMode::TwoValue).is_aligned(offset));

    let spc_offset = offset;
    let val_offset = HeapAlignment::spc_offset_to_val_offset(size.into(), spc_offset);

    let mask = size.mask(u64::MAX);

    let spc_boff = offset % 64;
    let heap_spc_word = &mut heap[(spc_offset / 64) as usize];
    let prev_spc = mask & (*heap_spc_word >> spc_boff);
    *heap_spc_word &= !(mask << spc_boff);
    *heap_spc_word |= spc << spc_boff;

    let val_boff = val_offset % 64;
    let heap_val_word = &mut heap[(val_offset / 64) as usize];
    let prev_val = mask & (*heap_val_word >> val_boff);
    *heap_val_word &= !(mask << val_boff);
    *heap_val_word |= val << val_boff;

    (prev_spc ^ spc) | (prev_val ^ val)
}

impl BytecodeInstruction for TvSetAligned {
    impl_set_args!(TvSetAligned, "tv.set_aligned");

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
        ));
    }

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
        let Self(SetArgs {
            rd,
            rs,
            size,
            imm10,
        }) = self;

        let code_offset = code[*pc as usize].0;
        *pc += 1;
        let offset = ((code_offset as u64) << 10) | (imm10 as u64);
        let value = regs[rs];
        let heap = state.heap.0.as_mut();
        let update_mask = tv_set_aligned(heap, offset, value, size);
        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for FvSetAligned {
    impl_set_args!(FvSetAligned, "fv.set_aligned");

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::FourValue,
            Some(self.0.size.into()),
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
        ));
    }

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
        let Self(SetArgs {
            rd,
            rs,
            size,
            imm10,
        }) = self;

        let code_offset = code[*pc as usize].0;
        *pc += 1;
        let offset = ((code_offset as u64) << 10) | (imm10 as u64);

        let (spc, val) = rs.to_spc_and_val();
        let spc = regs[spc];
        let val = regs[val];
        let heap = state.heap.0.as_mut();
        let update_mask = fv_set_aligned(heap, offset, spc, val, size);
        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for TvRelSetAligned {
    impl_set_rel_args!(TvRelSetAligned, "tv.rel_set_aligned");

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
        operands.push(RegInfo::register(
            "roff",
            self.0.roff,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
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
        let Self(SetRelArgs {
            rd,
            rs,
            roff,
            size,
            imm6,
        }) = self;
        let offset = imm6.get(regs[roff]);
        let value = regs[rs];
        let heap = state.heap.0.as_mut();
        let update_mask = tv_set_aligned(heap, offset, value, size);
        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for FvRelSetAligned {
    impl_set_rel_args!(FvRelSetAligned, "fv.rel_set_aligned");

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::FourValue,
            Some(self.0.size.into()),
        ));
        operands.push(RegInfo::register(
            "roff",
            self.0.roff,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
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
        let Self(SetRelArgs {
            rd,
            rs,
            roff,
            size,
            imm6,
        }) = self;
        let offset = imm6.get(regs[roff]);

        let (spc, val) = rs.to_spc_and_val();
        let spc = regs[spc];
        let val = regs[val];

        let heap = state.heap.0.as_mut();
        let updated = fv_set_aligned(heap, offset, spc, val, size);
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for TvSetHeapAligned {
    impl_set_heap_args!(TvSetHeapAligned, "tv.set_heap_aligned");

    fn num_additional_slots(&self) -> u8 {
        u8::from(self.0.size.0.is_none())
    }

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc + 1;
        let size = self.0.size.get(&mut pc, code);
        operands.push(RegInfo::heap("rs", self.0.rs, LogicMode::TwoValue, size));
        operands.push(RegInfo::heap(
            "roff",
            self.0.roff,
            LogicMode::TwoValue,
            size,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
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
        let Self(SetHeapArgs {
            rd,
            rs,
            roff,
            size,
            imm4,
        }) = self;
        let size = size.get(pc, code);
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

    fn num_additional_slots(&self) -> u8 {
        u8::from(self.0.size.0.is_none())
    }

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc + 1;
        let size = self.0.size.get(&mut pc, code);
        operands.push(RegInfo::heap("rs", self.0.rs, LogicMode::FourValue, size));
        operands.push(RegInfo::heap(
            "roff",
            self.0.roff,
            LogicMode::FourValue,
            size,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
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
        let Self(SetHeapArgs {
            rd,
            rs,
            roff,
            size,
            imm4,
        }) = self;
        let size = size.get(pc, code);
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
    pub fn tv_set_aligned(&mut self, rd: Reg, rs: Reg, at: u64, size: SixBitSize) {
        if at < (1u64 << (10 + 32)) {
            let imm10 = (at & 0x3FF) as u16;
            let imm42_11 = (at >> 10) as u32;
            self.data.push(
                TvSetAligned(SetArgs {
                    rd,
                    rs,
                    size,
                    imm10,
                })
                .encode(),
            );
            self.data.push(Bytecode(imm42_11));
            return;
        }

        self.load_u64(rd, at);
        self.tv_rel_set_aligned(rd, rs, rd, InlineAddrOffset::ZERO, size);
    }
    pub fn fv_set_aligned(&mut self, rd: Reg, rs: Reg, at: u64, size: SixBitSize) {
        if at < (1u64 << (10 + 32)) {
            let imm10 = (at & 0x3FF) as u16;
            let imm42_11 = (at >> 10) as u32;
            self.data.push(
                FvSetAligned(SetArgs {
                    rd,
                    rs,
                    size,
                    imm10,
                })
                .encode(),
            );
            self.data.push(Bytecode(imm42_11));
            return;
        }

        self.load_u64(rd, at);
        self.fv_rel_set_aligned(rd, rs, rd, InlineAddrOffset::ZERO, size);
    }

    pub fn tv_rel_set_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        imm6: InlineAddrOffset<6>,
        size: SixBitSize,
    ) {
        self.data.push(
            TvRelSetAligned(SetRelArgs {
                rd,
                rs,
                roff,
                size,
                imm6,
            })
            .encode(),
        )
    }
    pub fn fv_rel_set_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        imm6: InlineAddrOffset<6>,
        size: SixBitSize,
    ) {
        self.data.push(
            FvRelSetAligned(SetRelArgs {
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
        size: VectorSize,
        imm4: InlineAddrOffset<4>,
    ) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            TvSetHeapAligned(SetHeapArgs {
                rd,
                rs,
                roff,
                size: inline_size,
                imm4,
            })
            .encode(),
        );
        if inline_size.0.is_none() {
            self.data.push(Bytecode(size.get()));
        }
    }
    pub fn fv_set_heap_aligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        size: VectorSize,
        imm4: InlineAddrOffset<4>,
    ) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            FvSetHeapAligned(SetHeapArgs {
                rd,
                rs,
                roff,
                size: inline_size,
                imm4,
            })
            .encode(),
        );
        if inline_size.0.is_none() {
            self.data.push(Bytecode(size.get()));
        }
    }
}
