mod builder;
mod format;

use std::collections::HashSet;

pub use builder::{BasicBlockBuilder, ModuleBuilder};
pub use format::{ContextFormat, DisplayContext};
use indexmap::IndexSet;
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct ModuleKey; }
new_key_type! { pub struct SectionKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }
new_key_type! { pub struct VariableKey; }

#[derive(Debug, Clone)]
pub enum Value {
    Bit(bool),
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Self::Bit(..) => Type::Bit,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Time(pub u64);

pub enum BasicBlockTerminator {
    Wait(BasicBlockKey, Time),
    Watch(BasicBlockKey, Vec<SignalKey>),
    Jump(BasicBlockKey),
    Branch(VariableKey, BasicBlockKey, BasicBlockKey),
    Halt,
}

pub struct BasicBlock {
    pub name: String,
    pub instrs: Vec<Instruction>,
    pub terminator: BasicBlockTerminator,
}

impl BasicBlockTerminator {
    pub fn extend_next_rev(
        &self,
        bb_stack: &mut Vec<BasicBlockKey>,
        bb_seen: &mut HashSet<BasicBlockKey>,
    ) {
        match self {
            Self::Wait(bb, _) | Self::Watch(bb, _) | Self::Jump(bb) => {
                if bb_seen.insert(*bb) {
                    bb_stack.push(*bb);
                }
            }
            Self::Branch(_, true_bb, false_bb) => {
                if bb_seen.insert(*true_bb) {
                    bb_stack.push(*true_bb);
                }
                if bb_seen.insert(*false_bb) {
                    bb_stack.push(*false_bb);
                }
            }
            Self::Halt => {}
        }
    }
}

pub struct Variable {
    pub name: String,
    pub ty: Type,
}

pub struct Signal {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone)]
pub enum Type {
    Bit,
}

pub enum IntrinsicArg {
    StringLiteral(String),
    Variable(VariableKey),
}

#[derive(Debug, Clone, Copy)]
pub enum IntrinsicOp {
    Display,
    Finish,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    And,
    Or,
    Xor,
}

pub enum Instruction {
    Constant(VariableKey, Value),
    Unary(VariableKey, UnaryOp, VariableKey),
    Binary(VariableKey, BinaryOp, VariableKey, VariableKey),
    Intrinsic(IntrinsicOp, Vec<IntrinsicArg>),
    Probe(VariableKey, SignalKey),
    Drive(SignalKey, VariableKey),

    Instantiate(SectionKey),
    Signal(SignalKey),
}
impl Instruction {
    pub fn get_destination_variable(&self) -> Option<VariableKey> {
        match self {
            Self::Constant(dst, _)
            | Self::Unary(dst, _, _)
            | Self::Binary(dst, _, _, _)
            | Self::Probe(dst, _) => Some(*dst),
            Self::Intrinsic(_, _) | Self::Drive(_, _) | Self::Instantiate(_) | Self::Signal(_) => {
                None
            }
        }
    }
}

pub struct Module {
    pub name: String,
    pub sections: Vec<SectionKey>,
}

#[derive(Default)]
pub struct GlobalContext {
    pub modules: SlotMap<ModuleKey, Module>,
    pub sections: SlotMap<SectionKey, Section>,
    pub bbs: SlotMap<BasicBlockKey, BasicBlock>,
    pub vars: SlotMap<VariableKey, Variable>,
    pub signals: SlotMap<SignalKey, Signal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionVariant {
    Entity,
    Process,
    Function,
}

/// An entity, process or function.
pub struct Section {
    pub variant: SectionVariant,
    pub name: String,
    pub entry: BasicBlockKey,

    pub ins: IndexSet<SignalKey>,
    pub outs: IndexSet<SignalKey>,
}
