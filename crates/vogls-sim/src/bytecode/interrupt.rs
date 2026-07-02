use std::fmt;

use vogls_runtime::RuntimeState;

use super::reg::Regs;
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule,
};

pub struct Interrupt;

impl BytecodeInstruction for Interrupt {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Interrupt as u8);
        Self
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::Interrupt as u32)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("interrupt")
    }

    fn execute(
        self,
        _regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        panic!()
    }
}

impl BytecodeEncoder {
    pub fn panic(&mut self) {
        self.data.push(Interrupt.encode());
    }
}
