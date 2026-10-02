use std::ops::RangeInclusive;
use std::{cmp, fmt};

use vogls_codegen::SixBitSize;
use vogls_ir::{LogicMode, SCALAR_VSIZE, SignalSlice, VSIZE_32, VectorSize};
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_utils::{NonMaxU8, TableKey};

use crate::reg::{Reg, RegInfo, RegStorage, Regs};
use crate::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineNBitSize, Schedule, decode_bytecode_u64, encode_bytecode_u64, write_padded_mnemonic,
};

pub struct BytecodeWatchers {
    pub offsets: Vec<u64>,
    pub watchers: Vec<u64>,
    pub watcher_offsets: Vec<SignalSlice>,
}

impl BytecodeWatchers {
    #[inline(always)]
    pub fn get(&self, index: usize) -> &[u64] {
        let end = index.saturating_add(1);
        assert!(end < self.offsets.len());
        let start = self.offsets[index];
        let end = self.offsets[index.saturating_add(1)];
        &self.watchers[start as usize..end as usize]
    }
}

#[inline(always)]
pub fn wake_at_index(index: u64, schedule: &mut Schedule, listeners: &mut BytecodeListeners) {
    let index = index as usize;
    let bit = 1u64 << (index % 64);
    let is_listening = (listeners.active[index / 64] & bit) != 0;
    if is_listening {
        let offset = listeners.map[index];
        schedule.active.push(offset);
        listeners.active[index / 64] ^= bit;
    }
}

pub struct HeapShiftReduceOr {
    rd: Reg,
    rs: Reg,
    size: InlineNBitSize<8>,
    offset: Option<NonMaxU8>,
}

pub struct HeapShiftReduceOrRel {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: InlineNBitSize<12>,
}

pub struct ShiftReduceOr {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    lower_bound6: u8,
    upper_bound6: u8,
}
pub struct ShiftReduceOrOutlined {
    rd: Reg,
    rs: Reg,
    roff: Reg,
}

pub struct Wake {
    rs: Reg,
    offset6: u8,
    size: SixBitSize,
    // 8 bits unused
}

pub struct SetLastUpdateTime {
    rs: Reg,
}

pub struct PluginPoke {
    rs: Reg,
}

impl BytecodeInstruction for Wake {
    #[inline(always)]
    fn num_additional_slots(&self) -> u8 {
        4 // Watch Start + Watch End
    }

    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::Wake as u8);
        let v = c.0;
        Self {
            rs: Reg::new_masked(v >> 8),
            offset6: ((v >> 12) & 0x3F) as u8,
            size: SixBitSize::new_masked(v >> 18),
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::Wake as u32)
                | ((self.rs as u32) << 8)
                | ((self.offset6 as u32) << 12)
                | (self.size.encode() << 18),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { rs, offset6, size } = self;
        write_padded_mnemonic(f, "wake")?;
        write!(f, "{rs}, {offset6}, {size}")
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::new(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            None,
            RegStorage::Register,
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        _state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rs, offset6, size } = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        let is_update = regs[rs];
        let is_update = is_update.wrapping_shr(offset6 as u32);
        let is_update = size.mask(is_update);
        let is_update = is_update != 0;
        if is_update {
            let watchers = cldctx.watchers.watchers.as_slice();
            let watch_start = decode_bytecode_u64(&additional_slots[..]);
            let watch_end = decode_bytecode_u64(&additional_slots[2..]);

            for watch_index in &watchers[watch_start as usize..watch_end as usize] {
                wake_at_index(*watch_index, schedule, listeners);
            }
        }
    }
}

#[inline(always)]
pub fn heap_offset_reduce_or(heap: &mut [u64], offset: u64, length: VectorSize) -> bool {
    let woff = offset / 64;
    let boff = offset % 64;

    let mut reduce_or = 0u64;
    let fst_word = heap[woff as usize];
    let fst_word_size = SixBitSize::new((64 - boff as u32).min(length.get()) as u8).unwrap();
    reduce_or |= fst_word_size.mask(fst_word >> boff);

    let mut rem_size = length.get() - fst_word_size as u32;
    let mut word_idx = woff as usize + 1;
    while rem_size > 64 {
        reduce_or |= heap[word_idx];
        rem_size -= 64;
        word_idx += 1;
    }

    if let Some(last_word_size) = SixBitSize::new(rem_size as u8) {
        reduce_or |= last_word_size.mask(heap[word_idx]);
    }

    reduce_or != 0
}

impl BytecodeInstruction for HeapShiftReduceOr {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        if !self.size.is_inline() {
            slots += 1;
        }
        if self.offset.is_none() {
            slots += 1;
        }
        slots
    }

    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapShiftReduceOr as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: InlineNBitSize::new_masked(v >> 16),
            offset: NonMaxU8::new((v >> 24) as u8),
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::HeapShiftReduceOr as u32)
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.size.encode() << 16)
                | ((self.offset.map_or(u8::MAX, |v| v.get()) as u32) << 24),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            rd,
            rs,
            size,
            offset,
        } = self;
        write_padded_mnemonic(f, "heap_shift_reduce_or")?;
        write!(f, "{rd}, {rs}, {size}, ")?;
        match offset {
            None => f.write_str("<outlined>"),
            Some(offset) => fmt::Display::fmt(offset, f),
        }
    }

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc + 1;
        let size = self.size.get(&mut pc, code);
        operands.push(RegInfo::new(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            Some(size),
            RegStorage::Heap,
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::new(
            "rd",
            self.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
            RegStorage::Register,
        ));
    }

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
        let Self {
            rd,
            rs,
            size,
            offset,
        } = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        let mut slot_offset = 0;
        let size = size.get(&mut slot_offset, additional_slots);
        let offset = match offset {
            None => additional_slots[slot_offset as usize].0,
            Some(v) => v.get() as u32,
        };

        let bit_offset = regs[rs] + offset as u64;
        let heap = state.heap.0.as_mut();
        let reduce_or = heap_offset_reduce_or(heap, bit_offset, size);
        regs[rd] = u64::from(reduce_or);
    }
}

impl BytecodeInstruction for HeapShiftReduceOrRel {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 1; // Lower bound.
        slots += 1; // Upper bound.
        if !self.size.is_inline() {
            slots += 1;
        }
        slots
    }

    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapShiftReduceOrRel as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            size: InlineNBitSize::new_masked(v >> 20),
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::HeapShiftReduceOrRel as u32)
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.size.encode() << 20),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { rd, rs, roff, size } = self;
        write_padded_mnemonic(f, "heap_shift_reduce_or_rel")?;
        write!(f, "{rd}, {rs}, {roff}, {size}")
    }

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc + 3;
        let size = self.size.get(&mut pc, code);
        operands.push(RegInfo::new(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            Some(size),
            RegStorage::Heap,
        ));
        operands.push(RegInfo::new(
            "roff",
            self.roff,
            LogicMode::TwoValue,
            Some(VSIZE_32),
            RegStorage::Heap,
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::new(
            "rd",
            self.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
            RegStorage::Register,
        ));
    }

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
        let Self { rd, rs, roff, size } = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        // Inclusive range.
        let lower_bound = additional_slots[0].0;
        let upper_bound = additional_slots[1].0;
        debug_assert!(lower_bound <= upper_bound);
        let size = size.get(&mut 2, additional_slots);

        // Exclusive range.
        let src_start = (regs[roff] & 0xFFFF_FFFF) as u32;
        let src_end = src_start.wrapping_add(size.get());

        let clamped_start = cmp::max(lower_bound, src_start);
        let clamped_end = cmp::min(upper_bound.wrapping_add(1), src_end);

        let Some(bit_length) = VectorSize::new(clamped_end.saturating_sub(clamped_start)) else {
            regs[rd] = 0;
            return;
        };

        let offset = clamped_start - src_start;
        let bit_offset = regs[rs] + offset as u64;
        let heap = state.heap.0.as_mut();
        let reduce_or = heap_offset_reduce_or(heap, bit_offset, bit_length);
        regs[rd] = u64::from(reduce_or);
    }
}

impl BytecodeInstruction for SetLastUpdateTime {
    #[inline(always)]
    fn num_additional_slots(&self) -> u8 {
        2 // Index
    }

    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::SetLastUpdateTime as u8);
        let v = c.0;
        Self {
            rs: Reg::new_masked(v >> 8),
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode((BytecodeOpcode::SetLastUpdateTime as u32) | ((self.rs as u32) << 8))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { rs } = self;
        write_padded_mnemonic(f, "set_lupdt")?;
        write!(f, "{rs}")
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::new(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            None,
            RegStorage::Register,
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}

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
        let Self { rs } = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        let is_update = regs[rs];
        let is_update = is_update != 0;
        if is_update {
            let index = decode_bytecode_u64(&additional_slots[..]);
            state.last_active_time[index as usize] = state.time;
        }
    }
}

impl BytecodeInstruction for PluginPoke {
    #[inline(always)]
    fn num_additional_slots(&self) -> u8 {
        2 // Index
    }

    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::PluginPoke as u8);
        let v = c.0;
        Self {
            rs: Reg::new_masked(v >> 8),
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode((BytecodeOpcode::PluginPoke as u32) | ((self.rs as u32) << 8))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { rs } = self;
        write_padded_mnemonic(f, "plugin_poke")?;
        write!(f, "{rs}")
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::new(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            None,
            RegStorage::Register,
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rs } = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        let is_update = regs[rs];
        let is_update = is_update != 0;
        if is_update {
            let rt_index = decode_bytecode_u64(&additional_slots[..]);
            let rt_index = RtSignalKey::from_usize(rt_index as usize).unwrap();
            for plugin in cldctx.plugins.iter_mut() {
                plugin.poke_signal(rt_index);
            }
        }
    }
}

impl BytecodeInstruction for ShiftReduceOr {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::ShiftReduceOr as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            lower_bound6: ((v >> 20) & 0x3F) as u8,
            upper_bound6: ((v >> 26) & 0x3F) as u8,
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::ShiftReduceOr as u32)
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | ((self.lower_bound6 as u32) << 20)
                | ((self.upper_bound6 as u32) << 26),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            lower_bound6,
            upper_bound6,
        } = self;
        write_padded_mnemonic(f, "shift_reduce_or")?;
        write!(f, "{rd}, {rs}, {roff}, {lower_bound6}, {upper_bound6}")
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.extend([
            RegInfo::new(
                "rs",
                self.rs,
                LogicMode::TwoValue,
                None,
                RegStorage::Register,
            ),
            RegInfo::new(
                "roff",
                self.roff,
                LogicMode::TwoValue,
                Some(VSIZE_32),
                RegStorage::Register,
            ),
        ]);
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::new(
            "rd",
            self.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
            RegStorage::Register,
        ));
    }

    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs,
            roff,
            lower_bound6,
            upper_bound6,
        } = self;

        // Lower bound to upper bound is an inclusive range.
        let size = SixBitSize::new(upper_bound6 - lower_bound6 + 1).unwrap();

        // Exclusive range.
        let src_start = (regs[roff] & 0xFFFF_FFFF) as u32;
        let src_end = src_start.wrapping_add(size as u32);

        let clamped_start = cmp::max(lower_bound6 as u32, src_start);
        let clamped_end = cmp::min(upper_bound6.wrapping_add(1) as u32, src_end);

        let Some(bit_length) = SixBitSize::new(clamped_end.saturating_sub(clamped_start) as u8)
        else {
            regs[rd] = 0;
            return;
        };

        let offset = clamped_start - src_start;
        let reduce_or = bit_length.mask(regs[rs] >> offset) != 0;
        regs[rd] = u64::from(reduce_or);
    }
}

impl BytecodeInstruction for ShiftReduceOrOutlined {
    #[inline(always)]
    fn num_additional_slots(&self) -> u8 {
        2
    }

    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::ShiftReduceOrOutlined as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::ShiftReduceOrOutlined as u32)
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { rd, rs, roff } = self;
        write_padded_mnemonic(f, "shift_reduce_or_outlined")?;
        write!(f, "{rd}, {rs}, {roff}")
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.extend([
            RegInfo::new(
                "rs",
                self.rs,
                LogicMode::TwoValue,
                None,
                RegStorage::Register,
            ),
            RegInfo::new(
                "roff",
                self.roff,
                LogicMode::TwoValue,
                Some(VSIZE_32),
                RegStorage::Register,
            ),
        ]);
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::new(
            "rd",
            self.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
            RegStorage::Register,
        ));
    }

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd, rs, roff } = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        // Inclusive range.
        let lower_bound = additional_slots[0].0;
        let upper_bound = additional_slots[1].0;
        let size = VectorSize::new(upper_bound - lower_bound + 1).unwrap();

        // Exclusive range.
        let src_start = (regs[roff] & 0xFFFF_FFFF) as u32;
        let src_end = src_start.wrapping_add(size.get());

        let clamped_start = cmp::max(lower_bound, src_start);
        let clamped_end = cmp::min(upper_bound.wrapping_add(1), src_end);

        let Some(bit_length) = SixBitSize::new(clamped_end.saturating_sub(clamped_start) as u8)
        else {
            regs[rd] = 0;
            return;
        };

        let offset = clamped_start - src_start;
        let reduce_or = bit_length.mask(regs[rs] >> offset) != 0;
        regs[rd] = u64::from(reduce_or);
    }
}

impl BytecodeEncoder {
    pub fn wake(&mut self, rs: Reg, offset6: u8, size: SixBitSize, start: u64, end: u64) {
        self.data.push(Wake { rs, offset6, size }.encode());
        encode_bytecode_u64(self, start);
        encode_bytecode_u64(self, end);
    }

    pub fn set_lupdt(&mut self, rs: Reg, index: u64) {
        self.data.push(SetLastUpdateTime { rs }.encode());
        encode_bytecode_u64(self, index);
    }

    pub fn plugin_poke(&mut self, rs: Reg, signal: RtSignalKey) {
        self.data.push(PluginPoke { rs }.encode());
        encode_bytecode_u64(self, signal.as_u64());
    }

    pub fn heap_shift_reduce_or(&mut self, rd: Reg, rs: Reg, size: VectorSize, offset: u32) {
        let inline_size = InlineNBitSize::new(size);
        let inline_offset = offset.try_into().ok().and_then(NonMaxU8::new);
        self.data.push(
            HeapShiftReduceOr {
                rd,
                rs,
                size: inline_size,
                offset: inline_offset,
            }
            .encode(),
        );
        if !inline_size.is_inline() {
            self.data.push(Bytecode(size.get()));
        }
        if inline_offset.is_none() {
            self.data.push(Bytecode(offset));
        }
    }
    pub fn heap_shift_reduce_or_rel(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        src_size: VectorSize,
        bounds: RangeInclusive<u32>,
    ) {
        let inline_size = InlineNBitSize::new(src_size);
        self.data.push(
            HeapShiftReduceOrRel {
                rd,
                rs,
                roff,
                size: inline_size,
            }
            .encode(),
        );
        self.data.push(Bytecode(*bounds.start()));
        self.data.push(Bytecode(*bounds.end()));
        if !inline_size.is_inline() {
            self.data.push(Bytecode(src_size.get()));
        }
    }
    pub fn shift_reduce_or(&mut self, rd: Reg, rs: Reg, roff: Reg, bounds: RangeInclusive<u32>) {
        if *bounds.start() < 64 && *bounds.end() < 64 {
            self.data.push(
                ShiftReduceOr {
                    rd,
                    rs,
                    roff,
                    lower_bound6: *bounds.start() as u8,
                    upper_bound6: *bounds.end() as u8,
                }
                .encode(),
            );
        } else {
            self.data
                .push(ShiftReduceOrOutlined { rd, rs, roff }.encode());
            self.data.push(Bytecode(*bounds.start()));
            self.data.push(Bytecode(*bounds.end()));
        }
    }
}
