use vogls_utils::VgHashSet;

use crate::form::check_ir_form;
use crate::token_range::TokenRange;
use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryImmOpSimplification,
    BinaryOp, Bits, GlobalContext, INTEGER_VSIZE, Instruction, IntrinsicOp, LogicMode, Process,
    ProcessKey, ProcessKind, RandomKind, ResizeOp, ResizeOpSimplification, SCALAR_VSIZE,
    SelectMerge, ShiftImmOp, SignalKey, TIME_VSIZE, TemporalRegionKey, Time, UnaryOp,
    UnaryOpSimplification, VSIZE_32, VSIZE_64, VariableKey, VectorSize,
};

#[must_use]
pub struct BasicBlockBuilder {
    key: BasicBlockKey,
    tr: TemporalRegionKey,
    pub instrs: Vec<Instruction>,
}

pub struct ProcessBuilder {
    key: Option<ProcessKey>,
    trs: Vec<TemporalRegionKey>,
}

impl ProcessBuilder {
    pub fn new_anonymous(gl: &'_ mut GlobalContext) -> (Self, BasicBlockBuilder) {
        let bb_key = gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: TemporalRegionKey::default(),
            terminator: BasicBlockTerminator::Halt,
        });
        let entry = TemporalRegionKey::from_entry(bb_key);
        gl.bbs[bb_key].region = entry;
        (
            Self {
                key: None,
                trs: vec![entry],
            },
            BasicBlockBuilder {
                key: bb_key,
                tr: entry,
                instrs: Vec::new(),
            },
        )
    }

    pub fn new(
        gl: &'_ mut GlobalContext,
        kind: ProcessKind,
        origin: TokenRange,
    ) -> (Self, BasicBlockBuilder) {
        let bb_key = gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: TemporalRegionKey::default(),
            terminator: BasicBlockTerminator::Halt,
        });
        let entry = TemporalRegionKey::from_entry(bb_key);
        gl.bbs[bb_key].region = entry;
        let process_key = gl.processes.insert(Process {
            standing: None,
            kind,
            regions: Vec::new(),
            origin,
        });
        (
            Self {
                key: Some(process_key),
                trs: vec![entry],
            },
            BasicBlockBuilder {
                key: bb_key,
                tr: entry,
                instrs: Vec::new(),
            },
        )
    }

    pub fn key(&self) -> Option<ProcessKey> {
        self.key
    }

    pub fn finalize(self, gl: &mut GlobalContext) {
        check_ir_form(&self.trs, gl);

        if let Some(key) = self.key {
            gl.processes[key].regions = self.trs;
        }
    }

    pub fn next_temporal_region(&mut self, gl: &mut GlobalContext) -> TemporalRegionKey {
        let entry = gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: TemporalRegionKey::default(),
            terminator: BasicBlockTerminator::Halt,
        });
        let tr = TemporalRegionKey::from_entry(entry);
        gl.bbs[entry].region = tr;
        self.trs.push(tr);
        tr
    }

    pub fn push_temporal_region(&mut self, tr: TemporalRegionKey) {
        debug_assert!(!self.trs.contains(&tr));
        self.trs.push(tr);
    }

    pub fn set_standing(&self, gl: &mut GlobalContext, signals: Box<[SignalKey]>) {
        gl.processes[self.key.unwrap()].standing = Some(signals);
    }

    pub fn entry(&self) -> TemporalRegionKey {
        self.trs[0]
    }
}

pub struct PhiRef(BasicBlockKey, usize);
pub struct BranchRef(BasicBlockKey);

impl BranchRef {
    pub fn origin_key(&self) -> BasicBlockKey {
        self.0
    }
}

macro_rules! unary_ops {
    ($(($name:ident, $op:ident))+) => {
        $(
        pub fn $name(
            &mut self,
            gl: &mut GlobalContext,
            src: VariableKey,
        ) -> VariableKey {
            self.unary_op(gl, src, UnaryOp::$op)
        }
        )+
    };
}

macro_rules! resize_ops {
    ($(($name:ident, $op:ident))+) => {
        $(
        pub fn $name(
            &mut self,
            gl: &mut GlobalContext,
            src: VariableKey,
            size: VectorSize,
        ) -> VariableKey {
            self.resize_op(gl, size, src, ResizeOp::$op)
        }
        )+
    };
}

macro_rules! bin_ops {
    ($(($name:ident, $op:ident))+) => {
        $(
        pub fn $name(
            &mut self,
            gl: &mut GlobalContext,
            lhs: VariableKey,
            rhs: VariableKey,
        ) -> VariableKey {
            self.bin_op(gl, lhs, rhs, BinaryOp::$op)
        }
        )+
    };
}

macro_rules! bin_imm_ops {
    ($(($name:ident, $op:ident))+) => {
        $(
        pub fn $name(
            &mut self,
            gl: &mut GlobalContext,
            src: VariableKey,
            imm: Bits,
        ) -> VariableKey {
            self.bin_imm_op(gl, src, imm, BinaryImmOp::$op)
        }
        )+
    };
}

macro_rules! shift_imm_ops {
    ($(($name:ident, $op:ident))+) => {
        $(
        pub fn $name(
            &mut self,
            gl: &mut GlobalContext,
            src: VariableKey,
            imm: u32,
        ) -> VariableKey {
            self.shift_imm_op(gl, src, imm, ShiftImmOp::$op)
        }
        )+
    };
}

impl BasicBlockBuilder {
    pub fn key(&self) -> BasicBlockKey {
        self.key
    }

    pub fn instrs_len(&self) -> usize {
        self.instrs.len()
    }

    pub fn next_bb_non_temporal(&mut self, gl: &mut GlobalContext) -> BasicBlockKey {
        gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: self.tr,
            terminator: BasicBlockTerminator::Halt,
        })
    }

    pub fn next_bb_temporal(&mut self, gl: &mut GlobalContext) -> BasicBlockKey {
        let key = gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: self.tr,
            terminator: BasicBlockTerminator::Halt,
        });
        let region = TemporalRegionKey::from_entry(key);
        gl.bbs[key].region = region;
        key
    }

    pub fn next_builder_non_temporal(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb_non_temporal(gl);
        BasicBlockBuilder {
            key: next_key,
            tr: self.tr,
            instrs: Vec::new(),
        }
    }

    pub fn phi(
        &mut self,
        gl: &mut GlobalContext,
        srcs: Box<[(BasicBlockKey, VariableKey)]>,
    ) -> (VariableKey, PhiRef) {
        assert!(!srcs.is_empty());
        let (_, var) = srcs.first().unwrap();
        let size = gl.vars.size(*var);
        let mode = srcs.iter().fold(LogicMode::TwoValue, |acc, (_, v)| {
            if matches!((acc, v.mode()), (LogicMode::TwoValue, LogicMode::TwoValue)) {
                LogicMode::TwoValue
            } else {
                LogicMode::FourValue
            }
        });
        assert!(srcs.iter().all(|(_, v)| size == gl.vars.size(*v)));
        let dst = gl.vars.insert(mode, size);
        let offset = self.instrs.len();
        self.instrs.push(Instruction::Phi(dst, srcs));
        (dst, PhiRef(self.key(), offset))
    }

    pub fn update_phi_ref(
        &mut self,
        gl: &mut GlobalContext,
        phi_ref: PhiRef,
        idx: usize,
        bb: BasicBlockKey,
        var: VariableKey,
    ) {
        let instr = if self.key() == phi_ref.0 {
            &mut self.instrs[phi_ref.1]
        } else {
            &mut gl.bbs[phi_ref.0].instrs[phi_ref.1]
        };
        let Instruction::Phi(_, srcs) = instr else {
            panic!("not a phi");
        };
        srcs[idx] = (bb, var);
    }

    pub fn update_branch_truthy(
        &mut self,
        gl: &mut GlobalContext,
        branch_ref: BranchRef,
        bb: BasicBlockKey,
    ) {
        debug_assert_eq!(gl.bbs[branch_ref.0].region, gl.bbs[bb].region);
        let BasicBlockTerminator::Branch(_, truthy, _) = &mut gl.bbs[branch_ref.0].terminator
        else {
            panic!("not a branch");
        };
        *truthy = bb;
    }

    pub fn update_branch_falsy(
        &mut self,
        gl: &mut GlobalContext,
        branch_ref: BranchRef,
        bb: BasicBlockKey,
    ) {
        debug_assert_eq!(gl.bbs[branch_ref.0].region, gl.bbs[bb].region);
        let BasicBlockTerminator::Branch(_, _, falsy) = &mut gl.bbs[branch_ref.0].terminator else {
            panic!("not a branch");
        };
        *falsy = bb;
    }

    pub fn constant(&mut self, gl: &mut GlobalContext, value: Bits) -> VariableKey {
        let value = value.try_lower_mode();
        let mode = if value.contains_special() {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        };
        let variable = gl.vars.insert(mode, value.size());
        self.instrs.push(Instruction::Constant(variable, value));
        variable
    }

    pub fn constant_u32(&mut self, gl: &mut GlobalContext, value: u32) -> VariableKey {
        let variable = gl.vars.insert(LogicMode::TwoValue, VSIZE_32);
        self.instrs
            .push(Instruction::Constant(variable, Bits::new_u32(value)));
        variable
    }
    pub fn constant_u64(&mut self, gl: &mut GlobalContext, value: u64) -> VariableKey {
        let variable = gl.vars.insert(LogicMode::TwoValue, VSIZE_64);
        self.instrs
            .push(Instruction::Constant(variable, Bits::new_u64(value)));
        variable
    }

    unary_ops! {
        (binary_not, Not)
        (reduce_or, ReduceOr)
        (reduce_and, ReduceAnd)
        (reduce_xor, ReduceXor)
        (count_leading_zeros, LeadingZeros)
        (tv_to_fv, TvToFv)
        (fv_to_tv, FvToTv)
        (real_to_logical, RealToLogical)
        (real_to_u64, RealToU64)
        (real_to_i64, RealToI64)
        (real_from_unsigned_decimal, RealFromUnsignedDecimal)
        (real_from_signed_decimal, RealFromSignedDecimal)
        (real_neg, RealNeg)
        (real_truncate, RealTruncate)
        (real_ln, RealLn)
        (real_log10, RealLog10)
        (real_exp, RealExp)
        (real_sqrt, RealSqrt)
        (real_floor, RealFloor)
        (real_ceil, RealCeil)
        (real_sin, RealSin)
        (real_cos, RealCos)
        (real_tan, RealTan)
        (real_asin, RealASin)
        (real_acos, RealACos)
        (real_atan, RealATan)
        (real_sinh, RealSinH)
        (real_cosh, RealCosH)
        (real_tanh, RealTanH)
        (real_asinh, RealASinH)
        (real_acosh, RealACosH)
        (real_atanh, RealATanH)
    }

    resize_ops! {
        (truncate, Truncate)
        (zero_extend, ZeroExtend)
        (sign_extend, SignExtend)
    }

    bin_ops! {
        (max, Max)
        (min, Min)
        (and, And)
        (or, Or)
        (xor, Xor)
        (andnot, AndNot)
        (ornot, OrNot)
        (xnor, Xnor)
        (plus, Add)
        (minus, Sub)
        (multiply, Multiply)
        (divide, DivideX)
        (modulus, ModulusX)
        (power, Power)
        (concat, Concat)
        (negedge, Negedge)
        (copy_x, CopyX)
        (copy_z, CopyZ)
        (unsigned_le, UnsignedLessEqual)
        (case_equals, CaseEquality)
        (logical_shift_left, LogicalShiftLeft)
        (logical_shift_right, LogicalShiftRight)
        (arithmetic_shift_right, ArithmeticShiftRight)
        (real_add, RealAdd)
        (real_sub, RealSub)
        (real_mul, RealMul)
        (real_div, RealDiv)
        (real_pow, RealPow)
        (real_eq, RealEq)
        (real_ne, RealNe)
        (real_gt, RealGt)
        (real_geq, RealGeq)
        (real_lt, RealLt)
        (real_leq, RealLeq)
        (real_atan2, RealATan2)
        (real_hypot, RealHypot)
    }

    bin_imm_ops! {
        (max_constant, Max)
        (min_constant, Min)
        (and_constant, And)
        (or_constant, Or)
        (xor_constant, Xor)
        (plus_constant, Add)
        (minus_constant, Sub)
        (revminus_constant, RevSub)
        (multiply_constant, Multiply)
        (divide_constant, Divide)
        (revdivide_constant, RevDivideX)
        (modulus_constant, Modulus)
        (revmodulus_constant, RevModulusX)
        (power_constant, Power)
        (revpower_constant, RevPower)
        (unsigned_le_constant, UnsignedLessEqual)
        (unsigned_ge_constant, UnsignedGreaterEqual)
        (case_equals_constant, CaseEquality)
        (not_case_equals_constant, CaseInequality)
    }

    shift_imm_ops! {
        (logical_shift_left_imm, LogicalShiftLeft)
        (logical_shift_right_imm, LogicalShiftRight)
        (arith_shift_right_imm, ArithmeticShiftRight)
    }

    pub fn logical_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let src = self.reduce_or(gl, src);
        self.binary_not(gl, src)
    }

    pub fn unary_op(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        op: UnaryOp,
    ) -> VariableKey {
        let src_size = gl.vars.size(src);
        let dst_size = op.output_size(src_size);
        let Some(mode) = op.output_mode(src.mode()) else {
            panic!("invalid mode");
        };
        match op.simplify(src_size, mode) {
            UnaryOpSimplification::Keep => {
                let dst = gl.vars.insert(mode, dst_size);
                self.instrs.push(Instruction::Unary(dst, op, src));
                dst
            }
            UnaryOpSimplification::Source => src,
        }
    }
    pub fn resize_op(
        &mut self,
        gl: &mut GlobalContext,
        dst: VectorSize,
        src: VariableKey,
        op: ResizeOp,
    ) -> VariableKey {
        let size = gl.vars.size(src);
        let mode = op.output_mode(src.mode());
        match op.simplify(dst, size, mode) {
            ResizeOpSimplification::Keep => {
                let dst = gl.vars.insert(mode, dst);
                self.instrs.push(Instruction::Resize(dst, op, src));
                dst
            }
            ResizeOpSimplification::Source => src,
        }
    }
    pub fn bin_op(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
    ) -> VariableKey {
        let lhs_size = gl.vars.size(lhs);
        let rhs_size = gl.vars.size(rhs);
        let Some(size) = op.output_size(lhs_size, rhs_size) else {
            panic!("Invalid size combination for {op:?}: {lhs_size}, {rhs_size}");
        };
        let output_mode = op.output_mode(lhs.mode(), rhs.mode());
        let dst = gl.vars.insert(output_mode.dst, size);
        let lhs = self.convert_mode(gl, lhs, output_mode.lhs);
        let rhs = self.convert_mode(gl, rhs, output_mode.rhs);
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
        dst
    }
    pub fn bin_imm_op(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
        op: BinaryImmOp,
    ) -> VariableKey {
        let imm = imm.try_lower_mode();
        let Some(size) = op.output_size(gl.vars.size(src), imm.size()) else {
            panic!("Invalid size combination");
        };
        let imm_mode = if imm.contains_special() {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        };
        let output_mode = op.output_mode(src.mode(), imm_mode);
        let src = self.convert_mode(gl, src, output_mode.src);
        let mut dst = gl.vars.insert(output_mode.dst, size);
        match op.simplify(dst, src, &imm) {
            BinaryImmOpSimplification::Keep => {
                self.instrs.push(Instruction::BinaryImm(dst, op, src, imm))
            }
            BinaryImmOpSimplification::Source => dst = src,
            BinaryImmOpSimplification::Immediate => {
                self.instrs.push(Instruction::Constant(dst, imm))
            }
            BinaryImmOpSimplification::Constant(bits) => {
                self.instrs.push(Instruction::Constant(dst, bits))
            }
            BinaryImmOpSimplification::Instruction(i) => self.instrs.push(i),
        }
        dst
    }
    pub fn shift_imm_op(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: u32,
        op: ShiftImmOp,
    ) -> VariableKey {
        if imm == 0 {
            return src;
        }

        let src_size = gl.vars.size(src);
        let dst = gl.vars.insert(src.mode(), src_size);
        self.instrs.push(Instruction::ShiftImm(dst, op, src, imm));
        dst
    }

    pub fn convert_mode(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        mode: LogicMode,
    ) -> VariableKey {
        if src.mode() == mode {
            return src;
        }

        match mode {
            LogicMode::TwoValue => self.fv_to_tv(gl, src),
            LogicMode::FourValue => self.tv_to_fv(gl, src),
        }
    }

    pub fn select_bit(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        idx: VariableKey,
    ) -> VariableKey {
        self.slice(gl, src, idx, SCALAR_VSIZE)
    }
    pub fn select_bit_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        idx: u32,
    ) -> VariableKey {
        self.slice_constant(gl, src, idx, SCALAR_VSIZE)
    }
    pub fn slice_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        offset: u32,
        width: VectorSize,
    ) -> VariableKey {
        let src_size = gl.vars.size(src);
        assert!(gl.vars.size(src) >= width);
        if offset == 0 {
            return self.truncate(gl, src, width);
        }
        if offset >= src_size.get() {
            return self.constant(gl, Bits::new_unknown(width));
        }
        let dst = gl.vars.insert(src.mode(), width);
        self.instrs.push(Instruction::SliceImm(dst, src, offset));
        if offset <= src_size.get() - width.get() {
            return dst;
        }

        // Slice and SliceImm are slightly different in that out-of-bounds values are set as
        // unknown for Slice and set as zeros for SliceImm. We compensate to compensate for that
        // here.
        self.or_constant(
            gl,
            dst,
            Bits::new_unknown(width).logical_shift_left(src_size.get() - offset),
        )
    }
    pub fn slice(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        offset: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(offset), INTEGER_VSIZE);
        assert!(gl.vars.size(src) >= width);
        let dst = gl.vars.insert(LogicMode::FourValue, width);
        self.instrs.push(Instruction::Slice(dst, src, offset));
        dst
    }

    pub fn unsigned_lt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let ge = self.unsigned_le(gl, rhs, lhs);
        self.logical_neg(gl, ge)
    }
    pub fn unsigned_gt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let le = self.unsigned_le(gl, lhs, rhs);
        self.logical_neg(gl, le)
    }
    pub fn unsigned_ge(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.unsigned_le(gl, rhs, lhs)
    }

    pub fn signed_lt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let ge = self.signed_le(gl, rhs, lhs);
        self.logical_neg(gl, ge)
    }
    pub fn signed_gt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let le = self.signed_le(gl, lhs, rhs);
        self.logical_neg(gl, le)
    }
    pub fn signed_le(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(lhs), gl.vars.size(rhs));
        let mask = Bits::new_with_msb_one(gl.vars.size(lhs));
        let lhs = self.xor_constant(gl, lhs, mask.clone());
        let rhs = self.xor_constant(gl, rhs, mask);
        self.unsigned_le(gl, lhs, rhs)
    }
    pub fn signed_ge(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.signed_le(gl, rhs, lhs)
    }

    pub fn logical_or(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs = self.reduce_or(gl, lhs);
        let rhs = self.reduce_or(gl, rhs);
        self.or(gl, lhs, rhs)
    }
    pub fn logical_and(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs = self.reduce_or(gl, lhs);
        let rhs = self.reduce_or(gl, rhs);
        self.and(gl, lhs, rhs)
    }
    pub fn real_logical_and(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs = self.real_to_logical(gl, lhs);
        let rhs = self.real_to_logical(gl, rhs);
        self.and(gl, lhs, rhs)
    }
    pub fn real_logical_or(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs = self.real_to_logical(gl, lhs);
        let rhs = self.real_to_logical(gl, rhs);
        self.or(gl, lhs, rhs)
    }

    pub fn equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xnor = self.xnor(gl, lhs, rhs);
        self.reduce_and(gl, xnor)
    }
    pub fn not_equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xor = self.xor(gl, lhs, rhs);
        self.reduce_or(gl, xor)
    }
    pub fn not_case_equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let case_equals = self.case_equals(gl, lhs, rhs);
        self.logical_neg(gl, case_equals)
    }

    pub fn reduce_xnor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let xor = self.reduce_xor(gl, src);
        self.logical_neg(gl, xor)
    }
    pub fn reduce_nor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let or = self.reduce_or(gl, src);
        self.logical_neg(gl, or)
    }
    pub fn reduce_nand(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let and = self.reduce_and(gl, src);
        self.logical_neg(gl, and)
    }

    fn sign_invert(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        self.revminus_constant(gl, src, Bits::new_zeroed(gl.vars.size(src)))
    }

    fn signed_divmod_prep(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> (VariableKey, VariableKey, VariableKey, VariableKey) {
        let lhs_size = gl.vars.size(lhs);
        let size_m_1 = lhs_size.get() - 1;

        let sx = self.logical_shift_right_imm(gl, lhs, size_m_1);
        let sx = self.sign_invert(gl, sx);
        let sy = self.logical_shift_right_imm(gl, rhs, size_m_1);
        let sy = self.sign_invert(gl, sy);

        let ax = self.xor(gl, lhs, sx);
        let ax = self.minus(gl, ax, sx);
        let ay = self.xor(gl, rhs, sy);
        let ay = self.minus(gl, ay, sy);

        (sx, sy, ax, ay)
    }

    pub fn signed_divide(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let (sx, sy, ax, ay) = self.signed_divmod_prep(gl, lhs, rhs);

        let qu = self.divide(gl, ax, ay);
        let s = self.xor(gl, sx, sy);

        let q = self.xor(gl, qu, s);
        self.minus(gl, q, s)
    }
    pub fn signed_modulus(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let (sx, _, ax, ay) = self.signed_divmod_prep(gl, lhs, rhs);

        let ru = self.modulus(gl, ax, ay);
        let r = self.xor(gl, ru, sx);
        self.minus(gl, r, sx)
    }

    pub fn drive(&mut self, gl: &mut GlobalContext, signal: SignalKey, src: VariableKey) {
        self.drive_opt_partial(gl, signal, src, None);
    }
    pub fn drive_partial_constant(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: u32,
    ) -> VariableKey {
        let src_size = gl.vars.size(src);
        let dst = gl.vars.insert(LogicMode::TwoValue, src_size);
        let src = self.convert_mode(gl, src, gl.signals[signal].mode);
        self.instrs
            .push(Instruction::Drive(dst, signal, src, offset));
        dst
    }
    pub fn drive_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: VariableKey,
    ) {
        self.drive_opt_partial(gl, signal, src, Some(offset));
    }
    pub fn drive_opt_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        partial: Option<VariableKey>,
    ) -> VariableKey {
        let src_size = gl.vars.size(src);
        let dst = gl.vars.insert(LogicMode::TwoValue, src_size);
        let src = self.convert_mode(gl, src, gl.signals[signal].mode);
        match partial {
            None => self.instrs.push(Instruction::Drive(dst, signal, src, 0)),
            Some(offset) => {
                assert_eq!(gl.vars.size(offset), INTEGER_VSIZE);
                self.instrs
                    .push(Instruction::DriveSlice(dst, signal, src, offset))
            }
        }
        dst
    }

    pub fn probe(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        let s = &gl.signals[signal];
        let size = s.size;
        let dst = gl.vars.insert(s.mode, size);
        self.instrs.push(Instruction::Probe(dst, signal, 0));
        dst
    }

    pub fn probe_slice(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        offset: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let dst = gl.vars.insert(LogicMode::FourValue, width);
        self.instrs
            .push(Instruction::ProbeSlice(dst, signal, offset));
        dst
    }

    pub fn probe_slice_constant(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        offset: u32,
        width: VectorSize,
    ) -> VariableKey {
        let s = &gl.signals[signal];
        let src_size = s.size;
        let dst = gl.vars.insert(s.mode, width);
        self.instrs.push(Instruction::Probe(dst, signal, offset));
        if offset <= src_size.get() - width.get() {
            return dst;
        }

        // Slice and SliceImm are slightly different in that out-of-bounds values are set as
        // unknown for Slice and set as zeros for SliceImm. We compensate to compensate for that
        // here.
        self.or_constant(
            gl,
            dst,
            Bits::new_unknown(width).logical_shift_left(src_size.get() - offset),
        )
    }

    pub fn finalize(&mut self, gl: &mut GlobalContext, terminator: BasicBlockTerminator) {
        let bb = &mut gl.bbs[self.key];
        bb.instrs = std::mem::take(&mut self.instrs);
        bb.terminator = terminator;
    }

    pub fn jump(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_builder = self.next_builder_non_temporal(gl);
        self.finalize(gl, BasicBlockTerminator::Jump(next_builder.key()));
        next_builder
    }
    pub fn jump_to(&mut self, gl: &mut GlobalContext, bb: BasicBlockKey) {
        debug_assert_eq!(self.tr, gl.bbs[bb].region);
        self.finalize(gl, BasicBlockTerminator::Jump(bb));
    }

    pub fn next_terminate_later(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        self.finalize(gl, BasicBlockTerminator::Halt);
        self.next_builder_non_temporal(gl)
    }

    pub fn new_basic_block(&mut self, gl: &mut GlobalContext) -> BasicBlockKey {
        self.next_bb_non_temporal(gl)
    }

    pub fn continue_from(
        instrs: Vec<Instruction>,
        tr: TemporalRegionKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        BasicBlockBuilder {
            key: bb,
            tr,
            instrs,
        }
    }

    pub fn push_raw_instruction(&mut self, instruction: Instruction) {
        self.instrs.push(instruction);
    }

    pub fn into_instructions(self) -> Vec<Instruction> {
        self.instrs
    }

    pub fn branch(
        &mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
    ) -> (BranchRef, BasicBlockBuilder) {
        assert_eq!(gl.vars.size(condition), SCALAR_VSIZE);
        let branch_bb = self.key();

        let next_builder = self.next_builder_non_temporal(gl);
        let next_key = next_builder.key();
        self.finalize(
            gl,
            BasicBlockTerminator::Branch(condition, next_key, next_key),
        );
        (BranchRef(branch_bb), next_builder)
    }

    pub fn double_branch(
        &mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
    ) -> (BasicBlockBuilder, BasicBlockBuilder) {
        assert_eq!(gl.vars.size(condition), SCALAR_VSIZE);
        let next_builder_truthy = self.next_builder_non_temporal(gl);
        let next_builder_falsy = self.next_builder_non_temporal(gl);
        self.finalize(
            gl,
            BasicBlockTerminator::Branch(
                condition,
                next_builder_truthy.key(),
                next_builder_falsy.key(),
            ),
        );
        (next_builder_truthy, next_builder_falsy)
    }

    pub fn branch_true_to(
        mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let (bref, builder) = self.branch(gl, condition);
        self.update_branch_truthy(gl, bref, bb);
        builder
    }
    pub fn branch_false_to(
        mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let (bref, builder) = self.branch(gl, condition);
        self.update_branch_falsy(gl, bref, bb);
        builder
    }

    pub fn halt(mut self, gl: &mut GlobalContext) {
        self.finalize(gl, BasicBlockTerminator::Halt);
    }

    fn temporal_term_to(&mut self, gl: &mut GlobalContext, terminator: BasicBlockTerminator) {
        if cfg!(debug_assertions) {
            use BasicBlockTerminator as T;
            let tgt = match terminator {
                T::Wait(tgt, _)
                | T::VariableWait(tgt, _)
                | T::WaitRegion(tgt, _)
                | T::Watch(tgt, _) => tgt,
                T::Jump(..) => unreachable!(),
                T::Branch(..) => unreachable!(),
                T::Halt => unreachable!(),
            };
            assert_eq!(gl.bbs[tgt.entry()].region, tgt);
        }
        self.finalize(gl, terminator);
    }

    pub fn wait_to(&mut self, gl: &mut GlobalContext, time: Time, tr: TemporalRegionKey) {
        self.temporal_term_to(gl, BasicBlockTerminator::Wait(tr, time))
    }
    pub fn variable_wait_to(
        &mut self,
        gl: &mut GlobalContext,
        time: VariableKey,
        tr: TemporalRegionKey,
    ) {
        self.temporal_term_to(gl, BasicBlockTerminator::VariableWait(tr, time))
    }
    pub fn wait_region_to(&mut self, gl: &mut GlobalContext, region: u8, tr: TemporalRegionKey) {
        self.temporal_term_to(gl, BasicBlockTerminator::WaitRegion(tr, region))
    }
    pub fn watch_to(
        &mut self,
        gl: &mut GlobalContext,
        signals: Vec<SignalKey>,
        tr: TemporalRegionKey,
    ) {
        self.temporal_term_to(gl, BasicBlockTerminator::Watch(tr, signals))
    }
    pub fn temporal_jump_to(&mut self, gl: &mut GlobalContext, tr: TemporalRegionKey) {
        self.wait_to(gl, Time(0), tr);
    }

    pub fn intrinsic(
        &mut self,
        gl: &mut GlobalContext,
        op: IntrinsicOp,
        args: Box<[VariableKey]>,
    ) -> VariableKey {
        let dst = gl.vars.insert(LogicMode::TwoValue, SCALAR_VSIZE);
        self.instrs
            .push(Instruction::Intrinsic(dst, Box::new(op), args));
        dst
    }

    pub fn blackbox(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let dst = gl.vars.insert(src.mode(), gl.vars.size(src));
        self.instrs.push(Instruction::Intrinsic(
            dst,
            Box::new(IntrinsicOp::BlackBox),
            [src].into(),
        ));
        dst
    }
    pub fn time(&mut self, gl: &mut GlobalContext) -> VariableKey {
        let dst = gl.vars.insert(LogicMode::TwoValue, TIME_VSIZE);
        self.instrs.push(Instruction::Intrinsic(
            dst,
            Box::new(IntrinsicOp::Time),
            Default::default(),
        ));
        dst
    }
    pub fn random(
        &mut self,
        gl: &mut GlobalContext,
        kind: RandomKind,
        seed: VariableKey,
        args: &[VariableKey],
    ) -> VariableKey {
        let mode = if seed.mode().is_two_value() && args.iter().all(|v| v.mode().is_two_value()) {
            LogicMode::TwoValue
        } else {
            LogicMode::FourValue
        };
        let dst = gl.vars.insert(mode, VSIZE_64);
        self.instrs.push(Instruction::Intrinsic(
            dst,
            Box::new(IntrinsicOp::Random(kind)),
            std::iter::once(seed)
                .chain(args.iter().copied())
                .collect::<Box<[VariableKey]>>(),
        ));
        dst
    }

    pub fn equals_zero(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let not_equals_zero = self.reduce_or(gl, src);
        self.logical_neg(gl, not_equals_zero)
    }

    pub fn lupdt(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        let dst = gl.vars.insert(LogicMode::TwoValue, TIME_VSIZE);
        self.instrs.push(Instruction::LastUpdateTime(dst, signal));
        dst
    }

    pub fn select_merge(
        &mut self,
        gl: &mut GlobalContext,
        select: VariableKey,
        truthy: VariableKey,
        falsy: VariableKey,
    ) -> VariableKey {
        let size = gl.vars.size(truthy);
        assert_eq!(size, gl.vars.size(falsy));
        assert_eq!(SCALAR_VSIZE, gl.vars.size(select));
        let mode = select.mode().max(truthy.mode()).max(falsy.mode());
        let truthy = self.convert_mode(gl, truthy, mode);
        let falsy = self.convert_mode(gl, falsy, mode);
        let dst = gl.vars.insert(mode, size);
        self.instrs.push(Instruction::Select(
            dst,
            select,
            truthy,
            falsy,
            SelectMerge::Merge,
        ));
        dst
    }

    pub fn select(
        &mut self,
        gl: &mut GlobalContext,
        select: VariableKey,
        truthy: VariableKey,
        falsy: VariableKey,
    ) -> VariableKey {
        let size = gl.vars.size(truthy);
        assert_eq!(size, gl.vars.size(falsy));
        assert_eq!(SCALAR_VSIZE, gl.vars.size(select));
        let mode = truthy.mode().max(falsy.mode());
        let truthy = self.convert_mode(gl, truthy, mode);
        let falsy = self.convert_mode(gl, falsy, mode);
        let dst = gl.vars.insert(mode, size);
        self.instrs.push(Instruction::Select(
            dst,
            select,
            truthy,
            falsy,
            SelectMerge::FalseOnSpecial,
        ));
        dst
    }

    pub fn rev_imm_slice_x(
        &mut self,
        gl: &mut GlobalContext,
        value: Bits,
        offset: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let value = self.constant(gl, value);
        self.slice(gl, value, offset, width)
    }

    /// Convert all Z values to X values.
    pub fn z_to_x(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        match src.mode() {
            LogicMode::TwoValue => src,
            LogicMode::FourValue => {
                let size = gl.vars.size(src);
                self.and_constant(gl, src, Bits::new_ones(size))
            }
        }
    }

    pub fn nand(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let and = self.and(gl, lhs, rhs);
        self.binary_not(gl, and)
    }
    pub fn nor(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let and = self.or(gl, lhs, rhs);
        self.binary_not(gl, and)
    }

    pub fn posedge(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.negedge(gl, rhs, lhs)
    }

    pub fn finalize_and_switch_to(
        &mut self,
        gl: &mut GlobalContext,
        terminator: BasicBlockTerminator,
        to: BasicBlockKey,
    ) {
        self.finalize(gl, terminator);

        let bb = &mut gl.bbs[to];
        self.key = to;
        self.tr = bb.region;
        self.instrs = std::mem::take(&mut bb.instrs);
    }

    pub fn switch_to(&mut self, gl: &mut GlobalContext, to: BasicBlockKey) {
        std::mem::swap(&mut gl.bbs[self.key].instrs, &mut self.instrs);

        let bb = &mut gl.bbs[to];
        self.key = to;
        self.tr = bb.region;
        self.instrs = std::mem::take(&mut bb.instrs);
    }

    pub fn finished_switch_to(&mut self, gl: &mut GlobalContext, to: BasicBlockKey) {
        let bb = &mut gl.bbs[to];
        self.key = to;
        self.tr = bb.region;
        self.instrs = std::mem::take(&mut bb.instrs);
    }

    pub fn tr(&self) -> TemporalRegionKey {
        self.tr
    }

    pub fn jump_to_tr(&mut self, gl: &mut GlobalContext, tr: TemporalRegionKey) {
        if self.tr == tr {
            self.finalize(gl, BasicBlockTerminator::Jump(tr.entry()));
        } else {
            self.wait_to(gl, Time(0), tr);
        }
    }

    pub fn jump_to_any(&mut self, gl: &mut GlobalContext, bb: BasicBlockKey) {
        let target_tr = gl.bbs[bb].region;
        if self.tr == target_tr {
            self.finalize(gl, BasicBlockTerminator::Jump(bb));
        } else {
            debug_assert_eq!(target_tr.entry(), bb);
            self.wait_to(gl, Time(0), target_tr);
        }
    }

    pub fn mark_as_tr_root(
        &mut self,
        gl: &mut GlobalContext,
        root: BasicBlockKey,
    ) -> TemporalRegionKey {
        let tr = TemporalRegionKey::from_entry(root);

        let mut stack = vec![root];
        let mut seen = VgHashSet::<BasicBlockKey>::default();
        seen.insert(root);

        while let Some(bb_key) = stack.pop() {
            gl.bbs[bb_key].region = tr;
            gl.bbs[bb_key].terminator.for_each_non_temporal_bb(|next| {
                if seen.insert(next) {
                    stack.push(next);
                }
            });
        }

        if seen.contains(&self.key) {
            self.tr = tr;
        }

        tr
    }
}
