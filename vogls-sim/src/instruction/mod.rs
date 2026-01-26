use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{Bits, ResizeOp, Time, UnaryOp, VectorSize};

mod format;
mod lower;

pub use lower::lower_process_to_vm;

use crate::VcdScope;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VmSignalKey(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct StackRef {
    pub offset: usize,
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
    Assert(Box<DynFormatString>),
    VcdOpenFile(String),
    VcdAppendModule(VcdScope),
    VcdPause,
    VcdResume,
}

#[derive(Debug)]
pub enum VmInstruction {
    Constant(StackRef, Bits),

    TvUnary(StackRef, UnaryOp, VectorSize, StackRef),
    TvResize(StackRef, ResizeOp, VectorSize, VectorSize, StackRef),
    TvBinaryArithmetic(StackRef, BinaryArithmeticOp, VectorSize, StackRef, StackRef),
    TvBinaryComparison(StackRef, BinaryComparisonOp, VectorSize, StackRef, StackRef),
    TvShift(StackRef, ShiftOp, VectorSize, StackRef, StackRef),
    TvSelectBit(StackRef, VectorSize, StackRef, StackRef),
    TvConcat(StackRef, VectorSize, StackRef, VectorSize, StackRef),

    FvUnary(StackRef, UnaryOp, VectorSize, StackRef),
    FvResize(StackRef, ResizeOp, VectorSize, VectorSize, StackRef),
    FvBinaryArithmetic(StackRef, BinaryArithmeticOp, VectorSize, StackRef, StackRef),
    FvBinaryComparison(StackRef, BinaryComparisonOp, VectorSize, StackRef, StackRef),
    FvShift(StackRef, ShiftOp, VectorSize, StackRef, StackRef),
    FvSelectBit(StackRef, VectorSize, StackRef, StackRef),
    FvConcat(StackRef, VectorSize, StackRef, VectorSize, StackRef),

    TvToFv(StackRef, StackRef, VectorSize),
    FvToTv(StackRef, StackRef, VectorSize),

    Intrinsic(StackRef, Box<VmIntrinsicOp>, Box<[(StackRef, VectorSize)]>),

    Probe(StackRef, VmSignalKey),
    Drive(VmSignalKey, StackRef, u8, Option<(StackRef, VectorSize)>),

    Wait(Time),
    WaitRegion(u8),
    Watch(Vec<VmSignalKey>),

    Jump(usize),
    /// (condition, true_offset, false_offset)
    Branch(StackRef, usize, usize),
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
            Self::Assert(_) => "assert",
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
