use std::any::Any;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::sync::Arc;

use rand::RngExt;
use rand::rngs::SmallRng;
use vogls::Bits;
use vogls::utils::{VgHashMap, new_table_key};

use crate::CspAble;
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeGraph, ComputeInputs, ComputeNode,
    ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::value::Value;

new_table_key! { pub struct LazyArrayKey; }

#[derive(Clone, PartialEq)]
pub enum Array {
    Floats(Buffer<f64>),
    Ints(Buffer<i64>),
    UInts(Buffer<u64>),
    Bits(Bits, NonZeroU32),
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
        DslLazyArray(Arc::new(self.clone()) as _)
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
            Array::Bits(..) => todo!(),
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
}

#[derive(Clone)]
pub struct LazyArray(pub Arc<dyn ArrayNode>);

#[derive(Clone)]
pub struct DslLazyArray(pub Arc<dyn DslArrayNode>);

pub trait DslArrayNode: Send + Sync + 'static {
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}
pub trait ArrayNode: std::any::Any + Send + Sync + 'static {
    fn csp_eq(&self, other: &dyn ArrayNode) -> bool;
    fn csp_hash(&self, state: &mut dyn Hasher);
    fn len(&self, graph: &ComputeGraph) -> Option<usize>;
    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Array>;
}

impl CspAble for LazyArray {
    fn csp_eq(&self, other: &Self) -> bool {
        self.0.csp_eq(other.0.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, mut state: &mut H) {
        self.0.csp_hash(&mut state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}
impl ComputeNode for LazyArray {
    type Output = Array;
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        self.0.extend_inputs(deps);
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        self.0.compute(ctx, inputs)
    }
}
impl DslNode for DslLazyArray {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyArray(self.0.convert_one(converted));
        Key::Array(csp.arrays.insert(&mut graph.arrays, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.0.extend_inputs(f);
    }
}

impl DslArrayNode for Array {
    fn convert_one<'a>(&'a self, _converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode> {
        Arc::new(self.clone()) as _
    }
    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
impl ArrayNode for Array {
    fn csp_eq(&self, other: &dyn ArrayNode) -> bool {
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
    }
    fn csp_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
    fn len(&self, _graph: &ComputeGraph) -> Option<usize> {
        Some(self.len())
    }
    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {}
    fn compute(&self, _ctx: &ComputeContext, _inputs: &ComputeInputs) -> ComputeResult<Array> {
        Ok(self.clone())
    }
}
