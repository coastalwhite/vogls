use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::vcd::VcdVariableKey;
use vogls_ir::{Bits, LogicMode, ReadMem, ResizeOp, SignalSlice, Time, UnaryOp, VectorSize};

mod format;
mod lower;

pub use lower::lower_process_to_vm;
use vogls_runtime::RtSignalKey;
use vogls_utils::SecondaryTable;

use vogls_codegen::{Heap, HeapOffset, HeapRef};
use vogls_vcd::VcdScopeItem;

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
    Min,
    Max,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryComparisonOp {
    UnsignedLessEqual,
    CaseEquality,
}

#[derive(Debug, Clone, Copy)]
pub enum EdgeOp {
    Posedge,
    Negedge,
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
    VcdAppendModule(
        Vec<VcdScopeItem>,
        SecondaryTable<RtSignalKey, Box<[(VcdVariableKey, Option<SignalSlice>)]>>,
    ),
    VcdPause,
    VcdResume,
    ReadMem(HeapRef, Box<ReadMem>),
}

#[derive(Clone, Debug)]
pub enum VmInstruction {
    Constant(HeapOffset, Bits),

    TvUnary(HeapOffset, UnaryOp, HeapRef),
    TvResize(HeapRef, ResizeOp, HeapRef),
    TvBinaryArithmetic(HeapRef, BinaryArithmeticOp, HeapOffset, HeapOffset),
    TvBinaryComparison(HeapOffset, BinaryComparisonOp, HeapRef, HeapOffset),
    TvEdge(HeapOffset, EdgeOp, HeapOffset, HeapOffset),
    TvShift(HeapRef, ShiftOp, HeapOffset, HeapOffset),
    TvSlice(HeapRef, HeapRef, HeapOffset),
    TvConcat(HeapOffset, HeapRef, HeapRef),

    FvUnary(HeapOffset, UnaryOp, HeapRef),
    FvResize(HeapRef, ResizeOp, HeapRef),
    FvBinaryArithmetic(HeapRef, BinaryArithmeticOp, HeapOffset, HeapOffset),
    FvBinaryComparison(HeapOffset, BinaryComparisonOp, HeapRef, HeapOffset),
    FvEdge(HeapOffset, EdgeOp, HeapOffset, HeapOffset),
    FvShift(HeapRef, ShiftOp, HeapOffset, HeapOffset),
    FvSlice(HeapRef, HeapRef, HeapOffset),
    FvConcat(HeapOffset, HeapRef, HeapRef),

    TvToFv(HeapRef, HeapOffset),
    FvToTv(HeapRef, HeapOffset),

    Intrinsic(HeapOffset, Box<VmIntrinsicOp>, Box<[(HeapRef, LogicMode)]>),

    LastUpdateTime(HeapOffset, RtSignalKey),
    Drive(RtSignalKey, HeapRef, Option<HeapOffset>),

    Wait(Time),
    TvVariableWait(HeapOffset),
    FvVariableWait(HeapOffset),
    WaitRegion(u8),
    Watch(Vec<RtSignalKey>),

    Jump(usize),
    /// (condition, true_offset, false_offset)
    TvBranch(HeapOffset, usize, usize),
    FvBranch(HeapOffset, usize, usize),
    Halt,
}
impl VmInstruction {
    pub fn itrace(&self, stack: &Heap, signals: &[HeapRef], logic_mode: LogicMode) {
        use VmInstruction as I;
        eprint!("{self}");
        let items: &[(&'static str, bool, HeapRef)] = match self {
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
            I::TvEdge(dst, _, lhs, rhs) => &[
                ("dst", false, dst.to_scalar_ref()),
                ("lhs", false, lhs.to_scalar_ref()),
                ("rhs", false, rhs.to_scalar_ref()),
            ],
            I::TvShift(dst, _, src, shift) => &[
                ("dst", false, *dst),
                ("src", false, src.to_ref(dst.size)),
                ("shift", false, shift.to_32bit_ref()),
            ],
            I::TvSlice(dst, src, idx) => &[
                ("dst", true, *dst),
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
                UnaryOp::Neg => &[("dst", true, dst.to_ref(src.size)), ("src", true, *src)],
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
            I::FvBinaryComparison(dst, op, lhs, rhs) => match op {
                BinaryComparisonOp::CaseEquality => &[
                    ("dst", false, dst.to_scalar_ref()),
                    ("lhs", true, *lhs),
                    ("rhs", true, rhs.to_ref(lhs.size)),
                ],
                BinaryComparisonOp::UnsignedLessEqual => &[
                    ("dst", true, dst.to_scalar_ref()),
                    ("lhs", true, *lhs),
                    ("rhs", true, rhs.to_ref(lhs.size)),
                ],
            },
            I::FvEdge(dst, _, lhs, rhs) => &[
                ("dst", false, dst.to_scalar_ref()),
                ("lhs", true, lhs.to_scalar_ref()),
                ("rhs", true, rhs.to_scalar_ref()),
            ],
            I::FvShift(dst, _, src, shift) => &[
                ("dst", true, *dst),
                ("src", true, src.to_ref(dst.size)),
                ("shift", true, shift.to_32bit_ref()),
            ],
            I::FvSlice(dst, src, idx) => &[
                ("dst", true, *dst),
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
            I::LastUpdateTime(dst, _) => &[("dst", false, dst.to_64bit_ref())],
            I::Drive(dst, src, partial) => {
                eprint!(" ({})", signals[dst.as_usize()].offset);
                match partial {
                    None => &[
                        (
                            "dst",
                            logic_mode == LogicMode::FourValue,
                            signals[dst.as_usize()],
                        ),
                        ("src", logic_mode == LogicMode::FourValue, *src),
                    ],
                    Some(partial) => &[
                        (
                            "dst",
                            logic_mode == LogicMode::FourValue,
                            signals[dst.as_usize()],
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
            I::TvVariableWait(var) => &[("time", false, var.to_64bit_ref())],
            I::FvVariableWait(var) => &[("time", true, var.to_64bit_ref())],
            I::Wait(_) => &[],
            I::WaitRegion(_) => &[],
            I::Watch(_) => &[],
            I::Jump(_) => &[],
            I::TvBranch(_, _, _) => &[],
            I::FvBranch(_, _, _) => &[],
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

#[derive(Clone, Debug)]
pub struct VmProcess {
    pub instructions: Vec<VmInstruction>,
}

impl VmIntrinsicOp {
    pub fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Finish => "finish",
            Self::Random => "random",
            Self::Display(..) => "display",
            Self::Assert(..) => "assert",
            Self::VcdOpenFile(..) => "vcd.open_file",
            Self::VcdAppendModule(..) => "vcd.append_scope",
            Self::VcdPause => "vcd.pause",
            Self::VcdResume => "vcd.resume",
            Self::ReadMem(_, _) => "readmem",
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
            Self::Min => "min",
            Self::Max => "max",
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

impl EdgeOp {
    pub fn into_mnemonic(self) -> &'static str {
        match self {
            Self::Posedge => "posedge",
            Self::Negedge => "negedge",
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
