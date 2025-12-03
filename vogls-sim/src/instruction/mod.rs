use vogls_ir::{BinaryOp, IntrinsicOp, Time, UnaryOp};

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
    VariableBit(StackRef),
    VariableDecimal(StackRef),
}

#[derive(Debug)]
pub enum VmInstruction {
    ConstantBit(StackRef, bool),
    UnaryBit(StackRef, UnaryOp, StackRef),
    BinaryBit(StackRef, BinaryOp, StackRef, StackRef),

    ConstantDecimal(StackRef, i64),
    UnaryDecimal(StackRef, UnaryOp, StackRef),
    BinaryDecimal(StackRef, BinaryOp, StackRef, StackRef),

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
