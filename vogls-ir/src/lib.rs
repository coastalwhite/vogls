mod builder;
mod format;

pub use format::{ContextFormat, DisplayContext};
pub use builder::{BasicBlockBuilder, ModuleBuilder};
use indexmap::IndexSet;
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct ModuleKey; }
new_key_type! { pub struct SectionKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }
new_key_type! { pub struct VariableKey; }

#[derive(Clone)]
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

#[derive(Clone, Copy)]
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
    Value(Value),
}

pub enum IntrinsicOp {
    Display,
    Finish,
}

pub enum UnaryOp {
    Neg,
}

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
