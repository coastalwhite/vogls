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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    Float,
    Int,
    UInt,
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
        matches!(self, Self::Value(..))
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

impl DataType {
    pub fn type_str(&self) -> Option<&'static str> {
        match self {
            DataType::Float => Some("<f8"),
            DataType::Int => Some("<i8"),
            DataType::UInt => Some("<u8"),
            DataType::Bits(_) => None,
        }
    }

    pub fn itemsize(&self) -> isize {
        8 // f64 / i64 / u64 are all 8 bytes here
    }

    pub fn format(&self) -> &'static std::ffi::CStr {
        match self {
            Self::Float => c"d",
            Self::Int   => c"q",
            Self::UInt  => c"Q",
            Self::Bits(_) => todo!(),
        }
    }
}
