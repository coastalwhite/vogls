use std::any::Any;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use vogls::utils::{VgHashMap, new_table_key};
use vogls::{Bits, VectorSize};

use crate::CspAble;
use crate::array::{Array, UIntArray};
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::output::{DslLazyOutput, LazyOutputKey, Output};
use crate::typing::{DataType, Type, UIntType, ValueType};

new_table_key! { pub struct LazyValueKey; }

/// An unsigned integer together with its storage width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UIntValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

impl UIntValue {
    pub fn to_u64(self) -> u64 {
        match self {
            Self::U8(v) => v as u64,
            Self::U16(v) => v as u64,
            Self::U32(v) => v as u64,
            Self::U64(v) => v,
        }
    }

    pub fn ty(self) -> UIntType {
        match self {
            Self::U8(_) => UIntType::U8,
            Self::U16(_) => UIntType::U16,
            Self::U32(_) => UIntType::U32,
            Self::U64(_) => UIntType::U64,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Float(f64),
    Int(i64),
    UInt(UIntValue),
    Bits(Bits),
}
impl Value {
    pub fn to_lazy_dsl(&self) -> DslLazyValue {
        DslLazyValue {
            ty: Arc::new(Type::Value(self.ty())),
            f: Arc::new(self.clone()) as _,
        }
    }

    pub fn repeat(&self, n: usize) -> Array {
        match self {
            Self::Float(v) => Array::Floats(Buffer::from_vec(std::iter::repeat_n(*v, n).collect())),
            Self::Int(v) => Array::Ints(Buffer::from_vec(std::iter::repeat_n(*v, n).collect())),
            Self::UInt(v) => Array::UInts(match *v {
                UIntValue::U8(v) => {
                    UIntArray::U8(Buffer::from_vec(std::iter::repeat_n(v, n).collect()))
                }
                UIntValue::U16(v) => {
                    UIntArray::U16(Buffer::from_vec(std::iter::repeat_n(v, n).collect()))
                }
                UIntValue::U32(v) => {
                    UIntArray::U32(Buffer::from_vec(std::iter::repeat_n(v, n).collect()))
                }
                UIntValue::U64(v) => {
                    UIntArray::U64(Buffer::from_vec(std::iter::repeat_n(v, n).collect()))
                }
            }),
            Self::Bits(v) => {
                // @Performance: This is horrendous.
                let mut acc = v.clone();
                for _ in 1..n {
                    acc = Bits::concatenate(&acc, v);
                }
                Array::Bits(acc, v.size())
            }
        }
    }

    pub fn to_bits(&self, size: VectorSize) -> Bits {
        match self {
            Value::Float(_) => todo!(),
            Value::Int(_) => todo!(),
            Value::UInt(value) => Bits::from_u64(size, value.to_u64()),
            Value::Bits(bits) => bits.clone(),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            Self::Float(_) => DataType::Float,
            Self::Int(_) => DataType::Int,
            Self::UInt(v) => DataType::UInt(v.ty()),
            Self::Bits(bits) => DataType::Bits(bits.size()),
        }
    }

    pub fn ty(&self) -> ValueType {
        ValueType {
            data: self.data_type(),
        }
    }
}
impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Float(item) => item.to_bits().hash(state),
            Self::Int(item) => item.hash(state),
            Self::UInt(item) => item.hash(state),
            Self::Bits(bits) => {
                bits.hash(state);
            }
        }
    }
}
impl Eq for Value {}

impl DslValueNode for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Literal: {self:?}")
    }
    fn convert_one<'a>(&'a self, _converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ValueNode> {
        Arc::new(self.clone()) as _
    }
    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
impl ValueNode for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Literal: {self:?}")
    }
    fn csp_eq(&self, other: &dyn ValueNode) -> bool {
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
    }
    fn csp_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {}
    fn compute(&self, _ctx: &ComputeContext, _inputs: &ComputeInputs) -> ComputeResult<Value> {
        Ok(self.clone())
    }
}

pub struct LazyValue {
    pub ty: Arc<Type>,
    pub f: Arc<dyn ValueNode>,
}
#[derive(Clone)]
pub struct DslLazyValue {
    pub ty: Arc<Type>,
    pub f: Arc<dyn DslValueNode>,
}

impl ComputeNode for LazyValue {
    type Key = LazyValueKey;
    type Output = Value;

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.f.fmt(f)
    }
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        self.f.extend_inputs(deps);
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        self.f.compute(ctx, inputs)
    }
}
impl CspAble for LazyValue {
    fn csp_eq(&self, other: &Self) -> bool {
        self.f.csp_eq(other.f.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, mut state: &mut H) {
        self.f.csp_hash(&mut state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}
impl DslNode for DslLazyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.f.fmt(f)
    }
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyValue {
            ty: self.ty.clone(),
            f: self.f.convert_one(converted),
        };
        Key::Value(csp.values.insert(&mut graph.values, r))
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.f.extend_inputs(f);
    }
}

pub trait DslValueNode: Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ValueNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}
pub trait ValueNode: std::any::Any + Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    fn csp_eq(&self, other: &dyn ValueNode) -> bool;
    fn csp_hash(&self, state: &mut dyn Hasher);

    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Value>;
}

pub struct DslValueExtractOutput(pub DslLazyOutput);
#[derive(Clone, PartialEq, Hash)]
pub struct ValueExtractOutput(LazyOutputKey);

impl DslValueNode for DslValueExtractOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Extract Value")
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ValueNode> {
        Arc::new(ValueExtractOutput(
            converted[&DslPtr::from(&self.0 as &dyn DslNode)].as_output(),
        ))
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(&self.0);
    }
}
impl ValueNode for ValueExtractOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Extract Value")
    }

    fn csp_eq(&self, other: &dyn ValueNode) -> bool {
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
    }

    fn csp_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.outputs.push(self.0);
    }

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Value> {
        let Output::Value(v) = &inputs.outputs[&self.0] else {
            return Err(ComputeError::InvalidTypes);
        };
        Ok(v.clone())
    }
}
