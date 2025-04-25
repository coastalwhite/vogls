use vogls_ir::{BinaryOp, IntrinsicOp, Time, UnaryOp, Value};

mod format;
mod lower;

pub use lower::lower_process_to_vm;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VmSignalKey(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct StackRef {
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug)]
pub enum VmIntrinsicArg {
    StringLiteral(String),
    Variable(StackRef),
}

#[derive(Debug)]
pub enum VmInstruction {
    Constant(StackRef, Value),

    Unary(StackRef, UnaryOp, StackRef),
    Binary(StackRef, BinaryOp, StackRef, StackRef),
    Intrinsic(IntrinsicOp, Vec<VmIntrinsicArg>),

    Probe(StackRef, VmSignalKey),
    Drive(VmSignalKey, StackRef),

    Wait(Time),
    Watch(Vec<VmSignalKey>),

    Jump(usize),
    Branch(StackRef, usize, usize),
    Halt,
}

#[derive(Debug)]
pub struct VmProcess {
    pub stack_size: usize,
    pub instructions: Vec<VmInstruction>,
}
