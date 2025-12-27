use vogls_ir::{BinaryOp, Bits, IntrinsicOp, Time, UnaryOp, VectorSize};

mod format;
mod lower;

pub use lower::lower_process_to_vm;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VmSignalKey(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct StackRef {
    pub offset: usize,
}

#[derive(Debug)]
pub enum VmIntrinsicArg {
    StringLiteral(String),
    Variable(StackRef, VectorSize),
}

#[derive(Debug)]
pub enum VmInstruction {
    Constant(StackRef, Bits),

    Unary(StackRef, UnaryOp, StackRef),
    Binary(StackRef, BinaryOp, StackRef, StackRef),

    Move(StackRef, StackRef, VectorSize),

    Intrinsic(IntrinsicOp, Vec<VmIntrinsicArg>),

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
    pub bit_stack_size: usize,
    pub instructions: Vec<VmInstruction>,
}
