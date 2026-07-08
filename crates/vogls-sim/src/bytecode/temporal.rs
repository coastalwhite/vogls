use vogls_ir::LogicMode;
use vogls_runtime::RuntimeState;
use vogls_utils::NonMaxU32;

use std::fmt;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, InstructionPtr, Schedule, TimedEvent, write_padded_mnemonic,
    write_register,
};

pub struct Wake {
    rcond: Reg,
    index: u32,
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
    idx: Option<NonMaxU32>,
}

impl BytecodeInstruction for Wake {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Wake as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            index: v >> 12,
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
            BytecodeOpcode::Wake as u32 | ((self.rcond as u32) << 8) | ((self.index as u32) << 12),
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
            let i = index as usize;
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
            idx: NonMaxU32::new(v >> 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::LastUpdateTime as u32
                | ((self.rd as u32) << 8)
                | (self.idx.map_or(0, |v| v.get()) << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, idx } = self;
        write_padded_mnemonic(f, "lupdt")?;
        write!(f, "{rd}, ")?;
        match idx {
            None => f.write_str("|...|"),
            Some(v) => fmt::Display::fmt(v, f),
        }
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
        let i = self.idx.map_or_else(|| regs[Reg::X12], |v| v.get() as u64);
        regs[self.rd] = state.last_active_time[i as usize];
    }
}

impl BytecodeEncoder {
    pub fn wake(&mut self, rcond: Reg, index: u32) {
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

    pub fn last_update_time(&mut self, rd: Reg, index: u64) {
        if index >= (1u64 << 20) {
            self.load_u64(Reg::X12, index);
            self.data.push(LastUpdateTime { rd, idx: None }.encode());
        } else {
            self.data.push(
                LastUpdateTime {
                    rd,
                    idx: Some(NonMaxU32::new(index as u32).unwrap()),
                }
                .encode(),
            );
        }
    }
}
