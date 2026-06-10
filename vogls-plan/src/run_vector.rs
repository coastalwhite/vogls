use std::hash::Hasher;
use std::sync::Arc;

use vogls::utils::{VgHashMap, new_table_key};

use crate::CspAble;
use crate::array::Array;
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeGraph, ComputeInputs, ComputeNode,
    ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::value::Value;

new_table_key! { pub struct LazyRunVectorKey; }

#[derive(Clone)]
pub struct DslRunVector(pub Arc<dyn DslRunVectorNode>);
#[derive(Clone)]
pub struct LazyRunVector(pub Arc<dyn RunVectorNode>);

pub trait DslRunVectorNode: Send + Sync + 'static {
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn RunVectorNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}
pub trait RunVectorNode: std::any::Any + Send + Sync + 'static {
    fn csp_eq(&self, other: &dyn RunVectorNode) -> bool;
    fn csp_hash(&self, state: &mut dyn Hasher);

    fn width(&self, graph: &ComputeGraph) -> RunWidth;
    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<RunVector>;
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum RunOffsets {
    Scalar(usize),
    Constant(u64, usize),
    Offsets(Buffer<u64>),
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum RunWidth {
    Scalar,
    Constant(u64),
    Variable,
}

impl RunOffsets {
    pub fn num_runs(&self) -> usize {
        match self {
            Self::Scalar(num_runs) | Self::Constant(_, num_runs) => *num_runs,
            Self::Offsets(buffer) => buffer.len(),
        }
    }

    pub fn width(&self) -> RunWidth {
        match self {
            Self::Scalar(..) => RunWidth::Scalar,
            Self::Constant(width, ..) => RunWidth::Constant(*width),
            Self::Offsets(..) => RunWidth::Variable,
        }
    }

    pub fn iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = (u64, u64)> + ExactSizeIterator + DoubleEndedIterator + 'a {
        (0..self.num_runs()).map(move |i| match self {
            RunOffsets::Scalar(_) => (i as u64, 1),
            RunOffsets::Constant(width, _) => (i as u64 * *width, *width),
            RunOffsets::Offsets(buffer) if i == 0 => (0, buffer[0]),
            RunOffsets::Offsets(buffer) => (buffer[i], buffer[i + 1] - buffer[i]),
        })
    }
}

impl RunWidth {
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable)
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct RunVector {
    pub offsets: RunOffsets,
    pub data: Array,
}

#[derive(Clone)]
pub enum RunValue {
    Scalar(Value),
    Array(Array),
}

impl ComputeNode for LazyRunVector {
    type Output = RunVector;

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        self.0.extend_inputs(deps);
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        self.0.compute(ctx, inputs)
    }
}

impl CspAble for LazyRunVector {
    fn csp_eq(&self, other: &Self) -> bool {
        self.0.csp_eq(other.0.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, mut state: &mut H) {
        self.0.csp_hash(&mut state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}

impl DslNode for DslRunVector {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyRunVector(self.0.convert_one(converted));
        Key::RunVector(csp.run_vectors.insert(&mut graph.run_vectors, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.0.extend_inputs(f)
    }
}

impl LazyRunVector {
    pub fn width(&self, graph: &ComputeGraph) -> RunWidth {
        self.0.width(graph)
    }
}
