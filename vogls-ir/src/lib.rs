mod builder;
pub mod dyn_format_string;
pub mod evaluation;
mod format;
pub mod optimize;
pub mod token_range;
pub mod vcd;

use std::collections::HashSet;
use std::num::NonZeroU32;
pub use vogls_bits as bits;
pub use vogls_bits::{Bits, Mode, VectorSize};

pub use builder::{BasicBlockBuilder, BranchRef, PhiRef, new_anonymous_builder, new_process};
pub use format::{ContextFormat, DisplayContext};
use indexmap::IndexSet;
use slotmap::{SlotMap, new_key_type};

use self::dyn_format_string::DynFormatString;
use self::token_range::TokenRange;
use self::vcd::VcdScope;

new_key_type! { pub struct ProcessKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }
new_key_type! { pub struct VariableKey; }

#[derive(Debug, Clone, Copy)]
pub struct Time(pub u64);

#[derive(Debug, Clone)]
pub enum BasicBlockTerminator {
    Wait(BasicBlockKey, Time),
    VariableWait(BasicBlockKey, VariableKey),
    WaitRegion(BasicBlockKey, u8),
    Watch(BasicBlockKey, Vec<SignalKey>),
    Jump(BasicBlockKey),
    /// (condition, if_true, if_false)
    Branch(VariableKey, BasicBlockKey, BasicBlockKey),
    Halt,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub instrs: Vec<Instruction>,
    pub terminator: BasicBlockTerminator,
}
impl BasicBlock {
    pub fn map_bbs(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        for i in self.instrs.iter_mut() {
            i.map_bb(&mut f);
        }
        self.terminator.map_bb(f);
    }

    fn remove_fan_in_edge(&mut self, bb_key: BasicBlockKey) {
        for i in &mut self.instrs {
            if let Instruction::Phi(_dst, origins) = i {
                assert!(origins.len() >= 2);
                let idx = origins
                    .iter()
                    .position(|(obb, _)| *obb == bb_key)
                    .expect("phis are expected to have a variable per fan-in basic-block");
                if origins.len() == 2 {
                    todo!()
                    // *i = Instruction::Unary(*dst, UnaryOp::Copy, origins[1 - idx].1);
                } else {
                    let mut new_origins = Vec::with_capacity(origins.len() - 1);
                    new_origins.extend(&origins[..idx]);
                    new_origins.extend(&origins[idx + 1..]);
                    *origins = new_origins.into();
                }
            }
        }
    }

    pub fn for_each_fanout(&self, f: impl FnMut(BasicBlockKey)) {
        self.terminator.for_each_bb(f);
    }

    fn map_bb(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        for i in self.instrs.iter_mut() {
            i.map_bb(&mut f);
        }
        self.terminator.map_bb(f);
    }

    pub fn map_vars(&mut self, mut f: impl FnMut(VariableKey) -> VariableKey) {
        for i in &mut self.instrs {
            i.map_vars(&mut f);
        }
        self.terminator.map_vars(f);
    }

    fn for_each_var(&self, mut f: impl FnMut(VariableKey)) {
        for i in &self.instrs {
            i.for_each_var(&mut f);
        }
        self.terminator.for_each_var(f);
    }

    pub fn map_signals(&mut self, mut f: impl FnMut(SignalKey) -> SignalKey) {
        for i in &mut self.instrs {
            i.map_signals(&mut f);
        }
        self.terminator.map_signal(f);
    }
}

impl BasicBlockTerminator {
    pub fn extend_next_rev(
        &self,
        bb_stack: &mut Vec<BasicBlockKey>,
        bb_seen: &mut HashSet<BasicBlockKey>,
    ) {
        match self {
            Self::Wait(bb, _)
            | Self::VariableWait(bb, _)
            | Self::WaitRegion(bb, _)
            | Self::Watch(bb, _)
            | Self::Jump(bb) => {
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

    pub fn for_each_bb(&self, mut f: impl FnMut(BasicBlockKey)) {
        match self {
            Self::Wait(bb, _)
            | Self::VariableWait(bb, _)
            | Self::WaitRegion(bb, _)
            | Self::Watch(bb, _)
            | Self::Jump(bb) => {
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
            Self::VariableWait(_, v) | Self::Branch(v, _, _) => f(*v),
            Self::Wait(..)
            | Self::WaitRegion(..)
            | Self::Watch(..)
            | Self::Jump(_)
            | Self::Halt => {}
        }
    }

    fn map_vars(&mut self, mut f: impl FnMut(VariableKey) -> VariableKey) {
        match self {
            Self::VariableWait(_, v) | Self::Branch(v, _, _) => *v = f(*v),
            Self::Wait(..)
            | Self::WaitRegion(..)
            | Self::Watch(..)
            | Self::Jump(_)
            | Self::Halt => {}
        }
    }
    fn for_each_var(&self, mut f: impl FnMut(VariableKey)) {
        match self {
            Self::VariableWait(_, v) | Self::Branch(v, _, _) => f(*v),
            Self::Wait(..)
            | Self::WaitRegion(..)
            | Self::Watch(..)
            | Self::Jump(_)
            | Self::Halt => {}
        }
    }

    pub fn map_bb(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        match self {
            BasicBlockTerminator::Wait(bb, _)
            | BasicBlockTerminator::VariableWait(bb, _)
            | BasicBlockTerminator::WaitRegion(bb, _)
            | BasicBlockTerminator::Watch(bb, _)
            | BasicBlockTerminator::Jump(bb) => {
                *bb = f(*bb);
            }
            BasicBlockTerminator::Branch(_, bb1, bb2) => {
                *bb1 = f(*bb1);
                *bb2 = f(*bb2);
            }
            BasicBlockTerminator::Halt => {}
        }
    }

    #[expect(unused)]
    fn for_each_signal(&self, f: impl FnMut(SignalKey)) {
        match self {
            Self::Branch(..)
            | Self::VariableWait(..)
            | Self::Wait(..)
            | Self::WaitRegion(..)
            | Self::Jump(_)
            | Self::Halt => {}
            Self::Watch(_, signals) => signals.iter().copied().for_each(f),
        }
    }
    fn map_signal(&mut self, mut f: impl FnMut(SignalKey) -> SignalKey) {
        match self {
            Self::Branch(..)
            | Self::Wait(..)
            | Self::VariableWait(..)
            | Self::WaitRegion(..)
            | Self::Jump(_)
            | Self::Halt => {}
            Self::Watch(_, signals) => signals.iter_mut().for_each(|s| *s = f(*s)),
        }
    }
}

#[derive(Clone)]
pub struct Variable {
    pub size: VectorSize,
}

#[derive(Clone)]
pub struct Signal {
    pub name: String,
    pub size: VectorSize,
    pub initialize: Option<Bits>,
    pub origin: TokenRange,
}

pub const INTEGER_VSIZE: VectorSize = NonZeroU32::new(32).unwrap();
pub const TIME_VSIZE: VectorSize = NonZeroU32::new(64).unwrap();
pub const SCALAR_VSIZE: VectorSize = NonZeroU32::new(1).unwrap();

#[derive(Debug, Clone)]
pub enum IntrinsicOp {
    Time,
    Finish,
    Random,
    Display(Box<DynFormatString>),
    Assert(Box<DynFormatString>),
    VcdOpenFile(String),
    VcdAppendModule(VcdScope),
    VcdPause,
    VcdResume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Power,
    Multiply,
    Divide,
    Modulus,

    UnsignedLessEqual,
    SelectBit,
    LogicalShiftLeft,
    LogicalShiftRight,
    ArithmeticShiftRight,
    Concat,

    /// Copy X from rhs into lhs
    CopyX,
    /// Copy Z from rhs into lhs
    CopyZ,

    /// Minimum of lhs and rhs
    Min,
    /// Maximum of lhs and rhs
    Max,

    /// Exact bitpattern equality
    CaseEquality,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Constant(VariableKey, Bits),

    Unary(VariableKey, UnaryOp, VariableKey),
    Resize(VariableKey, ResizeOp, VariableKey),
    Binary(VariableKey, BinaryOp, VariableKey, VariableKey),

    Intrinsic(VariableKey, Box<IntrinsicOp>, Box<[VariableKey]>),
    LastUpdateTime(VariableKey, SignalKey),
    Probe(VariableKey, SignalKey),
    Drive(SignalKey, VariableKey, Option<(VariableKey, VectorSize)>),

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
            | Self::LastUpdateTime(dst, _)
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
            Self::Drive(_, src, partial) => {
                f(*src);
                if let Some((off, _)) = partial {
                    f(*off);
                }
            }
            Self::Constant(_, _) | Self::LastUpdateTime(_, _) | Self::Probe(..) => {}
        }
    }

    pub fn has_side_effects_on_call(&self) -> bool {
        match self {
            Self::Constant(..)
            | Self::Unary(..)
            | Self::Resize(..)
            | Self::Binary(..)
            | Self::Phi(..)
            | Self::LastUpdateTime(..)
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
            Self::Drive(_, src, partial) => {
                *src = f(*src);
                if let Some((off, _)) = partial {
                    *off = f(*off);
                }
            }
            Self::Constant(dst, _) | Self::Probe(dst, _) | Self::LastUpdateTime(dst, _) => {
                *dst = f(*dst);
            }
        }
    }
    fn for_each_var(&self, mut f: impl FnMut(VariableKey)) {
        match self {
            Self::Unary(dst, _, src) | Self::Resize(dst, _, src) => {
                f(*dst);
                f(*src)
            }
            Self::Binary(dst, _, src1, src2) => {
                f(*dst);
                f(*src1);
                f(*src2);
            }
            Self::Phi(dst, srcs) => {
                f(*dst);
                for (_, s) in srcs {
                    f(*s);
                }
            }
            Self::Intrinsic(dst, _, srcs) => {
                f(*dst);
                for s in srcs {
                    f(*s);
                }
            }
            Self::Drive(_, src, partial) => {
                f(*src);
                if let Some((off, _)) = partial {
                    f(*off);
                }
            }
            Self::Constant(dst, _) | Self::LastUpdateTime(dst, _) | Self::Probe(dst, _) => {
                f(*dst);
            }
        }
    }

    fn map_signals(&mut self, mut f: impl FnMut(SignalKey) -> SignalKey) {
        match self {
            Instruction::Probe(_, s)
            | Instruction::Drive(s, _, _)
            | Instruction::LastUpdateTime(_, s) => *s = f(*s),
            Instruction::Constant(..)
            | Instruction::Unary(..)
            | Instruction::Resize(..)
            | Instruction::Binary(..)
            | Instruction::Intrinsic(..)
            | Instruction::Phi(..) => {}
        }
    }

    fn map_bb(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        match self {
            Instruction::Constant(..)
            | Instruction::Unary(..)
            | Instruction::Resize(..)
            | Instruction::Binary(..)
            | Instruction::Intrinsic(..)
            | Instruction::Probe(..)
            | Instruction::Drive(..)
            | Instruction::LastUpdateTime(..) => {}
            Instruction::Phi(_, items) => items.iter_mut().for_each(|(bb, _)| {
                *bb = f(*bb);
            }),
        }
    }
}

impl UnaryOp {
    pub fn evaluate(self, src: &Bits) -> Bits {
        use UnaryOp as O;
        match self {
            O::Neg => src.bitwise_negate(),
            O::ReduceOr => Bits::from(src.reduce_or()),
            O::ReduceAnd => Bits::from(src.reduce_and()),
            O::ReduceXor => Bits::from(src.reduce_xor()),
        }
    }
}

impl ResizeOp {
    pub fn evaluate(self, src: &Bits, dst_size: VectorSize) -> Bits {
        use ResizeOp as O;
        match self {
            O::Truncate => src.truncate(dst_size),
            O::ZeroExtend => src.zero_extend(dst_size),
            O::SignExtend => src.sign_extend(dst_size),
        }
    }
}

impl BinaryOp {
    fn evaluate(self, lhs: &Bits, rhs: &Bits) -> Bits {
        use BinaryOp as O;
        match self {
            O::And => Bits::bitwise_and(lhs, rhs),
            O::Or => Bits::bitwise_or(lhs, rhs),
            O::Xor => Bits::bitwise_xor(lhs, rhs),
            O::Add => Bits::add(lhs, rhs),
            O::Sub => Bits::subtract(lhs, rhs),
            O::Power => Bits::power(lhs, rhs),
            O::Multiply => Bits::multiply(lhs, rhs),
            O::Divide => Bits::divide(lhs, rhs),
            O::Modulus => Bits::remainder(lhs, rhs),

            O::UnsignedLessEqual => Bits::from(Bits::is_unsigned_leq(lhs, rhs)),
            O::CaseEquality => Bits::from(lhs == rhs),
            O::SelectBit => Bits::from(lhs.select_bit(rhs.extract_exact_u32())),
            O::LogicalShiftLeft => lhs.logical_shift_left(rhs.extract_exact_u32()),
            O::LogicalShiftRight => lhs.logical_shift_right(rhs.extract_exact_u32()),
            O::ArithmeticShiftRight => lhs.arithmetic_shift_right(rhs.extract_exact_u32()),
            O::Concat => Bits::concatenate(lhs, rhs),
            O::CopyX => Bits::copyx(lhs, rhs),
            O::CopyZ => Bits::copyz(lhs, rhs),

            O::Min => Bits::min(lhs, rhs),
            O::Max => Bits::max(lhs, rhs),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicMode {
    #[default]
    TwoValue,
    FourValue,
}

#[derive(Default)]
pub struct GlobalContext {
    pub logic_mode: LogicMode,
    pub processes: SlotMap<ProcessKey, Process>,
    pub bbs: SlotMap<BasicBlockKey, BasicBlock>,
    pub vars: SlotMap<VariableKey, Variable>,
    pub signals: SlotMap<SignalKey, Signal>,
}

#[derive(Debug)]
pub struct Process {
    pub name: String,
    pub entry: BasicBlockKey,
    pub origin: TokenRange,
    pub ins: IndexSet<SignalKey>,
    pub outs: IndexSet<SignalKey>,
    pub lazy: bool,
}
