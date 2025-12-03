mod builder;
mod format;

use std::collections::HashSet;

pub use builder::{BasicBlockBuilder, ModuleBuilder};
pub use format::{ContextFormat, DisplayContext};
use indexmap::{IndexMap, IndexSet};
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct ModuleKey; }
new_key_type! { pub struct ProcessKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }
new_key_type! { pub struct VariableKey; }

#[derive(Debug, Clone)]
pub enum Value {
    Bit(bool),
    Decimal(i64),
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Self::Bit(..) => Type::Bit,
            Self::Decimal(..) => Type::Decimal,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Time(pub u64);

#[derive(Debug, Clone)]
pub enum BasicBlockTerminator {
    Wait(BasicBlockKey, Time),
    Watch(BasicBlockKey, Vec<SignalKey>),
    Jump(BasicBlockKey),
    Branch(VariableKey, BasicBlockKey, BasicBlockKey),
    Halt,
}

#[derive(Debug)]
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

#[derive(Debug, Clone)]
pub enum Type {
    Bit,
    Decimal,
}

#[derive(Debug, Clone)]
pub enum IntrinsicArg {
    StringLiteral(String),
    Variable(VariableKey),
}

#[derive(Debug, Clone, Copy)]
pub enum IntrinsicOp {
    Display,
    Finish,
    Assert,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    BinaryNeg,
    LogicalNeg,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    ConstantBit(VariableKey, bool),
    UnaryBit(VariableKey, UnaryOp, VariableKey),
    BinaryBit(VariableKey, BinaryOp, VariableKey, VariableKey),

    ConstantDecimal(VariableKey, i64),
    UnaryDecimal(VariableKey, UnaryOp, VariableKey),
    BinaryDecimal(VariableKey, BinaryOp, VariableKey, VariableKey),

    BitToDecimal(VariableKey, VariableKey),

    Intrinsic(IntrinsicOp, Vec<IntrinsicArg>),
    Probe(VariableKey, SignalKey),
    Drive(SignalKey, VariableKey),

    Instantiate(ModuleKey, Vec<SignalKey>),
    Spawn(ProcessKey, Vec<SignalKey>),
    Signal(SignalKey),
}

impl Instruction {
    pub fn get_destination_variable(&self) -> Option<VariableKey> {
        match self {
            Self::ConstantBit(dst, _)
            | Self::UnaryBit(dst, _, _)
            | Self::BinaryBit(dst, _, _, _)
            | Self::ConstantDecimal(dst, _)
            | Self::UnaryDecimal(dst, _, _)
            | Self::BinaryDecimal(dst, _, _, _)
            | Self::BitToDecimal(dst, _)
            | Self::Probe(dst, _) => Some(*dst),
            Self::Intrinsic(_, _)
            | Self::Drive(_, _)
            | Self::Spawn(_, _)
            | Self::Instantiate(_, _)
            | Self::Signal(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum ConnectionDirection {
    In,
    Out,
    Both,
}

#[derive(Debug)]
pub struct Connection {
    pub signal: SignalKey,
    pub direction: ConnectionDirection,
}

#[derive(Default)]
pub struct GlobalContext {
    pub modules: SlotMap<ModuleKey, Module>,
    pub processes: SlotMap<ProcessKey, Process>,
    pub bbs: SlotMap<BasicBlockKey, BasicBlock>,
    pub vars: SlotMap<VariableKey, Variable>,
    pub signals: SlotMap<SignalKey, Signal>,
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub initialize: ProcessKey,
    pub processes: Vec<ProcessKey>,
    pub io: IndexMap<String, Connection>,
}

#[derive(Debug)]
pub struct Process {
    pub name: String,
    pub entry: BasicBlockKey,
    pub ins: IndexSet<SignalKey>,
    pub outs: IndexSet<SignalKey>,
}
