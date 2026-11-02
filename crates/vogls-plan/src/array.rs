use std::any::Any;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::sync::Arc;

use vogls::Bits;
use vogls::utils::{VgHashMap, new_table_key};

use crate::CspAble;
use crate::agg::Agg;
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::output::{DslLazyOutput, LazyOutputKey, Output};
use crate::typing::{ArrayType, DataType, Type, UIntType};
use crate::value::{UIntValue, Value};

new_table_key! { pub struct LazyArrayKey; }

#[derive(Debug, Clone, PartialEq)]
pub enum Array {
    Floats(Buffer<f64>),
    Ints(Buffer<i64>),
    UInts(UIntArray),
    Bits(Bits, NonZeroU32),
}

/// Width-tagged unsigned integer storage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UIntArray {
    U8(Buffer<u8>),
    U16(Buffer<u16>),
    U32(Buffer<u32>),
    U64(Buffer<u64>),
}

pub trait UIntElement: Copy {
    fn to_u64(self) -> u64;

    /// Get a value from a `u64`.
    fn from_u64(value: u64) -> Self;

    /// The buffer of a [`UIntArray`] whose width is `Self`, else `None`.
    fn from_uint_array(array: &UIntArray) -> Option<&Buffer<Self>>;

    fn into_uint_array(buffer: Buffer<Self>) -> UIntArray;

    fn into_uint_value(self) -> UIntValue;
}

// The widening and narrowing expressions are passed in rather than written as
// `as` casts in the macro body, so that the `u64` arm does not become an
// `unnecessary_cast`.
macro_rules! impl_uint_element {
    ($t:ty, $variant:ident, |$slf:ident| $to_u64:expr, |$value:ident| $from_u64:expr) => {
        impl UIntElement for $t {
            #[inline(always)]
            fn to_u64($slf) -> u64 {
                $to_u64
            }
            #[inline(always)]
            fn from_u64($value: u64) -> Self {
                let narrowed = $from_u64;
                debug_assert_eq!(
                    <Self as UIntElement>::to_u64(narrowed),
                    $value,
                    "value does not fit the target width"
                );
                narrowed
            }
            fn from_uint_array(array: &UIntArray) -> Option<&Buffer<Self>> {
                match array {
                    UIntArray::$variant(items) => Some(items),
                    _ => None,
                }
            }
            fn into_uint_array(buffer: Buffer<Self>) -> UIntArray {
                UIntArray::$variant(buffer)
            }
            #[inline(always)]
            fn into_uint_value(self) -> UIntValue {
                UIntValue::$variant(self)
            }
        }
    };
}
impl_uint_element!(u8, U8, |self| self as u64, |value| value as u8);
impl_uint_element!(u16, U16, |self| self as u64, |value| value as u16);
impl_uint_element!(u32, U32, |self| self as u64, |value| value as u32);
impl_uint_element!(u64, U64, |self| self, |value| value);

impl UIntArray {
    pub fn len(&self) -> usize {
        match self {
            Self::U8(items) => items.len(),
            Self::U16(items) => items.len(),
            Self::U32(items) => items.len(),
            Self::U64(items) => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn zeros(ty: UIntType, len: usize) -> Self {
        with_match_uint_type!(ty, |$T| {
            <$T as UIntElement>::into_uint_array(Buffer::from_vec(vec![0; len]))
        })
    }

    /// The backing slice at a statically-known width.
    ///
    /// Panics if `T` is not this array's width.
    pub fn as_slice<T: UIntElement>(&self) -> &[T] {
        T::from_uint_array(self)
            .expect("as_slice called with a width other than the array's own")
            .as_slice()
    }

    pub fn ty(&self) -> UIntType {
        match self {
            Self::U8(_) => UIntType::U8,
            Self::U16(_) => UIntType::U16,
            Self::U32(_) => UIntType::U32,
            Self::U64(_) => UIntType::U64,
        }
    }

    pub fn as_byte_ptr(&self) -> *const u8 {
        match self {
            Self::U8(items) => items.as_ptr().cast(),
            Self::U16(items) => items.as_ptr().cast(),
            Self::U32(items) => items.as_ptr().cast(),
            Self::U64(items) => items.as_ptr().cast(),
        }
    }

    pub fn get(&self, idx: usize) -> UIntValue {
        match self {
            Self::U8(items) => UIntValue::U8(items[idx]),
            Self::U16(items) => UIntValue::U16(items[idx]),
            Self::U32(items) => UIntValue::U32(items[idx]),
            Self::U64(items) => UIntValue::U64(items[idx]),
        }
    }

    pub fn slice(&self, offset: usize, length: usize) -> Self {
        match self {
            Self::U8(items) => Self::U8(items[offset..][..length].to_vec().into()),
            Self::U16(items) => Self::U16(items[offset..][..length].to_vec().into()),
            Self::U32(items) => Self::U32(items[offset..][..length].to_vec().into()),
            Self::U64(items) => Self::U64(items[offset..][..length].to_vec().into()),
        }
    }
}

pub trait ArrayBuilder {
    fn reserve(&mut self, capacity: usize);
    fn extend(&mut self, arr: &Array);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn finish(self: Box<Self>) -> Array;
}

#[derive(Default)]
struct PrimitiveArrayBuilder<T>(Vec<T>);

pub trait Primitive: Copy + std::ops::Add<Self, Output = Self> {
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
macro_rules! impl_uint_primitive {
    ($t:ty, $variant:ident) => {
        impl Primitive for $t {
            fn from_value(value: &Value) -> Option<Self> {
                match value {
                    Value::UInt(UIntValue::$variant(v)) => Some(*v),
                    _ => None,
                }
            }
            fn from_array(array: &Array) -> Option<&Buffer<Self>> {
                match array {
                    Array::UInts(UIntArray::$variant(v)) => Some(v),
                    _ => None,
                }
            }
            fn into_array(buf: Buffer<Self>) -> Array {
                Array::UInts(UIntArray::$variant(buf))
            }
        }
    };
}
impl_uint_primitive!(u8, U8);
impl_uint_primitive!(u16, U16);
impl_uint_primitive!(u32, U32);
impl_uint_primitive!(u64, U64);
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
        DataType::UInt(UIntType::U8) => Box::new(PrimitiveArrayBuilder::<u8>::default()),
        DataType::UInt(UIntType::U16) => Box::new(PrimitiveArrayBuilder::<u16>::default()),
        DataType::UInt(UIntType::U32) => Box::new(PrimitiveArrayBuilder::<u32>::default()),
        DataType::UInt(UIntType::U64) => Box::new(PrimitiveArrayBuilder::<u64>::default()),
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_byte_ptr(&self) -> *const u8 {
        match self {
            Array::Floats(vs) => vs.as_ptr().cast(),
            Array::Ints(vs) => vs.as_ptr().cast(),
            Array::UInts(vs) => vs.as_byte_ptr(),
            Array::Bits(vs, _) => vs.as_u64_slice().as_ptr().cast(),
        }
    }

    pub fn get(&self, idx: usize) -> Value {
        match self {
            Array::Floats(i) => Value::Float(i[idx]),
            Array::Ints(i) => Value::Int(i[idx]),
            Array::UInts(i) => Value::UInt(i.get(idx)),
            Array::Bits(i, stride) => Value::Bits(i.slicex(idx as u32 * stride.get(), *stride)),
        }
    }

    pub fn slice(&self, offset: usize, length: usize) -> Array {
        match self {
            Array::Floats(i) => Array::Floats(i[offset..][..length].to_vec().into()),
            Array::Ints(i) => Array::Ints(i[offset..][..length].to_vec().into()),
            Array::UInts(i) => Array::UInts(i.slice(offset, length)),
            Array::Bits(..) => todo!(),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            Self::Floats(..) => DataType::Float,
            Self::Ints(..) => DataType::Int,
            Self::UInts(d) => DataType::UInt(d.ty()),
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

        // One arm per width, matching exactly the width `output_type` declared.
        // A node that declares a width and then yields another has broken the
        // `Map`/`Agg` contract, which `unreachable!` reports loudly.
        macro_rules! uint_arm {
            ($variant:ident) => {
                values
                    .map(|v| {
                        let v = v?;
                        let Value::UInt(UIntValue::$variant(v)) = v else {
                            unreachable!();
                        };
                        Ok(v)
                    })
                    .collect::<Result<Buffer<_>, E>>()
                    .map(|b| Self::UInts(UIntArray::$variant(b)))
            };
        }

        use DataType as DT;
        match arr_type.data {
            DT::Float => primitive_arm!(Float, Floats),
            DT::Int => primitive_arm!(Int, Ints),
            DT::UInt(UIntType::U8) => uint_arm!(U8),
            DT::UInt(UIntType::U16) => uint_arm!(U16),
            DT::UInt(UIntType::U32) => uint_arm!(U32),
            DT::UInt(UIntType::U64) => uint_arm!(U64),
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

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct ArrayGet(pub usize);

impl Agg for ArrayGet {
    type Inputs<Input>
        = [Input; 1]
    where
        Input: Send + Sync;

    type Scratches = ();

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArrayGet({})", self.0)
    }

    fn output_type(&self, inputs: Self::Inputs<DataType>) -> ComputeResult<DataType> {
        Ok(inputs[0])
    }
    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        _scratch: &mut Self::Scratches,
    ) -> ComputeResult<Value> {
        Ok(inputs[0].get(self.0))
    }
}
