use indexmap::IndexSet;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, Bits, GlobalContext, INTEGER_VSIZE,
    Instruction, IntrinsicOp, Process, ProcessKey, ResizeOp, SCALAR_VSIZE, SignalKey, TIME_VSIZE,
    Time, UnaryOp, Variable, VariableKey, VectorSize,
};

#[must_use]
pub struct BasicBlockBuilder {
    key: BasicBlockKey,
    process: ProcessKey,

    pub instrs: Vec<Instruction>,
    tmp_offset: usize,
    bbname_offset: usize,
}

pub fn new_process(gl: &'_ mut GlobalContext, name: String) -> BasicBlockBuilder {
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
    BasicBlockBuilder {
        key: bb_key,
        process: process_key,
        instrs: Vec::new(),
        tmp_offset: 0,
        bbname_offset: 0,
    }
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

    pub fn next_tmp_var(&mut self, gl: &mut GlobalContext, size: VectorSize) -> VariableKey {
        let name = format!("t{}", self.claim_tmp());
        gl.vars.insert(Variable { name, size })
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

    pub fn constant(&mut self, gl: &mut GlobalContext, value: Bits) -> VariableKey {
        let variable = self.next_tmp_var(gl, value.size());
        self.instrs.push(Instruction::Constant(variable, value));
        variable
    }

    pub fn constant_u32(&mut self, gl: &mut GlobalContext, value: u32) -> VariableKey {
        let variable = self.next_tmp_var(gl, VectorSize::new(32).unwrap());
        self.instrs.push(Instruction::Constant(
            variable,
            Bits::from_i64_truncated(value.into(), VectorSize::new(32).unwrap()),
        ));
        variable
    }

    pub fn concat(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs_size = gl.vars[lhs].size;
        let rhs_size = gl.vars[rhs].size;
        let dst = self.next_tmp_var(
            gl,
            VectorSize::new(lhs_size.get() + rhs_size.get()).unwrap(),
        );
        self.instrs.push(Instruction::Binary(
            dst,
            BinaryOp::Concat,
            lhs,
            rhs,
        ));
        dst
    }

    pub fn binary_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars[src].size;
        let dst = self.next_tmp_var(gl, size);
        self.instrs.push(Instruction::Unary(dst, UnaryOp::Neg, src));
        dst
    }

    pub fn logical_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars[src].size;
        let src = match size.get() {
            1 => src,
            _ => self.reduce_or(gl, src),
        };
        let dst = self.next_tmp_var(gl, VectorSize::new(1).unwrap());
        self.instrs.push(Instruction::Unary(dst, UnaryOp::Neg, src));
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
        self.instrs
            .push(Instruction::Binary(dst, op, lhs, rhs));
        dst
    }

    pub fn and(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::And)
    }
    pub fn or(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Or)
    }
    pub fn xor(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Xor)
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
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Multiply)
    }
    pub fn plus(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Add)
    }
    pub fn minus(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Sub)
    }
    pub fn divide(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Divide)
    }
    pub fn modulus(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Modulus)
    }
    pub fn multiply_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        constant: impl Into<i64>,
    ) -> VariableKey {
        let size = gl.vars[src].size;
        let constant = constant.into();

        if constant == 0 {
            self.constant(gl, Bits::from_i64_truncated(constant, size))
        } else if constant == 1 {
            src
        } else {
            let constant = self.constant(gl, Bits::from_i64_truncated(constant, size));
            self.multiply(gl, src, constant)
        }
    }

    pub fn select_bit(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        idx: VariableKey,
    ) -> VariableKey {
        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        assert_eq!(gl.vars[idx].size, INTEGER_VSIZE);
        self.instrs.push(Instruction::Binary(
            dst,
            BinaryOp::SelectBit,
            src,
            idx,
        ));
        dst
    }
    pub fn slice(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let size = gl.vars[src].size;
        if size == width {
            return src;
        }

        let dst = self.next_tmp_var(gl, width);
        self.instrs
            .push(Instruction::Resize(dst, ResizeOp::Truncate, src));
        dst
    }
    pub fn extract_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        offset: u32,
        width: VectorSize,
    ) -> VariableKey {
        let size = gl.vars[src].size;
        if offset == 0 && size == width {
            return src;
        }

        let offset = self.constant_u32(gl, offset);
        self.extract(gl, src, offset, width)
    }
    pub fn extract(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        offset: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let size = gl.vars[src].size;
        if size == width {
            return src;
        }

        let dst = self.logical_shift_right(gl, src, offset);
        self.slice(gl, dst, width)
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
        self.instrs.push(Instruction::Binary(
            dst,
            BinaryOp::UnsignedLessEqual,
            lhs,
            rhs,
        ));
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

    pub fn reduce_xor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars[src].size;
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.instrs
            .push(Instruction::Unary(dst, UnaryOp::ReduceXor, src));
        dst
    }
    pub fn reduce_or(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars[src].size;
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.instrs
            .push(Instruction::Unary(dst, UnaryOp::ReduceOr, src));
        dst
    }
    pub fn reduce_and(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars[src].size;
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = self.next_tmp_var(gl, SCALAR_VSIZE);
        self.instrs
            .push(Instruction::Unary(dst, UnaryOp::ReduceAnd, src));
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
        self.regioned_drive_opt_partial(gl, signal, src, 0, None);
    }
    pub fn drive_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: VariableKey,
        length: VectorSize,
    ) {
        self.regioned_drive_opt_partial(gl, signal, src, 0, Some((offset, length)));
    }
    pub fn drive_opt_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        partial: Option<(VariableKey, VectorSize)>,
    ) {
        self.regioned_drive_opt_partial(gl, signal, src, 0, partial);
    }

    pub fn regioned_drive(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        region: u8,
    ) {
        self.regioned_drive_opt_partial(gl, signal, src, region, None);
    }
    pub fn regioned_drive_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        region: u8,
        offset: VariableKey,
        length: VectorSize,
    ) {
        self.regioned_drive_opt_partial(gl, signal, src, region, Some((offset, length)));
    }
    pub fn regioned_drive_opt_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        region: u8,
        partial: Option<(VariableKey, VectorSize)>,
    ) {
        assert_eq!(
            gl.vars[src].size,
            partial.map_or(gl.signals[signal].size, |(_, w)| w)
        );
        gl.processes[self.process].outs.insert(signal);
        if let Some((offset, _)) = partial {
            assert_eq!(gl.vars[offset].size, INTEGER_VSIZE);
        }
        self.instrs
            .push(Instruction::Drive(signal, src, region, partial));
    }
    pub fn probe(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        gl.processes[self.process].ins.insert(signal);
        let size = gl.signals.get(signal).unwrap().size;
        let variable = self.next_tmp_var(gl, size);
        self.instrs.push(Instruction::Probe(variable, signal));
        variable
    }

    pub fn jump(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Jump(next_key);
        BasicBlockBuilder {
            key: next_key,

            process: self.process,

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

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }

    pub fn continue_with(
        &mut self,
        gl: &mut GlobalContext,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Halt;

        let next_bb = gl.bbs.get_mut(bb).unwrap();
        BasicBlockBuilder {
            key: bb,

            process: self.process,

            instrs: std::mem::take(&mut next_bb.instrs),

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

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }

    pub fn branch(
        &mut self,
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

    pub fn wait_region(mut self, gl: &mut GlobalContext, region: u8) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::WaitRegion(next_key, region);
        BasicBlockBuilder {
            key: next_key,

            process: self.process,

            instrs: Vec::new(),

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }

    pub fn watch(mut self, gl: &mut GlobalContext, signals: Vec<SignalKey>) -> BasicBlockBuilder {
        let next_key = self.next_bb(gl);
        let slf = gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Watch(next_key, signals);
        BasicBlockBuilder {
            key: next_key,

            process: self.process,

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

    pub fn zero_extend(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        size: VectorSize,
    ) -> VariableKey {
        if gl.vars[src].size == size {
            return src;
        }

        let dst = self.next_tmp_var(gl, size);
        self.instrs
            .push(Instruction::Resize(dst, ResizeOp::ZeroExtend, src));
        dst
    }
    pub fn sign_extend(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        size: VectorSize,
    ) -> VariableKey {
        if gl.vars[src].size == size {
            return src;
        }

        let dst = self.next_tmp_var(gl, size);
        self.instrs
            .push(Instruction::Resize(dst, ResizeOp::SignExtend, src));
        dst
    }

    pub fn process(&self) -> ProcessKey {
        self.process
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
        self.instrs
            .push(Instruction::Binary(dst, op, lhs, rhs));
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
        self.reduce_or(gl, src)
    }
}
