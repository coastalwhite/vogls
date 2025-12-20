use indexmap::IndexSet;

use crate::types::TypeKey;
use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction,
    IntrinsicArg, IntrinsicOp, Process, ProcessKey, SignalKey, Time, Type, TypeTable, UnaryOp,
    Value, Variable, VariableKey, VectorSize,
};

#[must_use]
pub struct BasicBlockBuilder {
    key: BasicBlockKey,
    process: ProcessKey,
    initializer: bool,

    pub instrs: Vec<Instruction>,
    tmp_offset: usize,
    bbname_offset: usize,
}

pub fn new_process(gl: &'_ mut GlobalContext, name: String) -> (ProcessKey, BasicBlockBuilder) {
    let bb_key = gl.bbs.insert(BasicBlock {
        name: String::from("entry"),
        instrs: Vec::new(),
        terminator: BasicBlockTerminator::Halt,
    });
    let process_key = gl.processes.insert(Process {
        name,
        entry: bb_key,

        ins: IndexSet::new(),
        outs: IndexSet::new(),
    });
    (
        process_key,
        BasicBlockBuilder {
            key: bb_key,
            process: process_key,
            initializer: false,
            instrs: Vec::new(),
            tmp_offset: 0,
            bbname_offset: 0,
        },
    )
}

pub struct PhiRef(BasicBlockKey, usize);
pub struct BranchRef(BasicBlockKey);

impl BranchRef {
    pub fn update(&self, gl: &mut GlobalContext, bb: BasicBlockKey) {
        let BasicBlockTerminator::Branch(_, _, snd) = &mut gl.bbs[self.0].terminator else {
            panic!("not a branch");
        };
        *snd = bb;
    }

    pub fn origin_key(&self) -> BasicBlockKey {
        self.0
    }
}

impl BasicBlockBuilder {
    pub fn key(&self) -> BasicBlockKey {
        self.key
    }

    pub fn claim_tmp(&mut self) -> usize {
        let t = self.tmp_offset;
        self.tmp_offset += 1;
        t
    }
    pub fn claim_bbname(&mut self) -> usize {
        let t = self.bbname_offset;
        self.bbname_offset += 1;
        t
    }

    pub fn instrs_len(&self) -> usize {
        self.instrs.len()
    }

    pub fn next_tmp_var(&mut self, gl: &mut GlobalContext, ty: TypeKey) -> VariableKey {
        let name = format!("t{}", self.claim_tmp());
        gl.vars.insert(Variable { name, ty })
    }
    pub fn next_bb(&mut self, gl: &mut GlobalContext) -> BasicBlockKey {
        let name = format!("L{}", self.claim_bbname());
        gl.bbs.insert(BasicBlock {
            name,
            instrs: Vec::new(),
            terminator: BasicBlockTerminator::Halt,
        })
    }

    pub fn next_builder(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        BasicBlockBuilder {
            key: next_key,

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }

    pub fn phi(
        &mut self,
        gl: &mut GlobalContext,
        srcs: Box<[(BasicBlockKey, VariableKey)]>,
    ) -> (VariableKey, PhiRef) {
        assert!(!srcs.is_empty());
        let (_, var) = srcs.first().unwrap();
        let ty = gl.vars[*var].ty.clone();
        let dst = self.next_tmp_var(gl, ty);
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

    pub fn constant(&mut self, gl: &mut GlobalContext, value: Value) -> VariableKey {
        let ty = gl.types.insert(value.get_type());
        let variable = self.next_tmp_var(gl, ty);
        let i = match value {
            Value::Bits(value) => Instruction::ConstantBit(variable, value),
            Value::Decimal(value) => Instruction::ConstantDecimal(variable, value),
        };
        self.instrs.push(i);
        variable
    }

    pub fn concat(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs_ty = &gl.vars[lhs].ty;
        let rhs_ty = &gl.vars[rhs].ty;

        let (Type::Bits(lhs_size), Type::Bits(rhs_size)) = (gl.types[*lhs_ty], gl.types[*rhs_ty])
        else {
            todo!();
        };

        let (lhs_size, rhs_size) = (lhs_size, rhs_size);
        let width = lhs_size + rhs_size;
        let ty = gl.types.insert(Type::Bits(width));
        let dst = self.next_tmp_var(gl, ty);
        self.instrs.push(Instruction::Binary(
            dst,
            BinaryOp::Concat(lhs_size, rhs_size),
            lhs,
            rhs,
        ));
        dst
    }

    pub fn binary_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let ty = gl.vars[src].ty;
        let dst = self.next_tmp_var(gl, ty);
        let i = match gl.types[gl.vars[src].ty] {
            Type::Bits(size) => Instruction::Unary(dst, UnaryOp::BitNeg(size), src),
            Type::Decimal => Instruction::Unary(dst, UnaryOp::DecimalNeg, src),
            Type::Array(..) => panic!(),
        };
        self.instrs.push(i);
        dst
    }

    pub fn logical_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let dst = self.next_tmp_var(gl, TypeTable::SCALAR_BIT);
        let src = match gl.types[gl.vars[src].ty] {
            Type::Bits(1) => src,
            Type::Bits(_) => self.reduce_or(gl, src),
            Type::Decimal => self.reduce_or(gl, src),
            Type::Array(..) => panic!(),
        };
        self.instrs
            .push(Instruction::Unary(dst, UnaryOp::BitNeg(1), src));
        dst
    }

    pub fn coerce_binary_bitwise_srcs(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> (VariableKey, VariableKey) {
        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        use Type as T;
        match (gl.types[lhs_ty], gl.types[rhs_ty]) {
            (T::Bits(x), T::Bits(y)) if x == y => (lhs, rhs),
            (T::Bits(x), T::Bits(y)) => {
                let out_size = x.max(y);
                let ty = gl.types.insert(T::Bits(out_size));
                let lhs = self.cast(gl, lhs, ty);
                let rhs = self.cast(gl, rhs, ty);
                (lhs, rhs)
            }
            (T::Bits(x), _) | (_, T::Bits(x)) => {
                let ty = gl.types.insert(T::Bits(x));
                let lhs = self.cast(gl, lhs, ty);
                let rhs = self.cast(gl, rhs, ty);
                (lhs, rhs)
            }
            (T::Decimal, T::Decimal) => (lhs, rhs),
            (T::Array(..), _) | (_, T::Array(..)) => panic!(),
        }
    }

    pub fn coerce_binary_bitwise(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> (VariableKey, VariableKey, VariableKey) {
        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        use Type as T;
        match (gl.types[lhs_ty], gl.types[rhs_ty]) {
            (T::Bits(x), T::Bits(y)) if x == y => {
                let dst = self.next_tmp_var(gl, lhs_ty);
                (dst, lhs, rhs)
            }
            (T::Bits(x), T::Bits(y)) => {
                let out_size = x.max(y);
                let ty = gl.types.insert(T::Bits(out_size));
                let lhs = self.cast(gl, lhs, ty);
                let rhs = self.cast(gl, rhs, ty);
                let dst = self.next_tmp_var(gl, ty);
                (dst, lhs, rhs)
            }
            (T::Bits(x), _) | (_, T::Bits(x)) => {
                let ty = gl.types.insert(T::Bits(x));
                let lhs = self.cast(gl, lhs, ty);
                let rhs = self.cast(gl, rhs, ty);
                let dst = self.next_tmp_var(gl, ty);
                (dst, lhs, rhs)
            }
            (T::Decimal, T::Decimal) => {
                let dst = self.next_tmp_var(gl, TypeTable::INT64);
                (dst, lhs, rhs)
            }
            (T::Array(..), _) | (_, T::Array(..)) => panic!(),
        }
    }

    pub fn and(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let (dst, lhs, rhs) = self.coerce_binary_bitwise(gl, lhs, rhs);
        let op = match gl.types[gl.vars[dst].ty] {
            Type::Bits(size) => BinaryOp::BitAnd(size),
            Type::Decimal => BinaryOp::DecimalAnd,
            Type::Array(..) => panic!(),
        };
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
        dst
    }
    pub fn or(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let (dst, lhs, rhs) = self.coerce_binary_bitwise(gl, lhs, rhs);
        let op = match gl.types[gl.vars[dst].ty] {
            Type::Bits(size) => BinaryOp::BitOr(size),
            Type::Decimal => BinaryOp::DecimalOr,
            Type::Array(..) => panic!(),
        };
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
        dst
    }
    pub fn xor(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let (dst, lhs, rhs) = self.coerce_binary_bitwise(gl, lhs, rhs);
        let op = match gl.types[gl.vars[dst].ty] {
            Type::Bits(size) => BinaryOp::BitXor(size),
            Type::Decimal => BinaryOp::DecimalXor,
            Type::Array(..) => panic!(),
        };
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
        dst
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

    pub fn multiply(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, TypeTable::INT64);

        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        assert_eq!(lhs_ty, TypeTable::INT64);
        assert_eq!(rhs_ty, TypeTable::INT64);

        self.instrs.push(Instruction::Binary(
            dst,
            BinaryOp::DecimalMultiply,
            lhs,
            rhs,
        ));
        dst
    }
    pub fn plus(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, TypeTable::INT64);

        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        assert_eq!(lhs_ty, TypeTable::INT64);
        assert_eq!(rhs_ty, TypeTable::INT64);

        self.instrs
            .push(Instruction::Binary(dst, BinaryOp::DecimalAdd, lhs, rhs));
        dst
    }
    pub fn minus(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, TypeTable::INT64);

        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        assert_eq!(lhs_ty, TypeTable::INT64);
        assert_eq!(rhs_ty, TypeTable::INT64);

        self.instrs
            .push(Instruction::Binary(dst, BinaryOp::DecimalSub, lhs, rhs));
        dst
    }
    pub fn i64_divide(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, TypeTable::INT64);

        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        assert_eq!(lhs_ty, TypeTable::INT64);
        assert_eq!(rhs_ty, TypeTable::INT64);

        self.instrs
            .push(Instruction::Binary(dst, BinaryOp::DecimalDivide, lhs, rhs));
        dst
    }
    pub fn i64_modulus(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, TypeTable::INT64);

        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        assert_eq!(lhs_ty, TypeTable::INT64);
        assert_eq!(rhs_ty, TypeTable::INT64);

        self.instrs
            .push(Instruction::Binary(dst, BinaryOp::DecimalModulus, lhs, rhs));
        dst
    }

    pub fn select_bit(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        idx: VariableKey,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, TypeTable::SCALAR_BIT);

        let Type::Bits(n) = gl.types[gl.vars[src].ty] else {
            panic!();
        };
        let idx = self.cast(gl, idx, TypeTable::INT64);

        self.instrs
            .push(Instruction::Binary(dst, BinaryOp::SelectBit(n), src, idx));
        dst
    }
    pub fn lsr(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        shift: VariableKey,
    ) -> VariableKey {
        let Type::Bits(n) = gl.types[gl.vars[src].ty] else {
            panic!();
        };

        let dst = self.next_tmp_var(gl, gl.vars[src].ty);
        let shift = self.cast(gl, shift, TypeTable::INT64);

        self.instrs.push(Instruction::Binary(
            dst,
            BinaryOp::LogicalShiftRight(n),
            src,
            shift,
        ));
        dst
    }
    pub fn slice(
        &mut self,
        gl: &mut GlobalContext,
        subject: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let Type::Bits(n) = gl.types[gl.vars[subject].ty] else {
            panic!();
        };

        if n == width {
            return subject;
        }

        let ty = gl.types.insert(Type::Bits(width));
        let dst = self.next_tmp_var(gl, ty);
        self.instrs.push(Instruction::Unary(
            dst,
            UnaryOp::BitSlice(n, width),
            subject,
        ));
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
        let dst = self.next_tmp_var(gl, TypeTable::SCALAR_BIT);

        let lhs_ty = gl.vars[lhs].ty;
        let rhs_ty = gl.vars[rhs].ty;

        use Type as T;
        let (lhs, rhs, op) = match (gl.types[lhs_ty], gl.types[rhs_ty]) {
            (T::Bits(x), T::Bits(y)) if x == y => (lhs, rhs, BinaryOp::UnsignedLessEqual(x)),
            (T::Decimal, T::Decimal) => (lhs, rhs, BinaryOp::DecimalLessEqual),
            (T::Bits(x), T::Bits(y)) => {
                let out_size = (x).max(y);
                let ty = gl.types.insert(T::Bits(out_size));
                let lhs = self.cast(gl, lhs, ty);
                let rhs = self.cast(gl, rhs, ty);
                (lhs, rhs, BinaryOp::UnsignedLessEqual(out_size))
            }
            (T::Bits(x), _) | (_, T::Bits(x)) => {
                let ty = gl.types.insert(T::Bits(x));
                let lhs = self.cast(gl, lhs, ty);
                let rhs = self.cast(gl, rhs, ty);
                (lhs, rhs, BinaryOp::UnsignedLessEqual(x))
            }
            (T::Array(..), _) | (_, T::Array(..)) => panic!(),
        };
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
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

    pub fn reduce_xor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        if gl.vars[src].ty == TypeTable::SCALAR_BIT {
            return src;
        }

        let dst = self.next_tmp_var(gl, TypeTable::SCALAR_BIT);
        let op = match gl.types[gl.vars[src].ty] {
            Type::Decimal => UnaryOp::DecimalReduceXor,
            Type::Bits(n) => UnaryOp::BitReduceXor(n),
            Type::Array(..) => panic!(),
        };
        self.instrs.push(Instruction::Unary(dst, op, src));
        dst
    }

    pub fn reduce_or(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        if gl.vars[src].ty == TypeTable::SCALAR_BIT {
            return src;
        }

        let dst = self.next_tmp_var(gl, TypeTable::SCALAR_BIT);
        let op = match gl.types[gl.vars[src].ty] {
            Type::Decimal => UnaryOp::DecimalReduceOr,
            Type::Bits(n) => UnaryOp::BitReduceOr(n),
            Type::Array(..) => panic!(),
        };
        self.instrs.push(Instruction::Unary(dst, op, src));
        dst
    }
    pub fn reduce_and(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        if gl.vars[src].ty == TypeTable::SCALAR_BIT {
            return src;
        }

        let dst = self.next_tmp_var(gl, TypeTable::SCALAR_BIT);
        let op = match gl.types[gl.vars[src].ty] {
            Type::Decimal => UnaryOp::DecimalReduceAnd,
            Type::Bits(n) => UnaryOp::BitReduceAnd(n),
            Type::Array(..) => panic!(),
        };
        self.instrs.push(Instruction::Unary(dst, op, src));
        dst
    }

    pub fn drive(&mut self, gl: &mut GlobalContext, signal: SignalKey, src: VariableKey) {
        let ty = gl.signals[signal].ty.clone();
        let src = self.cast(gl, src, ty);
        gl.processes[self.process].outs.insert(signal);
        self.instrs.push(Instruction::Drive(signal, src, None));
    }
    pub fn drive_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: VariableKey,
        length: VectorSize,
    ) {
        let ty = gl.types.insert(Type::Bits(length));
        let src = self.cast(gl, src, ty);
        gl.processes[self.process].outs.insert(signal);
        self.instrs
            .push(Instruction::Drive(signal, src, Some((offset, length))));
    }
    pub fn probe(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        gl.processes[self.process].ins.insert(signal);
        let ty = gl.signals.get(signal).unwrap().ty.clone();
        let variable = self.next_tmp_var(gl, ty);
        self.instrs.push(Instruction::Probe(variable, signal));
        variable
    }

    pub fn jump(mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Jump(next_key);
        BasicBlockBuilder {
            key: next_key,

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
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
        slf.terminator = BasicBlockTerminator::Halt;
        BasicBlockBuilder {
            key: next_key,

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }

    pub fn jump_to_with_dummy(
        mut self,
        gl: &mut GlobalContext,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Jump(bb);
        BasicBlockBuilder {
            key: next_key,

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }

    pub fn branch(
        mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
    ) -> (BranchRef, BasicBlockBuilder) {
        let branch_bb = self.key();
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Branch(condition, next_key, next_key);
        let builder = BasicBlockBuilder {
            key: next_key,

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
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

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
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

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
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

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }
    pub fn wait_to(mut self, gl: &mut GlobalContext, time: Time, bb: BasicBlockKey) {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Wait(bb, time);
    }

    pub fn watch(mut self, gl: &mut GlobalContext, signals: Vec<SignalKey>) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Watch(next_key, signals);
        BasicBlockBuilder {
            key: next_key,

            process: self.process,
            initializer: self.initializer,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }
    pub fn watch_to(mut self, gl: &mut GlobalContext, signals: Vec<SignalKey>, bb: BasicBlockKey) {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Watch(bb, signals);
    }

    pub fn watch_for_ins_to(self, gl: &mut GlobalContext, bb: BasicBlockKey) {
        let ins = &gl.processes[self.process].ins;
        if ins.is_empty() {
            self.halt(gl);
        } else {
            let signals = ins.iter().copied().collect::<Vec<_>>();
            self.watch_to(gl, signals, bb);
        }
    }

    pub fn intrinsic(&mut self, _gl: &mut GlobalContext, op: IntrinsicOp, args: Vec<IntrinsicArg>) {
        self.instrs.push(Instruction::Intrinsic(op, args));
    }

    pub fn cast(&mut self, gl: &mut GlobalContext, src: VariableKey, ty: TypeKey) -> VariableKey {
        if gl.vars[src].ty == ty {
            return src;
        }

        let dst = self.next_tmp_var(gl, ty);
        self.instrs.push(Instruction::Cast(dst, src));
        dst
    }
}
