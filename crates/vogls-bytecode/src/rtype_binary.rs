use std::fmt;

use vogls_bits::arithmetic::{
    fv_bitwise_and_elem, fv_bitwise_andnot_elem, fv_bitwise_or_elem, fv_bitwise_ornot_elem,
    fv_bitwise_xnor_elem, fv_bitwise_xor_elem,
};
use vogls_bits::copyxz::{copy_x, copy_z};
use vogls_bits::edge::{fv_negedge_u64, fv_posedge_u64};
use vogls_bits::shift::{fv_logical_shift_left, fv_logical_shift_right, fv_shift_arith_right, tv_shift_arith_right};
use vogls_bits::util::wrapping_u64_pow;
use vogls_ir::LogicMode;
use vogls_runtime::RuntimeState;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, Schedule, SixBitSize, write_padded_mnemonic, write_register,
};

pub struct BitwiseRType {
    pub rd: Reg,
    pub rs1: Reg,
    pub rs2: Reg,
}
pub struct SbsBitwiseRType {
    pub rd: Reg,
    pub rs1: Reg,
    pub rs2: Reg,
    pub size: SixBitSize,
}
pub struct FvShiftRType {
    pub rd: Reg,
    pub rs1: Reg,
    pub rs2: Reg,
    pub size: SixBitSize,
    pub offset_mode: LogicMode,
}

pub struct TvAnd(pub BitwiseRType);
pub struct TvOr(pub BitwiseRType);
pub struct TvXor(pub BitwiseRType);
pub struct TvAndNot(pub SbsBitwiseRType);
pub struct TvOrNot(pub SbsBitwiseRType);
pub struct TvXnor(pub SbsBitwiseRType);
pub struct TvCeq(pub BitwiseRType);
pub struct TvAdd(pub SbsBitwiseRType);
pub struct TvSub(pub SbsBitwiseRType);
pub struct TvMul(pub SbsBitwiseRType);
pub struct TvDivX(pub SbsBitwiseRType);
pub struct TvDiv0(pub SbsBitwiseRType);
pub struct TvModX(pub SbsBitwiseRType);
pub struct TvMod0(pub SbsBitwiseRType);
pub struct TvPow(pub SbsBitwiseRType);
pub struct TvUnsignedLeq(pub BitwiseRType);
pub struct TvUnsignedGt(pub BitwiseRType);
pub struct TvMin(pub BitwiseRType);
pub struct TvMax(pub BitwiseRType);
pub struct TvSll(pub SbsBitwiseRType);
pub struct TvSlr(pub SbsBitwiseRType);
pub struct TvSar(pub SbsBitwiseRType);
pub struct TvLeftShiftOr(pub SbsBitwiseRType);
pub struct CMov(pub BitwiseRType);

pub struct FvAnd(pub BitwiseRType);
pub struct FvOr(pub BitwiseRType);
pub struct FvXor(pub BitwiseRType);
pub struct FvAndNot(pub BitwiseRType);
pub struct FvOrNot(pub BitwiseRType);
pub struct FvXnor(pub BitwiseRType);
pub struct FvCeq(pub BitwiseRType);
pub struct FvBitwiseCeq(pub SbsBitwiseRType);
pub struct FvAdd(pub SbsBitwiseRType);
pub struct FvSub(pub SbsBitwiseRType);
pub struct FvMul(pub SbsBitwiseRType);
pub struct FvDivX(pub SbsBitwiseRType);
pub struct FvDiv0(pub SbsBitwiseRType);
pub struct FvModX(pub SbsBitwiseRType);
pub struct FvMod0(pub SbsBitwiseRType);
pub struct FvPow(pub SbsBitwiseRType);
pub struct FvPosedge(pub BitwiseRType);
pub struct FvNegedge(pub BitwiseRType);
pub struct FvUnsignedLeq(pub SbsBitwiseRType);
pub struct FvUnsignedGt(pub SbsBitwiseRType);
pub struct FvMin(pub SbsBitwiseRType);
pub struct FvMax(pub SbsBitwiseRType);
pub struct FvSll(pub FvShiftRType);
pub struct FvSlr(pub FvShiftRType);
pub struct FvSar(pub FvShiftRType);
pub struct FvSlrx(pub FvShiftRType);
pub struct FvLeftShiftOr(pub SbsBitwiseRType);
pub struct FvCopyX(pub SbsBitwiseRType);
pub struct FvCopyZ(pub SbsBitwiseRType);

impl BitwiseRType {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs1, rs2 } = self;
        write!(f, "{rd}, {rs1}, {rs2}")
    }
}
impl SbsBitwiseRType {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | (self.size.encode() << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs1, rs2, size } = self;
        write!(f, "{rd}, {rs1}, {rs2}, |{size}|")
    }
}
impl FvShiftRType {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
            offset_mode: if (v >> 26) & 1 != 0 {
                LogicMode::FourValue
            } else {
                LogicMode::TwoValue
            },
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | (self.size.encode() << 20)
                | ((self.offset_mode as u32) << 26),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs1,
            rs2,
            size,
            offset_mode: _,
        } = self;
        write!(f, "{rd}, {rs1}, {rs2}, |{size}|")
    }
}

macro_rules! impl_bitwise {
    ($variant:ident, $mnemonic:literal, $rd_mode:ident, $rs1_mode:ident, $rs2_mode:ident) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(BitwiseRType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
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
            write_register(f, regs, "rs1", self.0.rs1, LogicMode::$rs1_mode)?;
            f.write_str(", ")?;
            write_register(f, regs, "rs2", self.0.rs2, LogicMode::$rs2_mode)?;
            writeln!(f)?;
            Ok(())
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
            write_register(f, regs, "rd", self.0.rd, LogicMode::$rd_mode)?;
            writeln!(f)?;
            Ok(())
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}
macro_rules! impl_sbs_bitwise {
    ($variant:ident, $mnemonic:literal, $rd_mode:ident, $rs1_mode:ident, $rs2_mode:ident) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(SbsBitwiseRType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
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
            write_register(f, regs, "rs1", self.0.rs1, LogicMode::$rs1_mode)?;
            f.write_str(", ")?;
            write_register(f, regs, "rs2", self.0.rs2, LogicMode::$rs2_mode)?;
            writeln!(f)?;
            Ok(())
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
            write_register(f, regs, "rd", self.0.rd, LogicMode::$rd_mode)?;
            writeln!(f)?;
            Ok(())
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}
macro_rules! impl_fv_shift {
    ($variant:ident, $mnemonic:literal, $rd_mode:ident, $rs1_mode:ident, $rs2_mode:ident) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(FvShiftRType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
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
            write_register(f, regs, "rs1", self.0.rs1, LogicMode::$rs1_mode)?;
            f.write_str(", ")?;
            write_register(f, regs, "rs2", self.0.rs2, self.0.offset_mode)?;
            writeln!(f)?;
            Ok(())
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
            write_register(f, regs, "rd", self.0.rd, LogicMode::$rd_mode)?;
            writeln!(f)?;
            Ok(())
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}

impl BytecodeInstruction for CMov {
    impl_bitwise!(CMov, "cmov", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        if regs[rs1] != 0 {
            regs[rd] = regs[rs2];
        }
    }
}
impl BytecodeInstruction for TvAnd {
    impl_bitwise!(TvAnd, "tv.and", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = regs[rs1] & regs[rs2];
    }
}
impl BytecodeInstruction for TvOr {
    impl_bitwise!(TvOr, "tv.or", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = regs[rs1] | regs[rs2];
    }
}
impl BytecodeInstruction for TvXor {
    impl_bitwise!(TvXor, "tv.xor", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = regs[rs1] ^ regs[rs2];
    }
}
impl BytecodeInstruction for TvAndNot {
    impl_sbs_bitwise!(TvAndNot, "tv.andnot", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(regs[rs1] & !regs[rs2]);
    }
}
impl BytecodeInstruction for TvOrNot {
    impl_sbs_bitwise!(TvOrNot, "tv.ornot", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(regs[rs1] | !regs[rs2]);
    }
}
impl BytecodeInstruction for TvXnor {
    impl_sbs_bitwise!(TvXnor, "tv.xnor", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(!(regs[rs1] ^ regs[rs2]));
    }
}
impl BytecodeInstruction for TvCeq {
    impl_bitwise!(TvCeq, "tv.ceq", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = u64::from(regs[rs1] == regs[rs2]);
    }
}
impl BytecodeInstruction for TvAdd {
    impl_sbs_bitwise!(TvAdd, "tv.add", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(regs[rs1].wrapping_add(regs[rs2]));
    }
}
impl BytecodeInstruction for TvSub {
    impl_sbs_bitwise!(TvSub, "tv.sub", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(regs[rs1].wrapping_sub(regs[rs2]));
    }
}
impl BytecodeInstruction for TvMul {
    impl_sbs_bitwise!(TvMul, "tv.mul", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(regs[rs1].wrapping_mul(regs[rs2]));
    }
}
impl BytecodeInstruction for TvDivX {
    impl_sbs_bitwise!(TvDivX, "tv.divx", FourValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        match regs[rs1].checked_div(regs[rs2]) {
            None => {
                regs[rd_spc] = 0;
                regs[rd_val] = 0;
            }
            Some(value) => {
                regs[rd_spc] = size.mask(u64::MAX);
                regs[rd_val] = value;
            }
        }
    }
}
impl BytecodeInstruction for TvDiv0 {
    impl_sbs_bitwise!(TvDiv0, "tv.div0", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType {
            rd,
            rs1,
            rs2,
            size: _,
        } = self.0;
        regs[rd] = regs[rs1].checked_div(regs[rs2]).unwrap_or_default();
    }
}
impl BytecodeInstruction for TvModX {
    impl_sbs_bitwise!(TvModX, "tv.modx", FourValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        match regs[rs1].checked_rem(regs[rs2]) {
            None => {
                regs[rd_spc] = 0;
                regs[rd_val] = 0;
            }
            Some(value) => {
                regs[rd_spc] = size.mask(u64::MAX);
                regs[rd_val] = value;
            }
        }
    }
}
impl BytecodeInstruction for TvMod0 {
    impl_sbs_bitwise!(TvMod0, "tv.mod0", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType {
            rd,
            rs1,
            rs2,
            size: _,
        } = self.0;
        regs[rd] = regs[rs1].checked_div(regs[rs2]).unwrap_or_default();
    }
}
impl BytecodeInstruction for TvPow {
    impl_sbs_bitwise!(TvPow, "tv.pow", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(wrapping_u64_pow(regs[rs1], regs[rs2]));
    }
}
impl BytecodeInstruction for TvUnsignedLeq {
    impl_bitwise!(TvUnsignedLeq, "tv.uleq", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = u64::from(regs[rs1] <= regs[rs2]);
    }
}
impl BytecodeInstruction for TvUnsignedGt {
    impl_bitwise!(TvUnsignedGt, "tv.ugt", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = u64::from(regs[rs1] > regs[rs2]);
    }
}
impl BytecodeInstruction for TvMin {
    impl_bitwise!(TvMin, "tv.min", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = u64::min(regs[rs1], regs[rs2]);
    }
}
impl BytecodeInstruction for TvMax {
    impl_bitwise!(TvMax, "tv.max", TwoValue, TwoValue, TwoValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        regs[rd] = u64::max(regs[rs1], regs[rs2]);
    }
}
impl BytecodeInstruction for TvSll {
    impl_sbs_bitwise!(TvSll, "tv.sll", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(regs[rs1].unbounded_shl(regs[rs2] as u32));
    }
}
impl BytecodeInstruction for TvSlr {
    impl_sbs_bitwise!(TvSlr, "tv.slr", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = size.mask(regs[rs1].unbounded_shr(regs[rs2] as u32));
    }
}
impl BytecodeInstruction for TvSar {
    impl_sbs_bitwise!(TvSar, "tv.sar", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = tv_shift_arith_right(regs[rs1], regs[rs2] as u32, size.into());
    }
}
impl BytecodeInstruction for TvLeftShiftOr {
    impl_sbs_bitwise!(TvLeftShiftOr, "tv.lsor", TwoValue, TwoValue, TwoValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        regs[rd] = (regs[rs1] << size as u32) | regs[rs2];
    }
}

impl BytecodeInstruction for FvAnd {
    impl_bitwise!(FvAnd, "fv.and", FourValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_and_elem(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvOr {
    impl_bitwise!(FvOr, "fv.or", FourValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_or_elem(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvXor {
    impl_bitwise!(FvXor, "fv.xor", FourValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_xor_elem(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvAndNot {
    impl_bitwise!(FvAndNot, "fv.andnot", FourValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_andnot_elem(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvOrNot {
    impl_bitwise!(FvOrNot, "fv.ornot", FourValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_ornot_elem(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvXnor {
    impl_bitwise!(FvXnor, "fv.xnor", FourValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_xnor_elem(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvCeq {
    impl_bitwise!(FvCeq, "fv.ceq", TwoValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        regs[rd] = u64::from((regs[rs1_spc] == regs[rs2_spc]) & (regs[rs1_val] == regs[rs2_val]));
    }
}
impl BytecodeInstruction for FvBitwiseCeq {
    impl_sbs_bitwise!(
        FvBitwiseCeq,
        "fv.bitwise_ceq",
        TwoValue,
        FourValue,
        FourValue
    );

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        regs[rd] = size.mask((regs[rs1_spc] ^ !regs[rs2_spc]) & (regs[rs1_val] ^ !regs[rs2_val]));
    }
}
impl BytecodeInstruction for FvPosedge {
    impl_bitwise!(FvPosedge, "fv.posedge", TwoValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        regs[rd] = fv_posedge_u64(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvNegedge {
    impl_bitwise!(FvNegedge, "fv.negedge", TwoValue, FourValue, FourValue);

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
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        regs[rd] = fv_negedge_u64(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvAdd {
    impl_sbs_bitwise!(FvAdd, "fv.add", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = regs[rs1_val].wrapping_add(regs[rs2_val]) & mask;
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvSub {
    impl_sbs_bitwise!(FvSub, "fv.sub", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = regs[rs1_val].wrapping_sub(regs[rs2_val]) & mask;
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvMul {
    impl_sbs_bitwise!(FvMul, "fv.mul", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = regs[rs1_val].wrapping_mul(regs[rs2_val]) & mask;
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvDivX {
    impl_sbs_bitwise!(FvDivX, "fv.divx", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask && regs[rs2_val] != 0 {
            regs[rd_spc] = mask;
            regs[rd_val] = regs[rs1_val] / regs[rs2_val];
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvDiv0 {
    impl_sbs_bitwise!(FvDiv0, "fv.div0", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = regs[rs1_val].checked_div(regs[rs2_val]).unwrap_or_default();
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvModX {
    impl_sbs_bitwise!(FvModX, "fv.modx", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask && regs[rs2_val] != 0 {
            regs[rd_spc] = mask;
            regs[rd_val] = regs[rs1_val] % regs[rs2_val];
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvMod0 {
    impl_sbs_bitwise!(FvMod0, "fv.mod0", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = regs[rs1_val].checked_rem(regs[rs2_val]).unwrap_or_default();
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvPow {
    impl_sbs_bitwise!(FvPow, "fv.pow", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = wrapping_u64_pow(regs[rs1_val], regs[rs2_val]) & mask;
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvUnsignedLeq {
    impl_sbs_bitwise!(FvUnsignedLeq, "fv.uleq", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = 1;
            regs[rd_val] = u64::from(regs[rs1_val] <= regs[rs2_val]);
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvUnsignedGt {
    impl_sbs_bitwise!(FvUnsignedGt, "fv.ugt", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = 1;
            regs[rd_val] = u64::from(regs[rs1_val] > regs[rs2_val]);
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvMin {
    impl_sbs_bitwise!(FvMin, "fv.min", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = u64::min(regs[rs1_val], regs[rs2_val]);
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvMax {
    impl_sbs_bitwise!(FvMax, "fv.max", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();

        let mask = size.mask(u64::MAX);

        if regs[rs1_spc] == mask && regs[rs2_spc] == mask {
            regs[rd_spc] = mask;
            regs[rd_val] = u64::min(regs[rs1_val], regs[rs2_val]);
        } else {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        }
    }
}
impl BytecodeInstruction for FvSll {
    impl_fv_shift!(FvSll, "fv.sll", FourValue, FourValue, FourValue);

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
        let FvShiftRType {
            rd,
            rs1,
            rs2,
            size,
            offset_mode,
        } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();

        let shift = match offset_mode {
            LogicMode::TwoValue => regs[rs2] as u32,
            LogicMode::FourValue => {
                let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
                if regs[rs2_spc] != u32::MAX as u64 {
                    regs[rd_spc] = 0;
                    regs[rd_val] = 0;
                    return;
                }
                regs[rs2_val] as u32
            }
        };
        (regs[rd_spc], regs[rd_val]) =
            fv_logical_shift_left(regs[rs1_spc], regs[rs1_val], shift, size.into());
    }
}
impl BytecodeInstruction for FvSlr {
    impl_fv_shift!(FvSlr, "fv.slr", FourValue, FourValue, FourValue);

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
        let FvShiftRType {
            rd,
            rs1,
            rs2,
            size,
            offset_mode,
        } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();

        let shift = match offset_mode {
            LogicMode::TwoValue => regs[rs2] as u32,
            LogicMode::FourValue => {
                let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
                if regs[rs2_spc] != u32::MAX as u64 {
                    regs[rd_spc] = 0;
                    regs[rd_val] = 0;
                    return;
                }
                regs[rs2_val] as u32
            }
        };

        (regs[rd_spc], regs[rd_val]) =
            fv_logical_shift_right(regs[rs1_spc], regs[rs1_val], shift, size.into());
    }
}
impl BytecodeInstruction for FvSar {
    impl_fv_shift!(FvSar, "fv.sar", FourValue, FourValue, FourValue);

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
        let FvShiftRType {
            rd,
            rs1,
            rs2,
            size,
            offset_mode,
        } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();

        let shift = match offset_mode {
            LogicMode::TwoValue => regs[rs2] as u32,
            LogicMode::FourValue => {
                let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
                if regs[rs2_spc] != u32::MAX as u64 {
                    regs[rd_spc] = 0;
                    regs[rd_val] = 0;
                    return;
                }
                regs[rs2_val] as u32
            }
        };

        (regs[rd_spc], regs[rd_val]) =
            fv_shift_arith_right(regs[rs1_spc], regs[rs1_val], shift, size.into());
    }
}
impl BytecodeInstruction for FvSlrx {
    impl_fv_shift!(FvSlrx, "fv.slrx", FourValue, FourValue, FourValue);

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
        let FvShiftRType {
            rd,
            rs1,
            rs2,
            size,
            offset_mode,
        } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();

        let shift = match offset_mode {
            LogicMode::TwoValue => regs[rs2] as u32,
            LogicMode::FourValue => {
                let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
                if regs[rs2_spc] != u32::MAX as u64 {
                    regs[rd_spc] = 0;
                    regs[rd_val] = 0;
                    return;
                }
                regs[rs2_val] as u32
            }
        };

        regs[rd_spc] = size.mask(regs[rs1_spc].unbounded_shr(shift));
        regs[rd_val] = size.mask(regs[rs1_val].unbounded_shr(shift));
    }
}
impl BytecodeInstruction for FvLeftShiftOr {
    impl_sbs_bitwise!(FvLeftShiftOr, "fv.lsor", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        regs[rd_spc] = (regs[rs1_spc] << size as u32) | regs[rs2_spc];
        regs[rd_val] = (regs[rs1_val] << size as u32) | regs[rs2_val];
    }
}
impl BytecodeInstruction for FvCopyX {
    impl_sbs_bitwise!(FvCopyX, "fv.copyx", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType {
            rd,
            rs1,
            rs2,
            size: _,
        } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            copy_x(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}
impl BytecodeInstruction for FvCopyZ {
    impl_sbs_bitwise!(FvCopyZ, "fv.copyz", FourValue, FourValue, FourValue);

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
        let SbsBitwiseRType {
            rd,
            rs1,
            rs2,
            size: _,
        } = self.0;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs1_spc, rs1_val) = rs1.to_spc_and_val();
        let (rs2_spc, rs2_val) = rs2.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            copy_z(regs[rs1_spc], regs[rs1_val], regs[rs2_spc], regs[rs2_val]);
    }
}

macro_rules! impl_bytecode_methods {
    ($(($name:ident, $op:ident))*) => {
        impl BytecodeEncoder {
            $(pub fn $name(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
                self.data.push($op(BitwiseRType { rd, rs1, rs2 }).encode());
            })*
        }
    };
}
macro_rules! impl_bytecode_sbs_methods {
    ($(($name:ident, $op:ident))*) => {
        impl BytecodeEncoder {
            $(pub fn $name(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize) {
                self.data.push($op(SbsBitwiseRType { rd, rs1, rs2, size }).encode());
            })*
        }
    };
}
macro_rules! impl_bytecode_fv_shift_methods {
    ($(($name:ident, $op:ident))*) => {
        impl BytecodeEncoder {
            $(pub fn $name(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize, offset_mode: LogicMode) {
                self.data.push($op(FvShiftRType { rd, rs1, rs2, size, offset_mode }).encode());
            })*
        }
    };
}

impl_bytecode_methods! {
    (cmov, CMov)
    (and, TvAnd)
    (or, TvOr)
    (xor, TvXor)
    (ceq, TvCeq)
    (uleq, TvUnsignedLeq)
    (ugt, TvUnsignedGt)
    (min, TvMin)
    (max, TvMax)
    (fv_and, FvAnd)
    (fv_or, FvOr)
    (fv_xor, FvXor)
    (fv_andnot, FvAndNot)
    (fv_ornot, FvOrNot)
    (fv_xnor, FvXnor)
    (fv_ceq, FvCeq)
    (fv_posedge, FvPosedge)
    (fv_negedge, FvNegedge)
}

impl_bytecode_sbs_methods! {
    (andnot, TvAndNot)
    (ornot, TvOrNot)
    (xnor, TvXnor)
    (add, TvAdd)
    (sub, TvSub)
    (mul, TvMul)
    (divx, TvDivX)
    (div0, TvDiv0)
    (modx, TvModX)
    (mod0, TvMod0)
    (pow, TvPow)
    (sll, TvSll)
    (slr, TvSlr)
    (sar, TvSar)
    (lsor, TvLeftShiftOr)
    (fv_add, FvAdd)
    (fv_sub, FvSub)
    (fv_mul, FvMul)
    (fv_divx, FvDivX)
    (fv_div0, FvDiv0)
    (fv_modx, FvModX)
    (fv_mod0, FvMod0)
    (fv_pow, FvPow)
    (fv_uleq, FvUnsignedLeq)
    (fv_ugt, FvUnsignedGt)
    (fv_min, FvMin)
    (fv_max, FvMax)
    (fv_lsor, FvLeftShiftOr)
    (fv_copyx, FvCopyX)
    (fv_copyz, FvCopyZ)
    (fv_bitwise_ceq, FvBitwiseCeq)
}

impl_bytecode_fv_shift_methods! {
    (fv_sll, FvSll)
    (fv_slr, FvSlr)
    (fv_sar, FvSar)
    (fv_slrx, FvSlrx)
}
