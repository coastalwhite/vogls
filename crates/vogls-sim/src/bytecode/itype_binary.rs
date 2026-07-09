use std::fmt;

use vogls_bits::arithmetic::{fv_bitwise_and_elem, fv_bitwise_or_elem, fv_bitwise_xor_elem};
use vogls_bits::shift::fv_shift_arith_right;
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, Schedule, SignedImmediate, SixBitSize, write_padded_mnemonic,
    write_register,
};

pub struct IType {
    pub rd: Reg,
    pub rs: Reg,
    pub imm10: SignedImmediate<10>,
    pub size: SixBitSize,
}

pub struct TvAndi(pub IType);
pub struct TvOri(pub IType);
pub struct TvXori(pub IType);
pub struct TvAddi(pub IType);
pub struct TvSubi(pub IType);
pub struct TvMuli(pub IType);
pub struct TvRevSubi(pub IType);
pub struct TvMini(pub IType);
pub struct TvMaxi(pub IType);
pub struct TvUleqi(pub IType);
pub struct TvUgti(pub IType);
pub struct TvCeqi(pub IType);
pub struct TvCnei(pub IType);
pub struct TvSlli(pub IType);
pub struct TvSlri(pub IType);
pub struct TvSari(pub IType);

pub struct FvAndi(pub IType);
pub struct FvOri(pub IType);
pub struct FvXori(pub IType);
pub struct FvAddi(pub IType);
pub struct FvSubi(pub IType);
pub struct FvMuli(pub IType);
pub struct FvRevSubi(pub IType);
pub struct FvMini(pub IType);
pub struct FvMaxi(pub IType);
pub struct FvUleqi(pub IType);
pub struct FvUgti(pub IType);
pub struct FvCeqi(pub IType);
pub struct FvCnei(pub IType);
pub struct FvSlli(pub IType);
pub struct FvSlri(pub IType);
pub struct FvSari(pub IType);

impl IType {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: SignedImmediate::new_shifted(v, 22),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.size.0 as u32) << 16)
                | (self.imm10.encode() << 22),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            imm10,
            size,
        } = self;
        write!(f, "{rd}, {rs}, {imm10}, |{size}|")
    }
}

macro_rules! impl_bitwise {
    ($variant:ident, $mnemonic:literal, $rd_mode:ident, $rs_mode:ident) => {
        #[inline(always)]
        fn extract(v: Bytecode) -> Self {
            debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
            Self(IType::extract(v))
        }
        fn encode(&self) -> Bytecode {
            self.0.encode(BytecodeOpcode::$variant)
        }
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_padded_mnemonic(f, $mnemonic)?;
            self.0.fmt(f)
        }
        fn pre_exec_itrace(
            &self,
            f: &mut fmt::Formatter<'_>,
            regs: &Regs,
            _state: &RuntimeState,
        ) -> fmt::Result {
            f.write_str(EXEC_ITRACE_INDENT)?;
            write_register(f, regs, "rs", self.0.rs, LogicMode::$rs_mode)?;
            writeln!(f)?;
            Ok(())
        }
        fn post_exec_itrace(
            &self,
            f: &mut fmt::Formatter<'_>,
            regs: &Regs,
            _state: &RuntimeState,
        ) -> fmt::Result {
            f.write_str(EXEC_ITRACE_INDENT)?;
            write_register(f, regs, "rd", self.0.rd, LogicMode::$rd_mode)?;
            writeln!(f)?;
            Ok(())
        }
    };
}

impl BytecodeInstruction for TvAndi {
    impl_bitwise!(TvAndi, "tv.andi", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs] & i64::from(imm10.0) as u64);
    }
}
impl BytecodeInstruction for TvOri {
    impl_bitwise!(TvOri, "tv.ori", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs] | i64::from(imm10.0) as u64);
    }
}
impl BytecodeInstruction for TvXori {
    impl_bitwise!(TvXori, "tv.xori", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs] ^ i64::from(imm10.0) as u64);
    }
}
impl BytecodeInstruction for TvAddi {
    impl_bitwise!(TvAddi, "tv.addi", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs].wrapping_add(size.mask(i64::from(imm10.0) as u64)));
    }
}
impl BytecodeInstruction for TvSubi {
    impl_bitwise!(TvSubi, "tv.subi", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs].wrapping_sub(size.mask(i64::from(imm10.0) as u64)));
    }
}
impl BytecodeInstruction for TvRevSubi {
    impl_bitwise!(TvRevSubi, "tv.revsubi", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(size.mask(i64::from(imm10.0) as u64).wrapping_sub(regs[rs]));
    }
}
impl BytecodeInstruction for TvMuli {
    impl_bitwise!(TvMuli, "tv.muli", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs].wrapping_mul(size.mask(i64::from(imm10.0) as u64)));
    }
}
impl BytecodeInstruction for TvMini {
    impl_bitwise!(TvMini, "tv.mini", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = regs[rs].min(size.mask(i64::from(imm10.0) as u64));
    }
}
impl BytecodeInstruction for TvMaxi {
    impl_bitwise!(TvMaxi, "tv.maxi", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = regs[rs].max(size.mask(i64::from(imm10.0) as u64));
    }
}
impl BytecodeInstruction for TvUleqi {
    impl_bitwise!(TvUleqi, "tv.uleqi", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let imm = size.mask(i64::from(imm10.0) as u64);
        regs[rd] = u64::from(regs[rs] <= imm);
    }
}
impl BytecodeInstruction for TvUgti {
    impl_bitwise!(TvUgti, "tv.ugti", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let imm = size.mask(i64::from(imm10.0) as u64);
        regs[rd] = u64::from(regs[rs] > imm);
    }
}
impl BytecodeInstruction for TvCeqi {
    impl_bitwise!(TvCeqi, "tv.ceqi", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = u64::from(regs[rs] == size.mask(i64::from(imm10.0) as u64));
    }
}
impl BytecodeInstruction for TvCnei {
    impl_bitwise!(TvCnei, "tv.cnei", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = u64::from(regs[rs] != size.mask(i64::from(imm10.0) as u64));
    }
}
impl BytecodeInstruction for TvSlli {
    impl_bitwise!(TvSlli, "tv.slli", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs].unbounded_shl(imm10.get_unsigned()));
    }
}
impl BytecodeInstruction for TvSlri {
    impl_bitwise!(TvSlri, "tv.slri", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        regs[rd] = size.mask(regs[rs].unbounded_shr(imm10.get_unsigned()));
    }
}
impl BytecodeInstruction for TvSari {
    impl_bitwise!(TvSari, "tv.sari", TwoValue, TwoValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let unused_bits = 64 - VectorSize::from(size).get();
        let out = regs[rs] << unused_bits;
        let out = out as i64;
        let out = out.unbounded_shr(unused_bits + imm10.get_unsigned());
        regs[rd] = out as u64;
    }
}
impl BytecodeInstruction for FvAndi {
    impl_bitwise!(FvAndi, "fv.andi", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let imm = size.mask(i64::from(imm10.0) as u64);
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_and_elem(regs[rs_spc], regs[rs_val], size.mask(u64::MAX), imm);
    }
}
impl BytecodeInstruction for FvOri {
    impl_bitwise!(FvOri, "fv.ori", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let imm = size.mask(i64::from(imm10.0) as u64);
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_or_elem(regs[rs_spc], regs[rs_val], size.mask(u64::MAX), imm);
    }
}
impl BytecodeInstruction for FvXori {
    impl_bitwise!(FvXori, "fv.xori", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let imm = size.mask(i64::from(imm10.0) as u64);
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_xor_elem(regs[rs_spc], regs[rs_val], size.mask(u64::MAX), imm);
    }
}
impl BytecodeInstruction for FvAddi {
    impl_bitwise!(FvAddi, "fv.addi", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = mask;
            regs[rd_val] = mask & regs[rs_val].wrapping_add(imm);
        }
    }
}
impl BytecodeInstruction for FvSubi {
    impl_bitwise!(FvSubi, "fv.subi", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = mask;
            regs[rd_val] = mask & regs[rs_val].wrapping_sub(imm);
        }
    }
}
impl BytecodeInstruction for FvMuli {
    impl_bitwise!(FvMuli, "fv.muli", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = mask;
            regs[rd_val] = mask & regs[rs_val].wrapping_mul(imm);
        }
    }
}
impl BytecodeInstruction for FvRevSubi {
    impl_bitwise!(FvRevSubi, "fv.revsubi", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = mask;
            regs[rd_val] = mask & imm.wrapping_sub(regs[rs_val]);
        }
    }
}
impl BytecodeInstruction for FvMini {
    impl_bitwise!(FvMini, "fv.mini", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = mask;
            regs[rd_val] = imm.min(regs[rs_val]);
        }
    }
}
impl BytecodeInstruction for FvMaxi {
    impl_bitwise!(FvMaxi, "fv.maxi", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = mask;
            regs[rd_val] = imm.max(regs[rs_val]);
        }
    }
}
impl BytecodeInstruction for FvUleqi {
    impl_bitwise!(FvUleqi, "fv.uleqi", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = 1;
            regs[rd_val] = u64::from(regs[rs_val] <= imm);
        }
    }
}
impl BytecodeInstruction for FvUgti {
    impl_bitwise!(FvUgti, "fv.ugti", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let mask = size.mask(u64::MAX);
        let imm = i64::from(imm10.0) as u64 & mask;
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        if regs[rs_spc] != mask {
            regs[rd_spc] = 0;
            regs[rd_val] = 0;
        } else {
            regs[rd_spc] = 1;
            regs[rd_val] = u64::from(regs[rs_val] > imm);
        }
    }
}
impl BytecodeInstruction for FvCeqi {
    impl_bitwise!(FvCeqi, "fv.ceqi", TwoValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let imm = size.mask(i64::from(imm10.0) as u64);
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        regs[rd] = u64::from((regs[rs_spc] == size.mask(u64::MAX)) & (regs[rs_val] == imm));
    }
}
impl BytecodeInstruction for FvCnei {
    impl_bitwise!(FvCnei, "fv.cnei", TwoValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let imm = size.mask(i64::from(imm10.0) as u64);
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        regs[rd] = u64::from((regs[rs_spc] != size.mask(u64::MAX)) | (regs[rs_val] != imm));
    }
}
impl BytecodeInstruction for FvSlli {
    impl_bitwise!(FvSlli, "fv.slli", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let (rdspc, rdval) = rd.to_spc_and_val();
        let (rsspc, rsval) = rs.to_spc_and_val();
        regs[rdspc] = size.mask(regs[rsspc].unbounded_shl(imm10.get_unsigned()));
        regs[rdval] = size.mask(regs[rsval].unbounded_shl(imm10.get_unsigned()));
    }
}
impl BytecodeInstruction for FvSlri {
    impl_bitwise!(FvSlri, "fv.slri", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let (rdspc, rdval) = rd.to_spc_and_val();
        let (rsspc, rsval) = rs.to_spc_and_val();
        regs[rdspc] = size.mask(regs[rsspc].unbounded_shr(imm10.get_unsigned()));
        regs[rdval] = size.mask(regs[rsval].unbounded_shr(imm10.get_unsigned()));
    }
}
impl BytecodeInstruction for FvSari {
    impl_bitwise!(FvSari, "fv.sari", FourValue, FourValue);

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let IType {
            rd,
            rs,
            imm10,
            size,
        } = self.0;
        let (rdspc, rdval) = rd.to_spc_and_val();
        let (rsspc, rsval) = rs.to_spc_and_val();
        (regs[rdspc], regs[rdval]) =
            fv_shift_arith_right(regs[rsspc], regs[rsval], imm10.get_unsigned(), size.into());
    }
}

macro_rules! impl_bytecode_methods {
    ($(($name:ident, $op:ident))*) => {
        impl BytecodeEncoder {
            $(pub fn $name(&mut self, rd: Reg, rs: Reg, imm10: SignedImmediate<10>, size: SixBitSize) {
                self.data.push($op(IType { rd, rs, imm10, size }).encode());
            })*
        }
    };
}

impl_bytecode_methods! {
    (andi, TvAndi)
    (ori, TvOri)
    (xori, TvXori)
    (addi, TvAddi)
    (subi, TvSubi)
    (muli, TvMuli)
    (revsubi, TvRevSubi)
    (mini, TvMini)
    (maxi, TvMaxi)
    (uleqi, TvUleqi)
    (ugti, TvUgti)
    (ceqi, TvCeqi)
    (cnei, TvCnei)
    (slli, TvSlli)
    (slri, TvSlri)
    (sari, TvSari)
    (fv_andi, FvAndi)
    (fv_ori, FvOri)
    (fv_xori, FvXori)
    (fv_addi, FvAddi)
    (fv_subi, FvSubi)
    (fv_muli, FvMuli)
    (fv_revsubi, FvRevSubi)
    (fv_mini, FvMini)
    (fv_maxi, TvMaxi)
    (fv_uleqi, FvUleqi)
    (fv_ugti, TvUgti)
    (fv_ceqi, FvCeqi)
    (fv_cnei, FvCnei)
    (fv_slli, FvSlli)
    (fv_slri, FvSlri)
    (fv_sari, FvSari)
}

impl BytecodeEncoder {
    pub fn contains_special(&mut self, rd: Reg, rs: Reg, size: SixBitSize) {
        self.cnei(rd, rs, SignedImmediate::MINUS_ONE, size)
    }
    pub fn contains_no_special(&mut self, rd: Reg, rs: Reg, size: SixBitSize) {
        self.ceqi(rd, rs, SignedImmediate::MINUS_ONE, size)
    }
}
