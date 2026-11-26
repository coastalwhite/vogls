use std::fmt;

use vogls_bits::SixBitSize;
use vogls_ir::LogicMode;
use vogls_runtime::RuntimeState;

use crate::{
    SignedImmediate, decode_bytecode_u64, encode_bytecode_u64, safe_additional_slots,
    write_padded_mnemonic,
};

use super::reg::{Reg, RegInfo, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule,
};

/// Loads a constant of `52` bits into register `rd`.
pub struct LoadSlot {
    rd: Reg,
    imm20: u32,
}

/// Loads a constant of `64` bits into register `rd`.
pub struct LoadSlots {
    rd: Reg,
    // 20 unused bits
}

/// Loads a constant of 20 bits into register `rd`.
///
/// The constant is sign-extended from 20 to 64 bits.
pub struct LoadI20 {
    rd: Reg,
    imm20: i32,
}

/// Loads constant special and value planes of 10 bits into register `rd`,`rd+1`.
///
/// The constants are sign-extended from 10 to 64 bits.
pub struct Load2I10 {
    rd: Reg,
    spc: i16,
    val: i16,
}

impl BytecodeInstruction for LoadSlot {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::LoadSlot as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            imm20: v >> 12,
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        debug_assert!(self.imm20 < (1 << 20));
        Bytecode(BytecodeOpcode::LoadSlot as u32 | ((self.rd as u32) << 8) | (self.imm20 << 12))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, imm20 } = self;
        write_padded_mnemonic(f, "load_slot")?;
        write!(f, "{rd}, {imm20:#x}")
    }

    fn num_additional_slots(&self) -> u8 {
        1
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
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd, imm20 } = self;

        let additional_slots = safe_additional_slots(*pc, code);
        *pc += self.num_additional_slots() as u64;

        let value = u64::from(imm20) | (u64::from(additional_slots[0].0) << 20);
        regs[rd] = value;
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register("rd", self.rd, LogicMode::TwoValue, None));
    }
}

impl BytecodeInstruction for LoadSlots {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::LoadSlots as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::LoadSlots as u32 | ((self.rd as u32) << 8))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd } = self;
        write_padded_mnemonic(f, "load_slots")?;
        write!(f, "{rd}")
    }

    fn num_additional_slots(&self) -> u8 {
        2
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
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd } = self;

        let additional_slots = safe_additional_slots(*pc, code);
        *pc += self.num_additional_slots() as u64;

        let value = decode_bytecode_u64(additional_slots);
        regs[rd] = value;
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register("rd", self.rd, LogicMode::TwoValue, None));
    }
}

impl BytecodeInstruction for LoadI20 {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::LoadI20 as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            imm20: (v as i32) >> 12,
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        debug_assert!((-(1i32 << 19)..(1i32 << 19)).contains(&self.imm20));
        Bytecode(
            BytecodeOpcode::LoadI20 as u32
                | ((self.rd as u32) << 8)
                | (((self.imm20 as u32) & 0xF_FFFF) << 12),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, imm20 } = self;
        write_padded_mnemonic(f, "load_i20")?;
        write!(f, "{rd}, {imm20}")
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
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd, imm20 } = self;
        regs[rd] = i64::from(imm20) as u64;
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register("rd", self.rd, LogicMode::TwoValue, None));
    }
}

impl BytecodeInstruction for Load2I10 {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::Load2I10 as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            spc: (((v << 10) as i32) >> 22) as i16,
            val: ((v as i32) >> 22) as i16,
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        debug_assert!(
            SixBitSize::N10.fits_signed(self.spc) && SixBitSize::N10.fits_signed(self.val)
        );
        Bytecode(
            BytecodeOpcode::Load2I10 as u32
                | ((self.rd as u32) << 8)
                | (((self.spc as u32) & 0x3FF) << 12)
                | (((self.val as u32) & 0x3FF) << 22),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, spc, val } = self;
        write_padded_mnemonic(f, "load_2i10")?;
        write!(f, "{rd}, {spc}, {val}")
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
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd, spc, val } = self;
        let (rdspc, rdval) = rd.to_spc_and_val();
        regs[rdspc] = i64::from(spc) as u64;
        regs[rdval] = i64::from(val) as u64;
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register("rd", self.rd, LogicMode::FourValue, None));
    }
}

impl BytecodeEncoder {
    pub fn load_i20(&mut self, rd: Reg, imm20: i32) {
        debug_assert!(SixBitSize::N20.fits_signed(imm20));
        self.data.push(LoadI20 { rd, imm20 }.encode());
    }

    pub fn load_2i10(&mut self, rd: Reg, spc: i16, val: i16) {
        debug_assert!(SixBitSize::N10.fits_signed(spc));
        debug_assert!(SixBitSize::N10.fits_signed(val));
        self.data.push(Load2I10 { rd, spc, val }.encode());
    }

    /// Loads the `value` into `rd` by choosing an efficient instructions.
    pub fn load_u64(&mut self, rd: Reg, value: u64) {
        if SixBitSize::N20.fits_signed(value as i64) {
            self.load_i20(rd, value as i64 as i32);
        } else if value.count_ones() == value.trailing_ones() {
            self.ori(
                rd,
                rd,
                SignedImmediate::MINUS_ONE,
                SixBitSize::new_masked(value.count_ones() - 1),
            );
        } else if value.leading_zeros() >= 12 {
            let imm20 = (value & 0xF_FFFF) as u32;
            let imm52_20 = (value >> 20) as u32;
            self.data.push(LoadSlot { rd, imm20 }.encode());
            self.data.push(Bytecode(imm52_20));
        } else {
            self.data.push(LoadSlots { rd }.encode());
            encode_bytecode_u64(self, value);
        }
    }

    /// Loads the `spc` and `val` into `rd`, `rd+1` by choosing an efficient instructions.
    pub fn load_fv(&mut self, rd: Reg, spc: u64, val: u64) {
        if SixBitSize::N10.fits_signed(spc as i64) && SixBitSize::N10.fits_signed(val as i64) {
            self.load_2i10(rd, spc as i64 as i16, val as i64 as i16);
        } else {
            let (rdspc, rdval) = rd.to_spc_and_val();
            self.load_u64(rdspc, spc);
            self.load_u64(rdval, val);
        }
    }
}
