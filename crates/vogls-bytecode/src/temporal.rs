use vogls_ir::LogicMode;
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_utils::TableKey;

use std::fmt;

use super::reg::{Reg, RegInfo, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineIndex, InstructionPtr, Schedule, SignedImmediate, TimedEvent, write_padded_mnemonic,
};

/// Wake the listener at the given index if the condition is non-zero.
pub struct Wake {
    rcond: Reg,
    index: InlineIndex<20>,
}

/// Wake the listeners at the given index if the condition is non-zero.
pub struct WakeMultiple {
    rcond: Reg,
    index: InlineIndex<20>,
}

pub struct PluginPoke {
    rcond: Reg,
    index: InlineIndex<20>,
}

pub struct RescheduleRegion {
    pub region: u8,
    pub offset: SignedImmediate<16>,
}

pub struct RescheduleWait {
    pub rtime: Reg,
    pub offset: SignedImmediate<20>,
}
pub struct RescheduleListen {
    pub index: u32,
}
pub struct NextEvent;

pub struct LastUpdateTime {
    rd: Reg,
    index: InlineIndex<20>,
}
pub struct SetLupdt {
    rcond: Reg,
    index: InlineIndex<20>,
}
pub struct TvCorrectFirst {
    rcond: Reg,
    index: InlineIndex<20>,
}

#[inline(always)]
pub fn wake(index: u64, schedule: &mut Schedule, listeners: &mut BytecodeListeners) {
    let index = index as usize;
    let bit = 1u64 << (index % 64);
    let is_listening = (listeners.active[index / 64] & bit) != 0;
    if is_listening {
        let offset = listeners.map[index];
        schedule.active.push(offset);
        listeners.active[index / 64] ^= bit;
    }
}

impl BytecodeInstruction for Wake {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Wake as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            index: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::Wake as u32 | ((self.rcond as u32) << 8) | (self.index.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, index } = self;
        write_padded_mnemonic(f, "wake")?;
        write!(f, "{rcond}, {index}")
    }

    fn num_additional_slots(&self) -> u8 {
        if self.index.is_inline() { 0 } else { 2 }
    }

    #[inline(always)]
    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        _state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rcond, index } = self;
        let index = index.get(pc, code);
        if regs[rcond] != 0 {
            wake(index, schedule, listeners);
        }
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rcond",
            self.rcond,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}
impl BytecodeInstruction for WakeMultiple {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::WakeMultiple as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            index: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::WakeMultiple as u32
                | ((self.rcond as u32) << 8)
                | (self.index.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, index } = self;
        write_padded_mnemonic(f, "wake")?;
        write!(f, "{rcond}, {index}")
    }

    fn num_additional_slots(&self) -> u8 {
        if self.index.is_inline() { 0 } else { 2 }
    }

    #[inline(always)]
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
        let Self { rcond, index } = self;
        let signal_key = index.get(pc, code);
        if regs[rcond] != 0 {
            let watchers = cldctx.watchers.get(signal_key as usize);
            for &index in watchers {
                wake(index, schedule, listeners);
            }
        }
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rcond",
            self.rcond,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}

impl BytecodeInstruction for PluginPoke {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::PluginPoke as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            index: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::PluginPoke as u32
                | ((self.rcond as u32) << 8)
                | (self.index.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, index } = self;
        write_padded_mnemonic(f, "plugin_poke")?;
        write!(f, "{rcond}, {index}")
    }

    fn num_additional_slots(&self) -> u8 {
        if self.index.is_inline() { 0 } else { 2 }
    }

    #[inline(always)]
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
        let Self { rcond, index } = self;
        let i = index.get(pc, code) as usize;
        if regs[rcond] != 0 {
            let i = RtSignalKey::from_usize(i).unwrap();
            for plugin in cldctx.plugins.iter_mut() {
                plugin.poke_signal(i);
            }
        }
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rcond",
            self.rcond,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}

impl BytecodeInstruction for RescheduleWait {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::RescheduleWait as u8);
        let v = v.0;
        Self {
            rtime: Reg::new_masked(v >> 8),
            offset: SignedImmediate::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::RescheduleWait as u32
                | ((self.rtime as u32) << 8)
                | (self.offset.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rtime, offset } = self;
        write_padded_mnemonic(f, "reschedule_wait")?;
        write!(f, "{rtime}, {offset}")?;
        Ok(())
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rtime, offset } = self;

        let time = regs[rtime];
        let next_pc = (*pc).wrapping_add_signed(i64::from(offset.0));
        if time == 0 {
            *pc = next_pc;
            return;
        }

        let Some(time) = state.time.checked_add(time) else {
            return time_overflow(cldctx);
        };
        schedule.next_time = schedule.next_time.min(time);
        schedule.future.push(TimedEvent {
            time,
            pc: InstructionPtr(next_pc),
        });

        *pc = schedule
            .pop(state, cldctx.plugins)
            .map_or(u64::MAX, |ptr| ptr.0);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rtime",
            self.rtime,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}

#[cold]
fn time_overflow(cldctx: &mut ColdContext) {
    cldctx.return_value = 1;
    cldctx.stderr.write_all(b"Time overflow!").unwrap();
}

impl BytecodeInstruction for RescheduleRegion {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::RescheduleRegion as u8);
        let v = v.0;
        Self {
            region: ((v >> 8) & 0xFF) as u8,
            offset: SignedImmediate::new_shifted(v, 16),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::RescheduleRegion as u32
                | ((self.region as u32) << 8)
                | (self.offset.encode() << 16),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { region, offset } = self;
        write_padded_mnemonic(f, "reschedule_region")?;
        write!(f, "{region}, {offset}")?;
        Ok(())
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        _regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { region, offset } = self;
        let next_pc = (*pc).wrapping_add_signed(i64::from(offset.0));
        if region == 0 {
            *pc = next_pc;
            return;
        }

        schedule.regions[region as usize - 1].push(InstructionPtr(next_pc));
        *pc = schedule
            .pop(state, cldctx.plugins)
            .map_or(u64::MAX, |ptr| ptr.0);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}

impl BytecodeInstruction for NextEvent {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::NextEvent as u8);
        Self
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::NextEvent as u32)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "next_event")
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        _regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        *pc = schedule
            .pop(state, cldctx.plugins)
            .map_or(u64::MAX, |ptr| ptr.0);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}

impl BytecodeInstruction for RescheduleListen {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::RescheduleListen as u8);
        let v = v.0;
        Self { index: v >> 8 }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::RescheduleListen as u32 | (self.index << 8))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { index } = self;
        write_padded_mnemonic(f, "reschedule_listen")?;
        write!(f, "{index}")
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        _regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        listeners.arm(self.index as usize);
        *pc = schedule
            .pop(state, cldctx.plugins)
            .map_or(u64::MAX, |ptr| ptr.0);
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}

impl BytecodeInstruction for LastUpdateTime {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::LastUpdateTime as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            index: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::LastUpdateTime as u32
                | ((self.rd as u32) << 8)
                | (self.index.encode() << 12),
        )
    }

    fn num_additional_slots(&self) -> u8 {
        if self.index.is_inline() { 0 } else { 2 }
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, index: idx } = self;
        write_padded_mnemonic(f, "lupdt")?;
        write!(f, "{rd}, {idx}")
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
        let i = self.index.get(pc, code);
        regs[self.rd] = state.last_active_time[i as usize];
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register("rd", self.rd, LogicMode::TwoValue, None));
    }
}

impl BytecodeInstruction for SetLupdt {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::SetLupdt as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            index: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::SetLupdt as u32
                | ((self.rcond as u32) << 8)
                | (self.index.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, index: idx } = self;
        write_padded_mnemonic(f, "set_lupdt")?;
        write!(f, "{rcond}, {idx}")
    }

    fn num_additional_slots(&self) -> u8 {
        if self.index.is_inline() { 0 } else { 2 }
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
        let i = self.index.get(pc, code);
        let i = &mut state.last_active_time[i as usize];
        if regs[self.rcond] != 0 {
            *i = state.time;
        }
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rcond",
            self.rcond,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
}

impl BytecodeInstruction for TvCorrectFirst {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::TvCorrectFirst as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            index: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::TvCorrectFirst as u32
                | ((self.rcond as u32) << 8)
                | (self.index.encode() << 12),
        )
    }

    fn num_additional_slots(&self) -> u8 {
        if self.index.is_inline() { 0 } else { 2 }
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, index: idx } = self;
        write_padded_mnemonic(f, "tv.correct_first")?;
        write!(f, "{rcond}, {idx}")
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
        let i = self.index.get(pc, code);
        let word = (i / 64) as usize;
        let boff = (i % 64) as usize;
        let i = &mut state.tvl_first_write[word];
        regs[self.rcond] |= ((!*i) >> boff) & 1;
        *i |= 1u64 << boff;
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rcond",
            self.rcond,
            LogicMode::TwoValue,
            None,
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rcond",
            self.rcond,
            LogicMode::TwoValue,
            None,
        ));
    }
}

impl BytecodeEncoder {
    pub fn wake(&mut self, rcond: Reg, index: u64) {
        let inline_index = InlineIndex::new(index);
        self.data.push(
            Wake {
                rcond,
                index: inline_index,
            }
            .encode(),
        );
        if inline_index.0.is_none() {
            self.data.push(Bytecode((index >> 32) as u32));
            self.data.push(Bytecode((index & 0xFFFF_FFFF) as u32));
        }
    }
    pub fn wake_multiple(&mut self, rcond: Reg, index: u64) {
        let inline_index = InlineIndex::new(index);
        self.data.push(
            WakeMultiple {
                rcond,
                index: inline_index,
            }
            .encode(),
        );
        if inline_index.0.is_none() {
            self.data.push(Bytecode((index >> 32) as u32));
            self.data.push(Bytecode((index & 0xFFFF_FFFF) as u32));
        }
    }
    pub fn plugin_poke(&mut self, rcond: Reg, index: u64) {
        let inline_index = InlineIndex::new(index);
        self.data.push(
            PluginPoke {
                rcond,
                index: inline_index,
            }
            .encode(),
        );
        if inline_index.0.is_none() {
            self.data.push(Bytecode((index >> 32) as u32));
            self.data.push(Bytecode((index & 0xFFFF_FFFF) as u32));
        }
    }

    pub fn wait(&mut self, rtime: Reg, offset: i64) {
        let offset = SignedImmediate::new(offset).unwrap();
        self.data.push(RescheduleWait { rtime, offset }.encode());
    }

    pub fn wait_region(&mut self, region: u8, offset: i64) {
        let offset = SignedImmediate::new(offset).unwrap();
        self.data.push(RescheduleRegion { region, offset }.encode());
    }

    pub fn next_event(&mut self) {
        self.data.push(NextEvent.encode());
    }

    pub fn reschedule_listen(&mut self, index: u32) {
        self.data.push(RescheduleListen { index }.encode());
    }

    pub fn set_lupdt(&mut self, rcond: Reg, index: u64) {
        let inline_index = InlineIndex::new(index);
        self.data.push(
            SetLupdt {
                rcond,
                index: inline_index,
            }
            .encode(),
        );
        if inline_index.0.is_none() {
            self.data.push(Bytecode((index >> 32) as u32));
            self.data.push(Bytecode((index & 0xFFFF_FFFF) as u32));
        }
    }

    pub fn tv_correct_first(&mut self, rcond: Reg, index: u64) {
        let inline_index = InlineIndex::new(index);
        self.data.push(
            TvCorrectFirst {
                rcond,
                index: inline_index,
            }
            .encode(),
        );
        if inline_index.0.is_none() {
            self.data.push(Bytecode((index >> 32) as u32));
            self.data.push(Bytecode((index & 0xFFFF_FFFF) as u32));
        }
    }

    pub fn last_update_time(&mut self, rd: Reg, index: u64) {
        let inline_index = InlineIndex::new(index);
        self.data.push(
            LastUpdateTime {
                rd,
                index: inline_index,
            }
            .encode(),
        );
        if inline_index.0.is_none() {
            self.data.push(Bytecode((index >> 32) as u32));
            self.data.push(Bytecode((index & 0xFFFF_FFFF) as u32));
        }
    }
}
