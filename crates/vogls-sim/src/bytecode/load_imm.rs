use std::fmt;

use vogls_runtime::RuntimeState;

use crate::bytecode::MNEMONIC_ALIGN;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule,
};

pub struct LoadImm {
    rd: Reg,
    clear: bool,
    sign_extend: bool,
    segment: u8,
    imm: i16,
}

impl BytecodeInstruction for LoadImm {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::LoadImm as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            clear: (v >> 12) & 1 != 0,
            sign_extend: (v >> 13) & 1 != 0,
            segment: ((v >> 14) & 0x3) as u8,
            imm: ((v as i32) >> 16) as i16,
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::LoadImm as u32
                | ((self.rd as u32) << 8)
                | ((self.clear as u32) << 12)
                | ((self.sign_extend as u32) << 13)
                | ((self.segment as u32) << 14)
                | ((self.imm as u16 as u32) << 16),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            clear,
            sign_extend,
            segment,
            imm,
        } = self;
        write!(
            f,
            "{:<1$}{rd}, {imm}, c:{clear}, e:{sign_extend}, seg:{segment}",
            "load_imm", MNEMONIC_ALIGN
        )
    }

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            clear,
            sign_extend,
            segment,
            imm,
        } = self;
        if clear {
            regs[rd] = 0;
        }
        let imm = if sign_extend {
            i64::from(imm) as u64
        } else {
            imm as u16 as u64
        };
        regs[rd] |= imm << (segment * 16);
    }
}

impl BytecodeEncoder {
    pub fn load_imm16(&mut self, rd: Reg, clear: bool, sign_extend: bool, segment: u8, imm: i16) {
        self.data.push(
            LoadImm {
                rd,
                clear,
                sign_extend,
                segment,
                imm,
            }
            .encode(),
        );
    }
}
