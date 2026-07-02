use std::fmt;

use vogls_runtime::RuntimeState;

use crate::bytecode::MNEMONIC_ALIGN;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext, Schedule
};

pub struct Jump(pub i32);
pub struct RelJump {
    pub rs: Reg,
    pub imm: i32,
}
pub struct Branch {
    pub rcond: Reg,
    pub imm: i32,
}

impl BytecodeInstruction for Jump {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Jump as u8);
        let v = v.0;
        Self((v as i32) >> 8)
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::Jump as u32 | (self.0 as u32) << 8)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(imm) = self;
        write!(f, "{:<1$}{imm}", "jump", MNEMONIC_ALIGN)
    }

    fn execute(
        self,
        _regs: &mut Regs,
        pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self(imm) = self;
        *pc = pc.wrapping_sub(1).wrapping_add_signed(i64::from(imm));
    }
}

impl BytecodeInstruction for RelJump {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::RelJump as u8);
        let v = v.0;
        Self {
            rs: Reg::new_masked(v >> 8),
            imm: (v as i32) >> 12,
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::RelJump as u32 | ((self.rs as u32) << 8) | (self.imm as u32) << 12)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rs, imm } = self;
        write!(f, "{:<1$}{rs}, {imm}", "reljump", MNEMONIC_ALIGN)
    }

    fn execute(
        self,
        regs: &mut Regs,
        pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rs, imm } = self;
        *pc = regs[rs].wrapping_add_signed(i64::from(imm));
    }
}

impl BytecodeInstruction for Branch {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Branch as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            imm: (v as i32) >> 12,
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::Branch as u32 | ((self.rcond as u32) << 8) | (self.imm as u32) << 12,
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond: cond, imm } = self;
        write!(f, "{:<1$}{cond}, {imm}", "jump", MNEMONIC_ALIGN)
    }

    fn execute(
        self,
        regs: &mut Regs,
        pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rcond: cond, imm } = self;
        if regs[cond] != 0 {
            *pc = pc.wrapping_sub(1).wrapping_add_signed(i64::from(imm));
        }
    }
}

impl BytecodeEncoder {
    pub fn jump(&mut self, imm: i32) {
        self.data.push(Jump(imm).encode());
    }
    pub fn reljump(&mut self, rs: Reg, imm: i32) {
        self.data.push(RelJump { rs, imm }.encode());
    }
    pub fn branch(&mut self, rcond: Reg, imm: i32) {
        self.data.push(Branch { rcond, imm }.encode());
    }
}
