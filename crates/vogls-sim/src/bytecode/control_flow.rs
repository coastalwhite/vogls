use std::fmt;

use vogls_runtime::RuntimeState;

use crate::bytecode::{MNEMONIC_ALIGN, write_padded_mnemonic};

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule, SignedImmediate,
};

pub struct Jump(pub SignedImmediate<24>);
pub struct RelJump {
    pub rs: Reg,
    pub imm: SignedImmediate<20>,
}
pub struct Branch {
    pub rcond: Reg,
    pub imm: SignedImmediate<20>,
}

impl BytecodeInstruction for Jump {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Jump as u8);
        let v = v.0;
        Self(SignedImmediate::new_shifted(v, 8))
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::Jump as u32 | (self.0.encode() << 8))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(imm) = self;
        write_padded_mnemonic(f, "jump")?;
        fmt::Display::fmt(imm, f)?;
        Ok(())
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
        *pc = pc.wrapping_add_signed(i64::from(imm.0));
    }
}

impl BytecodeInstruction for RelJump {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::RelJump as u8);
        let v = v.0;
        Self {
            rs: Reg::new_masked(v >> 8),
            imm: SignedImmediate::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::RelJump as u32 | ((self.rs as u32) << 8) | (self.imm.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rs, imm } = self;
        write_padded_mnemonic(f, "reljump")?;
        write!(f, "{rs}, {imm}")?;
        Ok(())
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
        *pc = regs[rs].wrapping_add_signed(i64::from(imm.0));
    }
}

impl BytecodeInstruction for Branch {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Branch as u8);
        let v = v.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            imm: SignedImmediate::new_shifted(v, 12),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::Branch as u32 | ((self.rcond as u32) << 8) | (self.imm.encode() << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond: cond, imm } = self;
        write_padded_mnemonic(f, "branch")?;
        write!(f, "{cond}, {imm}")?;
        Ok(())
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
            *pc = pc.wrapping_add_signed(i64::from(imm.0));
        }
    }
}

impl BytecodeEncoder {
    pub fn jump(&mut self, imm: SignedImmediate<24>) {
        self.data.push(Jump(imm).encode());
    }
    pub fn reljump(&mut self, rs: Reg, imm: SignedImmediate<20>) {
        self.data.push(RelJump { rs, imm }.encode());
    }
    pub fn branch(&mut self, rcond: Reg, imm: SignedImmediate<20>) {
        self.data.push(Branch { rcond, imm }.encode());
    }
}
