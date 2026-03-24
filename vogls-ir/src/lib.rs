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
use slotmap::{SlotMap, new_key_type};
use vogls_utils::NonMaxU32;

use self::dyn_format_string::DynFormatString;
use self::token_range::TokenRange;
use self::vcd::VcdOutput;

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

    pub fn for_each_var(&self, mut f: impl FnMut(VariableKey)) {
        for i in &self.instrs {
            i.for_each_var(&mut f);
        }
        self.terminator.for_each_var(f);
    }

    pub fn try_for_each_dst_var<E>(
        &self,
        mut f: impl FnMut(VariableKey) -> Result<(), E>,
    ) -> Result<(), E> {
        for i in &self.instrs {
            if let Some(dst) = i.get_destination_variable() {
                f(dst)?;
            }
        }
        Ok(())
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
    pub fn map_signal(&mut self, mut f: impl FnMut(SignalKey) -> SignalKey) {
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

    pub fn is_temporal(&self) -> bool {
        matches!(
            self,
            Self::Wait(..) | Self::VariableWait(..) | Self::WaitRegion(..) | Self::Watch(..)
        )
    }
}

#[derive(Clone)]
pub struct Variable {
    pub size: VectorSize,
}

#[derive(Debug, Clone)]
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
    VcdAppendModule(VcdOutput),
    VcdPause,
    VcdResume,

    ReadMem(Box<ReadMem>),
}

#[derive(Debug, Clone)]
pub struct ReadMem {
    pub path: String,
    pub signal: SignalKey,
    pub offset: u32,
    pub limit: u32,
    pub stride: VectorSize,
    pub binary: bool,
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
    /// Extract the least-significant bits from the source.
    Truncate,
    /// Extend the source with `0` bits.
    ZeroExtend,
    /// Extend the source with the most-significant bit.
    SignExtend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// A bitwise-and operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// Results are calculated according to the following truth-table.
    ///
    /// ```
    /// & | x  z  1  0
    /// --+-----------
    /// x | x  x  x  0
    /// z | x  x  x  0
    /// 1 | x  x  1  0
    /// 0 | 0  0  0  0
    /// ```
    And,

    /// A bitwise-or operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// Results are calculated according to the following truth-table.
    ///
    /// ```
    /// | | x  z  1  0
    /// --+-----------
    /// x | x  x  1  x
    /// z | x  x  1  x
    /// 1 | 1  1  1  1
    /// 0 | x  x  1  0
    /// ```
    Or,

    /// A bitwise exclusive-or operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// Results are calculated according to the following truth-table.
    ///
    /// ```
    /// ^ | x  z  1  0
    /// --+-----------
    /// x | x  x  x  x
    /// z | x  x  x  x
    /// 1 | x  x  0  1
    /// 0 | x  x  1  0
    /// ```
    Xor,

    /// Wrapping arithmetic addition operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    Add,
    /// Wrapping arithmetic subtraction operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    Sub,
    /// Wrapping arithmetic power operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    Power,
    /// Wrapping arithmetic multiplication operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    Multiply,
    /// Wrapping arithmetic division operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    Divide,
    /// Wrapping arithmetic modulus operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    Modulus,

    /// Unsigned relational less-than equals operator.
    ///
    /// Takes two operands of equal size and outputs a single bit.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    UnsignedLessEqual,

    /// Extract the destination size bits from the source. Starting from a specified offset. If the
    /// offset plus the destination size are larger than the source size, additional `x` bits are
    /// inserted.
    Slice,

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
    /// Positive edge
    Posedge,
    /// Negedge edge
    Negedge,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Constant(VariableKey, Bits),

    Unary(VariableKey, UnaryOp, VariableKey),
    Resize(VariableKey, ResizeOp, VariableKey),
    Binary(VariableKey, BinaryOp, VariableKey, VariableKey),

    Intrinsic(VariableKey, Box<IntrinsicOp>, Box<[VariableKey]>),
    /// Store the 64-bit simulation time when a signal was last updated.
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

    pub fn get_destination_variable_mut(&mut self) -> Option<&mut VariableKey> {
        match self {
            Self::Constant(dst, _)
            | Self::Unary(dst, _, _)
            | Self::Resize(dst, _, _)
            | Self::Binary(dst, _, _, _)
            | Self::Phi(dst, _)
            | Self::LastUpdateTime(dst, _)
            | Self::Probe(dst, _)
            | Self::Intrinsic(dst, _, _) => Some(dst),
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

    pub fn for_each_src(&self, mut f: impl FnMut(VariableKey)) {
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
            Self::Constant(_, _) | Self::LastUpdateTime(_, _) | Self::Probe(_, _) => {}
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
    fn evaluate(self, lhs: &Bits, rhs: &Bits, dst_size: VectorSize) -> Bits {
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
            O::Slice => Bits::from(lhs.slice(rhs.extract_exact_u32(), dst_size)),
            O::LogicalShiftLeft => lhs.logical_shift_left(rhs.extract_exact_u32()),
            O::LogicalShiftRight => lhs.logical_shift_right(rhs.extract_exact_u32()),
            O::ArithmeticShiftRight => lhs.arithmetic_shift_right(rhs.extract_exact_u32()),
            O::Concat => Bits::concatenate(lhs, rhs),
            O::CopyX => Bits::copyx(lhs, rhs),
            O::CopyZ => Bits::copyz(lhs, rhs),

            O::Min => Bits::min(lhs, rhs),
            O::Max => Bits::max(lhs, rhs),

            O::Posedge => Bits::from(vogls_bits::edge::fv_posedge(
                lhs.select_value(0),
                rhs.select_value(0),
            )),
            O::Negedge => Bits::from(vogls_bits::edge::fv_negedge(
                lhs.select_value(0),
                rhs.select_value(0),
            )),
        }
    }

    pub fn always_outputs_bool(&self) -> bool {
        matches!(self, Self::CaseEquality | Self::Posedge | Self::Negedge)
    }

    pub fn always_outputs_four_value(&self) -> bool {
        matches!(self, Self::Slice)
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
impl LogicMode {
    pub fn other(&self) -> LogicMode {
        match self {
            Self::TwoValue => Self::FourValue,
            Self::FourValue => Self::TwoValue,
        }
    }
}

impl From<Mode> for LogicMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::TwoValue => Self::TwoValue,
            Mode::FourValue => Self::FourValue,
        }
    }
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
    pub lazy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SignalSlice {
    width: VectorSize,
    lsb: NonMaxU32,
}

impl SignalSlice {
    pub fn new(msb: u32, lsb: u32) -> Option<Self> {
        let lsb = NonMaxU32::new(lsb)?;
        let width = VectorSize::new(msb.checked_sub(lsb.get())?.checked_add(1)?)?;
        Some(Self { width, lsb })
    }

    pub fn with_end(width: VectorSize) -> Self {
        Self {
            lsb: NonMaxU32::ZERO,
            width,
        }
    }
    pub fn from_width(lsb: u32, width: VectorSize) -> Option<Self> {
        lsb.checked_add(width.get())?;
        let lsb = NonMaxU32::new(lsb)?;
        Some(Self { width, lsb })
    }
    pub fn from_range(range: std::ops::Range<u32>) -> Option<Self> {
        Self::from_width(range.start, VectorSize::new(range.end - range.start)?)
    }

    pub fn width(self) -> VectorSize {
        self.width
    }
    pub fn lsb(self) -> u32 {
        self.lsb.get()
    }
    pub fn msb(self) -> u32 {
        self.lsb.get() + (self.width.get() - 1)
    }

    pub fn shift(self, amount: u32) -> Option<SignalSlice> {
        self.msb().checked_add(amount)?;
        Some(Self {
            lsb: NonMaxU32::new(self.lsb.get() + amount).unwrap(),
            width: self.width,
        })
    }

    pub fn shift_back(self, amount: u32) -> Option<SignalSlice> {
        Some(Self {
            lsb: NonMaxU32::new(self.lsb().checked_sub(amount)?).unwrap(),
            width: self.width,
        })
    }

    pub fn relative_slice(self, subslice: SignalSlice) -> Option<SignalSlice> {
        if subslice.lsb() < self.lsb() || subslice.msb() > self.msb() {
            return None;
        }
        subslice.shift_back(self.lsb())
    }

    pub fn subslice(self, s: SignalSlice) -> Option<SignalSlice> {
        if s.msb() >= self.width().get() {
            return None;
        }
        Self::from_width(self.lsb() + s.lsb(), s.width())
    }

    pub fn concat(self, other: SignalSlice) -> Option<SignalSlice> {
        if self.lsb.get() + self.width.get() != other.lsb.get() {
            return None;
        }
        Some(Self {
            lsb: self.lsb,
            width: self.width.checked_add(other.width().get()).unwrap(),
        })
    }
}
