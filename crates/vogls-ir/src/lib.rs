mod builder;
pub mod dyn_format_string;
pub mod evaluation;
mod form;
mod format;
pub mod optimize;
pub mod orders;
pub mod parse;
pub mod token_range;
mod variable;
pub mod vcd;
pub mod watchers;

use std::fmt;
use std::num::NonZeroU32;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};
pub use vogls_bits as bits;
pub use vogls_bits::{Bits, Mode, VectorSize};

pub use builder::{BasicBlockBuilder, BranchRef, PhiRef, ProcessBuilder};
pub use format::{ContextFormat, DisplayContext, LabelDisplay};
use slotmap::{SlotMap, new_key_type};
use vogls_utils::NonMaxU32;

use self::dyn_format_string::DynFormatString;
use self::token_range::TokenRange;
pub use self::variable::{VariableKey, VariableMap};
use self::vcd::VcdOutput;

new_key_type! { pub struct ProcessKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TemporalRegionKey(BasicBlockKey);

impl TemporalRegionKey {
    pub fn from_entry(key: BasicBlockKey) -> Self {
        Self(key)
    }

    pub fn entry(self) -> BasicBlockKey {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Time(pub u64);

#[derive(Debug, Clone)]
pub enum BasicBlockTerminator {
    Wait(TemporalRegionKey, Time),
    VariableWait(TemporalRegionKey, VariableKey),
    WaitRegion(TemporalRegionKey, u8),
    Watch(TemporalRegionKey, Vec<SignalKey>),

    Jump(BasicBlockKey),
    /// (condition, if_true, if_false)
    Branch(VariableKey, BasicBlockKey, BasicBlockKey),
    Halt,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub instrs: Vec<Instruction>,
    pub region: TemporalRegionKey,
    pub terminator: BasicBlockTerminator,
}
impl BasicBlock {
    pub fn map_temporal_bbs(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        for i in self.instrs.iter_mut() {
            i.map_bb(&mut f);
        }
        self.terminator.map_temporal_bb(f);
    }
    pub fn map_non_temporal_bbs(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        for i in self.instrs.iter_mut() {
            i.map_bb(&mut f);
        }
        self.terminator.map_non_temporal_bb(f);
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
    pub fn for_each_temporal_bb(&self, mut f: impl FnMut(BasicBlockKey)) {
        match self {
            Self::Wait(bb, ..)
            | Self::VariableWait(bb, ..)
            | Self::WaitRegion(bb, ..)
            | Self::Watch(bb, ..) => f(bb.entry()),

            Self::Jump(bb) => {
                f(*bb);
            }
            Self::Branch(_, true_bb, false_bb) => {
                f(*true_bb);
                f(*false_bb);
            }
            Self::Halt => {}
        }
    }
    pub fn for_each_non_temporal_bb(&self, mut f: impl FnMut(BasicBlockKey)) {
        match self {
            Self::Wait(..) | Self::VariableWait(..) | Self::WaitRegion(..) | Self::Watch(..) => {}

            Self::Jump(bb) => {
                f(*bb);
            }
            Self::Branch(_, true_bb, false_bb) => {
                f(*true_bb);
                f(*false_bb);
            }
            Self::Halt => {}
        }
    }

    pub fn for_each_var_src(&self, mut f: impl FnMut(VariableKey)) {
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

    pub fn map_temporal_bb(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        match self {
            BasicBlockTerminator::Wait(tr, _)
            | BasicBlockTerminator::VariableWait(tr, _)
            | BasicBlockTerminator::WaitRegion(tr, _)
            | BasicBlockTerminator::Watch(tr, _) => {
                tr.0 = f(tr.0);
            }

            BasicBlockTerminator::Jump(bb) => {
                *bb = f(*bb);
            }
            BasicBlockTerminator::Branch(_, bb1, bb2) => {
                *bb1 = f(*bb1);
                *bb2 = f(*bb2);
            }
            BasicBlockTerminator::Halt => {}
        }
    }
    pub fn map_non_temporal_bb(&mut self, mut f: impl FnMut(BasicBlockKey) -> BasicBlockKey) {
        match self {
            BasicBlockTerminator::Wait(..)
            | BasicBlockTerminator::VariableWait(..)
            | BasicBlockTerminator::WaitRegion(..)
            | BasicBlockTerminator::Watch(..) => {}

            BasicBlockTerminator::Jump(bb) => {
                *bb = f(*bb);
            }
            BasicBlockTerminator::Branch(_, bb1, bb2) => {
                *bb1 = f(*bb1);
                *bb2 = f(*bb2);
            }
            BasicBlockTerminator::Halt => {}
        }
    }

    pub fn for_each_signal(&self, f: impl FnMut(SignalKey)) {
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

#[derive(Clone, Copy)]
pub struct SignalFlags(u8);

impl SignalFlags {
    pub const ALL: Self = Self(0b0011u8);
    pub const EMPTY: Self = Self(0b0000u8);

    pub const EXT_DRIVE: Self = Self(0b0001u8);
    pub const EXT_PROBE: Self = Self(0b0010u8);

    pub fn contains(self, rhs: Self) -> bool {
        self.0 & rhs.0 == rhs.0
    }
}

impl BitOr<Self> for SignalFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign<Self> for SignalFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
impl BitAnd<Self> for SignalFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign<Self> for SignalFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}
impl BitXor<Self> for SignalFlags {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign<Self> for SignalFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl fmt::Debug for SignalFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalFlags")
            .field("drive", &self.contains(Self::EXT_DRIVE))
            .field("probe", &self.contains(Self::EXT_PROBE))
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub name: String,
    pub size: VectorSize,
    pub initialize: Option<Bits>,
    pub mode: LogicMode,
    pub flags: SignalFlags,
    pub origin: TokenRange,
}

pub const SCALAR_VSIZE: VectorSize = NonZeroU32::new(1).unwrap();
pub const VSIZE_32: VectorSize = NonZeroU32::new(32).unwrap();
pub const VSIZE_64: VectorSize = NonZeroU32::new(64).unwrap();

pub const INTEGER_VSIZE: VectorSize = VSIZE_32;
pub const TIME_VSIZE: VectorSize = VSIZE_64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RandomKind {
    Uniform,
    Normal,
    Exponential,
    Poisson,
    ChiSquare,
    T,
    Erlang,
}

#[derive(Debug, Clone)]
pub enum IntrinsicOp {
    Time,
    Finish,
    /// Random returns a 64-bit number containing (new_seed, result) packed as two 32-bit numbers.
    Random(RandomKind),
    Display(Box<DynFormatString>),
    Assert(Box<DynFormatString>),
    VcdOpenFile(String),
    VcdAppendModule(VcdOutput),
    VcdPause,
    VcdResume,

    /// This is a no-op instruction that prevents optimization through it.
    BlackBox,

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    ReduceOr,
    ReduceAnd,
    ReduceXor,
    LeadingZeros,

    TvToFv,
    FvToTv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizeOp {
    /// Extract the least-significant bits from the source.
    Truncate,
    /// Extend the source with `0` bits.
    ZeroExtend,
    /// Extend the source with the most-significant bit.
    SignExtend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    ///
    /// Division by zero equals X.
    DivideX,
    /// Wrapping arithmetic division operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    ///
    /// Division by zero equals 0.
    Divide0,
    /// Wrapping arithmetic modulus operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    ///
    /// Modulus by zero equals X.
    ModulusX,
    /// Wrapping arithmetic modulus operator.
    ///
    /// Takes two operands of equal size and outputs equal size.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    ///
    /// Modulus by zero equals 0.
    Modulus0,

    /// Unsigned relational less-than equals operator.
    ///
    /// Takes two operands of equal size and outputs a single bit.
    ///
    /// If either operands contains an unknown `x` or high-impedance `z` bit, the result is all
    /// unknown bits.
    UnsignedLessEqual,

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

    /// ANDNOT(a, b) = AND(a, NOT(b))
    AndNot,
    /// ORNOT(a, b) = OR(a, NOT(b))
    OrNot,
    /// XNOR(a, b) = XOR(a, NOT(b)) = NOT(XOR(a, b))
    Xnor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryImmOp {
    And,
    Or,
    Xor,

    Add,
    /// Operand - Imm
    Sub,
    /// Operand ** Imm
    Power,
    Multiply,
    /// Operand / Imm
    ///
    /// Division by zero equals 0.
    Divide,
    /// Operand % Imm
    ///
    /// Division by zero equals 0.
    Modulus,

    /// Imm - Operand
    RevSub,
    /// Imm ** Operand
    RevPower,
    /// Imm / Operand
    ///
    /// Division by zero equals X.
    RevDivideX,
    /// Imm / Operand
    ///
    /// Division by zero equals 0.
    RevDivide0,
    /// Imm % Operand
    ///
    /// Division by zero equals X
    RevModulusX,
    /// Imm % Operand
    ///
    /// Division by zero equals 0
    RevModulus0,

    /// Operand <= Imm
    UnsignedLessEqual,
    /// Imm <= Operand
    UnsignedGreaterEqual,

    /// { Operand, Imm }
    ConcatRight,
    /// { Imm, Operand }
    ConcatLeft,

    Min,
    Max,

    BitwiseCaseEquality,
    CaseEquality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShiftImmOp {
    /// Operand << Imm
    LogicalShiftLeft,
    /// Operand >> Imm
    LogicalShiftRight,
    /// Operand >>> Imm
    ArithmeticShiftRight,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Constant(VariableKey, Bits),

    Unary(VariableKey, UnaryOp, VariableKey),
    Resize(VariableKey, ResizeOp, VariableKey),
    Binary(VariableKey, BinaryOp, VariableKey, VariableKey),
    BinaryImm(VariableKey, BinaryImmOp, VariableKey, Bits),

    Slice(VariableKey, VariableKey, VariableKey),
    SliceImm(VariableKey, VariableKey, u32),
    ShiftImm(VariableKey, ShiftImmOp, VariableKey, u32),

    Select(VariableKey, VariableKey, VariableKey, VariableKey),

    Intrinsic(VariableKey, Box<IntrinsicOp>, Box<[VariableKey]>),
    /// Store the 64-bit simulation time when a signal was last updated.
    LastUpdateTime(VariableKey, SignalKey),
    Probe(VariableKey, SignalKey, u32),
    ProbeSlice(VariableKey, SignalKey, VariableKey),

    /// Update the value of a signal, poking it if the value is different than before.
    ///
    /// A drive can be a "partial" drive, meaning that the source value is offset by a certain
    /// amount of bits, and no bits beyond a certain point will be affected.
    Drive(SignalKey, VariableKey, u32),

    DriveSlice(SignalKey, VariableKey, VariableKey),

    Phi(VariableKey, Box<[(BasicBlockKey, VariableKey)]>),
}

impl Instruction {
    pub fn copy(vars: &VariableMap, dst: VariableKey, src: VariableKey) -> Self {
        assert_eq!(vars.size(dst), vars.size(src));
        use LogicMode as M;
        match (dst.mode(), src.mode()) {
            (M::TwoValue, M::FourValue) => Instruction::Unary(dst, UnaryOp::FvToTv, src),
            (M::FourValue, M::TwoValue) => Instruction::Unary(dst, UnaryOp::TvToFv, src),
            (M::TwoValue, M::TwoValue) | (M::FourValue, M::FourValue) => {
                Instruction::Resize(dst, ResizeOp::Truncate, src)
            }
        }
    }

    pub fn get_destination_variable(&self) -> Option<VariableKey> {
        match self {
            Self::Constant(dst, _)
            | Self::Unary(dst, _, _)
            | Self::Resize(dst, _, _)
            | Self::Binary(dst, _, _, _)
            | Self::BinaryImm(dst, _, _, _)
            | Self::Slice(dst, _, _)
            | Self::SliceImm(dst, _, _)
            | Self::ShiftImm(dst, _, _, _)
            | Self::Select(dst, _, _, _)
            | Self::Phi(dst, _)
            | Self::LastUpdateTime(dst, _)
            | Self::Probe(dst, _, _)
            | Self::ProbeSlice(dst, _, _)
            | Self::Intrinsic(dst, _, _) => Some(*dst),
            Self::Drive(..) | Self::DriveSlice(..) => None,
        }
    }

    pub fn get_destination_variable_mut(&mut self) -> Option<&mut VariableKey> {
        match self {
            Self::Constant(dst, _)
            | Self::Unary(dst, _, _)
            | Self::Resize(dst, _, _)
            | Self::Binary(dst, _, _, _)
            | Self::BinaryImm(dst, _, _, _)
            | Self::Slice(dst, _, _)
            | Self::SliceImm(dst, _, _)
            | Self::ShiftImm(dst, _, _, _)
            | Self::Select(dst, _, _, _)
            | Self::Phi(dst, _)
            | Self::LastUpdateTime(dst, _)
            | Self::Probe(dst, _, _)
            | Self::ProbeSlice(dst, _, _)
            | Self::Intrinsic(dst, _, _) => Some(dst),
            Self::Drive(..) | Self::DriveSlice(..) => None,
        }
    }

    pub fn has_side_effects_on_call(&self) -> bool {
        match self {
            Self::Constant(..)
            | Self::Unary(..)
            | Self::Resize(..)
            | Self::Binary(..)
            | Self::BinaryImm(..)
            | Self::Slice(_, _, _)
            | Self::SliceImm(_, _, _)
            | Self::ShiftImm(_, _, _, _)
            | Self::Select(..)
            | Self::Phi(..)
            | Self::LastUpdateTime(..)
            | Self::Probe(..)
            | Self::ProbeSlice(..) => false,
            Self::Drive(..) | Self::DriveSlice(..) | Self::Intrinsic(..) => true,
        }
    }

    fn map_vars(&mut self, mut f: impl FnMut(VariableKey) -> VariableKey) {
        match self {
            Self::Unary(dst, _, src)
            | Self::Resize(dst, _, src)
            | Self::BinaryImm(dst, _, src, _)
            | Self::SliceImm(dst, src, _)
            | Self::ShiftImm(dst, _, src, _)
            | Self::ProbeSlice(dst, _, src) => {
                *dst = f(*dst);
                *src = f(*src)
            }
            Self::Binary(dst, _, src1, src2) | Self::Slice(dst, src1, src2) => {
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
            Self::DriveSlice(_, src, partial) => {
                *src = f(*src);
                *partial = f(*partial);
            }
            Self::Drive(_, src, _) => {
                *src = f(*src);
            }
            Self::Select(dst, cond, truthy, falsy) => {
                *dst = f(*dst);
                *cond = f(*cond);
                *truthy = f(*truthy);
                *falsy = f(*falsy);
            }
            Self::Constant(dst, _) | Self::Probe(dst, _, _) | Self::LastUpdateTime(dst, _) => {
                *dst = f(*dst);
            }
        }
    }
    fn for_each_var(&self, mut f: impl FnMut(VariableKey)) {
        match self {
            Self::Unary(dst, _, src)
            | Self::Resize(dst, _, src)
            | Self::BinaryImm(dst, _, src, _)
            | Self::SliceImm(dst, src, _)
            | Self::ShiftImm(dst, _, src, _)
            | Self::ProbeSlice(dst, _, src) => {
                f(*dst);
                f(*src)
            }
            Self::Binary(dst, _, src1, src2) | Self::Slice(dst, src1, src2) => {
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
            Self::Drive(_, src, _) => {
                f(*src);
            }
            Self::DriveSlice(_, src, partial) => {
                f(*src);
                f(*partial);
            }
            Self::Select(dst, cond, truthy, falsy) => {
                f(*dst);
                f(*cond);
                f(*truthy);
                f(*falsy);
            }
            Self::Constant(dst, _) | Self::LastUpdateTime(dst, _) | Self::Probe(dst, _, _) => {
                f(*dst);
            }
        }
    }

    pub fn for_each_src(&self, mut f: impl FnMut(VariableKey)) {
        match self {
            Self::Unary(_, _, src)
            | Self::Resize(_, _, src)
            | Self::BinaryImm(_, _, src, _)
            | Self::SliceImm(_, src, _)
            | Self::ShiftImm(_, _, src, _)
            | Self::ProbeSlice(_, _, src)
            | Self::Drive(_, src, _) => f(*src),
            Self::Binary(_, _, src1, src2) | Self::Slice(_, src1, src2) => {
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
            Self::DriveSlice(_, src, partial) => {
                f(*src);
                f(*partial);
            }
            Self::Select(_, cond, truthy, falsy) => {
                f(*cond);
                f(*truthy);
                f(*falsy);
            }
            Self::Constant(_, _) | Self::LastUpdateTime(_, _) | Self::Probe(_, _, _) => {}
        }
    }
    fn map_src_vars(&mut self, mut f: impl FnMut(VariableKey) -> VariableKey) {
        match self {
            Self::Unary(_, _, src)
            | Self::Resize(_, _, src)
            | Self::BinaryImm(_, _, src, _)
            | Self::SliceImm(_, src, _)
            | Self::ShiftImm(_, _, src, _)
            | Self::ProbeSlice(_, _, src)
            | Self::Drive(_, src, _) => *src = f(*src),
            Self::Binary(_, _, src1, src2) | Self::Slice(_, src1, src2) => {
                *src1 = f(*src1);
                *src2 = f(*src2);
            }
            Self::Phi(_, srcs) => {
                for (_, s) in srcs {
                    *s = f(*s);
                }
            }
            Self::Intrinsic(_, _, srcs) => {
                for s in srcs {
                    *s = f(*s);
                }
            }
            Self::DriveSlice(_, src, partial) => {
                *src = f(*src);
                *partial = f(*partial);
            }
            Self::Select(_, cond, truthy, falsy) => {
                *cond = f(*cond);
                *truthy = f(*truthy);
                *falsy = f(*falsy);
            }
            Self::Constant(_, _) | Self::LastUpdateTime(_, _) | Self::Probe(_, _, _) => {}
        }
    }

    fn map_signals(&mut self, mut f: impl FnMut(SignalKey) -> SignalKey) {
        match self {
            Instruction::Probe(_, s, _)
            | Instruction::ProbeSlice(_, s, _)
            | Instruction::Drive(s, _, _)
            | Instruction::DriveSlice(s, _, _)
            | Instruction::LastUpdateTime(_, s) => *s = f(*s),
            Instruction::Constant(..)
            | Instruction::Unary(..)
            | Instruction::Resize(..)
            | Instruction::Binary(..)
            | Instruction::BinaryImm(..)
            | Instruction::Slice(..)
            | Instruction::SliceImm(..)
            | Instruction::ShiftImm(..)
            | Instruction::Select(..)
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
            | Instruction::BinaryImm(..)
            | Instruction::Slice(..)
            | Instruction::SliceImm(..)
            | Instruction::ShiftImm(..)
            | Instruction::Select(..)
            | Instruction::Intrinsic(..)
            | Instruction::Probe(..)
            | Instruction::ProbeSlice(..)
            | Instruction::Drive(..)
            | Instruction::DriveSlice(..)
            | Instruction::LastUpdateTime(..) => {}
            Instruction::Phi(_, items) => items.iter_mut().for_each(|(bb, _)| {
                *bb = f(*bb);
            }),
        }
    }

    pub fn for_each_bits(&self, mut f: impl FnMut(&Bits)) {
        match self {
            Instruction::Constant(_, bits) | Instruction::BinaryImm(_, _, _, bits) => f(bits),
            Instruction::Unary(..)
            | Instruction::Resize(..)
            | Instruction::Binary(..)
            | Instruction::Slice(..)
            | Instruction::SliceImm(..)
            | Instruction::ShiftImm(..)
            | Instruction::Select(..)
            | Instruction::Intrinsic(..)
            | Instruction::Phi(..)
            | Instruction::Probe(..)
            | Instruction::ProbeSlice(..)
            | Instruction::Drive(..)
            | Instruction::DriveSlice(..)
            | Instruction::LastUpdateTime(..) => {}
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
            O::TvToFv => src.clone(),
            O::FvToTv => src.special_to_zero(),
            O::LeadingZeros => Bits::from_u64(INTEGER_VSIZE, src.leading_zeroes().into()),
        }
    }

    fn simplify(self, src_size: VectorSize, src_mode: LogicMode) -> UnaryOpSimplification {
        use UnaryOp as O;
        match self {
            O::ReduceOr | O::ReduceAnd | O::ReduceXor
                if src_mode == LogicMode::TwoValue && src_size == SCALAR_VSIZE =>
            {
                UnaryOpSimplification::Source
            }
            O::Neg
            | O::ReduceOr
            | O::ReduceAnd
            | O::ReduceXor
            | O::LeadingZeros
            | O::TvToFv
            | O::FvToTv => UnaryOpSimplification::Keep,
        }
    }

    fn output_size(self, size: VectorSize) -> VectorSize {
        match self {
            UnaryOp::Neg | UnaryOp::TvToFv | UnaryOp::FvToTv => size,
            UnaryOp::ReduceOr | UnaryOp::ReduceAnd | UnaryOp::ReduceXor => SCALAR_VSIZE,
            UnaryOp::LeadingZeros => VSIZE_32,
        }
    }

    pub fn output_mode(self, src: LogicMode) -> Option<LogicMode> {
        use UnaryOp as O;
        match self {
            O::Neg | O::ReduceOr | O::ReduceAnd | O::ReduceXor => Some(src),

            O::TvToFv if src == LogicMode::TwoValue => Some(LogicMode::FourValue),
            O::FvToTv if src == LogicMode::FourValue => Some(LogicMode::TwoValue),
            O::TvToFv | O::FvToTv => None,

            O::LeadingZeros => Some(LogicMode::TwoValue),
        }
    }

    fn supports_tv_pushdown(&self) -> bool {
        use UnaryOp as O;
        match self {
            O::Neg | O::ReduceOr | O::ReduceAnd | O::ReduceXor | O::LeadingZeros => true,
            O::TvToFv | O::FvToTv => false,
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

    fn simplify(
        self,
        dst_size: VectorSize,
        src_size: VectorSize,
        mode: LogicMode,
    ) -> ResizeOpSimplification {
        _ = mode;
        if dst_size == src_size {
            ResizeOpSimplification::Source
        } else {
            ResizeOpSimplification::Keep
        }
    }

    fn output_mode(self, src: LogicMode) -> LogicMode {
        src
    }
}

impl BinaryOp {
    fn evaluate(self, lhs: &Bits, rhs: &Bits, dst_size: VectorSize) -> Bits {
        use BinaryOp as O;
        match self {
            O::And => Bits::bitwise_and(lhs, rhs),
            O::Or => Bits::bitwise_or(lhs, rhs),
            O::Xor => Bits::bitwise_xor(lhs, rhs),
            O::AndNot => Bits::bitwise_andnot(lhs, rhs),
            O::OrNot => Bits::bitwise_ornot(lhs, rhs),
            O::Xnor => Bits::bitwise_xnor(lhs, rhs),
            O::Add => Bits::add(lhs, rhs),
            O::Sub => Bits::subtract(lhs, rhs),
            O::Power => Bits::power(lhs, rhs),
            O::Multiply => Bits::multiply(lhs, rhs),
            O::DivideX => Bits::divide_x(lhs, rhs),
            O::Divide0 => Bits::divide_0(lhs, rhs),
            O::ModulusX => Bits::remainder_x(lhs, rhs),
            O::Modulus0 => Bits::remainder_0(lhs, rhs),

            O::UnsignedLessEqual => Bits::from(Bits::is_unsigned_leq(lhs, rhs)),
            O::CaseEquality => Bits::from(lhs == rhs),
            O::LogicalShiftLeft => match rhs.extract_exact_u32() {
                None => Bits::new_unknown(dst_size),
                Some(amount) => lhs.logical_shift_left(amount),
            },
            O::LogicalShiftRight => match rhs.extract_exact_u32() {
                None => Bits::new_unknown(dst_size),
                Some(amount) => lhs.logical_shift_right(amount),
            },
            O::ArithmeticShiftRight => match rhs.extract_exact_u32() {
                None => Bits::new_unknown(dst_size),
                Some(amount) => lhs.arithmetic_shift_right(amount),
            },
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
        // matches!(self, Self::Divide | Self::Modulus)
        false
    }

    pub fn output_size(self, lhs: VectorSize, rhs: VectorSize) -> Option<VectorSize> {
        use BinaryOp as O;
        match self {
            O::And
            | O::Or
            | O::Xor
            | O::AndNot
            | O::OrNot
            | O::Xnor
            | O::Add
            | O::Sub
            | O::Power
            | O::Multiply
            | O::DivideX
            | O::Divide0
            | O::ModulusX
            | O::Modulus0
            | O::CopyX
            | O::CopyZ
            | O::Min
            | O::Max => {
                if lhs != rhs {
                    return None;
                }
                Some(lhs)
            }
            O::UnsignedLessEqual | O::CaseEquality => {
                if lhs != rhs {
                    return None;
                }
                Some(SCALAR_VSIZE)
            }
            O::LogicalShiftLeft | O::LogicalShiftRight | O::ArithmeticShiftRight => {
                if rhs != INTEGER_VSIZE {
                    return None;
                }
                Some(lhs)
            }
            O::Concat => lhs.checked_add(rhs.get()),
            O::Posedge | O::Negedge => {
                if lhs != SCALAR_VSIZE || rhs != SCALAR_VSIZE {
                    return None;
                }
                Some(SCALAR_VSIZE)
            }
        }
    }

    pub fn output_mode(self, lhs: LogicMode, rhs: LogicMode) -> BinaryOutputMode {
        use BinaryOp as O;
        match self {
            O::And
            | O::Or
            | O::Xor
            | O::AndNot
            | O::OrNot
            | O::Xnor
            | O::Add
            | O::Sub
            | O::Power
            | O::Divide0
            | O::Modulus0
            | O::Multiply
            | O::Concat
            | O::CopyX
            | O::CopyZ
            | O::Min
            | O::Max
            | O::UnsignedLessEqual
            | O::LogicalShiftLeft
            | O::LogicalShiftRight
            | O::ArithmeticShiftRight => {
                let dst_mode = lhs.max(rhs);
                BinaryOutputMode {
                    dst: dst_mode,
                    lhs: dst_mode,
                    rhs: dst_mode,
                }
            }
            O::DivideX | O::ModulusX => {
                let convert_mode = lhs.max(rhs);
                BinaryOutputMode {
                    dst: LogicMode::FourValue,
                    lhs: convert_mode,
                    rhs: convert_mode,
                }
            }
            O::CaseEquality | O::Posedge | O::Negedge => {
                let convert_mode = lhs.max(rhs);
                BinaryOutputMode {
                    dst: LogicMode::TwoValue,
                    lhs: convert_mode,
                    rhs: convert_mode,
                }
            }
        }
    }

    fn tv_pushdown_variant(self) -> Option<TvPushdownVariant> {
        match self {
            Self::And
            | Self::Or
            | Self::Xor
            | Self::AndNot
            | Self::OrNot
            | Self::Xnor
            | Self::Add
            | Self::Sub
            | Self::Power
            | Self::Multiply
            | Self::UnsignedLessEqual
            | Self::Min
            | Self::Max
            | Self::Concat
            | Self::Divide0
            | Self::Modulus0
            | Self::CopyX
            | Self::CopyZ => Some(TvPushdownVariant::CastOutput),

            Self::CaseEquality | Self::Posedge | Self::Negedge => {
                Some(TvPushdownVariant::KeepOutput)
            }

            Self::DivideX | Self::ModulusX => None,
            Self::LogicalShiftLeft | Self::LogicalShiftRight | Self::ArithmeticShiftRight => None,
        }
    }
}

enum TvPushdownVariant {
    KeepOutput,
    CastOutput,
}

pub struct BinaryOutputMode {
    pub dst: LogicMode,
    pub lhs: LogicMode,
    pub rhs: LogicMode,
}

pub struct BinaryImmOutputMode {
    pub dst: LogicMode,
    pub src: LogicMode,
}

impl BinaryImmOp {
    fn evaluate(self, src: &Bits, imm: &Bits) -> Bits {
        use BinaryImmOp as O;
        match self {
            O::And => Bits::bitwise_and(src, imm),
            O::Or => Bits::bitwise_or(src, imm),
            O::Xor => Bits::bitwise_xor(src, imm),
            O::Add => Bits::add(src, imm),
            O::Sub => Bits::subtract(src, imm),
            O::Power => Bits::power(src, imm),
            O::Multiply => Bits::multiply(src, imm),
            O::Divide => Bits::divide_0(src, imm),
            O::Modulus => Bits::remainder_0(src, imm),

            O::RevSub => Bits::subtract(imm, src),
            O::RevPower => Bits::power(imm, src),
            O::RevDivideX => Bits::divide_x(imm, src),
            O::RevDivide0 => Bits::divide_0(imm, src),
            O::RevModulusX => Bits::remainder_x(imm, src),
            O::RevModulus0 => Bits::remainder_0(imm, src),

            O::UnsignedLessEqual => Bits::from(Bits::is_unsigned_leq(src, imm)),
            O::UnsignedGreaterEqual => Bits::from(Bits::is_unsigned_leq(imm, src)),
            O::BitwiseCaseEquality => Bits::bitwise_case_equality(src, imm),
            O::CaseEquality => Bits::from(src == imm),
            O::ConcatLeft => Bits::concatenate(imm, src),
            O::ConcatRight => Bits::concatenate(src, imm),

            O::Min => Bits::min(src, imm),
            O::Max => Bits::max(src, imm),
        }
    }

    pub fn always_outputs_bool(&self) -> bool {
        matches!(self, Self::CaseEquality)
    }

    pub fn always_outputs_four_value(&self) -> bool {
        false
        // matches!(self, Self::RevDivide | Self::RevModulus)
    }

    fn simplify(self, dst: VariableKey, src: VariableKey, imm: &Bits) -> BinaryImmOpSimplification {
        use BinaryImmOp as O;
        use BinaryImmOpSimplification as S;
        match self {
            O::And => {
                let num_special = imm.count_special();
                if num_special == imm.size().get() {
                    return S::Constant(Bits::new_unknown(imm.size()));
                }

                let num_ones = imm.count_ones();
                if num_ones == imm.size().get() && src.mode() == LogicMode::TwoValue {
                    return S::Source;
                } else if num_special == 0 && num_ones == 0 {
                    return S::Immediate;
                }

                S::Keep
            }
            O::Or => {
                let num_special = imm.count_special();
                if num_special == imm.size().get() {
                    return S::Constant(Bits::new_unknown(imm.size()));
                }

                let num_ones = imm.count_ones();
                if num_ones == imm.size().get() {
                    return S::Immediate;
                } else if num_special == 0 && num_ones == 0 && src.mode() == LogicMode::TwoValue {
                    return S::Source;
                }

                S::Keep
            }
            O::Xor => {
                let num_special = imm.count_special();
                if num_special == imm.size().get() {
                    return S::Constant(Bits::new_unknown(imm.size()));
                }

                let num_ones = imm.count_ones();
                if num_ones == imm.size().get() {
                    return S::Instruction(Instruction::Unary(dst, UnaryOp::Neg, src));
                } else if num_special == 0 && num_ones == 0 && src.mode() == LogicMode::TwoValue {
                    return S::Source;
                }

                S::Keep
            }

            O::Add
            | O::Sub
            | O::Power
            | O::Multiply
            | O::Divide
            | O::Modulus
            | O::RevSub
            | O::RevPower
            | O::RevDivideX
            | O::RevModulusX
            | O::RevDivide0
            | O::RevModulus0
            | O::UnsignedLessEqual
            | O::UnsignedGreaterEqual
            | O::Min
            | O::Max
                if imm.contains_special() =>
            {
                S::Constant(Bits::new_unknown(imm.size()))
            }

            O::Add | O::Sub => {
                if src.mode() == LogicMode::FourValue {
                    S::Keep
                } else if imm.eq_zero() && src.mode() == LogicMode::TwoValue {
                    S::Source
                } else {
                    S::Keep
                }
            }
            O::Power => {
                if src.mode() == LogicMode::FourValue {
                    S::Keep
                } else if imm.eq_one() {
                    S::Source
                } else {
                    S::Keep
                }
            }
            O::Multiply => {
                if src.mode() == LogicMode::FourValue {
                    S::Keep
                } else if imm.eq_zero() {
                    S::Immediate
                } else if imm.eq_one() {
                    S::Source
                } else {
                    S::Keep
                }
            }
            O::Divide => {
                if src.mode() == LogicMode::FourValue {
                    S::Keep
                } else if imm.eq_zero() {
                    S::Constant(Bits::new_ones(imm.size()))
                } else if imm.eq_one() {
                    S::Source
                } else {
                    S::Keep
                }
            }
            O::Modulus => {
                if src.mode() == LogicMode::TwoValue && (imm.eq_zero() || imm.eq_one()) {
                    S::Constant(Bits::new_zeroed(imm.size()))
                } else {
                    S::Keep
                }
            }
            O::RevSub => S::Keep,
            O::RevPower => {
                if src.mode() == LogicMode::TwoValue && imm.eq_one() {
                    S::Immediate
                } else {
                    S::Keep
                }
            }
            O::RevDivideX | O::RevDivide0 | O::RevModulusX | O::RevModulus0 => S::Keep,
            O::UnsignedLessEqual => S::Keep,
            O::UnsignedGreaterEqual => S::Keep,

            O::ConcatRight => S::Keep,
            O::ConcatLeft => {
                if imm.eq_zero() {
                    S::Instruction(Instruction::Resize(dst, ResizeOp::ZeroExtend, src))
                } else {
                    S::Keep
                }
            }
            O::Min => {
                if src.mode() == LogicMode::TwoValue {
                    S::Keep
                } else if imm.eq_zero() {
                    S::Immediate
                } else if imm.count_ones() == imm.size().get() {
                    S::Source
                } else {
                    S::Keep
                }
            }
            O::Max => {
                if src.mode() == LogicMode::TwoValue {
                    S::Keep
                } else if imm.count_ones() == imm.size().get() {
                    S::Immediate
                } else if imm.eq_zero() {
                    S::Source
                } else {
                    S::Keep
                }
            }
            O::CaseEquality if src.mode().is_two_value() && src.is_scalar() => {
                if imm.eq_zero() {
                    S::Instruction(Instruction::Unary(dst, UnaryOp::Neg, src))
                } else {
                    S::Source
                }
            }
            O::CaseEquality => S::Keep,
            O::BitwiseCaseEquality if src.mode().is_two_value() => S::Instruction(
                Instruction::BinaryImm(dst, O::Xor, src, imm.bitwise_negate()),
            ),
            O::BitwiseCaseEquality => S::Keep,
        }
    }

    fn output_size(&self, src_size: VectorSize, imm_size: VectorSize) -> Option<VectorSize> {
        use BinaryImmOp as O;
        match self {
            O::And
            | O::Or
            | O::Xor
            | O::Add
            | O::Sub
            | O::Power
            | O::Multiply
            | O::Divide
            | O::Modulus
            | O::RevSub
            | O::RevPower
            | O::RevDivideX
            | O::RevDivide0
            | O::RevModulusX
            | O::RevModulus0
            | O::Min
            | O::Max
            | O::BitwiseCaseEquality => {
                if src_size != imm_size {
                    return None;
                }
                Some(src_size)
            }
            O::UnsignedLessEqual | O::UnsignedGreaterEqual | O::CaseEquality => {
                if src_size != imm_size {
                    return None;
                }
                Some(SCALAR_VSIZE)
            }
            O::ConcatLeft | O::ConcatRight => src_size.checked_add(imm_size.get()),
        }
    }

    pub fn output_mode(&self, src: LogicMode, imm: LogicMode) -> BinaryImmOutputMode {
        use BinaryImmOp as O;
        match self {
            O::And
            | O::Or
            | O::Xor
            | O::Add
            | O::Sub
            | O::Power
            | O::RevSub
            | O::RevPower
            | O::RevDivide0
            | O::RevModulus0
            | O::Multiply
            | O::ConcatLeft
            | O::ConcatRight
            | O::Min
            | O::Max
            | O::UnsignedLessEqual
            | O::UnsignedGreaterEqual
            | O::Divide
            | O::Modulus => {
                let dst_mode = src.max(imm);
                BinaryImmOutputMode {
                    dst: dst_mode,
                    src: dst_mode,
                }
            }
            O::RevDivideX | O::RevModulusX => {
                let convert = src.max(imm);
                BinaryImmOutputMode {
                    dst: LogicMode::TwoValue,
                    src: convert,
                }
            }
            O::CaseEquality | O::BitwiseCaseEquality => {
                let convert = src.max(imm);
                BinaryImmOutputMode {
                    dst: LogicMode::TwoValue,
                    src: convert,
                }
            }
        }
    }

    fn tv_pushdown_variant(self) -> Option<TvPushdownVariant> {
        match self {
            Self::And
            | Self::Or
            | Self::Xor
            | Self::Add
            | Self::Sub
            | Self::Power
            | Self::Multiply
            | Self::Divide
            | Self::Modulus
            | Self::RevSub
            | Self::ConcatRight
            | Self::ConcatLeft
            | Self::RevDivide0
            | Self::RevModulus0
            | Self::RevPower
            | Self::UnsignedLessEqual
            | Self::UnsignedGreaterEqual
            | Self::Min
            | Self::Max => Some(TvPushdownVariant::CastOutput),

            Self::BitwiseCaseEquality | Self::CaseEquality => Some(TvPushdownVariant::KeepOutput),
            Self::RevDivideX | Self::RevModulusX => None,
        }
    }
}

impl ShiftImmOp {
    fn evaluate(self, src: &Bits, amount: u32) -> Bits {
        match self {
            ShiftImmOp::LogicalShiftLeft => src.logical_shift_left(amount),
            ShiftImmOp::LogicalShiftRight => src.logical_shift_right(amount),
            ShiftImmOp::ArithmeticShiftRight => src.arithmetic_shift_right(amount),
        }
    }

    fn simplify(self, size: VectorSize, amount: u32) -> ShiftImmOpSimplification {
        use ShiftImmOp as O;
        use ShiftImmOpSimplification as S;
        match self {
            O::LogicalShiftLeft | O::LogicalShiftRight | O::ArithmeticShiftRight if amount == 0 => {
                S::Source
            }
            O::LogicalShiftLeft | O::LogicalShiftRight if amount >= size.get() => {
                S::Constant(Bits::new_zeroed(size))
            }
            O::LogicalShiftLeft | O::LogicalShiftRight | O::ArithmeticShiftRight => S::Keep,
        }
    }
}

enum ResizeOpSimplification {
    Keep,
    Source,
}

enum UnaryOpSimplification {
    Keep,
    Source,
}

enum BinaryImmOpSimplification {
    Keep,
    Source,
    Immediate,
    Constant(Bits),
    Instruction(Instruction),
}

enum ShiftImmOpSimplification {
    Keep,
    Source,
    Constant(Bits),
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LogicMode {
    #[default]
    TwoValue,
    FourValue,
}
impl LogicMode {
    pub fn other(self) -> LogicMode {
        match self {
            Self::TwoValue => Self::FourValue,
            Self::FourValue => Self::TwoValue,
        }
    }

    #[inline(always)]
    pub fn is_two_value(self) -> bool {
        matches!(self, Self::TwoValue)
    }

    #[inline(always)]
    pub fn is_four_value(self) -> bool {
        matches!(self, Self::FourValue)
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
impl From<LogicMode> for Mode {
    fn from(value: LogicMode) -> Self {
        match value {
            LogicMode::TwoValue => Self::TwoValue,
            LogicMode::FourValue => Self::FourValue,
        }
    }
}

#[derive(Default, Clone)]
pub struct GlobalContext {
    pub global_seed: Option<SignalKey>,
    pub processes: SlotMap<ProcessKey, Process>,
    pub bbs: SlotMap<BasicBlockKey, BasicBlock>,
    pub signals: SlotMap<SignalKey, Signal>,
    pub vars: VariableMap,
}

macro_rules! define_process_kinds {
    ($($kind:ident => $mnem:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy)]
        pub enum ProcessKind {
            $($kind,)+
        }

        impl ProcessKind {
            pub const NUM_KINDS: usize = 0 $( + { _ = Self::$kind; 1 } )+;
            pub const KINDS: [Self; Self::NUM_KINDS] = [$(Self::$kind),+];
            pub fn into_static_str(self) -> &'static str {
                match self {
                    $(Self::$kind => $mnem,)+
                }
            }
        }
    };
}

define_process_kinds! {
    Assign => "assign",
    Always => "always",
    Initial => "initial",
    Fuse => "fuse",
    Specify => "specify",
    NonBlockingAssignment => "nba",
    Udp => "udp",
    Port => "port",
    Fork => "fork",
    Other => "other",
}

#[derive(Debug, Clone)]
pub struct Process {
    pub kind: ProcessKind,

    // @Performance: Use UnitVec here.
    pub regions: Vec<TemporalRegionKey>,

    pub origin: TokenRange,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalSlice {
    width: VectorSize,
    lsb: NonMaxU32,
}

impl fmt::Debug for SignalSlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignalSlice({}:{})", self.msb(), self.lsb())
    }
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

    #[must_use]
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

    pub fn overlaps(self, other: Self) -> bool {
        self.lsb() <= other.msb() && other.lsb() <= self.msb()
    }
}

impl Time {
    pub fn from_fs(fs: u64) -> Self {
        Self(fs)
    }
    pub fn from_u32_ps(ps: u32) -> Self {
        Self(ps as u64 * 1_000)
    }
    pub fn from_u32_ns(ns: u32) -> Self {
        Self(ns as u64 * 1_000_000)
    }
    pub fn from_u32_us(us: u32) -> Self {
        Self(us as u64 * 1_000_000_000)
    }

    pub fn try_from_u64_ps(s: u64) -> Option<Self> {
        pub const PS_TO_FS: u64 = 1_000;
        s.checked_mul(PS_TO_FS).map(Self)
    }
    pub fn try_from_u64_ns(s: u64) -> Option<Self> {
        pub const NS_TO_FS: u64 = 1_000_000;
        s.checked_mul(NS_TO_FS).map(Self)
    }
    pub fn try_from_u64_us(s: u64) -> Option<Self> {
        pub const US_TO_FS: u64 = 1_000_000_000;
        s.checked_mul(US_TO_FS).map(Self)
    }
    pub fn try_from_u64_ms(s: u64) -> Option<Self> {
        pub const MS_TO_FS: u64 = 1_000_000_000_000;
        s.checked_mul(MS_TO_FS).map(Self)
    }
    pub fn try_from_u64_s(s: u64) -> Option<Self> {
        pub const S_TO_FS: u64 = 1_000_000_000_000_000;
        s.checked_mul(S_TO_FS).map(Self)
    }
}
