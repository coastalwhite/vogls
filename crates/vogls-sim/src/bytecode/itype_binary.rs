use std::fmt;

use vogls_bits::arithmetic::{fv_bitwise_and_elem, fv_bitwise_or_elem, fv_bitwise_xor_elem};
use vogls_runtime::RuntimeState;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule, SixBitSize, write_padded_mnemonic,
};

pub struct IType {
    pub rd: Reg,
    pub rs: Reg,
    pub imm10: i16,
    pub size: SixBitSize,
}

pub struct TvAndi(pub IType);
pub struct TvOri(pub IType);
pub struct TvXori(pub IType);
pub struct TvAddi(pub IType);
pub struct TvSubi(pub IType);
pub struct TvCeqi(pub IType);
pub struct TvCnei(pub IType);
pub struct TvSlli(pub IType);

pub struct FvAndi(pub IType);
pub struct FvOri(pub IType);
pub struct FvXori(pub IType);
pub struct FvCeqi(pub IType);
pub struct FvCnei(pub IType);

impl IType {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: ((v as i32) >> 22) as i16,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.size.0 as u32) << 16)
                | ((self.imm10 as u16 as u32) << 22),
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
    ($variant:ident, $mnemonic:literal) => {
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
    };
}

impl BytecodeInstruction for TvAndi {
    impl_bitwise!(TvAndi, "tv.andi");

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
        regs[rd] = regs[rs] & size.mask(i64::from(imm10) as u64);
    }
}
impl BytecodeInstruction for TvOri {
    impl_bitwise!(TvOri, "tv.ori");

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
        regs[rd] = regs[rs] | size.mask(i64::from(imm10) as u64);
    }
}
impl BytecodeInstruction for TvXori {
    impl_bitwise!(TvXori, "tv.xori");

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
        regs[rd] = regs[rs] ^ size.mask(i64::from(imm10) as u64);
    }
}
impl BytecodeInstruction for TvAddi {
    impl_bitwise!(TvAddi, "tv.addi");

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
        regs[rd] = size.mask(regs[rs].wrapping_add(size.mask(i64::from(imm10) as u64)));
    }
}
impl BytecodeInstruction for TvSubi {
    impl_bitwise!(TvSubi, "tv.subi");

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
        regs[rd] = size.mask(regs[rs].wrapping_sub(size.mask(i64::from(imm10) as u64)));
    }
}
impl BytecodeInstruction for TvCeqi {
    impl_bitwise!(TvCeqi, "tv.ceqi");

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
        regs[rd] = u64::from(regs[rs] == size.mask(i64::from(imm10) as u64));
    }
}
impl BytecodeInstruction for TvCnei {
    impl_bitwise!(TvCnei, "tv.cnei");

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
        regs[rd] = u64::from(regs[rs] != size.mask(i64::from(imm10) as u64));
    }
}
impl BytecodeInstruction for TvSlli {
    impl_bitwise!(TvSlli, "tv.slli");

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
        regs[rd] = size.mask(regs[rs].wrapping_shl(imm10 as u16 as u32));
    }
}
impl BytecodeInstruction for FvAndi {
    impl_bitwise!(FvAndi, "fv.andi");

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
        let imm = size.mask(i64::from(imm10) as u64);
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_and_elem(regs[rs_spc], regs[rs_val], size.mask(u64::MAX), imm);
    }
}
impl BytecodeInstruction for FvOri {
    impl_bitwise!(FvOri, "fv.ori");

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
        let imm = size.mask(i64::from(imm10) as u64);
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_or_elem(regs[rs_spc], regs[rs_val], size.mask(u64::MAX), imm);
    }
}
impl BytecodeInstruction for FvXori {
    impl_bitwise!(FvXori, "fv.xori");

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
        let imm = size.mask(i64::from(imm10) as u64);
        let (rd_spc, rd_val) = rd.to_spc_and_val();
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        (regs[rd_spc], regs[rd_val]) =
            fv_bitwise_xor_elem(regs[rs_spc], regs[rs_val], size.mask(u64::MAX), imm);
    }
}
impl BytecodeInstruction for FvCeqi {
    impl_bitwise!(FvCeqi, "fv.ceqi");

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
        let imm = size.mask(i64::from(imm10) as u64);
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        regs[rd] = u64::from((regs[rs_spc] == size.mask(u64::MAX)) & (regs[rs_val] == imm));
    }
}
impl BytecodeInstruction for FvCnei {
    impl_bitwise!(FvCnei, "fv.cnei");

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
        let imm = size.mask(i64::from(imm10) as u64);
        let (rs_spc, rs_val) = rs.to_spc_and_val();
        regs[rd] = u64::from((regs[rs_spc] != size.mask(u64::MAX)) | (regs[rs_val] != imm));
    }
}

macro_rules! impl_bytecode_methods {
    ($(($name:ident, $op:ident))*) => {
        impl BytecodeEncoder {
            $(pub fn $name(&mut self, rd: Reg, rs: Reg, imm10: i16, size: SixBitSize) {
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
    (ceqi, TvCeqi)
    (cnei, TvCnei)
    (slli, TvSlli)
    (fv_andi, FvAndi)
    (fv_ori, FvOri)
    (fv_xori, FvXori)
    (fv_ceqi, FvCeqi)
    (fv_cnei, FvCnei)
}
