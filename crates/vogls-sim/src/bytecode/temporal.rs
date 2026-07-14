use vogls_ir::LogicMode;
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_utils::TableKey;

use std::fmt;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, InlineIndex, InstructionPtr, Schedule, SignedImmediate, TimedEvent,
    write_padded_mnemonic, write_register,
};

pub struct Wake {
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
    idx: InlineIndex<20>,
}
pub struct SetLupdt {
    rcond: Reg,
    idx: InlineIndex<20>,
}
pub struct TvCorrectFirst {
    rcond: Reg,
    idx: InlineIndex<20>,
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

    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        writeln!(f, "{EXEC_ITRACE_INDENT}rcond = {}", regs[self.rcond] != 0)
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

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rcond, index } = self;
        if regs[rcond] != 0 {
            let i = index.get(regs, Reg::X15) as usize;
            let bit = 1u64 << (i % 64);
            let is_listening = (listeners.active[i / 64] & bit) != 0;
            if is_listening {
                let offset = listeners.map[i];
                schedule.active.push(offset);
                listeners.active[i / 64] ^= bit;
            }
        }
    }
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

    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        writeln!(f, "{EXEC_ITRACE_INDENT}rcond = {}", regs[self.rcond] != 0)
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

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rcond, index } = self;
        if regs[rcond] != 0 {
            let i = index.get(regs, Reg::X15) as usize;
            let i = RtSignalKey::from_usize(i).unwrap();
            for plugin in cldctx.plugins.iter_mut() {
                plugin.poke_signal(i);
            }
        }
    }
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
    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rtime", self.rtime, LogicMode::TwoValue)?;
        writeln!(f)
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
        let i = self.index as usize;
        listeners.active[i / 64] |= 1u64 << (i % 64);
        *pc = schedule
            .pop(state, cldctx.plugins)
            .map_or(u64::MAX, |ptr| ptr.0);
    }
}

impl BytecodeInstruction for LastUpdateTime {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::LastUpdateTime as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            idx: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::LastUpdateTime as u32
                | ((self.rd as u32) << 8)
                | (self.idx.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, idx } = self;
        write_padded_mnemonic(f, "lupdt")?;
        write!(f, "{rd}, {idx}")
    }

    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rd", self.rd, LogicMode::TwoValue)?;
        writeln!(f)
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
        let i = self.idx.get(regs, Reg::X15);
        regs[self.rd] = state.last_active_time[i as usize];
    }
}

impl BytecodeInstruction for SetLupdt {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::SetLupdt as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            idx: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::SetLupdt as u32
                | ((self.rcond as u32) << 8)
                | (self.idx.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, idx } = self;
        write_padded_mnemonic(f, "set_lupdt")?;
        write!(f, "{rcond}, {idx}")
    }

    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rcond", self.rcond, LogicMode::TwoValue)?;
        writeln!(f)
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
        let i = self.idx.get(regs, Reg::X15);
        let i = &mut state.last_active_time[i as usize];
        if regs[self.rcond] != 0 {
            *i = state.time;
        }
    }
}

impl BytecodeInstruction for TvCorrectFirst {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::TvCorrectFirst as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            idx: InlineIndex::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::TvCorrectFirst as u32
                | ((self.rcond as u32) << 8)
                | (self.idx.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, idx } = self;
        write_padded_mnemonic(f, "tv.correct_first")?;
        write!(f, "{rcond}, {idx}")
    }

    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rcond", self.rcond, LogicMode::TwoValue)?;
        writeln!(f)
    }
    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rcond", self.rcond, LogicMode::TwoValue)?;
        writeln!(f)
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
        let i = self.idx.get(regs, Reg::X15);
        let word = (i / 64) as usize;
        let boff = (i % 64) as usize;
        let i = &mut state.tvl_first_write[word];
        regs[self.rcond] |= ((!*i) >> boff) & 1;
        *i |= 1u64 << boff;
    }
}

impl BytecodeEncoder {
    pub fn wake(&mut self, rcond: Reg, index: InlineIndex<20>) {
        self.data.push(Wake { rcond, index }.encode());
    }
    pub fn plugin_poke(&mut self, rcond: Reg, index: u64) {
        let index = InlineIndex::new(index, self, Reg::X15);
        self.data.push(PluginPoke { rcond, index }.encode());
    }

    pub fn wait(&mut self, rtime: Reg, offset: i64) {
        let offset = SignedImmediate::new(offset).unwrap();
        self.data.push(RescheduleWait { rtime, offset }.encode());
    }

    pub fn wait_region(&mut self, region: u8, offset: i64) {
        let offset = SignedImmediate::new(offset).unwrap();
        self.data.push(
            RescheduleRegion {
                region: region,
                offset,
            }
            .encode(),
        );
    }

    pub fn next_event(&mut self) {
        self.data.push(NextEvent.encode());
    }

    pub fn reschedule_listen(&mut self, index: u32) {
        self.data.push(RescheduleListen { index }.encode());
    }

    pub fn set_lupdt(&mut self, rcond: Reg, idx: InlineIndex<20>) {
        self.data.push(SetLupdt { rcond, idx }.encode());
    }

    pub fn tv_correct_first(&mut self, rcond: Reg, idx: InlineIndex<20>) {
        self.data.push(TvCorrectFirst { rcond, idx }.encode());
    }

    pub fn last_update_time(&mut self, rd: Reg, idx: InlineIndex<20>) {
        self.data.push(LastUpdateTime { rd, idx }.encode());
    }
}
