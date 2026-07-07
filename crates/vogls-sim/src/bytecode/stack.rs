use vogls_codegen::lsra::StackItemKind;
use vogls_runtime::RuntimeState;

use std::fmt;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule, SignedImmediate, write_padded_mnemonic,
};

pub struct StackOffset {
    rd: Reg,
    kind: StackItemKind,
    offset: SignedImmediate<17>,
}

impl BytecodeInstruction for StackOffset {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::StackOffset as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            kind: match (v >> 12) & 0x7 {
                0 => StackItemKind::B1,
                1 => StackItemKind::B2,
                2 => StackItemKind::B4,
                3 => StackItemKind::B8,
                4 => StackItemKind::B16,
                5 => StackItemKind::B32,
                _ => StackItemKind::B64,
            },
            offset: SignedImmediate::new_shifted(v, 15),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::StackOffset as u32
                | ((self.rd as u32) << 8)
                | ((self.kind as u32) << 12)
                | (self.offset.encode() << 15),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "stack_offset")?;
        let Self { rd, kind, offset } = self;
        let kind = 1u32 << *kind as u32;
        write!(f, "{rd}, [{kind}]{offset}")
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
        let Self { rd, kind, offset } = self;
        let offset = offset.0 << kind as u32;
        regs[rd] = regs.stack_offset.wrapping_add_signed(i64::from(offset));
    }
}

impl BytecodeEncoder {
    pub fn stack_offset(&mut self, rd: Reg, kind: StackItemKind, offset: SignedImmediate<17>) {
        self.data.push(StackOffset { rd, kind, offset }.encode());
    }
}
