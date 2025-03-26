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
}

#[must_use]
pub struct BasicBlockBuilder<'a> {
    gl: &'a mut GlobalContext,
    key: BasicBlockKey,

    section: SectionKey,
    section_variant: SectionVariant,

    instrs: Vec<Instruction>,
    tmp_offset: usize,
    bbname_offset: usize,
}

impl<'a> BasicBlockBuilder<'a> {
    pub fn key(&self) -> BasicBlockKey {
        self.key
    }
}

impl ModuleBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            sections: Vec::new(),
        }
    }

    pub fn finish(self) -> Module {
        Module {
            name: self.name,
            sections: self.sections,
        }
    }

    pub fn process<'a>(
        &mut self,
        gl: &'a mut GlobalContext,
        name: String,
    ) -> (SectionKey, BasicBlockBuilder<'a>) {
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
                gl,
                key: bb_key,
                instrs: Vec::new(),
                section: section_key,
                section_variant: SectionVariant::Process,
                tmp_offset: 0,
                bbname_offset: 0,
            },
        )
    }

    pub fn entity<'a>(
        &mut self,
        gl: &'a mut GlobalContext,
        name: String,
    ) -> (SectionKey, BasicBlockBuilder<'a>) {
        let variant = SectionVariant::Entity;
        let bb_key = gl.bbs.insert(BasicBlock {
            name: String::from("entry"),
            instrs: Vec::new(),
            terminator: BasicBlockTerminator::Halt,
        });
        let section_key = gl.sections.insert(Section {
            variant,
            name,
            entry: bb_key,

            ins: IndexSet::new(),
            outs: IndexSet::new(),
        });
        self.sections.push(section_key);
        (
            section_key,
            BasicBlockBuilder {
                gl,
                key: bb_key,
                instrs: Vec::new(),
                section: section_key,
                section_variant: variant,
                tmp_offset: 0,
                bbname_offset: 0,
            },
        )
    }
}

impl<'a> BasicBlockBuilder<'a> {
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

    pub fn next_tmp_var(&mut self, ty: Type) -> VariableKey {
        let name = format!("t{}", self.claim_tmp());
        self.gl.vars.insert(Variable { name, ty })
    }
    pub fn next_bb(&mut self) -> BasicBlockKey {
        let name = format!("L{}", self.claim_bbname());
        self.gl.bbs.insert(BasicBlock {
            name,
            instrs: Vec::new(),
            terminator: BasicBlockTerminator::Halt,
        })
    }

    pub fn constant(&mut self, value: Value) -> VariableKey {
        let variable = self.next_tmp_var(value.get_type());
        self.instrs.push(Instruction::Constant(variable, value));
        variable
    }

    pub fn unary_op(&mut self, op: UnaryOp, src: VariableKey) -> VariableKey {
        let ty = self.gl.vars.get(src).unwrap().ty.clone();
        let variable = self.next_tmp_var(ty);
        self.instrs.push(Instruction::Unary(variable, op, src));
        variable
    }

    pub fn neg(&mut self, src: VariableKey) -> VariableKey {
        self.unary_op(UnaryOp::Neg, src)
    }

    pub fn bin_op(&mut self, op: BinaryOp, lhs: VariableKey, rhs: VariableKey) -> VariableKey {
        let ty = self.gl.vars.get(lhs).unwrap().ty.clone();
        let variable = self.next_tmp_var(ty);
        self.instrs
            .push(Instruction::Binary(variable, op, lhs, rhs));
        variable
    }

    pub fn and(&mut self, lhs: VariableKey, rhs: VariableKey) -> VariableKey {
        self.bin_op(BinaryOp::And, lhs, rhs)
    }
    pub fn or(&mut self, lhs: VariableKey, rhs: VariableKey) -> VariableKey {
        self.bin_op(BinaryOp::Or, lhs, rhs)
    }
    pub fn xor(&mut self, lhs: VariableKey, rhs: VariableKey) -> VariableKey {
        self.bin_op(BinaryOp::Xor, lhs, rhs)
    }

    pub fn drive(&mut self, signal: SignalKey, variable: VariableKey) {
        if self.section_variant == SectionVariant::Process {
            self.gl.sections.get_mut(self.section).unwrap().outs.insert(signal);
        }

        self.instrs.push(Instruction::Drive(signal, variable));
    }
    pub fn probe(&mut self, signal: SignalKey) -> VariableKey {
        if self.section_variant == SectionVariant::Process {
            self.gl.sections.get_mut(self.section).unwrap().ins.insert(signal);
        }

        let ty = self.gl.signals.get(signal).unwrap().ty.clone();
        let variable = self.next_tmp_var(ty);
        self.instrs.push(Instruction::Probe(variable, signal));
        variable
    }

    pub fn jump(mut self) -> BasicBlockBuilder<'a> {
        let next_key = self.next_bb();
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Jump(next_key);
        BasicBlockBuilder {
            gl: self.gl,
            key: next_key,
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }
    pub fn jump_to(mut self, bb: BasicBlockKey) {
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Jump(bb);
    }

    pub fn branch_true_to(
        mut self,
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder<'a> {
        let next_key = self.next_bb();
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Branch(condition, bb, next_key);
        BasicBlockBuilder {
            gl: self.gl,
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
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder<'a> {
        let next_key = self.next_bb();
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Branch(condition, next_key, bb);
        BasicBlockBuilder {
            gl: self.gl,
            key: next_key,
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }

    pub fn halt(mut self) {
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Halt;
    }

    pub fn wait(mut self, time: Time) -> BasicBlockBuilder<'a> {
        let next_key = self.next_bb();
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Wait(next_key, time);
        BasicBlockBuilder {
            gl: self.gl,
            key: next_key,
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }
    pub fn wait_to(mut self, time: Time, bb: BasicBlockKey) {
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Wait(bb, time);
    }

    pub fn watch(mut self, signals: Vec<SignalKey>) -> BasicBlockBuilder<'a> {
        let next_key = self.next_bb();
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Watch(next_key, signals);
        BasicBlockBuilder {
            gl: self.gl,
            key: next_key,
            instrs: Vec::new(),

            section: self.section,
            section_variant: self.section_variant,

            tmp_offset: self.tmp_offset,
            bbname_offset: self.bbname_offset,
        }
    }
    pub fn watch_to(mut self, signals: Vec<SignalKey>, bb: BasicBlockKey) {
        let slf = self.gl.bbs.get_mut(self.key).unwrap();
        slf.instrs = std::mem::take(&mut self.instrs);
        slf.terminator = BasicBlockTerminator::Watch(bb, signals);
    }

    pub fn intrinsic(&mut self, op: IntrinsicOp, args: Vec<IntrinsicArg>) {
        self.instrs.push(Instruction::Intrinsic(op, args));
    }

    pub fn instantiate(&mut self, process: SectionKey) {
        assert_eq!(self.section_variant, SectionVariant::Entity);
        assert_ne!(
            self.gl.sections.get(process).unwrap().variant,
            SectionVariant::Function
        );
        self.instrs.push(Instruction::Instantiate(process));
    }
    pub fn signal(&mut self, signal: SignalKey) {
        assert_eq!(self.section_variant, SectionVariant::Entity);
        self.instrs.push(Instruction::Signal(signal));
    }
}
