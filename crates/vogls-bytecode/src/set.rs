use std::{cmp, fmt};

use vogls_bits::set_subslice::tv_cell_set;
use vogls_codegen::{HeapAlignment, HeapOffset};
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use super::reg::{Reg, RegInfo, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineAddrOffset, InlineNBitSize, Schedule, SixBitSize, write_padded_mnemonic,
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
pub struct SetUnaligned(pub SetArgs);

pub struct TvRelSetAligned(pub SetRelArgs);
pub struct FvRelSetAligned(pub SetRelArgs);
pub struct SetRelUnaligned(pub SetRelArgs);
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

/// Set a word on the heap at a specific offset.
///
/// This overwrites all the bits in `offset..offset + size` with `value` and returns a mask of
/// which bits in value changed bits on the heap. If an addressed bit falls outside of
/// `base..base + base_size`, it cannot be changed and any write is considered a no-op.
///
/// # Invariants
///
/// - `value` should be premasked according to `size`.
/// - `base..base + base_size` should be a valid u64 range.
#[inline(always)]
fn set_unaligned(
    heap: &mut [u64],
    offset: u64,
    value: u64,
    size: SixBitSize,
    base: u64,
    base_size: VectorSize,
) -> u64 {
    debug_assert_eq!(value, size.mask(value));

    let base_min = base;
    let base_max = base + base_size.get() as u64;
    if (offset < base_min) | (offset.saturating_add(size as u64) > base_max) {
        return set_unaligned_oob(heap, offset, value, size, base, base_size);
    }

    set_unaligned_bounded(heap, offset, value, size)
}

#[cold]
#[inline(never)]
fn set_unaligned_oob(
    heap: &mut [u64],
    offset: u64,
    value: u64,
    size: SixBitSize,
    base: u64,
    base_size: VectorSize,
) -> u64 {
    let base_min = base;
    let base_max = base + base_size.get() as u64;
    let write_min = offset;
    let write_max = offset + size as u64;

    let trim_min = cmp::max(base_min, write_min);
    let trim_max = cmp::min(base_max, write_max);

    let trim_size = trim_max.saturating_sub(trim_min);
    debug_assert!(trim_size < size as u64);
    let Some(trim_size) = SixBitSize::new(trim_size as u8) else {
        return 0;
    };

    let min_shift = trim_min - offset;
    let trim_value = trim_size.mask(value >> min_shift);

    set_unaligned_bounded(heap, trim_min, trim_value, trim_size) << min_shift
}

#[inline(always)]
fn set_unaligned_bounded(heap: &mut [u64], offset: u64, value: u64, size: SixBitSize) -> u64 {
    let mask = size.mask(u64::MAX);
    let end_offset = offset + size as u64 - 1;

    let word = (offset / 64) as usize;
    let boff = offset % 64;
    let endword = (end_offset / 64) as usize;

    if word == endword {
        let word = &mut heap[word];
        let prev = mask & (*word >> boff);
        *word &= !(mask << boff);
        *word |= value << boff;
        return prev ^ value;
    }

    // Since word == endword, boff should never be 0.
    debug_assert_ne!(boff, 0);
    let tgt: &mut [u64; 2] = (&mut heap[word..word + 2])
        .try_into()
        .expect("Unable to take heap words");
    let prev = mask & ((tgt[0] >> boff) | (tgt[1] << (64 - boff)));
    tgt[0] &= !(mask << boff);
    tgt[0] |= value << boff;
    tgt[1] &= !(mask >> (64 - boff));
    tgt[1] |= value >> (64 - boff);
    prev ^ value
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

    fn num_slots(&self) -> u8 {
        2
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

    fn num_slots(&self) -> u8 {
        2
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

impl BytecodeInstruction for SetUnaligned {
    impl_set_args!(SetUnaligned, "set_unaligned");

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

    fn num_slots(&self) -> u8 {
        2
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
        let val = regs[rs];
        let heap = state.heap.0.as_mut();
        let updated = set_unaligned(heap, offset, val, size);
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for SetRelUnaligned {
    impl_set_rel_args!(SetRelUnaligned, "set_rel_unaligned");

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
        let val = regs[rs];
        let updated = set_unaligned(state.heap.0.as_mut(), offset, val, size);
        regs[rd] = u64::from(updated);
    }
}

impl BytecodeInstruction for SetHeapUnaligned {
    impl_set_heap_args!(SetHeapUnaligned, "set_heap_unaligned");

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc;
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

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc;
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

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc;
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
    pub fn set_unaligned(&mut self, rd: Reg, rs: Reg, at: u64, size: SixBitSize) {
        if at < (1u64 << (10 + 32)) {
            let imm10 = (at & 0x3FF) as u16;
            let imm42_11 = (at >> 10) as u32;
            self.data.push(
                SetUnaligned(SetArgs {
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
        self.rel_set_unaligned(rd, rs, rd, InlineAddrOffset::ZERO, size);
    }
    pub fn rel_set_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        imm6: InlineAddrOffset<6>,
        size: SixBitSize,
    ) {
        self.data.push(
            SetRelUnaligned(SetRelArgs {
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
    pub fn set_heap_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        size: VectorSize,
        imm4: InlineAddrOffset<4>,
    ) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            SetHeapUnaligned(SetHeapArgs {
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
