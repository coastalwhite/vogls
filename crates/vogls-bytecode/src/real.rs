use std::fmt;

use vogls_ir::{LogicMode, SCALAR_VSIZE, VSIZE_64, VectorSize};
use vogls_runtime::RuntimeState;

use crate::reg::{Reg, RegInfo, Regs};
use crate::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule, write_padded_mnemonic,
};

pub struct RealInstr {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: RealInstrOp,
}

#[derive(Clone, Copy)]
enum RealInstrOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,

    Eq,
    Ne,

    Lt,
    Leq,
    Gt,
    Geq,

    ToLogical,
    ToU64,
    ToI64,
    FromTvSigned,
    FromFvSigned,
    FromTvUnsigned,
    FromFvUnsigned,

    Ln,
    Log10,
    Exp,
    Sqrt,
    Floor,
    Ceil,
    Sin,
    Cos,
    Tan,
    ASin,
    ACos,
    ATan,
    SinH,
    CosH,
    TanH,
    ASinH,
    ACosH,
    ATanH,
    ATan2,
    Hypot,

    Neg,
    Truncate,
}

impl RealInstrOp {
    fn mnemonic(self) -> &'static str {
        match self {
            Self::Add => "real.add",
            Self::Sub => "real.sub",
            Self::Mul => "real.mul",
            Self::Div => "real.div",
            Self::Pow => "real.pow",
            Self::Eq => "real.eq",
            Self::Ne => "real.ne",
            Self::Lt => "real.lt",
            Self::Leq => "real.leq",
            Self::Gt => "real.gt",
            Self::Geq => "real.geq",
            Self::ToLogical => "real.to_logical",
            Self::ToU64 => "real.to_u64",
            Self::ToI64 => "real.to_i64",
            Self::FromTvSigned => "real.from_tv_signed",
            Self::FromFvSigned => "real.from_fv_signed",
            Self::FromTvUnsigned => "real.from_tv_unsigned",
            Self::FromFvUnsigned => "real.from_fv_unsigned",
            Self::Ln => "real.ln",
            Self::Log10 => "real.log10",
            Self::Exp => "real.exp",
            Self::Sqrt => "real.sqrt",
            Self::Floor => "real.floor",
            Self::Ceil => "real.ceil",
            Self::Sin => "real.sin",
            Self::Cos => "real.cos",
            Self::Tan => "real.tan",
            Self::ASin => "real.asin",
            Self::ACos => "real.acos",
            Self::ATan => "real.atan",
            Self::SinH => "real.sinh",
            Self::CosH => "real.cosh",
            Self::TanH => "real.tanh",
            Self::ASinH => "real.asinh",
            Self::ACosH => "real.acosh",
            Self::ATanH => "real.atanh",
            Self::ATan2 => "real.atan2",
            Self::Hypot => "real.hypot",
            Self::Neg => "real.neg",
            Self::Truncate => "real.truncate",
        }
    }

    fn new_masked(v: u32) -> Self {
        match v {
            0 => Self::Add,
            1 => Self::Sub,
            2 => Self::Mul,
            3 => Self::Div,
            4 => Self::Pow,
            5 => Self::Eq,
            6 => Self::Ne,
            7 => Self::Lt,
            8 => Self::Leq,
            9 => Self::Gt,
            10 => Self::Geq,
            11 => Self::ToLogical,
            12 => Self::ToU64,
            13 => Self::ToI64,
            14 => Self::FromTvSigned,
            15 => Self::FromFvSigned,
            16 => Self::FromTvUnsigned,
            17 => Self::FromFvUnsigned,

            18 => Self::Ln,
            19 => Self::Log10,
            20 => Self::Exp,
            21 => Self::Sqrt,
            22 => Self::Floor,
            23 => Self::Ceil,
            24 => Self::Sin,
            25 => Self::Cos,
            26 => Self::Tan,
            27 => Self::ASin,
            28 => Self::ACos,
            29 => Self::ATan,
            30 => Self::SinH,
            31 => Self::CosH,
            32 => Self::TanH,
            33 => Self::ASinH,
            34 => Self::ACosH,
            35 => Self::ATanH,
            36 => Self::ATan2,
            37 => Self::Hypot,
            38 => Self::Neg,
            39 => Self::Truncate,
            _ => Self::Truncate,
        }
    }
}

impl BytecodeInstruction for RealInstr {
    fn num_additional_slots(&self) -> u8 {
        use RealInstrOp as O;
        match self.op {
            O::Add
            | O::Sub
            | O::Mul
            | O::Div
            | O::Pow
            | O::Eq
            | O::Ne
            | O::Lt
            | O::Leq
            | O::Gt
            | O::Geq
            | O::ToLogical
            | O::ToU64
            | O::ToI64
            | O::Neg
            | O::Truncate
            | O::Ln
            | O::Log10
            | O::Exp
            | O::Sqrt
            | O::Floor
            | O::Ceil
            | O::Sin
            | O::Cos
            | O::Tan
            | O::ASin
            | O::ACos
            | O::ATan
            | O::SinH
            | O::CosH
            | O::TanH
            | O::ASinH
            | O::ACosH
            | O::ATanH
            | O::ATan2
            | O::Hypot => 0,
            O::FromTvSigned | O::FromTvUnsigned | O::FromFvSigned | O::FromFvUnsigned => 1,
        }
    }
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            op: RealInstrOp::new_masked(v >> 20),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::RealInstr as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.op as u32) << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs1, rs2, op } = self;
        write_padded_mnemonic(f, op.mnemonic())?;
        write!(f, "{rd}, {rs1}, {rs2}")
    }

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        use LogicMode as M;
        use RealInstrOp as O;
        match self.op {
            O::Add
            | O::Sub
            | O::Mul
            | O::Div
            | O::Pow
            | O::Eq
            | O::Ne
            | O::Lt
            | O::Leq
            | O::Gt
            | O::Geq
            | O::ATan2
            | O::Hypot => {
                operands.extend([
                    RegInfo::register("rs1", self.rs1, M::TwoValue, Some(VSIZE_64)),
                    RegInfo::register("rs2", self.rs2, M::TwoValue, Some(VSIZE_64)),
                ]);
            }
            O::ToLogical
            | O::ToU64
            | O::ToI64
            | O::Neg
            | O::Truncate
            | O::Ln
            | O::Log10
            | O::Exp
            | O::Sqrt
            | O::Floor
            | O::Ceil
            | O::Sin
            | O::Cos
            | O::Tan
            | O::ASin
            | O::ACos
            | O::ATan
            | O::SinH
            | O::CosH
            | O::TanH
            | O::ASinH
            | O::ACosH
            | O::ATanH => {
                operands.push(RegInfo::register(
                    "rs",
                    self.rs1,
                    M::TwoValue,
                    Some(VSIZE_64),
                ));
            }
            O::FromTvSigned | O::FromTvUnsigned => {
                let size =
                    VectorSize::new(code[pc as usize + 1].0).expect("Expected non-zero size");
                operands.push(RegInfo::register("rs", self.rs1, M::TwoValue, Some(size)));
            }
            O::FromFvSigned | O::FromFvUnsigned => {
                let size =
                    VectorSize::new(code[pc as usize + 1].0).expect("Expected non-zero size");
                operands.push(RegInfo::register("rs", self.rs1, M::FourValue, Some(size)));
            }
        }
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        use LogicMode as M;
        use RealInstrOp as O;
        let dst = match self.op {
            O::Add
            | O::Sub
            | O::Mul
            | O::Div
            | O::Pow
            | O::ToU64
            | O::ToI64
            | O::FromTvSigned
            | O::FromFvSigned
            | O::FromTvUnsigned
            | O::FromFvUnsigned
            | O::Neg
            | O::Truncate
            | O::Ln
            | O::Log10
            | O::Exp
            | O::Sqrt
            | O::Floor
            | O::Ceil
            | O::Sin
            | O::Cos
            | O::Tan
            | O::ASin
            | O::ACos
            | O::ATan
            | O::SinH
            | O::CosH
            | O::TanH
            | O::ASinH
            | O::ACosH
            | O::ATanH
            | O::ATan2
            | O::Hypot => RegInfo::register("rd", self.rd, M::TwoValue, Some(VSIZE_64)),
            O::Eq | O::Ne | O::Lt | O::Leq | O::Gt | O::Geq | O::ToLogical => {
                RegInfo::register("rd", self.rd, M::TwoValue, Some(SCALAR_VSIZE))
            }
        };
        operands.push(dst);
    }

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self { rd, rs1, rs2, op } = self;
        use RealInstrOp as O;
        match op {
            O::Add => regs[rd] = (f64::from_bits(regs[rs1]) + f64::from_bits(regs[rs2])).to_bits(),
            O::Sub => regs[rd] = (f64::from_bits(regs[rs1]) - f64::from_bits(regs[rs2])).to_bits(),
            O::Mul => regs[rd] = (f64::from_bits(regs[rs1]) * f64::from_bits(regs[rs2])).to_bits(),
            O::Div => regs[rd] = (f64::from_bits(regs[rs1]) / f64::from_bits(regs[rs2])).to_bits(),
            O::Pow => {
                regs[rd] = f64::from_bits(regs[rs1])
                    .powf(f64::from_bits(regs[rs2]))
                    .to_bits()
            }
            O::Neg => regs[rd] = (-f64::from_bits(regs[rs1])).to_bits(),
            O::Truncate => regs[rd] = f64::from_bits(regs[rs1]).trunc().to_bits(),
            O::Ln => regs[rd] = f64::from_bits(regs[rs1]).ln().to_bits(),
            O::Log10 => regs[rd] = f64::from_bits(regs[rs1]).log10().to_bits(),
            O::Exp => regs[rd] = f64::from_bits(regs[rs1]).exp().to_bits(),
            O::Sqrt => regs[rd] = f64::from_bits(regs[rs1]).sqrt().to_bits(),
            O::Floor => regs[rd] = f64::from_bits(regs[rs1]).floor().to_bits(),
            O::Ceil => regs[rd] = f64::from_bits(regs[rs1]).ceil().to_bits(),
            O::Sin => regs[rd] = f64::from_bits(regs[rs1]).sin().to_bits(),
            O::Cos => regs[rd] = f64::from_bits(regs[rs1]).cos().to_bits(),
            O::Tan => regs[rd] = f64::from_bits(regs[rs1]).tan().to_bits(),
            O::ASin => regs[rd] = f64::from_bits(regs[rs1]).asin().to_bits(),
            O::ACos => regs[rd] = f64::from_bits(regs[rs1]).acos().to_bits(),
            O::ATan => regs[rd] = f64::from_bits(regs[rs1]).atan().to_bits(),
            O::SinH => regs[rd] = f64::from_bits(regs[rs1]).sinh().to_bits(),
            O::CosH => regs[rd] = f64::from_bits(regs[rs1]).cosh().to_bits(),
            O::TanH => regs[rd] = f64::from_bits(regs[rs1]).tanh().to_bits(),
            O::ASinH => regs[rd] = f64::from_bits(regs[rs1]).asinh().to_bits(),
            O::ACosH => regs[rd] = f64::from_bits(regs[rs1]).acosh().to_bits(),
            O::ATanH => regs[rd] = f64::from_bits(regs[rs1]).atanh().to_bits(),
            O::ATan2 => {
                regs[rd] = f64::from_bits(regs[rs1])
                    .atan2(f64::from_bits(regs[rs2]))
                    .to_bits()
            }
            O::Hypot => {
                regs[rd] = f64::from_bits(regs[rs1])
                    .hypot(f64::from_bits(regs[rs2]))
                    .to_bits()
            }

            O::Eq => regs[rd] = u64::from(f64::from_bits(regs[rs1]) == f64::from_bits(regs[rs2])),
            O::Ne => regs[rd] = u64::from(f64::from_bits(regs[rs1]) != f64::from_bits(regs[rs2])),
            O::Lt => regs[rd] = u64::from(f64::from_bits(regs[rs1]) < f64::from_bits(regs[rs2])),
            O::Leq => regs[rd] = u64::from(f64::from_bits(regs[rs1]) <= f64::from_bits(regs[rs2])),
            O::Gt => regs[rd] = u64::from(f64::from_bits(regs[rs1]) > f64::from_bits(regs[rs2])),
            O::Geq => regs[rd] = u64::from(f64::from_bits(regs[rs1]) >= f64::from_bits(regs[rs2])),

            O::ToLogical => regs[rd] = u64::from(f64::from_bits(regs[rs1]) != 0.0),
            O::ToU64 => regs[rd] = f64::from_bits(regs[rs1]) as u64,
            O::ToI64 => regs[rd] = f64::from_bits(regs[rs1]) as i64 as u64,
            O::FromTvSigned => {
                let size = code[*pc as usize].unwrap_size();
                *pc += 1;
                if size <= VSIZE_64 {
                    let shift = 64 - size.get();
                    let value = ((regs[rs1] as i64) << shift) >> shift;
                    regs[rd] = (value as i64 as f64).to_bits();
                } else {
                    let addr = regs.get_as_addr(rs1);
                    regs[rd] = state
                        .heap
                        .load_tv_bits(addr.to_ref(size))
                        .as_signed_f64()
                        .to_bits();
                }
            }
            O::FromFvSigned => {
                let size = code[*pc as usize].unwrap_size();
                *pc += 1;
                if size <= VSIZE_64 {
                    let (spc, val) = rs1.to_spc_and_val();
                    let shift = 64 - size.get();
                    let value = (((regs[spc] & regs[val]) as i64) << shift) >> shift;
                    regs[rd] = (value as f64).to_bits();
                } else {
                    let addr = regs.get_as_addr(rs1);
                    regs[rd] = state
                        .heap
                        .load_fv_bits(addr.to_ref(size))
                        .as_signed_f64()
                        .to_bits();
                }
            }
            O::FromTvUnsigned => {
                let size = code[*pc as usize].unwrap_size();
                *pc += 1;
                if size <= VSIZE_64 {
                    regs[rd] = (regs[rs1] as f64).to_bits();
                } else {
                    let addr = regs.get_as_addr(rs1);
                    regs[rd] = state
                        .heap
                        .load_tv_bits(addr.to_ref(size))
                        .as_unsigned_f64()
                        .to_bits();
                }
            }
            O::FromFvUnsigned => {
                let size = code[*pc as usize].unwrap_size();
                *pc += 1;
                if size <= VSIZE_64 {
                    let (spc, val) = rs1.to_spc_and_val();
                    regs[rd] = ((regs[spc] & regs[val]) as f64).to_bits();
                } else {
                    let addr = regs.get_as_addr(rs1);
                    regs[rd] = state
                        .heap
                        .load_fv_bits(addr.to_ref(size))
                        .as_unsigned_f64()
                        .to_bits();
                }
            }
        }
    }
}

impl BytecodeEncoder {
    fn real_op(&mut self, rd: Reg, rs1: Reg, rs2: Reg, op: RealInstrOp) {
        self.data.push(RealInstr { rd, rs1, rs2, op }.encode());
    }
    pub fn real_add(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Add);
    }
    pub fn real_sub(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Sub);
    }
    pub fn real_mul(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Mul);
    }
    pub fn real_div(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Div);
    }
    pub fn real_pow(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Pow);
    }
    pub fn real_eq(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Eq);
    }
    pub fn real_ne(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Ne);
    }
    pub fn real_lt(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Lt);
    }
    pub fn real_leq(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Leq);
    }
    pub fn real_gt(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Gt);
    }
    pub fn real_geq(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Geq);
    }

    pub fn real_neg(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Neg);
    }
    pub fn real_truncate(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Truncate);
    }
    pub fn real_ln(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Ln);
    }
    pub fn real_log10(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Log10);
    }
    pub fn real_exp(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Exp);
    }
    pub fn real_sqrt(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Sqrt);
    }
    pub fn real_floor(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Floor);
    }
    pub fn real_ceil(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Ceil);
    }
    pub fn real_sin(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Sin);
    }
    pub fn real_cos(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Cos);
    }
    pub fn real_tan(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::Tan);
    }
    pub fn real_asin(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ASin);
    }
    pub fn real_acos(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ACos);
    }
    pub fn real_atan(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ATan);
    }
    pub fn real_sinh(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::SinH);
    }
    pub fn real_cosh(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::CosH);
    }
    pub fn real_tanh(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::TanH);
    }
    pub fn real_asinh(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ASinH);
    }
    pub fn real_acosh(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ACosH);
    }
    pub fn real_atanh(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ATanH);
    }
    pub fn real_atan2(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::ATan2);
    }
    pub fn real_hypot(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        self.real_op(rd, rs1, rs2, RealInstrOp::Hypot);
    }

    pub fn real_to_logical(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ToLogical);
    }
    pub fn real_to_u64(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ToU64);
    }
    pub fn real_to_i64(&mut self, rd: Reg, rs: Reg) {
        self.real_op(rd, rs, rs, RealInstrOp::ToI64);
    }
    pub fn real_from_tv_signed(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.real_op(rd, rs, rs, RealInstrOp::FromTvSigned);
        self.data.push(Bytecode(size.get()));
    }
    pub fn real_from_fv_signed(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.real_op(rd, rs, rs, RealInstrOp::FromFvSigned);
        self.data.push(Bytecode(size.get()));
    }
    pub fn real_from_tv_unsigned(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.real_op(rd, rs, rs, RealInstrOp::FromTvUnsigned);
        self.data.push(Bytecode(size.get()));
    }
    pub fn real_from_fv_unsigned(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.real_op(rd, rs, rs, RealInstrOp::FromFvUnsigned);
        self.data.push(Bytecode(size.get()));
    }
}
