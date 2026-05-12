use std::hash::Hash;
use std::num::NonZeroU32;
use std::sync::Arc;

use rand::RngExt;
use rand::rngs::SmallRng;
use vogls::utils::{VgHashMap, new_table_key};
use vogls::{Bits, VectorSize};

use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeGraph, ComputeInputs, ComputeNode,
    ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};

new_table_key! { pub struct LazyArrayKey; }
new_table_key! { pub struct LazyValueKey; }

#[derive(Clone, PartialEq)]
pub enum Value {
    Float(f64),
    Int(i64),
    UInt(u64),
    Bits(Bits),
}
impl Value {
    pub fn to_lazy_dsl(&self) -> DslLazyValue {
        DslLazyValue::Constant(self.clone())
    }

    pub fn repeat(&self, n: usize) -> Array {
        match self {
            Self::Float(v) => Array::Floats(std::iter::repeat_n(*v, n).collect()),
            Self::Int(v) => Array::Ints(std::iter::repeat_n(*v, n).collect()),
            Self::UInt(v) => Array::UInts(std::iter::repeat_n(*v, n).collect()),
            Self::Bits(_) => todo!(),
        }
    }

    pub fn to_bits(&self, size: VectorSize) -> Bits {
        match self {
            Value::Float(_) => todo!(),
            Value::Int(_) => todo!(),
            Value::UInt(value) => Bits::from_u64(size, *value),
            Value::Bits(bits) => bits.clone(),
        }
    }
}
impl Eq for Value {}

#[derive(Clone, PartialEq)]
pub enum Array {
    Floats(Arc<[f64]>),
    Ints(Arc<[i64]>),
    UInts(Arc<[u64]>),
    Bits(Bits, NonZeroU32),
}

impl Eq for Array {}
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
        DslLazyArray::Constant(self.clone())
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

    pub fn as_u64(&self) -> Option<&Arc<[u64]>> {
        match self {
            Self::UInts(vs) => Some(vs),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<&Arc<[i64]>> {
        match self {
            Self::Ints(vs) => Some(vs),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ArrayAgg {
    Min,
}
impl ArrayAgg {
    fn compute(self, array: &Array) -> ComputeResult<Value> {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum LazyValue {
    Constant(Value),
    Aggregation(LazyArrayKey, ArrayAgg),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum LazyArray {
    Constant(Array),
    Random { seed: u64, length: usize },
}
impl LazyArray {
    pub fn len(&self) -> usize {
        match self {
            Self::Constant(arr) => arr.len(),
            Self::Random { seed: _, length } => *length,
        }
    }
}

#[derive(Clone)]
pub enum DslLazyValue {
    Constant(Value),
    Aggregation(Arc<DslLazyArray>, ArrayAgg),
}

#[derive(Clone)]
pub enum DslLazyArray {
    Constant(Array),
    Random { seed: u64, length: usize },
}

impl ComputeNode for LazyValue {
    type Output = Value;
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        match self {
            Self::Constant(..) => {}
            Self::Aggregation(arr, ..) => deps.arrays.push(*arr),
        }
    }
    fn compute(
        &self,
        _ctx: &ComputeContext,
        inputs: &ComputeInputs,
    ) -> ComputeResult<Self::Output> {
        Ok(match self {
            Self::Constant(v) => v.clone(),
            Self::Aggregation(v, agg) => agg.compute(&inputs.arrays[v])?,
        })
    }
}

impl ComputeNode for LazyArray {
    type Output = Array;
    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {
        match self {
            Self::Constant(..) => {}
            Self::Random { .. } => {}
        }
    }
    fn compute(
        &self,
        _ctx: &ComputeContext,
        _inputs: &ComputeInputs,
    ) -> ComputeResult<Self::Output> {
        use Array as A;
        Ok(match self {
            Self::Constant(arr) => arr.clone(),
            Self::Random { seed, length } => {
                let rng = <SmallRng as rand::SeedableRng>::seed_from_u64(*seed);
                A::UInts(rng.random_iter().take(*length).collect())
            }
        })
    }
}

impl DslNode for DslLazyValue {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        use LazyValue as V;
        let r = match self {
            Self::Constant(v) => V::Constant(v.clone()),
            Self::Aggregation(arr, agg) => V::Aggregation(
                converted[&DslPtr::from(arr.as_ref() as &dyn DslNode)].as_array(),
                *agg,
            ),
        };
        Key::Value(
            *csp.values
                .entry(r.clone())
                .or_insert_with(|| graph.values.insert(r)),
        )
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        match self {
            Self::Constant(_) => {}
            Self::Aggregation(v, _) => f.push(v.as_ref()),
        }
    }
}

impl DslNode for DslLazyArray {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        _converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        use LazyArray as V;
        let r = match self {
            Self::Constant(v) => V::Constant(v.clone()),
            Self::Random { seed, length } => V::Random {
                seed: *seed,
                length: *length,
            },
        };
        Key::Array(
            *csp.arrays
                .entry(r.clone())
                .or_insert_with(|| graph.arrays.insert(r)),
        )
    }

    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {
        match self {
            Self::Constant(_) => {}
            Self::Random { .. } => {}
        }
    }
}
