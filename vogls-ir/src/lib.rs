mod builder;
pub mod dyn_format_string;
mod format;
pub mod optimize;

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
pub use vogls_bits::{Bits, VectorSize};

pub use builder::{BasicBlockBuilder, BranchRef, PhiRef, new_process};
pub use format::{ContextFormat, DisplayContext};
use indexmap::IndexSet;
use slotmap::{SlotMap, new_key_type};

use self::dyn_format_string::DynFormatString;

new_key_type! { pub struct ProcessKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }
new_key_type! { pub struct VariableKey; }

#[derive(Debug, Clone, Copy)]
pub struct Time(pub u64);

#[derive(Debug, Clone)]
pub enum BasicBlockTerminator {
    Wait(BasicBlockKey, Time),
    WaitRegion(BasicBlockKey, u8),
    Watch(BasicBlockKey, Vec<SignalKey>),
    Jump(BasicBlockKey),
    /// (condition, if_true, if_false)
    Branch(VariableKey, BasicBlockKey, BasicBlockKey),
    Halt,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub name: String,
    pub instrs: Vec<Instruction>,
    pub terminator: BasicBlockTerminator,
}
impl BasicBlock {
    fn map_bbs(&mut self, map: &HashMap<BasicBlockKey, BasicBlockKey>) {
        for i in self.instrs.iter_mut() {
            match i {
                Instruction::Constant(..)
                | Instruction::Unary(..)
                | Instruction::Binary(..)
                | Instruction::Resize(..)
                | Instruction::Intrinsic(..)
                | Instruction::Probe(..)
                | Instruction::Drive(..) => {}
                Instruction::Phi(_, items) => items.iter_mut().for_each(|(bb, _)| {
                    *bb = map.get(bb).copied().unwrap_or(*bb);
                }),
            }
        }

        match &mut self.terminator {
            BasicBlockTerminator::Wait(bb, _)
            | BasicBlockTerminator::WaitRegion(bb, _)
            | BasicBlockTerminator::Watch(bb, _)
            | BasicBlockTerminator::Jump(bb) => {
                *bb = map.get(bb).copied().unwrap_or(*bb);
            }
            BasicBlockTerminator::Branch(_, bb1, bb2) => {
                *bb1 = map.get(bb1).copied().unwrap_or(*bb1);
                *bb2 = map.get(bb2).copied().unwrap_or(*bb2);
            }
            BasicBlockTerminator::Halt => {}
        }
    }
}

impl BasicBlockTerminator {
    pub fn extend_next_rev(
        &self,
        bb_stack: &mut Vec<BasicBlockKey>,
        bb_seen: &mut HashSet<BasicBlockKey>,
    ) {
        match self {
            Self::Wait(bb, _) | Self::WaitRegion(bb, _) | Self::Watch(bb, _) | Self::Jump(bb) => {
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

    fn for_each_bb(&mut self, mut f: impl FnMut(BasicBlockKey)) {
        match self {
            Self::Wait(bb, _) | Self::WaitRegion(bb, _) | Self::Watch(bb, _) | Self::Jump(bb) => {
                f(*bb);
            }
            Self::Branch(_, true_bb, false_bb) => {
                f(*true_bb);
                f(*false_bb);
            }
            Self::Halt => {}
        }
    }

    fn for_each_var_src(&self, mut f: impl FnMut(VariableKey)) {
        match self {
            Self::Branch(v, _, _) => f(*v),
            Self::Wait(..)
            | Self::WaitRegion(..)
            | Self::Watch(..)
            | Self::Jump(_)
            | Self::Halt => {}
        }
    }

    fn map_vars(&mut self, mut f: impl FnMut(VariableKey) -> VariableKey) {
        match self {
            Self::Branch(v, _, _) => *v = f(*v),
            Self::Wait(..)
            | Self::WaitRegion(..)
            | Self::Watch(..)
            | Self::Jump(_)
            | Self::Halt => {}
        }
    }
}

#[derive(Clone)]
pub struct Variable {
    pub name: String,
    pub size: VectorSize,
}

pub struct Signal {
    pub name: String,
    pub size: VectorSize,
    pub initialize: Option<Bits>,
}

pub const INTEGER_VSIZE: VectorSize = NonZeroU32::new(32).unwrap();
pub const TIME_VSIZE: VectorSize = NonZeroU32::new(64).unwrap();
pub const SCALAR_VSIZE: VectorSize = NonZeroU32::new(1).unwrap();

#[derive(Debug, Clone)]
pub enum IntrinsicOp {
    Time,
    Finish,
    Display(Box<DynFormatString>),
    Assert(Box<DynFormatString>),
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    ReduceOr,
    ReduceAnd,
    ReduceXor,
}

#[derive(Debug, Clone, Copy)]
pub enum ResizeOp {
    Truncate,
    ZeroExtend,
    SignExtend,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    And,
    Or,
    Xor,
    Add,
    Sub,
    Multiply,
    Divide,
    Modulus,
    UnsignedLessEqual,
    SelectBit,
    LogicalShiftLeft,
    LogicalShiftRight,
    ArithmeticShiftRight,
    Concat,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Constant(VariableKey, Bits),

    Unary(VariableKey, UnaryOp, VariableKey),
    Resize(VariableKey, ResizeOp, VariableKey),
    Binary(VariableKey, BinaryOp, VariableKey, VariableKey),

    Intrinsic(VariableKey, Box<IntrinsicOp>, Box<[VariableKey]>),
    Probe(VariableKey, SignalKey),
    Drive(
        SignalKey,
        VariableKey,
        u8,
        Option<(VariableKey, VectorSize)>,
    ),

    Phi(VariableKey, Box<[(BasicBlockKey, VariableKey)]>),
}

impl Instruction {
    pub fn get_destination_variable(&self) -> Option<VariableKey> {
        match self {
            Self::Constant(dst, _)
            | Self::Unary(dst, _, _)
            | Self::Resize(dst, _, _)
            | Self::Binary(dst, _, _, _)
            | Self::Phi(dst, _)
            | Self::Probe(dst, _)
            | Self::Intrinsic(dst, _, _) => Some(*dst),
            Self::Drive(..) => None,
        }
    }

    fn for_each_var_src(&self, mut f: impl FnMut(VariableKey)) {
        match self {
            Self::Unary(_, _, src) | Self::Resize(_, _, src) => f(*src),
            Self::Binary(_, _, src1, src2) => {
                f(*src1);
                f(*src2);
            }
            Self::Phi(_, srcs) => {
                for (_, s) in srcs {
                    f(*s);
                }
            }
            Self::Intrinsic(_, _, srcs) => {
                for s in srcs {
                    f(*s);
                }
            }
            Self::Drive(_, src, _, partial) => {
                f(*src);
                if let Some((off, _)) = partial {
                    f(*off);
                }
            }
            Self::Constant(_, _) | Self::Probe(_, _) => {}
        }
    }

    pub fn has_side_effects_on_call(&self) -> bool {
        match self {
            Self::Constant(..)
            | Self::Unary(..)
            | Self::Resize(..)
            | Self::Binary(..)
            | Self::Phi(..)
            | Self::Probe(..) => false,
            Self::Drive(..) | Self::Intrinsic(..) => true,
        }
    }

    fn map_vars(&mut self, mut f: impl FnMut(VariableKey) -> VariableKey) {
        match self {
            Self::Unary(dst, _, src) | Self::Resize(dst, _, src) => {
                *dst = f(*dst);
                *src = f(*src)
            }
            Self::Binary(dst, _, src1, src2) => {
                *dst = f(*dst);
                *src1 = f(*src1);
                *src2 = f(*src2);
            }
            Self::Phi(dst, srcs) => {
                *dst = f(*dst);
                for (_, s) in srcs {
                    *s = f(*s);
                }
            }
            Self::Intrinsic(dst, _, srcs) => {
                *dst = f(*dst);
                for s in srcs {
                    *s = f(*s);
                }
            }
            Self::Drive(_, src, _, partial) => {
                *src = f(*src);
                if let Some((off, _)) = partial {
                    *off = f(*off);
                }
            }
            Self::Constant(dst, _) | Self::Probe(dst, _) => {
                *dst = f(*dst);
            }
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
