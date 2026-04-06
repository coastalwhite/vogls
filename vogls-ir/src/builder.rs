use crate::token_range::TokenRange;
use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryImmOpSimplification,
    BinaryOp, Bits, GlobalContext, INTEGER_VSIZE, Instruction, IntrinsicOp, Process, ProcessKey,
    ResizeOp, SCALAR_VSIZE, SignalKey, TIME_VSIZE, Time, UnaryOp, Variable, VariableKey,
    VectorSize,
};

#[must_use]
pub struct BasicBlockBuilder {
    key: BasicBlockKey,
    pub instrs: Vec<Instruction>,
}

pub fn new_process(
    gl: &'_ mut GlobalContext,
    name: String,
    origin: TokenRange,
) -> (ProcessKey, BasicBlockBuilder) {
    let bb_key = gl.bbs.insert(BasicBlock {
        instrs: Vec::new(),
        terminator: BasicBlockTerminator::Halt,
    });
    let process_key = gl.processes.insert(Process {
        name,
        entry: bb_key,
        origin,
    });
    (
        process_key,
        BasicBlockBuilder {
            key: bb_key,
            instrs: Vec::new(),
        },
    )
}
pub fn new_anonymous_builder(gl: &'_ mut GlobalContext) -> BasicBlockBuilder {
    let bb_key = gl.bbs.insert(BasicBlock {
        instrs: Vec::new(),
        terminator: BasicBlockTerminator::Halt,
    });
    BasicBlockBuilder {
        key: bb_key,
        instrs: Vec::new(),
    }
}

pub struct PhiRef(BasicBlockKey, usize);
pub struct BranchRef(BasicBlockKey);

impl BranchRef {
    pub fn origin_key(&self) -> BasicBlockKey {
        self.0
    }
}

macro_rules! arithmetic_op {
    ($(($name:ident, $op:ident),)+) => {
        $(
        pub fn $name(
            &mut self,
            gl: &mut GlobalContext,
            lhs: VariableKey,
            rhs: VariableKey,
        ) -> VariableKey {
            self.bin_arithmetic(gl, lhs, rhs, BinaryOp::$op)
        }
        )+
    }
}

impl BasicBlockBuilder {
    pub fn key(&self) -> BasicBlockKey {
        self.key
    }

    pub fn instrs_len(&self) -> usize {
        self.instrs.len()
    }

    pub fn next_tmp_var(&mut self, gl: &mut GlobalContext, size: VectorSize) -> VariableKey {
        gl.vars.insert(Variable { size })
    }
    pub fn next_bb(&mut self, gl: &mut GlobalContext) -> BasicBlockKey {
        gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            terminator: BasicBlockTerminator::Halt,
        })
    }

    pub fn next_builder(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        BasicBlockBuilder {
            key: next_key,
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
        assert!(
            srcs.iter()
                .all(|(_, v)| gl.vars[*var].size == gl.vars[*v].size)
        );
        let dst = self.next_tmp_var(gl, gl.vars[*var].size);
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

    pub fn update_branch_ref(
        &mut self,
        gl: &mut GlobalContext,
        branch_ref: BranchRef,
        bb: BasicBlockKey,
    ) {
        let BasicBlockTerminator::Branch(_, _, snd) = &mut gl.bbs[branch_ref.0].terminator else {
            panic!("not a branch");
        };
        *snd = bb;
    }

    pub fn constant(&mut self, gl: &mut GlobalContext, value: Bits) -> VariableKey {
        let variable = self.next_tmp_var(gl, value.size());
        self.instrs.push(Instruction::Constant(variable, value));
        variable
    }

    pub fn constant_u32(&mut self, gl: &mut GlobalContext, value: u32) -> VariableKey {
        let variable = self.next_tmp_var(gl, VectorSize::new(32).unwrap());
        self.instrs
            .push(Instruction::Constant(variable, Bits::new_u32(value)));
        variable
    }
    pub fn constant_u64(&mut self, gl: &mut GlobalContext, value: u64) -> VariableKey {
        let variable = self.next_tmp_var(gl, VectorSize::new(64).unwrap());
        self.instrs
            .push(Instruction::Constant(variable, Bits::new_u64(value)));
        variable
    }

    pub fn concat(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let size = VectorSize::new(gl.vars[lhs].size.get() + gl.vars[rhs].size.get()).unwrap();
        let dst = self.next_tmp_var(gl, size);
        self.bin_op(gl, lhs, rhs, BinaryOp::Concat, dst);
        dst
    }

    pub fn binary_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let Variable { size } = gl.vars[src];
        let dst = self.next_tmp_var(gl, size);
        self.unary_op(gl, src, UnaryOp::Neg, dst);
        dst
    }

    pub fn logical_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let Variable { size } = gl.vars[src];
        let src = match size.get() {
            1 => src,
            _ => self.reduce_or(gl, src),
        };
        let dst = self.next_tmp_var(gl, VectorSize::new(1).unwrap());
        self.unary_op(gl, src, UnaryOp::Neg, dst);
        dst
    }

    pub fn unary_op(
        &mut self,
        _gl: &mut GlobalContext,
        src: VariableKey,
        op: UnaryOp,
        dst: VariableKey,
    ) {
        self.instrs.push(Instruction::Unary(dst, op, src));
    }
    pub fn resize_op(
        &mut self,
        _gl: &mut GlobalContext,
        dst: VariableKey,
        op: ResizeOp,
        src: VariableKey,
    ) {
        self.instrs.push(Instruction::Resize(dst, op, src));
    }
    pub fn bin_op(
        &mut self,
        _gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
        dst: VariableKey,
    ) {
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
    }
    pub fn bin_imm_op(
        &mut self,
        src: VariableKey,
        imm: Bits,
        op: BinaryImmOp,
        dst: &mut VariableKey,
    ) {
        match op.simplify(*dst, src, &imm) {
            BinaryImmOpSimplification::Keep => {
                self.instrs.push(Instruction::BinaryImm(*dst, op, src, imm))
            }
            BinaryImmOpSimplification::Source => *dst = src,
            BinaryImmOpSimplification::Immediate => {
                self.instrs.push(Instruction::Constant(*dst, imm))
            }
            BinaryImmOpSimplification::Constant(bits) => {
                self.instrs.push(Instruction::Constant(*dst, bits))
            }
            BinaryImmOpSimplification::Instruction(i) => self.instrs.push(i),
        }
    }
    fn copy_op(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
    ) -> VariableKey {
        let size = gl.vars[lhs].size;
        assert_eq!(size, gl.vars[rhs].size);
        let dst = self.next_tmp_var(gl, size);
        self.bin_op(gl, lhs, rhs, op, dst);
        dst
    }

    pub fn bin_arithmetic(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
    ) -> VariableKey {
        let size = gl.vars[lhs].size;
        assert_eq!(size, gl.vars[rhs].size);
        let dst = self.next_tmp_var(gl, size);
        self.bin_op(gl, lhs, rhs, op, dst);
        dst
    }
    pub fn bin_imm_arithmetic(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
        op: BinaryImmOp,
    ) -> VariableKey {
        let size = gl.vars[src].size;
        assert_eq!(size, imm.size());
        let mut dst = self.next_tmp_var(gl, size);
        self.bin_imm_op(src, imm, op, &mut dst);
        dst
    }

    // Bitwise Operations
    arithmetic_op! {
        (and, And),
        (or, Or),
        (xor, Xor),
        (plus, Add),
        (minus, Sub),
        (multiply, Multiply),
        (divide, Divide),
        (modulus, Modulus),
        (power, Power),
    }
    pub fn xnor(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xor = self.xor(gl, lhs, rhs);
        let xnor = self.binary_neg(gl, xor);
        xnor
    }
    pub fn andnot(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let rhs = self.binary_neg(gl, rhs);
        self.and(gl, lhs, rhs)
    }
    pub fn ornot(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let rhs = self.binary_neg(gl, rhs);
        self.or(gl, lhs, rhs)
    }

    // Bitwise Immediate Operations
    pub fn and_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        let num_special = imm.count_special();
        if num_special == imm.size().get() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        let num_ones = imm.count_ones();
        if num_ones == imm.size().get() {
            return src;
        } else if num_special == 0 && num_ones == 0 {
            return self.constant(gl, imm);
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::And)
    }
    pub fn or_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        let num_special = imm.count_special();
        if num_special == imm.size().get() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        let num_ones = imm.count_ones();
        if num_ones == imm.size().get() {
            return self.constant(gl, imm);
        } else if num_special == 0 && num_ones == 0 {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Or)
    }
    pub fn xor_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        let num_special = imm.count_special();
        if num_special == imm.size().get() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        let num_ones = imm.count_ones();
        if num_ones == imm.size().get() {
            return self.binary_neg(gl, src);
        } else if num_special == 0 && num_ones == 0 {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Xor)
    }
    pub fn plus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Add)
    }
    pub fn minus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Sub)
    }
    pub fn multiply_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_zeroed(imm.size()));
        } else if imm.eq_one() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Multiply)
    }
    pub fn power_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_u32(1).truncate_or_zero_extend(imm.size()));
        } else if imm.eq_one() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Power)
    }
    pub fn divide_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_ones(imm.size()));
        } else if imm.eq_one() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Power)
    }
    pub fn modulus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_ones(imm.size()));
        } else if imm.eq_one() {
            return self.constant(gl, Bits::new_zeroed(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Power)
    }
    pub fn revminus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevSub)
    }
    pub fn revpower_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_one() {
            return self.constant(gl, imm);
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevPower)
    }
    pub fn revdivide_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevDivide)
    }
    pub fn revmodulus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars[src].size, imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevModulus)
    }

    pub fn copy_x(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.copy_op(gl, lhs, rhs, BinaryOp::CopyX)
    }
    pub fn copy_z(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.copy_op(gl, lhs, rhs, BinaryOp::CopyZ)
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
    pub fn truncate(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let Variable { size } = gl.vars[src];
        if size == width {
            return src;
        }

        let dst = self.next_tmp_var(gl, width);
        self.resize_op(gl, dst, ResizeOp::Truncate, src);
        dst
    }
    pub fn slice_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        offset: u32,
        width: VectorSize,
    ) -> VariableKey {
        let src_size = gl.vars[src].size;
        assert!(gl.vars[src].size >= width);
        if offset == 0 {
            return self.truncate(gl, src, width);
        }
        if offset >= src_size.get() {
            return self.constant(gl, Bits::new_unknown(width));
        }
        let dst = self.next_tmp_var(gl, width);
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
        assert_eq!(gl.vars[offset].size, INTEGER_VSIZE);
        assert!(gl.vars[src].size >= width);
        let dst = self.next_tmp_var(gl, width);
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
    pub fn unsigned_le(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars[lhs].size, gl.vars[rhs].size);
        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.bin_op(gl, lhs, rhs, BinaryOp::UnsignedLessEqual, dst);
        dst
    }
    pub fn unsigned_ge(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.unsigned_le(gl, rhs, lhs)
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

    pub fn equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xor = self.xor(gl, lhs, rhs);
        let no_equals = self.reduce_or(gl, xor);
        let xnor = self.binary_neg(gl, no_equals);
        xnor
    }
    pub fn not_equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xor = self.xor(gl, lhs, rhs);
        let no_equals = self.reduce_or(gl, xor);
        no_equals
    }
    pub fn case_equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars[lhs].size, gl.vars[rhs].size);
        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.bin_op(gl, lhs, rhs, BinaryOp::CaseEquality, dst);
        dst
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

    pub fn reduce_xor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let Variable { size } = gl.vars[src];
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.unary_op(gl, src, UnaryOp::ReduceXor, dst);
        dst
    }
    pub fn reduce_or(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let Variable { size } = gl.vars[src];
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.unary_op(gl, src, UnaryOp::ReduceOr, dst);
        dst
    }
    pub fn reduce_and(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let Variable { size } = gl.vars[src];
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.unary_op(gl, src, UnaryOp::ReduceAnd, dst);
        dst
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

    pub fn drive(&mut self, gl: &mut GlobalContext, signal: SignalKey, src: VariableKey) {
        self.drive_opt_partial(gl, signal, src, None);
    }
    pub fn drive_partial_constant(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: u32,
        length: VectorSize,
    ) {
        if offset == 0 && length == gl.signals[signal].size {
            return self.drive_opt_partial(gl, signal, src, None);
        }

        let offset = self.constant_u32(gl, offset);
        self.drive_opt_partial(gl, signal, src, Some((offset, length)));
    }
    pub fn drive_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: VariableKey,
        length: VectorSize,
    ) {
        self.drive_opt_partial(gl, signal, src, Some((offset, length)));
    }
    pub fn drive_opt_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        partial: Option<(VariableKey, VectorSize)>,
    ) {
        assert_eq!(
            gl.vars[src].size,
            partial.map_or(gl.signals[signal].size, |(_, w)| w)
        );
        if let Some((offset, _)) = partial {
            assert_eq!(gl.vars[offset].size, INTEGER_VSIZE);
        }
        self.instrs.push(Instruction::Drive(signal, src, partial));
    }

    pub fn probe(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        let size = gl.signals.get(signal).unwrap().size;
        let dst = self.next_tmp_var(gl, size);
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
        let dst = self.next_tmp_var(gl, width);
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
        let src_size = gl.signals[signal].size;
        let dst = self.next_tmp_var(gl, width);
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

    pub fn jump(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Jump(next_key);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }
    pub fn jump_to(mut self, gl: &mut GlobalContext, bb: BasicBlockKey) {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Jump(bb);
    }

    pub fn next_terminate_later(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }

    pub fn continue_with(
        &mut self,
        gl: &mut GlobalContext,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);

        let next_bb = gl.bbs.get_mut(bb).unwrap();
        BasicBlockBuilder {
            key: bb,
            instrs: std::mem::take(&mut next_bb.instrs),
        }
    }

    pub fn continue_from(instrs: Vec<Instruction>, bb: BasicBlockKey) -> BasicBlockBuilder {
        BasicBlockBuilder {
            key: bb,
            instrs: instrs,
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
        assert_eq!(gl.vars[condition].size, SCALAR_VSIZE);
        let branch_bb = self.key();
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Branch(condition, next_key, next_key);
        let builder = BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        };
        (BranchRef(branch_bb), builder)
    }

    pub fn branch_true_to(
        mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Branch(condition, bb, next_key);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }
    pub fn branch_false_to(
        mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Branch(condition, next_key, bb);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }

    pub fn halt(mut self, gl: &mut GlobalContext) {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Halt;
    }

    pub fn wait(mut self, gl: &mut GlobalContext, time: Time) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Wait(next_key, time);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }
    pub fn wait_to(mut self, gl: &mut GlobalContext, time: Time, bb: BasicBlockKey) {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Wait(bb, time);
    }
    pub fn variable_wait(mut self, gl: &mut GlobalContext, time: VariableKey) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::VariableWait(next_key, time);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }
    pub fn variable_wait_to(
        mut self,
        gl: &mut GlobalContext,
        time: VariableKey,
        bb: BasicBlockKey,
    ) {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::VariableWait(bb, time);
    }

    pub fn wait_region(mut self, gl: &mut GlobalContext, region: u8) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::WaitRegion(next_key, region);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }

    pub fn watch(mut self, gl: &mut GlobalContext, signals: Vec<SignalKey>) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Watch(next_key, signals);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }
    pub fn watch_to(mut self, gl: &mut GlobalContext, signals: Vec<SignalKey>, bb: BasicBlockKey) {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Watch(bb, signals);
    }

    pub fn intrinsic(
        &mut self,
        gl: &mut GlobalContext,
        op: IntrinsicOp,
        args: Box<[VariableKey]>,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.instrs
            .push(Instruction::Intrinsic(dst, Box::new(op), args));
        dst
    }

    pub fn time(&mut self, gl: &mut GlobalContext) -> VariableKey {
        let dst = self.next_tmp_var(gl, TIME_VSIZE);
        self.instrs.push(Instruction::Intrinsic(
            dst,
            Box::new(IntrinsicOp::Time),
            Default::default(),
        ));
        dst
    }
    pub fn random(&mut self, gl: &mut GlobalContext) -> VariableKey {
        let dst = self.next_tmp_var(gl, INTEGER_VSIZE);
        self.instrs.push(Instruction::Intrinsic(
            dst,
            Box::new(IntrinsicOp::Random),
            Default::default(),
        ));
        dst
    }

    pub fn zero_extend(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        new_size: VectorSize,
    ) -> VariableKey {
        let Variable { size } = gl.vars[src];
        if size == new_size {
            return src;
        }

        let dst = self.next_tmp_var(gl, new_size);
        self.resize_op(gl, dst, ResizeOp::ZeroExtend, src);
        dst
    }
    pub fn sign_extend(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        new_size: VectorSize,
    ) -> VariableKey {
        let Variable { size } = gl.vars[src];
        if size == new_size {
            return src;
        }

        let dst = self.next_tmp_var(gl, new_size);
        self.resize_op(gl, dst, ResizeOp::SignExtend, src);
        dst
    }

    pub fn shift(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
    ) -> VariableKey {
        let lhs_size = gl.vars[lhs].size;
        assert_eq!(gl.vars[rhs].size, INTEGER_VSIZE);
        let dst = self.next_tmp_var(gl, lhs_size);
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
        dst
    }
    pub fn logical_shift_left(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.shift(gl, lhs, rhs, BinaryOp::LogicalShiftLeft)
    }
    pub fn logical_shift_right(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.shift(gl, lhs, rhs, BinaryOp::LogicalShiftRight)
    }
    pub fn arithmetic_shift_right(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.shift(gl, lhs, rhs, BinaryOp::ArithmeticShiftRight)
    }

    pub fn equals_zero(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let not_equals_zero = self.reduce_or(gl, src);
        self.logical_neg(gl, not_equals_zero)
    }

    pub fn lupdt(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        let dst = self.next_tmp_var(gl, TIME_VSIZE);
        self.instrs.push(Instruction::LastUpdateTime(dst, signal));
        dst
    }

    pub fn min(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Min)
    }
    pub fn max(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Max)
    }
    pub fn select(
        &mut self,
        gl: &mut GlobalContext,
        select: VariableKey,
        truthy: VariableKey,
        falsy: VariableKey,
    ) -> VariableKey {
        let size = gl.vars[truthy].size;
        assert_eq!(size, gl.vars[falsy].size);
        assert_eq!(SCALAR_VSIZE, gl.vars[select].size);
        let mask = self.sign_extend(gl, select, size);
        let mask_inv = self.binary_neg(gl, mask);
        let truthy = self.and(gl, mask, truthy);
        let falsy = self.and(gl, mask_inv, falsy);
        self.or(gl, truthy, falsy)
    }

    pub fn posedge(
        &mut self,
        gl: &mut GlobalContext,
        before: VariableKey,
        after: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars[before].size, SCALAR_VSIZE);
        assert_eq!(gl.vars[after].size, SCALAR_VSIZE);
        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.bin_op(gl, before, after, BinaryOp::Posedge, dst);
        dst
    }
    pub fn negedge(
        &mut self,
        gl: &mut GlobalContext,
        before: VariableKey,
        after: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars[before].size, SCALAR_VSIZE);
        assert_eq!(gl.vars[after].size, SCALAR_VSIZE);
        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.bin_op(gl, before, after, BinaryOp::Negedge, dst);
        dst
    }

    pub fn minus_revconstant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        constant: Bits,
    ) -> VariableKey {
        let constant = self.constant(gl, constant);
        self.minus(gl, constant, src)
    }
}
