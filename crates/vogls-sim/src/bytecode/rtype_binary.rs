use std::fmt;

use vogls_bits::arithmetic::{
    fv_bitwise_and_elem, fv_bitwise_andnot_elem, fv_bitwise_or_elem, fv_bitwise_ornot_elem,
    fv_bitwise_xor_elem,
};
use vogls_bits::edge::{fv_negedge_u64, fv_posedge_u64};
use vogls_bits::shift::fv_shift_arith_right;
use vogls_bits::util::wrapping_u64_pow;
use vogls_ir::VectorSize;
use vogls_runtime::RuntimeState;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule, SixBitSize, write_padded_mnemonic,
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
pub struct TvSlr(pub BitwiseRType);
pub struct TvSar(pub SbsBitwiseRType);

pub struct FvAnd(pub BitwiseRType);
pub struct FvOr(pub BitwiseRType);
pub struct FvXor(pub BitwiseRType);
pub struct FvAndNot(pub BitwiseRType);
pub struct FvOrNot(pub BitwiseRType);
pub struct FvCeq(pub BitwiseRType);
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
pub struct FvSll(pub SbsBitwiseRType);
pub struct FvSlr(pub SbsBitwiseRType);
pub struct FvSar(pub SbsBitwiseRType);

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
                | ((self.size.0 as u32) << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs1, rs2, size } = self;
        write!(f, "{rd}, {rs1}, {rs2}, |{size}|")
    }
}

macro_rules! impl_bitwise {
    ($variant:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(BitwiseRType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}
macro_rules! impl_sbs_bitwise {
    ($variant:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(SbsBitwiseRType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
    };
}

impl BytecodeInstruction for TvAnd {
    impl_bitwise!(TvAnd, "tv.and");

    fn execute(
        self,
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
    impl_bitwise!(TvOr, "tv.or");

    fn execute(
        self,
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
    impl_bitwise!(TvXor, "tv.xor");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvAndNot, "tv.andnot");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvOrNot, "tv.ornot");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvXnor, "tv.xnor");

    fn execute(
        self,
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
    impl_bitwise!(TvCeq, "tv.ceq");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvAdd, "tv.add");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvSub, "tv.sub");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvMul, "tv.mul");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvDivX, "tv.divx");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvDiv0, "tv.div0");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvModX, "tv.modx");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvMod0, "tv.mod0");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvPow, "tv.pow");

    fn execute(
        self,
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
    impl_bitwise!(TvUnsignedLeq, "tv.uleq");

    fn execute(
        self,
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
    impl_bitwise!(TvUnsignedGt, "tv.ugt");

    fn execute(
        self,
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
    impl_bitwise!(TvMin, "tv.min");

    fn execute(
        self,
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
    impl_bitwise!(TvMax, "tv.max");

    fn execute(
        self,
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
    impl_sbs_bitwise!(TvSll, "tv.sll");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        if regs[rs2] >= 64 {
            regs[rd] = 0;
        } else {
            regs[rd] = size.mask(regs[rs1] << regs[rs2]);
        }
    }
}
impl BytecodeInstruction for TvSlr {
    impl_bitwise!(TvSlr, "tv.slr");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let BitwiseRType { rd, rs1, rs2 } = self.0;
        if regs[rs2] >= 64 {
            regs[rd] = 0;
        } else {
            regs[rd] = regs[rs1] >> regs[rs2];
        }
    }
}
impl BytecodeInstruction for TvSar {
    impl_sbs_bitwise!(TvSar, "tv.sar");

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let SbsBitwiseRType { rd, rs1, rs2, size } = self.0;
        let unused_bits = 64 - VectorSize::from(size).get();
        let out = regs[rs1] << unused_bits;
        let out = out as i64;
        let out = out.unbounded_shr(unused_bits + regs[rs2] as u32);
        regs[rd] = out as u64;
    }
}

impl BytecodeInstruction for FvAnd {
    impl_bitwise!(FvAnd, "fv.and");

    fn execute(
        self,
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
    impl_bitwise!(FvOr, "fv.or");

    fn execute(
        self,
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
    impl_bitwise!(FvXor, "fv.xor");

    fn execute(
        self,
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
    impl_bitwise!(FvAndNot, "fv.andnot");

    fn execute(
        self,
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
    impl_bitwise!(FvOrNot, "fv.ornot");

    fn execute(
        self,
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
impl BytecodeInstruction for FvCeq {
    impl_bitwise!(FvCeq, "fv.ceq");

    fn execute(
        self,
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
impl BytecodeInstruction for FvPosedge {
    impl_bitwise!(FvPosedge, "fv.posedge");

    fn execute(
        self,
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
    impl_bitwise!(FvNegedge, "fv.negedge");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvAdd, "fv.add");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvSub, "fv.sub");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvMul, "fv.mul");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvDivX, "fv.divx");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvDiv0, "fv.div0");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvModX, "fv.modx");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvMod0, "fv.mod0");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvPow, "fv.pow");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvUnsignedLeq, "fv.uleq");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvUnsignedGt, "fv.ugt");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvMin, "fv.min");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvMax, "fv.max");

    fn execute(
        self,
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
    impl_sbs_bitwise!(FvSll, "fv.sll");

    fn execute(
        self,
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

        if regs[rs2_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
            return;
        }

        let shift = regs[rs2_val];
        if shift >= 64 {
            regs[rd_spc] = mask;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = size.mask(regs[rs1_spc] << shift) | ((1u64 << shift) - 1);
            regs[rd_val] = size.mask(regs[rs1_val] << shift);
        }
    }
}
impl BytecodeInstruction for FvSlr {
    impl_sbs_bitwise!(FvSlr, "fv.slr");

    fn execute(
        self,
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

        if regs[rs2_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
            return;
        }

        let shift = regs[rs2_val];
        if shift >= 64 {
            regs[rd_spc] = mask;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = size.mask(regs[rs1_spc] >> shift)
                | (((1u64 << shift) - 1) << (VectorSize::from(size).get() - shift as u32));
            regs[rd_val] = size.mask(regs[rs1_val] >> shift);
        }
    }
}
impl BytecodeInstruction for FvSar {
    impl_sbs_bitwise!(FvSar, "fv.sar");

    fn execute(
        self,
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

        if regs[rs2_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
            return;
        }

        let shift = regs[rs2_val].min(u32::MAX as u64) as u32;
        let size = VectorSize::from(size);
        (regs[rd_spc], regs[rd_val]) =
            fv_shift_arith_right(regs[rs1_spc], regs[rs1_val], shift, size);
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

impl_bytecode_methods! {
    (and, TvAnd)
    (or, TvOr)
    (xor, TvXor)
    (ceq, TvCeq)
    (uleq, TvUnsignedLeq)
    (ugt, TvUnsignedGt)
    (min, TvMin)
    (max, TvMax)
    (slr, TvSlr)
    (fv_and, FvAnd)
    (fv_or, FvOr)
    (fv_xor, FvXor)
    (fv_andnot, FvAndNot)
    (fv_ornot, FvOrNot)
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
    (sar, TvSar)
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
    (fv_sll, FvSll)
    (fv_slr, FvSlr)
    (fv_sar, FvSar)
}
