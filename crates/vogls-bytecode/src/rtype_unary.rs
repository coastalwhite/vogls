use std::fmt;

use vogls_bits::arithmetic::fv_bitwise_inv_elem;
use vogls_bits::reduce::{fv_reduce_and_elem, fv_reduce_or_elem, fv_reduce_xor_elem};
use vogls_codegen::SixBitSize;
use vogls_ir::LogicMode;
use vogls_runtime::RuntimeState;

use crate::MNEMONIC_ALIGN;

use super::reg::{Reg, RegInfo, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule,
};

pub struct BitwiseRUType {
    pub rd: Reg,
    pub rs: Reg,
}
pub struct SbsBitwiseRUType {
    pub rd: Reg,
    pub rs: Reg,
    pub size: SixBitSize,
}

pub struct TvCountOnes(pub BitwiseRUType);
pub struct TvLeadingZeros(pub SbsBitwiseRUType);
pub struct FvNot(pub BitwiseRUType);
pub struct FvReduceAnd(pub SbsBitwiseRUType);
pub struct FvReduceOr(pub SbsBitwiseRUType);
pub struct FvReduceXor(pub SbsBitwiseRUType);
pub struct FvLeadingZeros(pub SbsBitwiseRUType);

impl BitwiseRUType {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(opcode as u32 | ((self.rd as u32) << 8) | ((self.rs as u32) << 12))
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs } = self;
        write!(f, "{rd}, {rs}")
    }
}
impl SbsBitwiseRUType {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.size.encode() << 16),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, size } = self;
        write!(f, "{rd}, {rs}, |{size}|")
    }
}

macro_rules! impl_bitwise {
    ($variant:ident, $mnemonic:literal, $rd_mode:ident, $rs_mode:ident) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(BitwiseRUType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
            operands.push(RegInfo::register(
                "rs",
                self.0.rs,
                LogicMode::$rs_mode,
                None,
            ));
        }
        fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::$rd_mode,
                None,
            ));
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:<1$}", $mnemonic, MNEMONIC_ALIGN)?;
            self.0.fmt(f)
        }
    };
}
macro_rules! impl_sbs_bitwise {
    ($variant:ident, $mnemonic:literal, $rd_mode:ident, $rs_mode:ident) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(SbsBitwiseRUType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
            operands.push(RegInfo::register(
                "rs",
                self.0.rs,
                LogicMode::$rs_mode,
                Some(self.0.size.into()),
            ));
        }
        fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::$rd_mode,
                Some(self.0.size.into()),
            ));
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:<1$}", $mnemonic, MNEMONIC_ALIGN)?;
            self.0.fmt(f)
        }
    };
}

impl BytecodeInstruction for TvCountOnes {
    impl_bitwise!(TvCountOnes, "tv.count_ones", TwoValue, TwoValue);

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
        let BitwiseRUType { rd, rs } = self.0;
        regs[rd] = regs[rs].count_ones().into();
    }
}

impl BytecodeInstruction for TvLeadingZeros {
    impl_sbs_bitwise!(TvLeadingZeros, "tv.leading_zeros", TwoValue, TwoValue);

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
        let SbsBitwiseRUType { rd, rs, size } = self.0;
        regs[rd] = u64::from(regs[rs].leading_zeros()).wrapping_sub(64 - size as u64);
    }
}

impl BytecodeInstruction for FvNot {
    impl_bitwise!(FvNot, "fv.not", FourValue, FourValue);

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
        let BitwiseRUType { rd, rs } = self.0;
        let (dspc, dval) = rd.to_spc_and_val();
        let (spc, val) = rs.to_spc_and_val();
        (regs[dspc], regs[dval]) = fv_bitwise_inv_elem(regs[spc], regs[val]);
    }
}
impl BytecodeInstruction for FvReduceAnd {
    impl_sbs_bitwise!(FvReduceAnd, "fv.reduce_and", FourValue, FourValue);

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
        let SbsBitwiseRUType { rd, rs, size } = self.0;
        let (dspc, dval) = rd.to_spc_and_val();
        let (spc, val) = rs.to_spc_and_val();
        let value = fv_reduce_and_elem(regs[spc], regs[val], size.into());
        (regs[dspc], regs[dval]) = (value.spc().into(), value.val().into());
    }
}
impl BytecodeInstruction for FvReduceOr {
    impl_sbs_bitwise!(FvReduceOr, "fv.reduce_or", FourValue, FourValue);

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
        let SbsBitwiseRUType { rd, rs, size } = self.0;
        let (dspc, dval) = rd.to_spc_and_val();
        let (spc, val) = rs.to_spc_and_val();
        let value = fv_reduce_or_elem(regs[spc], regs[val], size.into());
        (regs[dspc], regs[dval]) = (value.spc().into(), value.val().into());
    }
}
impl BytecodeInstruction for FvReduceXor {
    impl_sbs_bitwise!(FvReduceXor, "fv.reduce_xor", FourValue, FourValue);

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
        let SbsBitwiseRUType { rd, rs, size } = self.0;
        let (dspc, dval) = rd.to_spc_and_val();
        let (spc, val) = rs.to_spc_and_val();
        let value = fv_reduce_xor_elem(regs[spc], regs[val], size.into());
        (regs[dspc], regs[dval]) = (value.spc().into(), value.val().into());
    }
}

impl BytecodeInstruction for FvLeadingZeros {
    impl_sbs_bitwise!(FvLeadingZeros, "fv.leading_zeros", FourValue, FourValue);

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
        let SbsBitwiseRUType { rd, rs, size } = self.0;
        let (rdspc, rdval) = rd.to_spc_and_val();
        let (rsspc, rsval) = rs.to_spc_and_val();

        if regs[rsspc] != size.mask(u64::MAX) {
            regs[rdspc] = 0;
            regs[rdval] = 0;
            return;
        }

        regs[rdspc] = u64::from(u32::MAX);
        regs[rdval] = u64::from(regs[rsval].leading_zeros()).wrapping_sub(64 - size as u64);
    }
}

macro_rules! impl_bytecode_methods {
    ($(($name:ident, $op:ident))*) => {
        impl BytecodeEncoder {
            $(pub fn $name(&mut self, rd: Reg, rs: Reg) {
                self.data.push($op(BitwiseRUType { rd, rs }).encode());
            })*
        }
    };
}
macro_rules! impl_bytecode_sbs_methods {
    ($(($name:ident, $op:ident))*) => {
        impl BytecodeEncoder {
            $(pub fn $name(&mut self, rd: Reg, rs: Reg, size: SixBitSize) {
                self.data.push($op(SbsBitwiseRUType { rd, rs,size }).encode());
            })*
        }
    };
}

impl_bytecode_methods! {
    (count_ones, TvCountOnes)
    (fv_not, FvNot)
}

impl_bytecode_sbs_methods! {
    (leading_zeros, TvLeadingZeros)
    (fv_reduce_and, FvReduceAnd)
    (fv_reduce_or, FvReduceOr)
    (fv_reduce_xor, FvReduceXor)
    (fv_leading_zeros, FvLeadingZeros)
}
