use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{Bits, ResizeOp, Time, UnaryOp, VectorSize};

mod format;
mod lower;

pub use lower::{StackBuilder, lower_process_to_vm};

use crate::VcdScope;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VmSignalKey(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct StackOffset(pub usize);
#[derive(Debug, Clone, Copy)]
pub struct StackRef {
    pub offset: StackOffset,
    pub size: VectorSize,
}
impl StackRef {
    pub fn to_fv_size(mut self) -> StackRef {
        self.size = self.size.checked_mul(VectorSize::new(2).unwrap()).unwrap();
        self
    }
}

impl StackOffset {
    pub fn to_ref(self, size: VectorSize) -> StackRef {
        StackRef { offset: self, size }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryArithmeticOp {
    And,
    Or,
    Xor,
    Add,
    Sub,
    Multiply,
    Divide,
    Modulus,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryComparisonOp {
    UnsignedLessEqual,
}

#[derive(Debug, Clone, Copy)]
pub enum ShiftOp {
    LogicalLeft,
    LogicalRight,
    ArithmeticRight,
}

#[derive(Debug, Clone)]
pub enum VmIntrinsicOp {
    Time,
    Finish,
    Display(Box<DynFormatString>),
    AssertTv(Box<DynFormatString>),
    AssertFv(Box<DynFormatString>),
    VcdOpenFile(String),
    VcdAppendModule(VcdScope),
    VcdPause,
    VcdResume,
}

#[derive(Debug)]
pub enum VmInstruction {
    Constant(StackOffset, Bits),

    TvUnary(StackOffset, UnaryOp, StackRef),
    TvResize(StackRef, ResizeOp, StackRef),
    TvBinaryArithmetic(StackRef, BinaryArithmeticOp, StackOffset, StackOffset),
    TvBinaryComparison(StackOffset, BinaryComparisonOp, StackRef, StackOffset),
    TvShift(StackRef, ShiftOp, StackOffset, StackOffset),
    TvSelectBit(StackOffset, StackRef, StackOffset),
    TvConcat(StackOffset, StackRef, StackRef),

    FvUnary(StackOffset, UnaryOp, StackRef),
    FvResize(StackRef, ResizeOp, StackRef),
    FvBinaryArithmetic(StackRef, BinaryArithmeticOp, StackOffset, StackOffset),
    FvBinaryComparison(StackOffset, BinaryComparisonOp, StackRef, StackOffset),
    FvShift(StackOffset, ShiftOp, VectorSize, StackOffset, StackOffset),
    FvSelectBit(StackOffset, StackRef, StackOffset),
    FvConcat(
        StackOffset,
        VectorSize,
        StackOffset,
        VectorSize,
        StackOffset,
    ),

    TvToFv(StackRef, StackOffset),
    FvToTv(StackRef, StackOffset),

    Intrinsic(
        StackOffset,
        Box<VmIntrinsicOp>,
        Box<[(StackOffset, VectorSize)]>,
    ),

    Drive(VmSignalKey, StackRef, Option<StackOffset>),

    Wait(Time),
    WaitRegion(u8),
    Watch(Vec<VmSignalKey>),

    Jump(usize),
    /// (condition, true_offset, false_offset)
    Branch(StackOffset, usize, usize),
    Halt,
}

#[derive(Debug)]
pub struct VmProcess {
    pub instructions: Vec<VmInstruction>,
}

impl VmIntrinsicOp {
    pub fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Finish => "finish",
            Self::Display(_) => "display",
            Self::AssertTv(_) => "tv.assert",
            Self::AssertFv(_) => "fv.assert",
            Self::VcdOpenFile(_) => "vcd.open_file",
            Self::VcdAppendModule(_) => "vcd.append_scope",
            Self::VcdPause => "vcd.pause",
            Self::VcdResume => "vcd.resume",
        }
    }
}

impl BinaryArithmeticOp {
    pub fn into_mnemonic(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Modulus => "rem",
        }
    }
}

impl BinaryComparisonOp {
    pub fn into_mnemonic(self) -> &'static str {
        match self {
            Self::UnsignedLessEqual => "leq",
        }
    }
}

impl ShiftOp {
    pub fn into_mnemonic(self) -> &'static str {
        match self {
            Self::LogicalLeft => "lsl",
            Self::LogicalRight => "lsr",
            Self::ArithmeticRight => "asr",
        }
    }
}
