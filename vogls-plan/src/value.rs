use std::any::Any;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use vogls::{Bits, VectorSize};
use vogls::utils::{new_table_key, VgHashMap};

use crate::CspAble;
use crate::array::Array;
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeGraph, ComputeInputs, ComputeNode,
    ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};

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
        DslLazyValue(Arc::new(self.clone()) as _)
    }

    pub fn repeat(&self, n: usize) -> Array {
        match self {
            Self::Float(v) => Array::Floats(Buffer::from_vec(std::iter::repeat_n(*v, n).collect())),
            Self::Int(v) => Array::Ints(Buffer::from_vec(std::iter::repeat_n(*v, n).collect())),
            Self::UInt(v) => Array::UInts(Buffer::from_vec(std::iter::repeat_n(*v, n).collect())),
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
    fn convert_one<'a>(
        &'a self,
        _converted: &'a VgHashMap<DslPtr, Key>,
    ) -> Arc<dyn ValueNode> {
        Arc::new(self.clone()) as _
    }
    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
impl ValueNode for Value {
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

#[derive(Clone)]
pub struct LazyValue(Arc<dyn ValueNode>);
#[derive(Clone)]
pub struct DslLazyValue(Arc<dyn DslValueNode>);

impl ComputeNode for LazyValue {
    type Output = Value;
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        self.0.extend_inputs(deps);
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        self.0.compute(ctx, inputs)
    }
}
impl CspAble for LazyValue {
    fn csp_eq(&self, other: &Self) -> bool {
        self.0.csp_eq(other.0.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, mut state: &mut H) {
        self.0.csp_hash(&mut state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}
impl DslNode for DslLazyValue {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyValue(self.0.convert_one(converted));
        Key::Value(csp.values.insert(&mut graph.values, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.0.extend_inputs(f);
    }
}

pub trait DslValueNode: Send + Sync + 'static {
    fn convert_one<'a>(
        &'a self,
        converted: &'a VgHashMap<DslPtr, Key>,
    ) -> Arc<dyn ValueNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}
pub trait ValueNode: std::any::Any + Send + Sync + 'static {
    fn csp_eq(&self, other: &dyn ValueNode) -> bool;
    fn csp_hash(&self, state: &mut dyn Hasher);

    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Value>;
}
