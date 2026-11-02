use std::sync::Arc;

use vogls::VectorSize;
use vogls::utils::IndexMap;

#[derive(PartialEq, Eq, Clone)]
pub enum Type {
    Value(ValueType),
    Array(ArrayType),
    Plan(PlanType),
    RunVector(RunVectorType),
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TypeKind {
    Value,
    Array,
    Plan,
    RunVector,
}

/// The storage width of an unsigned integer datatype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UIntType {
    U8,
    U16,
    U32,
    U64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    Float,
    Int,
    UInt(UIntType),
    Bits(VectorSize),
}

#[derive(PartialEq, Eq, Clone)]
pub struct ValueType {
    pub data: DataType,
}

#[derive(PartialEq, Eq, Clone)]
pub struct ArrayType {
    pub data: DataType,
    pub length: Option<usize>,
}

#[derive(PartialEq, Eq, Clone)]
pub struct PlanType {
    pub components: Arc<IndexMap<String, Type>>,
}

#[derive(PartialEq, Eq, Clone)]
pub struct RunVectorType {
    pub data: DataType,
    pub length: Option<usize>,
    pub width: RunWidth,
}

impl Type {
    pub fn kind(&self) -> TypeKind {
        match self {
            Type::Value(..) => TypeKind::Value,
            Type::Array(..) => TypeKind::Array,
            Type::Plan(..) => TypeKind::Plan,
            Type::RunVector(..) => TypeKind::RunVector,
        }
    }

    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(..))
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(..))
    }
    pub fn is_plan(&self) -> bool {
        matches!(self, Self::Plan(..))
    }
    pub fn is_run_vector(&self) -> bool {
        matches!(self, Self::RunVector(..))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum RunWidth {
    Scalar,
    Constant(u64),
    Variable,
}

impl RunWidth {
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable)
    }

    pub fn size(&self) -> Option<usize> {
        match self {
            RunWidth::Scalar => Some(1),
            RunWidth::Constant(n) => Some(*n as usize),
            RunWidth::Variable => None,
        }
    }
}

impl UIntType {
    pub const fn max_value(&self) -> u64 {
        match self {
            Self::U8 => u8::MAX as u64,
            Self::U16 => u16::MAX as u64,
            Self::U32 => u32::MAX as u64,
            Self::U64 => u64::MAX,
        }
    }

    pub const fn size_in_bytes(&self) -> isize {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 => 4,
            Self::U64 => 8,
        }
    }

    /// The numpy `typestr` for this width.
    pub fn type_str(&self) -> &'static str {
        match self {
            Self::U8 => "|u1",
            Self::U16 => "<u2",
            Self::U32 => "<u4",
            Self::U64 => "<u8",
        }
    }

    pub fn format(&self) -> &'static std::ffi::CStr {
        match self {
            Self::U8 => c"B",
            Self::U16 => c"H",
            Self::U32 => c"I",
            Self::U64 => c"Q",
        }
    }
}

impl DataType {
    pub fn type_str(&self) -> Option<&'static str> {
        match self {
            DataType::Float => Some("<f8"),
            DataType::Int => Some("<i8"),
            DataType::UInt(width) => Some(width.type_str()),
            DataType::Bits(_) => None,
        }
    }

    pub fn itemsize(&self) -> isize {
        match self {
            Self::Float | Self::Int => 8,
            Self::UInt(width) => width.size_in_bytes(),
            Self::Bits(_) => 8,
        }
    }

    pub fn format(&self) -> &'static std::ffi::CStr {
        match self {
            Self::Float => c"d",
            Self::Int => c"q",
            Self::UInt(width) => width.format(),
            Self::Bits(_) => todo!(),
        }
    }
}
