use indexmap::{IndexMap, IndexSet};

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, Connection, ConnectionDirection,
    GlobalContext, Instruction, IntrinsicArg, IntrinsicOp, Module, ModuleKey, Process, ProcessKey,
    SignalKey, Time, Type, UnaryOp, Value, Variable, VariableKey,
};

#[must_use]
pub struct ModuleBuilder {
    key: ModuleKey,
    pub entity: BasicBlockBuilder,
}

#[must_use]
pub struct BasicBlockBuilder {
    key: BasicBlockKey,
    module: ModuleKey,
    process: ProcessKey,
    initializer: bool,

    instrs: Vec<Instruction>,
    tmp_offset: usize,
    bbname_offset: usize,
}

impl BasicBlockBuilder {
    pub fn key(&self) -> BasicBlockKey {
        self.key
    }
}

impl ModuleBuilder {
    pub fn new(name: String, gl: &mut GlobalContext) -> Self {
        let bb_key = gl.bbs.insert(BasicBlock {
            name: String::from("entry"),
            instrs: Vec::new(),
            terminator: BasicBlockTerminator::Halt,
        });
        let initialize = gl.processes.insert(Process {
            name: format!("{name}-init"),
            entry: bb_key,
            ins: Default::default(),
            outs: Default::default(),
        });
        let key = gl.modules.insert(Module {
            name,
            initialize,
            processes: Vec::new(),
            io: IndexMap::default(),
        });

        Self {
            key,
            entity: BasicBlockBuilder {
                key: bb_key,
                module: key,
                process: initialize,
                initializer: true,
                instrs: Vec::new(),
                tmp_offset: 0,
                bbname_offset: 0,
            },
        }
    }

    pub fn finish(self, gl: &mut GlobalContext) -> ModuleKey {
        self.entity.halt(gl);
        self.key
    }

    pub fn process(
        &mut self,
        gl: &'_ mut GlobalContext,
        name: String,
    ) -> (ProcessKey, BasicBlockBuilder) {
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
        gl.modules[self.key].processes.push(process_key);
        (
            process_key,
            BasicBlockBuilder {
                key: bb_key,
                module: self.key,
                process: process_key,
                initializer: false,
                instrs: Vec::new(),
                tmp_offset: 0,
                bbname_offset: 0,
            },
        )
    }
}

impl BasicBlockBuilder {
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

    pub fn next_tmp_var(&mut self, gl: &mut GlobalContext, ty: Type) -> VariableKey {
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

    pub fn constant(&mut self, gl: &mut GlobalContext, value: Value) -> VariableKey {
        let variable = self.next_tmp_var(gl, value.get_type());
        let i = match value {
            Value::Bits(value) => Instruction::ConstantBit(variable, value),
            Value::Decimal(value) => Instruction::ConstantDecimal(variable, value),
        };
        self.instrs.push(i);
        variable
    }

    pub fn binary_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let ty = gl.vars[src].ty.clone();
        let dst = self.next_tmp_var(gl, ty);
        let i = match &gl.vars[src].ty {
            Type::Bits(size) => Instruction::Unary(dst, UnaryOp::BitNeg(*size), src),
            Type::Decimal => Instruction::Unary(dst, UnaryOp::DecimalNeg, src),
        };
        self.instrs.push(i);
        dst
    }

    pub fn logical_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let dst = self.next_tmp_var(gl, Type::Bits(1));
        let src = match &gl.vars[src].ty {
            Type::Bits(1) => src,
            Type::Bits(_) => self.reduce_or(gl, src),
            Type::Decimal => self.reduce_or(gl, src),
        };
        self.instrs
            .push(Instruction::Unary(dst, UnaryOp::BitNeg(1), src));
        dst
    }

    pub fn coerce_binary_bitwise(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> (VariableKey, VariableKey, VariableKey) {
        let lhs_ty = &gl.vars[lhs].ty;
        let rhs_ty = &gl.vars[rhs].ty;

        use Type as T;
        match (lhs_ty, rhs_ty) {
            (T::Bits(x), T::Bits(y)) if x == y => {
                let dst = self.next_tmp_var(gl, T::Bits(*x));
                (dst, lhs, rhs)
            }
            (T::Bits(x), T::Bits(y)) => {
                let out_size = (*x).max(*y);
                let lhs = self.cast(gl, lhs, T::Bits(out_size));
                let rhs = self.cast(gl, rhs, T::Bits(out_size));
                let dst = self.next_tmp_var(gl, T::Bits(out_size));
                (dst, lhs, rhs)
            }
            (T::Bits(x), _) | (_, T::Bits(x)) => {
                let x = *x;
                let lhs = self.cast(gl, lhs, T::Bits(x));
                let rhs = self.cast(gl, rhs, T::Bits(x));
                let dst = self.next_tmp_var(gl, T::Bits(x));
                (dst, lhs, rhs)
            }
            (T::Decimal, T::Decimal) => {
                let dst = self.next_tmp_var(gl, T::Decimal);
                (dst, lhs, rhs)
            }
        }
    }

    pub fn and(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let (dst, lhs, rhs) = self.coerce_binary_bitwise(gl, lhs, rhs);
        let op = match &gl.vars[dst].ty {
            Type::Bits(size) => BinaryOp::BitAnd(*size),
            Type::Decimal => BinaryOp::DecimalAnd,
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
        let op = match &gl.vars[dst].ty {
            Type::Bits(size) => BinaryOp::BitOr(*size),
            Type::Decimal => BinaryOp::DecimalOr,
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
        let op = match &gl.vars[dst].ty {
            Type::Bits(size) => BinaryOp::BitXor(*size),
            Type::Decimal => BinaryOp::DecimalXor,
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

    pub fn push_in_port(&mut self, gl: &mut GlobalContext, name: String, signal: SignalKey) {
        assert!(self.initializer);
        gl.modules[self.module].io.insert(
            name,
            Connection {
                signal,
                direction: ConnectionDirection::In,
            },
        );
    }
    pub fn push_out_port(&mut self, gl: &mut GlobalContext, name: String, signal: SignalKey) {
        assert!(self.initializer);
        gl.modules[self.module].io.insert(
            name,
            Connection {
                signal,
                direction: ConnectionDirection::Out,
            },
        );
    }

    pub fn reduce_or(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        match &gl.vars[src].ty {
            Type::Decimal => {
                let dst = self.next_tmp_var(gl, Type::Bits(1));
                self.instrs
                    .push(Instruction::Unary(dst, UnaryOp::DecimalReduceOr, src));
                dst
            }
            Type::Bits(1) => src,
            Type::Bits(n) => {
                let n = *n;
                let dst = self.next_tmp_var(gl, Type::Bits(1));
                self.instrs
                    .push(Instruction::Unary(dst, UnaryOp::BitReduceOr(n), src));
                dst
            }
        }
    }
    pub fn reduce_and(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        match &gl.vars[src].ty {
            Type::Decimal => {
                let dst = self.next_tmp_var(gl, Type::Bits(1));
                self.instrs
                    .push(Instruction::Unary(dst, UnaryOp::DecimalReduceAnd, src));
                dst
            }
            Type::Bits(1) => src,
            Type::Bits(n) => {
                let n = *n;
                let dst = self.next_tmp_var(gl, Type::Bits(1));
                self.instrs
                    .push(Instruction::Unary(dst, UnaryOp::BitReduceAnd(n), src));
                dst
            }
        }
    }

    pub fn drive(&mut self, gl: &mut GlobalContext, signal: SignalKey, src: VariableKey) {
        let ty = gl.signals[signal].ty.clone();
        let src = self.cast(gl, src, ty);
        gl.processes[self.process].outs.insert(signal);
        self.instrs.push(Instruction::Drive(signal, src));
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

            module: self.module,
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

            module: self.module,
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

            module: self.module,
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

            module: self.module,
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

            module: self.module,
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

    pub fn spawn(&mut self, _gl: &mut GlobalContext, process: ProcessKey, ports: Vec<SignalKey>) {
        assert!(self.initializer);
        self.instrs.push(Instruction::Spawn(process, ports));
    }

    pub fn instantiate(
        &mut self,
        _gl: &mut GlobalContext,
        module: ModuleKey,
        ports: Vec<SignalKey>,
    ) {
        assert!(self.initializer);
        self.instrs.push(Instruction::Instantiate(module, ports));
    }

    pub fn signal(&mut self, _gl: &mut GlobalContext, signal: SignalKey) {
        assert!(self.initializer);
        self.instrs.push(Instruction::Signal(signal));
    }

    fn cast(&mut self, gl: &mut GlobalContext, src: VariableKey, ty: Type) -> VariableKey {
        if &gl.vars[src].ty == &ty {
            return src;
        }

        let dst = self.next_tmp_var(gl, ty);
        self.instrs.push(Instruction::Cast(dst, src));
        dst
    }
}
