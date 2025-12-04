use vogls_ir::{BinaryOp, Bits, IntrinsicOp, Time, Type, UnaryOp, VectorSize};

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
    VariableBits(StackRef, VectorSize),
    VariableDecimal(StackRef),
}

#[derive(Debug)]
pub enum VmInstruction {
    ConstantBit(StackRef, Bits),
    ConstantDecimal(StackRef, i64),

    Unary(StackRef, UnaryOp, StackRef),
    Binary(StackRef, BinaryOp, StackRef, StackRef),

    Cast(StackRef, Type, StackRef, Type),

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
    pub bit_stack_size: usize,
    pub decimal_stack_size: usize,
    pub instructions: Vec<VmInstruction>,
}
