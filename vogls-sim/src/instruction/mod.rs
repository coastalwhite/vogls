use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{Bits, ResizeOp, Time, UnaryOp, VectorSize};

mod format;
mod lower;

pub use lower::lower_process_to_vm;

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
    Constant(StackOffset, Bits),

    TvUnary(StackOffset, UnaryOp, VectorSize, StackOffset),
    TvResize(StackOffset, ResizeOp, VectorSize, VectorSize, StackOffset),
    TvBinaryArithmetic(
        StackOffset,
        BinaryArithmeticOp,
        VectorSize,
        StackOffset,
        StackOffset,
    ),
    TvBinaryComparison(
        StackOffset,
        BinaryComparisonOp,
        VectorSize,
        StackOffset,
        StackOffset,
    ),
    TvShift(StackOffset, ShiftOp, VectorSize, StackOffset, StackOffset),
    TvSelectBit(StackOffset, VectorSize, StackOffset, StackOffset),
    TvConcat(
        StackOffset,
        VectorSize,
        StackOffset,
        VectorSize,
        StackOffset,
    ),

    FvUnary(StackOffset, UnaryOp, VectorSize, StackOffset),
    FvResize(StackOffset, ResizeOp, VectorSize, VectorSize, StackOffset),
    FvBinaryArithmetic(
        StackOffset,
        BinaryArithmeticOp,
        VectorSize,
        StackOffset,
        StackOffset,
    ),
    FvBinaryComparison(
        StackOffset,
        BinaryComparisonOp,
        VectorSize,
        StackOffset,
        StackOffset,
    ),
    FvShift(StackOffset, ShiftOp, VectorSize, StackOffset, StackOffset),
    FvSelectBit(StackOffset, VectorSize, StackOffset, StackOffset),
    FvConcat(
        StackOffset,
        VectorSize,
        StackOffset,
        VectorSize,
        StackOffset,
    ),

    TvToFv(StackOffset, StackOffset, VectorSize),
    FvToTv(StackOffset, StackOffset, VectorSize),

    Intrinsic(
        StackOffset,
        Box<VmIntrinsicOp>,
        Box<[(StackOffset, VectorSize)]>,
    ),

    Probe(StackOffset, VmSignalKey),
    Drive(
        VmSignalKey,
        StackOffset,
        u8,
        Option<(StackOffset, VectorSize)>,
    ),

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
