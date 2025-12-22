use vogls_ir::{BinaryOp, Bits, IntrinsicOp, Time, TypeKey, UnaryOp, VectorSize};

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
    VariableBits(StackRef, VectorSize),
    VariableDecimal(StackRef),
}

#[derive(Debug)]
pub enum VmInstruction {
    ConstantBit(StackRef, Bits),
    ConstantDecimal(StackRef, i64),

    Unary(StackRef, UnaryOp, StackRef),
    Binary(StackRef, BinaryOp, StackRef, StackRef),

    Cast(StackRef, TypeKey, StackRef, TypeKey),
    Move(StackRef, StackRef, TypeKey),

    Intrinsic(IntrinsicOp, Vec<VmIntrinsicArg>),

    Probe(StackRef, VmSignalKey),
    Drive(VmSignalKey, StackRef, u8, Option<(StackRef, VectorSize)>),

    ArrProbe(StackRef, VmSignalKey, StackRef),
    ArrDrive(
        VmSignalKey,
        StackRef,
        StackRef,
        u8,
        Option<(StackRef, VectorSize)>,
    ),

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
    pub decimal_stack_size: usize,
    pub instructions: Vec<VmInstruction>,
}
