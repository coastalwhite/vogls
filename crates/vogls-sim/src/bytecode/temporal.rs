use vogls_ir::LogicMode;
use vogls_runtime::RuntimeState;
use vogls_utils::NonMaxU32;

use std::fmt;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, InlineIndex, InstructionPtr, Schedule, TimedEvent, write_padded_mnemonic,
    write_register,
};

pub struct Wake {
    rcond: Reg,
    index: InlineIndex<20>,
}

pub struct Reschedule {
    rtime: Reg,
    region: u8,
    schedule_self: bool,
}

pub struct StartListen {
    index: u32,
}

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
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        write!(f, "{EXEC_ITRACE_INDENT}rcond = {}", regs[self.rcond] != 0)
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

    fn execute(
        self,
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

impl BytecodeInstruction for Reschedule {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Reschedule as u8);
        let v = v.0;
        Self {
            rtime: Reg::new_masked(v >> 8),
            region: ((v >> 12) & 0xFF) as u8,
            schedule_self: (v >> 20) & 1 != 0,
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::Reschedule as u32
                | ((self.rtime as u32) << 8)
                | ((self.region as u32) << 12)
                | (self.schedule_self as u32) << 20,
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rtime,
            region,
            schedule_self,
        } = self;
        if *schedule_self {
            write_padded_mnemonic(f, "next_event")?;
        } else if *region == 0 {
            write_padded_mnemonic(f, "wait")?;
            write!(f, "{rtime}")?;
        } else {
            write_padded_mnemonic(f, "wait_region")?;
            write!(f, "{}", region - 1)?;
        }
        Ok(())
    }

    fn execute(
        self,
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rtime,
            region,
            schedule_self,
        } = self;
        if schedule_self {
            if region == 0 {
                let time = regs[rtime];
                if time == 0 {
                    return;
                }

                let time = state.time + time;
                schedule.next_time = schedule.next_time.min(time);
                schedule.future.push(TimedEvent {
                    time,
                    pc: InstructionPtr(*pc),
                });
            } else {
                let region = region as usize;
                if region == 1 {
                    return;
                }

                schedule.regions[region as usize - 2].push(InstructionPtr(*pc));
            }
        }

        *pc = schedule.pop(&mut state.time).map_or(u64::MAX, |ptr| ptr.0);
    }
}

impl BytecodeInstruction for StartListen {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::StartListen as u8);
        let v = v.0;
        Self { index: v >> 8 }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::StartListen as u32 | (self.index << 8))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { index } = self;
        write_padded_mnemonic(f, "start_listen")?;
        write!(f, "{index}")
    }

    fn execute(
        self,
        _regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let i = self.index as usize;
        listeners.active[i / 64] |= 1u64 << (i % 64);
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
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rd", self.rd, LogicMode::TwoValue)?;
        Ok(())
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
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rcond", self.rcond, LogicMode::TwoValue)?;
        Ok(())
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
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rcond", self.rcond, LogicMode::TwoValue)?;
        Ok(())
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
        let i = self.idx.get(regs, Reg::X15);
        let word = (i / 64) as usize;
        let boff = (i % 64) as usize;
        let i = &mut state.tvl_first_write[word];
        regs[self.rcond] |= (*i >> boff) & 1;
        *i |= 1u64 << boff;
    }
}

impl BytecodeEncoder {
    pub fn wake(&mut self, rcond: Reg, index: InlineIndex<20>) {
        self.data.push(Wake { rcond, index }.encode());
    }

    pub fn wait(&mut self, rtime: Reg) {
        self.data.push(
            Reschedule {
                rtime,
                region: 0,
                schedule_self: true,
            }
            .encode(),
        );
    }

    pub fn wait_region(&mut self, region: u8) {
        self.data.push(
            Reschedule {
                rtime: Reg::X0,
                region: region + 1,
                schedule_self: true,
            }
            .encode(),
        );
    }

    pub fn next_event(&mut self) {
        self.data.push(
            Reschedule {
                rtime: Reg::X0,
                region: 0,
                schedule_self: false,
            }
            .encode(),
        );
    }

    pub fn start_listen(&mut self, index: u32) {
        self.data.push(StartListen { index }.encode());
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
