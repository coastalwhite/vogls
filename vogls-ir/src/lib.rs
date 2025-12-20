mod builder;
mod format;

use std::collections::HashSet;
use std::fmt;

pub use builder::{BasicBlockBuilder, BranchRef, PhiRef, new_process};
pub use format::{ContextFormat, DisplayContext};
use indexmap::IndexSet;
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct ProcessKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }
new_key_type! { pub struct VariableKey; }

// @TODO: Do some smarter stuff here. Probably we can use the lsb to say small big and they put a
// pointer in the u64.

#[derive(Debug, Clone)]
pub enum Bits {
    Small(u64, VectorSize),
    Big(VectorSize, Box<[u8]>),
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::Small(value, size) if size % 4 == 0 => write!(f, "{size}'h{value:X}"),
            Bits::Small(value, size) => write!(f, "{size}'b{value:b}"),
            Bits::Big(size, _) => write!(f, "{size}'b.."),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bits(Bits),
    Decimal(i64),
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Self::Bits(Bits::Small(_, size)) => Type::Bits(*size),
            Self::Bits(Bits::Big(size, _)) => Type::Bits(*size),
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
    /// (condition, if_true, if_false)
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
                if bb_seen.insert(*false_bb) {
                    bb_stack.push(*false_bb);
                }
                if bb_seen.insert(*true_bb) {
                    bb_stack.push(*true_bb);
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

pub type VectorSize = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Bits(VectorSize),
    Decimal,
}
impl Type {
    pub fn to_net_width(&self) -> Option<VectorSize> {
        match self {
            Type::Bits(n) => Some(*n),
            Type::Decimal => None,
        }
    }

    pub fn try_net_width(&self) -> Result<VectorSize, ()> {
        self.to_net_width().ok_or(())
    }
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
    AssertEq(bool),
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    BitNeg(VectorSize),
    BitReduceOr(VectorSize),
    BitReduceAnd(VectorSize),
    BitReduceXor(VectorSize),
    BitSlice(VectorSize, VectorSize),

    DecimalNeg,
    DecimalReduceOr,
    DecimalReduceAnd,
    DecimalReduceXor,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    BitAnd(VectorSize),
    BitOr(VectorSize),
    BitXor(VectorSize),

    DecimalAnd,
    DecimalOr,
    DecimalXor,

    DecimalAdd,
    DecimalSub,
    DecimalMultiply,

    UnsignedLessEqual(VectorSize),
    DecimalLessEqual,

    SelectBit(VectorSize),
    LogicalShiftRight(VectorSize),
    Concat(VectorSize, VectorSize),
}

#[derive(Debug, Clone)]
pub enum Instruction {
    ConstantBit(VariableKey, Bits),
    ConstantDecimal(VariableKey, i64),

    Unary(VariableKey, UnaryOp, VariableKey),
    Binary(VariableKey, BinaryOp, VariableKey, VariableKey),

    Cast(VariableKey, VariableKey),

    Intrinsic(IntrinsicOp, Vec<IntrinsicArg>),
    Probe(VariableKey, SignalKey),
    Drive(SignalKey, VariableKey, Option<(VariableKey, VectorSize)>),

    Phi(VariableKey, Box<[(BasicBlockKey, VariableKey)]>),
}

impl Instruction {
    pub fn get_destination_variable(&self) -> Option<VariableKey> {
        match self {
            Self::ConstantBit(dst, _)
            | Self::ConstantDecimal(dst, _)
            | Self::Unary(dst, _, _)
            | Self::Binary(dst, _, _, _)
            | Self::Cast(dst, _)
            | Self::Phi(dst, _)
            | Self::Probe(dst, _) => Some(*dst),
            Self::Intrinsic(_, _) | Self::Drive(_, _, _) => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    pub processes: SlotMap<ProcessKey, Process>,
    pub bbs: SlotMap<BasicBlockKey, BasicBlock>,
    pub vars: SlotMap<VariableKey, Variable>,
    pub signals: SlotMap<SignalKey, Signal>,
}

#[derive(Debug)]
pub struct Process {
    pub name: String,
    pub entry: BasicBlockKey,
    pub ins: IndexSet<SignalKey>,
    pub outs: IndexSet<SignalKey>,
}
