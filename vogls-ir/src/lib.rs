mod builder;
mod format;
mod types;

use std::collections::HashSet;
use std::fmt;

pub use builder::{BasicBlockBuilder, BranchRef, PhiRef, new_process};
pub use format::{ContextFormat, DisplayContext};
use indexmap::IndexSet;
use slotmap::{SlotMap, new_key_type};
pub use types::{ArrayWidth, Type};

new_key_type! { pub struct ProcessKey; }
new_key_type! { pub struct BasicBlockKey; }
new_key_type! { pub struct SignalKey; }
new_key_type! { pub struct VariableKey; }

// @TODO: Do some smarter stuff here. Probably we can use the lsb to say small big and they put a
// pointer in the u64.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Bits {
    Small(u64, VectorSize),
    Big(VectorSize, Box<[u8]>),
}

impl Bits {
    pub fn as_slice(&self) -> &[u8] {
        const { assert!(cfg!(target_endian = "little")) }
        match self {
            Bits::Small(value, size) => &bytemuck::bytes_of(value)[..size.div_ceil(8) as usize],
            Bits::Big(_, value) => value.as_ref(),
        }
    }

    pub fn new_zeroed(size: VectorSize) -> Self {
        if size > 64 {
            Self::Big(
                size,
                std::iter::repeat_n(0, size.div_ceil(8) as usize).collect(),
            )
        } else {
            Self::Small(0, size)
        }
    }

    pub fn load_from_slice(slice: &[u8], size: VectorSize) -> Self {
        if size < 64 {
            let mut value = 0u64;
            for &b in &slice[..size.div_ceil(8) as usize] {
                value <<= 8;
                value |= b as u64;
            }
            Self::Small(value, size)
        } else {
            Self::Big(size, slice.into())
        }
    }

    pub fn size(&self) -> VectorSize {
        match self {
            Bits::Small(_, s) => *s,
            Bits::Big(s, _) => *s,
        }
    }

    pub fn sign_extend(self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self;
        }

        assert!(self.size() < new_size);
        match self {
            Bits::Small(v, _) if new_size <= 64 => Bits::Small(v, new_size),
            _ => {
                let old_size = self.size();
                let mut bytes =
                    std::iter::repeat_n(0, new_size.div_ceil(8) as usize).collect::<Box<[u8]>>();
                bytes[..old_size.div_ceil(8) as usize].copy_from_slice(self.as_slice());
                Bits::Big(new_size, bytes)
            }
        }
    }

    pub fn from_i64_truncated(value: i64, size: VectorSize) -> Bits {
        if size == 64 {
            Bits::Small(value as u64, size)
        } else if size < 64 {
            Bits::Small((value as u64) & ((1u64 << size) - 1), size)
        } else {
            let mut bytes =
                std::iter::repeat_n(0, size.div_ceil(8) as usize).collect::<Box<[u8]>>();
            bytes[..8].copy_from_slice(&bytemuck::bytes_of(&value));
            Bits::Big(size, bytes)
        }
    }

    pub fn not_eq_zero(&self) -> bool {
        match self {
            Bits::Small(v, _) => *v != 0,
            Bits::Big(_, v) => v.iter().any(|b| *b != 0),
        }
    }

    pub fn concatenate(lhs: Bits, rhs: Bits) -> Bits {
        match (lhs, rhs) {
            (Bits::Small(lv, ls), Bits::Small(rv, rs)) if lv + rv <= 64 => {
                Bits::Small((lv << rs) | rv, ls + rs)
            }
            _ => todo!(),
        }
    }
}

macro_rules! impl_arithmetic {
    ($(($f:ident, $op:ident)),+ $(,)?) => {
        impl Bits {
        $(
        pub fn $f(lhs: Self, rhs: Self) -> Self {
            assert_eq!(lhs.size(), rhs.size());
            match (lhs, rhs) {
                (Self::Small(l, s), Self::Small(r, _)) => Self::Small(l.$op(r) & ((1u64 << s) - 1), s),
                (Self::Big(_s, _l), Self::Big(_, _r)) => todo!(),
                _ => unreachable!(),
            }
        }
        )+
        }
    }
}

impl_arithmetic! {
    (multiply, wrapping_mul),
    (add, wrapping_add),
    (subtract, wrapping_sub),
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::Small(value, size) if size % 4 == 0 => write!(f, "{size}'h{value:X}"),
            Bits::Small(value, size) => write!(f, "{size}'b{value:b}"),
            Bits::Big(size, _) => write!(f, "{size}'b.."),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bits(Bits),
    Decimal(i64),
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Self::Bits(Bits::Small(_, size)) => Type::Bits(*size),
            Self::Bits(Bits::Big(size, _)) => Type::Bits(*size),
            Self::Decimal(..) => Type::Decimal,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Time(pub u64);

#[derive(Debug, Clone)]
pub enum BasicBlockTerminator {
    Wait(BasicBlockKey, Time),
    WaitRegion(BasicBlockKey, u8),
    Watch(BasicBlockKey, Vec<SignalKey>),
    Jump(BasicBlockKey),
    /// (condition, if_true, if_false)
    Branch(VariableKey, BasicBlockKey, BasicBlockKey),
    Halt,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub name: String,
    pub instrs: Vec<Instruction>,
    pub terminator: BasicBlockTerminator,
}

impl BasicBlockTerminator {
    pub fn extend_next_rev(
        &self,
        bb_stack: &mut Vec<BasicBlockKey>,
        bb_seen: &mut HashSet<BasicBlockKey>,
    ) {
        match self {
            Self::Wait(bb, _) | Self::WaitRegion(bb, _) | Self::Watch(bb, _) | Self::Jump(bb) => {
                if bb_seen.insert(*bb) {
                    bb_stack.push(*bb);
                }
            }
            Self::Branch(_, true_bb, false_bb) => {
                if bb_seen.insert(*false_bb) {
                    bb_stack.push(*false_bb);
                }
                if bb_seen.insert(*true_bb) {
                    bb_stack.push(*true_bb);
                }
            }
            Self::Halt => {}
        }
    }
}

pub struct Variable {
    pub name: String,
    pub ty: Type,
}

pub struct Signal {
    pub name: String,
    pub width: Option<ArrayWidth>,
    pub ty: Type,
}

pub type VectorSize = u32;

#[derive(Debug, Clone)]
pub enum IntrinsicArg {
    StringLiteral(String),
    Variable(VariableKey),
}

#[derive(Debug, Clone, Copy)]
pub enum IntrinsicOp {
    Display,
    Finish,
    Assert,
    AssertEq(bool),
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    BitNeg(VectorSize),
    BitReduceOr(VectorSize),
    BitReduceAnd(VectorSize),
    BitReduceXor(VectorSize),
    BitSlice(VectorSize, VectorSize),

    DecimalNeg,
    DecimalReduceOr,
    DecimalReduceAnd,
    DecimalReduceXor,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    BitAnd(VectorSize),
    BitOr(VectorSize),
    BitXor(VectorSize),

    DecimalAnd,
    DecimalOr,
    DecimalXor,

    DecimalAdd,
    DecimalSub,
    DecimalMultiply,
    DecimalDivide,
    DecimalModulus,

    UnsignedLessEqual(VectorSize),
    DecimalLessEqual,

    SelectBit(VectorSize),
    LogicalShiftRight(VectorSize),
    Concat(VectorSize, VectorSize),
}

#[derive(Debug, Clone)]
pub enum Instruction {
    ConstantBit(VariableKey, Bits),
    ConstantDecimal(VariableKey, i64),

    Unary(VariableKey, UnaryOp, VariableKey),
    Binary(VariableKey, BinaryOp, VariableKey, VariableKey),

    Cast(VariableKey, VariableKey),

    Intrinsic(IntrinsicOp, Vec<IntrinsicArg>),
    ArrProbe(VariableKey, SignalKey, VariableKey),
    ArrDrive(
        SignalKey,
        VariableKey,
        VariableKey,
        u8,
        Option<(VariableKey, VectorSize)>,
    ),
    Probe(VariableKey, SignalKey),
    Drive(
        SignalKey,
        VariableKey,
        u8,
        Option<(VariableKey, VectorSize)>,
    ),

    Phi(VariableKey, Box<[(BasicBlockKey, VariableKey)]>),
}

impl Instruction {
    pub fn get_destination_variable(&self) -> Option<VariableKey> {
        match self {
            Self::ConstantBit(dst, _)
            | Self::ConstantDecimal(dst, _)
            | Self::Unary(dst, _, _)
            | Self::Binary(dst, _, _, _)
            | Self::Cast(dst, _)
            | Self::Phi(dst, _)
            | Self::Probe(dst, _)
            | Self::ArrProbe(dst, _, _) => Some(*dst),
            Self::Intrinsic(..) | Self::ArrDrive(..) | Self::Drive(..) => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectionDirection {
    In,
    Out,
    Both,
}

#[derive(Debug)]
pub struct Connection {
    pub signal: SignalKey,
    pub direction: ConnectionDirection,
}

#[derive(Default)]
pub struct GlobalContext {
    pub processes: SlotMap<ProcessKey, Process>,
    pub bbs: SlotMap<BasicBlockKey, BasicBlock>,
    pub vars: SlotMap<VariableKey, Variable>,
    pub signals: SlotMap<SignalKey, Signal>,
}

#[derive(Debug)]
pub struct Process {
    pub name: String,
    pub entry: BasicBlockKey,
    pub ins: IndexSet<SignalKey>,
    pub outs: IndexSet<SignalKey>,
}
