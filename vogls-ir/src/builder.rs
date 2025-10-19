use indexmap::IndexSet;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction,
    IntrinsicArg, IntrinsicOp, Module, Section, SectionKey, SectionVariant, SignalKey, Time, Type,
    UnaryOp, Value, Variable, VariableKey,
};

#[must_use]
pub struct ModuleBuilder {
    name: String,
    sections: Vec<SectionKey>,
    pub entity: BasicBlockBuilder,
}

#[must_use]
pub struct BasicBlockBuilder {
    key: BasicBlockKey,

    section: SectionKey,
    section_variant: SectionVariant,

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
        let variant = SectionVariant::Entity;
        let bb_key = gl.bbs.insert(BasicBlock {
            name: String::from("entry"),
            instrs: Vec::new(),
            terminator: BasicBlockTerminator::Halt,
        });
        let section_key = gl.sections.insert(Section {
            variant,
            name: name.clone(),
            entry: bb_key,

            ins: IndexSet::new(),
            outs: IndexSet::new(),
        });
        let mut sections = Vec::new();
        sections.push(section_key);

        Self {
            name,
            sections,
            entity: BasicBlockBuilder {
                key: bb_key,
                section: section_key,
                section_variant: SectionVariant::Entity,
                instrs: Vec::new(),
                tmp_offset: 0,
                bbname_offset: 0,
            },
        }
    }

    pub fn finish(self, gl: &mut GlobalContext) -> Module {
        self.entity.halt(gl);
        Module {
            name: self.name,
            sections: self.sections,
            io: Default::default(),
        }
    }

    pub fn process(
        &mut self,
        gl: &'_ mut GlobalContext,
        name: String,
    ) -> (SectionKey, BasicBlockBuilder) {
        let bb_key = gl.bbs.insert(BasicBlock {
            name: String::from("entry"),
            instrs: Vec::new(),
            terminator: BasicBlockTerminator::Halt,
        });
        let section_key = gl.sections.insert(Section {
            variant: SectionVariant::Process,
            name,
            entry: bb_key,

            ins: IndexSet::new(),
            outs: IndexSet::new(),
        });
        self.sections.push(section_key);
        (
            section_key,
            BasicBlockBuilder {
                key: bb_key,
                instrs: Vec::new(),
                section: section_key,
                section_variant: SectionVariant::Process,
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
        self.instrs.push(Instruction::Constant(variable, value));
        variable
    }

    pub fn unary_op(
        &mut self,
        gl: &mut GlobalContext,
        op: UnaryOp,
        src: VariableKey,
    ) -> VariableKey {
        let ty = gl.vars.get(src).unwrap().ty.clone();
        let variable = self.next_tmp_var(gl, ty);
        self.instrs.push(Instruction::Unary(variable, op, src));
        variable
    }

    pub fn binary_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        self.unary_op(gl, UnaryOp::BinaryNeg, src)
    }

    pub fn logical_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        self.unary_op(gl, UnaryOp::LogicalNeg, src)
    }

    pub fn bin_op(
        &mut self,
        gl: &mut GlobalContext,
        op: BinaryOp,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let ty = gl.vars.get(lhs).unwrap().ty.clone();
        let variable = self.next_tmp_var(gl, ty);
        self.instrs
            .push(Instruction::Binary(variable, op, lhs, rhs));
        variable
    }

    pub fn and(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_op(gl, BinaryOp::And, lhs, rhs)
    }
    pub fn or(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_op(gl, BinaryOp::Or, lhs, rhs)
    }
    pub fn xor(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_op(gl, BinaryOp::Xor, lhs, rhs)
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
        let xnor = self.logical_neg(gl, xor);
        xnor
    }

    pub fn push_in_port(&mut self, gl: &mut GlobalContext, signal: SignalKey) {
        assert_eq!(self.section_variant, SectionVariant::Entity);
        gl.sections
            .get_mut(self.section)
            .unwrap()
            .ins
            .insert(signal);
    }
    pub fn push_out_port(&mut self, gl: &mut GlobalContext, signal: SignalKey) {
        assert_eq!(self.section_variant, SectionVariant::Entity);
        gl.sections
            .get_mut(self.section)
            .unwrap()
            .outs
            .insert(signal);
    }

    pub fn drive(&mut self, gl: &mut GlobalContext, signal: SignalKey, variable: VariableKey) {
        if self.section_variant == SectionVariant::Process {
            gl.sections
                .get_mut(self.section)
                .unwrap()
                .outs
                .insert(signal);
        }

        self.instrs.push(Instruction::Drive(signal, variable));
    }
    pub fn probe(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        if self.section_variant == SectionVariant::Process {
            gl.sections
                .get_mut(self.section)
                .unwrap()
                .ins
                .insert(signal);
        }

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
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

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
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

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
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

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
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

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
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

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
        let ins = &gl.sections.get(self.section).unwrap().ins;
        if ins.is_empty() {
            self.halt(gl);
        } else {
            let signals = ins.iter().copied().collect::<Vec<_>>();
            self.watch_to(gl, signals, bb);
        }
    }

    pub fn intrinsic(&mut self, gl: &mut GlobalContext, op: IntrinsicOp, args: Vec<IntrinsicArg>) {
        self.instrs.push(Instruction::Intrinsic(op, args));
    }

    pub fn instantiate(
        &mut self,
        gl: &mut GlobalContext,
        process: SectionKey,
        ports: Vec<SignalKey>,
    ) {
        assert_eq!(self.section_variant, SectionVariant::Entity);
        let section = gl.sections.get(process).unwrap();
        assert_ne!(section.variant, SectionVariant::Function);
        assert_eq!(section.ins.len() + section.outs.len(), ports.len());

        self.instrs.push(Instruction::Instantiate(process, ports));
    }

    pub fn signal(&mut self, gl: &mut GlobalContext, signal: SignalKey) {
        assert_eq!(self.section_variant, SectionVariant::Entity);
        self.instrs.push(Instruction::Signal(signal));
    }
}
