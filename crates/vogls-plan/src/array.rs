use std::any::Any;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::sync::Arc;

use vogls::Bits;
use vogls::utils::{VgHashMap, new_table_key};

use crate::CspAble;
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::output::{DslLazyOutput, LazyOutputKey, Output};
use crate::typing::{ArrayType, DataType, Type};
use crate::value::Value;

new_table_key! { pub struct LazyArrayKey; }

#[derive(Debug, Clone, PartialEq)]
pub enum Array {
    Floats(Buffer<f64>),
    Ints(Buffer<i64>),
    UInts(Buffer<u64>),
    Bits(Bits, NonZeroU32),
}

pub trait ArrayBuilder {
    fn reserve(&mut self, capacity: usize);
    fn extend(&mut self, arr: &Array);
    fn len(&self) -> usize;
    fn finish(self: Box<Self>) -> Array;
}

#[derive(Default)]
struct PrimitiveArrayBuilder<T>(Vec<T>);

trait Primitive: Copy {
    fn from_value(value: &Value) -> Option<Self>;
    fn from_array(array: &Array) -> Option<&Buffer<Self>>;
    fn into_array(buf: Buffer<Self>) -> Array;
}

impl Primitive for f64 {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Float(v) => Some(*v),
            _ => None,
        }
    }
    fn from_array(array: &Array) -> Option<&Buffer<Self>> {
        match array {
            Array::Floats(v) => Some(v),
            _ => None,
        }
    }
    fn into_array(buf: Buffer<Self>) -> Array {
        Array::Floats(buf)
    }
}
impl Primitive for u64 {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::UInt(v) => Some(*v),
            _ => None,
        }
    }
    fn from_array(array: &Array) -> Option<&Buffer<Self>> {
        match array {
            Array::UInts(v) => Some(v),
            _ => None,
        }
    }
    fn into_array(buf: Buffer<Self>) -> Array {
        Array::UInts(buf)
    }
}
impl Primitive for i64 {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }
    fn from_array(array: &Array) -> Option<&Buffer<Self>> {
        match array {
            Array::Ints(v) => Some(v),
            _ => None,
        }
    }
    fn into_array(buf: Buffer<Self>) -> Array {
        Array::Ints(buf)
    }
}

impl<T: Primitive> ArrayBuilder for PrimitiveArrayBuilder<T> {
    fn reserve(&mut self, capacity: usize) {
        self.0.reserve(capacity);
    }
    fn extend(&mut self, arr: &Array) {
        let buf = T::from_array(arr).unwrap();
        self.0.extend_from_slice(buf.as_slice());
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn finish(self: Box<Self>) -> Array {
        T::into_array(Buffer::from_vec(self.0))
    }
}

pub fn new_array_builder(data_type: &DataType) -> Box<dyn ArrayBuilder> {
    match data_type {
        DataType::Float => Box::new(PrimitiveArrayBuilder::<f64>::default()),
        DataType::Int => Box::new(PrimitiveArrayBuilder::<i64>::default()),
        DataType::UInt => Box::new(PrimitiveArrayBuilder::<u64>::default()),
        DataType::Bits(_) => todo!(),
    }
}

impl Eq for Array {}

impl Hash for Array {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        self.len().hash(state);
        match self {
            Array::Floats(items) => items.iter().for_each(|v| v.to_bits().hash(state)),
            Array::Ints(items) => items.hash(state),
            Array::UInts(items) => items.hash(state),
            Array::Bits(bits, stride) => {
                stride.hash(state);
                bits.hash(state);
            }
        }
    }
}

impl Array {
    pub fn to_lazy_dsl(&self) -> DslLazyArray {
        DslLazyArray {
            ty: Arc::new(Type::Array(self.ty())),
            f: Arc::new(self.clone()) as _,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Floats(items) => items.len(),
            Self::Ints(items) => items.len(),
            Self::UInts(items) => items.len(),
            Self::Bits(bits, stride) => (bits.size().get() / stride.get()) as usize,
        }
    }

    pub fn get(&self, idx: usize) -> Value {
        match self {
            Array::Floats(i) => Value::Float(i[idx]),
            Array::Ints(i) => Value::Int(i[idx]),
            Array::UInts(i) => Value::UInt(i[idx]),
            Array::Bits(i, stride) => Value::Bits(i.slicex(idx as u32 * stride.get(), *stride)),
        }
    }

    pub fn as_u64(&self) -> Option<&Buffer<u64>> {
        match self {
            Self::UInts(vs) => Some(vs),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<&Buffer<i64>> {
        match self {
            Self::Ints(vs) => Some(vs),
            _ => None,
        }
    }

    pub fn slice(&self, offset: usize, length: usize) -> Array {
        match self {
            Array::Floats(i) => Array::Floats(i[offset..][..length].to_vec().into()),
            Array::Ints(i) => Array::Ints(i[offset..][..length].to_vec().into()),
            Array::UInts(i) => Array::UInts(i[offset..][..length].to_vec().into()),
            Array::Bits(..) => todo!(),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            Self::Floats(..) => DataType::Float,
            Self::Ints(..) => DataType::Int,
            Self::UInts(..) => DataType::UInt,
            Self::Bits(.., stride) => DataType::Bits(*stride),
        }
    }

    pub fn ty(&self) -> ArrayType {
        ArrayType {
            data: self.data_type(),
            length: Some(self.len()),
        }
    }

    pub fn try_from_value_iter<E>(
        arr_type: &ArrayType,
        values: impl Iterator<Item = Result<Value, E>>,
    ) -> Result<Self, E> {
        macro_rules! primitive_arm {
            ($value:ident, $arr:ident) => {
                values
                    .map(|v| {
                        let v = v?;
                        let Value::$value(v) = v else {
                            unreachable!();
                        };
                        Ok(v)
                    })
                    .collect::<Result<Buffer<_>, E>>()
                    .map(Self::$arr)
            };
        }

        use DataType as DT;
        match arr_type.data {
            DT::Float => primitive_arm!(Float, Floats),
            DT::Int => primitive_arm!(Int, Ints),
            DT::UInt => primitive_arm!(UInt, UInts),
            DT::Bits(_) => todo!(),
        }
    }
}

#[derive(Clone)]
pub struct LazyArray {
    pub ty: Arc<Type>,
    pub f: Arc<dyn ArrayNode>,
}

#[derive(Clone)]
pub struct DslLazyArray {
    pub ty: Arc<Type>,
    pub f: Arc<dyn DslArrayNode>,
}
impl DslLazyArray {
    pub fn ty(&self) -> &ArrayType {
        let Type::Array(ty) = self.ty.as_ref() else {
            unreachable!()
        };
        ty
    }
}

pub trait DslArrayNode: Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}
pub trait ArrayNode: std::any::Any + Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn csp_eq(&self, other: &dyn ArrayNode) -> bool;
    fn csp_hash(&self, state: &mut dyn Hasher);
    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Array>;
}

impl CspAble for LazyArray {
    fn csp_eq(&self, other: &Self) -> bool {
        self.f.csp_eq(other.f.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, mut state: &mut H) {
        self.f.type_id().hash(state);
        self.f.csp_hash(&mut state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}
impl ComputeNode for LazyArray {
    type Key = LazyArrayKey;
    type Output = Array;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.f.fmt(f)
    }
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        self.f.extend_inputs(deps);
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        self.f.compute(ctx, inputs)
    }
}
impl DslNode for DslLazyArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.f.fmt(f)
    }
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyArray {
            ty: self.ty.clone(),
            f: self.f.convert_one(converted),
        };
        Key::Array(csp.arrays.insert(&mut graph.arrays, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.f.extend_inputs(f);
    }
}

impl DslArrayNode for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Literal Array")
    }
    fn convert_one<'a>(&'a self, _converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode> {
        Arc::new(self.clone()) as _
    }
    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
impl ArrayNode for Array {
    impl_dyn_eq_hash!(ArrayNode);
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Literal Array")
    }
    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {}
    fn compute(&self, _ctx: &ComputeContext, _inputs: &ComputeInputs) -> ComputeResult<Array> {
        Ok(self.clone())
    }
}

pub struct DslArrayExtractOutput(pub DslLazyOutput);
#[derive(Clone, PartialEq, Hash)]
pub struct ArrayExtractOutput(LazyOutputKey);

impl DslArrayNode for DslArrayExtractOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Extract Array")
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode> {
        Arc::new(ArrayExtractOutput(
            converted[&DslPtr::from(&self.0 as &dyn DslNode)].as_output(),
        ))
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(&self.0);
    }
}
impl ArrayNode for ArrayExtractOutput {
    impl_dyn_eq_hash!(ArrayNode);

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Extract Array")
    }

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.outputs.push(self.0);
    }

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Array> {
        let Output::Array(v) = &inputs.outputs[&self.0] else {
            return Err(ComputeError::InvalidTypes);
        };
        Ok(v.clone())
    }
}
