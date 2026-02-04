use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{Bits, INTEGER_VSIZE, LogicMode, ResizeOp, SCALAR_VSIZE, Time, UnaryOp, VectorSize};

mod format;
mod lower;

pub use lower::{StackBuilder, lower_process_to_vm};

use crate::{Stack, VcdScope};

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
    pub fn to_scalar_ref(self) -> StackRef {
        self.to_ref(SCALAR_VSIZE)
    }
    fn to_32bit_ref(self) -> StackRef {
        self.to_ref(INTEGER_VSIZE)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryArithmeticOp {
    And,
    Or,
    Xor,
    Add,
    Sub,
    Power,
    Multiply,
    Divide,
    Modulus,
    CopyX,
    CopyZ,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryComparisonOp {
    UnsignedLessEqual,
    CaseEquality,
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
    Random,
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
    FvShift(StackRef, ShiftOp, StackOffset, StackOffset),
    FvSelectBit(StackOffset, StackRef, StackOffset),
    FvConcat(StackOffset, StackRef, StackRef),

    TvToFv(StackRef, StackOffset),
    FvToTv(StackRef, StackOffset),

    Intrinsic(
        StackOffset,
        Box<VmIntrinsicOp>,
        Box<[(StackRef, LogicMode)]>,
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
impl VmInstruction {
    pub fn itrace(&self, stack: &Stack, signals: &[StackRef], logic_mode: LogicMode) {
        use VmInstruction as I;
        eprint!("{self}");
        let items: &[(&'static str, bool, StackRef)] = match self {
            I::Constant(dst, src) => &[("dst", src.contains_special(), dst.to_ref(src.size()))],
            I::TvUnary(dst, op, src) => match op {
                UnaryOp::Neg => &[("dst", false, dst.to_ref(src.size)), ("src", false, *src)],
                UnaryOp::ReduceOr | UnaryOp::ReduceAnd | UnaryOp::ReduceXor => {
                    &[("dst", false, dst.to_scalar_ref()), ("src", false, *src)]
                }
            },
            I::TvResize(dst, _, src) => &[("dst", false, *dst), ("src", false, *src)],
            I::TvBinaryArithmetic(dst, _, lhs, rhs) => &[
                ("dst", false, *dst),
                ("lhs", false, lhs.to_ref(dst.size)),
                ("rhs", false, rhs.to_ref(dst.size)),
            ],
            I::TvBinaryComparison(dst, _, lhs, rhs) => &[
                ("dst", false, dst.to_scalar_ref()),
                ("lhs", false, *lhs),
                ("rhs", false, rhs.to_ref(lhs.size)),
            ],
            I::TvShift(dst, _, src, shift) => &[
                ("dst", false, *dst),
                ("src", false, src.to_ref(dst.size)),
                ("shift", false, shift.to_32bit_ref()),
            ],
            I::TvSelectBit(dst, src, idx) => &[
                ("dst", false, dst.to_scalar_ref()),
                ("src", false, *src),
                ("idx", false, idx.to_32bit_ref()),
            ],
            I::TvConcat(dst, lhs, rhs) => &[
                (
                    "dst",
                    false,
                    dst.to_ref(VectorSize::new(lhs.size.get() + rhs.size.get()).unwrap()),
                ),
                ("lhs", false, *lhs),
                ("rhs", false, *rhs),
            ],
            I::FvUnary(dst, op, src) => match op {
                UnaryOp::Neg => &[("dst", false, dst.to_ref(src.size)), ("src", true, *src)],
                UnaryOp::ReduceOr | UnaryOp::ReduceAnd | UnaryOp::ReduceXor => {
                    &[("dst", true, dst.to_scalar_ref()), ("src", true, *src)]
                }
            },
            I::FvResize(dst, _, src) => &[("dst", true, *dst), ("src", true, *src)],
            I::FvBinaryArithmetic(dst, _, lhs, rhs) => &[
                ("dst", true, *dst),
                ("lhs", true, lhs.to_ref(dst.size)),
                ("rhs", true, rhs.to_ref(dst.size)),
            ],
            I::FvBinaryComparison(dst, _, lhs, rhs) => &[
                ("dst", true, dst.to_scalar_ref()),
                ("lhs", true, *lhs),
                ("rhs", true, rhs.to_ref(lhs.size)),
            ],
            I::FvShift(dst, _, src, shift) => &[
                ("dst", true, *dst),
                ("src", true, src.to_ref(dst.size)),
                ("shift", true, shift.to_32bit_ref()),
            ],
            I::FvSelectBit(dst, src, idx) => &[
                ("dst", true, dst.to_scalar_ref()),
                ("src", true, *src),
                ("idx", true, idx.to_32bit_ref()),
            ],
            I::FvConcat(dst, lhs, rhs) => &[
                (
                    "dst",
                    true,
                    dst.to_ref(VectorSize::new(lhs.size.get() + rhs.size.get()).unwrap()),
                ),
                ("lhs", true, *lhs),
                ("rhs", true, *rhs),
            ],
            I::TvToFv(dst, src) => &[("dst", true, *dst), ("src", false, src.to_ref(dst.size))],
            I::FvToTv(dst, src) => &[("dst", false, *dst), ("src", true, src.to_ref(dst.size))],
            I::Intrinsic(_, _, _) => &[],
            I::Drive(dst, src, partial) => {
                eprint!(" ({})", signals[dst.0 as usize].offset);
                match partial {
                    None => &[
                        (
                            "dst",
                            logic_mode == LogicMode::FourValue,
                            signals[dst.0 as usize],
                        ),
                        ("src", logic_mode == LogicMode::FourValue, *src),
                    ],
                    Some(partial) => &[
                        (
                            "dst",
                            logic_mode == LogicMode::FourValue,
                            signals[dst.0 as usize],
                        ),
                        ("src", logic_mode == LogicMode::FourValue, *src),
                        (
                            "offset",
                            logic_mode == LogicMode::FourValue,
                            partial.to_32bit_ref(),
                        ),
                    ],
                }
            }
            I::Wait(_) => &[],
            I::WaitRegion(_) => &[],
            I::Watch(_) => &[],
            I::Jump(_) => &[],
            I::Branch(_, _, _) => &[],
            I::Halt => &[],
        };
        for (name, fv, stack_ref) in items {
            eprint!(
                " : {name} = {}",
                if *fv {
                    stack.load_fv_bits(*stack_ref)
                } else {
                    stack.load_tv_bits(*stack_ref)
                }
            );
        }
        eprintln!();
    }
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
            Self::Random => "random",
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
            Self::Power => "pow",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Modulus => "rem",
            Self::CopyX => "copyx",
            Self::CopyZ => "copyz",
        }
    }
}

impl BinaryComparisonOp {
    pub fn into_mnemonic(self) -> &'static str {
        match self {
            Self::UnsignedLessEqual => "leq",
            Self::CaseEquality => "ceq",
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
