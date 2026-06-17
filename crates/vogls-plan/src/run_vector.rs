use std::fmt;
use std::hash::{Hash as _, Hasher};
use std::sync::Arc;

use vogls::utils::{VgHashMap, new_table_key};

use crate::CspAble;
use crate::array::Array;
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::output::{DslLazyOutput, LazyOutputKey, Output};
use crate::typing::{RunVectorType, RunWidth, Type};
use crate::value::Value;

new_table_key! { pub struct LazyRunVectorKey; }

#[derive(Clone)]
pub struct DslRunVector {
    pub ty: Arc<Type>,
    pub f: Arc<dyn DslRunVectorNode>,
}
impl DslRunVector {
    pub fn ty(&self) -> &RunVectorType {
        let Type::RunVector(t) = self.ty.as_ref() else {
            unreachable!()
        };
        t
    }
}
#[derive(Clone)]
pub struct LazyRunVector {
    pub ty: Arc<Type>,
    pub f: Arc<dyn RunVectorNode>,
}

pub trait DslRunVectorNode: Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn RunVectorNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}
pub trait RunVectorNode: std::any::Any + Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
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
            RunOffsets::Offsets(buffer) => (buffer[i - 1], buffer[i] - buffer[i - 1]),
        })
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
    type Key = LazyRunVectorKey;
    type Output = RunVector;

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

impl CspAble for LazyRunVector {
    fn csp_eq(&self, other: &Self) -> bool {
        self.f.csp_eq(other.f.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, mut state: &mut H) {
        self.f.type_id().hash(state);
        self.f.csp_hash(&mut state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}

impl DslNode for DslRunVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.f.fmt(f)
    }
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyRunVector {
            ty: self.ty.clone(),
            f: self.f.convert_one(converted),
        };
        Key::RunVector(csp.run_vectors.insert(&mut graph.run_vectors, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.f.extend_inputs(f)
    }
}

impl RunVector {
    pub fn array_iter(&self) -> impl Iterator<Item = Array> {
        self.offsets
            .iter()
            .map(|(offset, length)| self.data.slice(offset as usize, length as usize))
    }

    pub fn ty(&self) -> RunVectorType {
        RunVectorType {
            data: self.data.data_type(),
            length: Some(self.offsets.num_runs()),
            width: self.offsets.width(),
        }
    }

    pub fn gather_array_at_x(&self, x: usize) -> Array {
        use Array as A;
        use RunOffsets as O;
        let height = self.offsets.num_runs();
        match (&self.data, &self.offsets) {
            (_, O::Scalar(_)) => self.data.clone(),

            (A::Floats(b), O::Constant(stride, _)) => {
                A::Floats(gather_strided(&b, height, *stride as usize, x))
            }
            (A::Ints(b), O::Constant(stride, _)) => {
                A::Ints(gather_strided(&b, height, *stride as usize, x))
            }
            (A::UInts(b), O::Constant(stride, _)) => {
                A::UInts(gather_strided(&b, height, *stride as usize, x))
            }

            (A::Floats(b), O::Offsets(offsets)) => A::Floats(gather_offsets(&b, &offsets, x)),
            (A::Ints(b), O::Offsets(offsets)) => A::Ints(gather_offsets(&b, &offsets, x)),
            (A::UInts(b), O::Offsets(offsets)) => A::UInts(gather_offsets(&b, &offsets, x)),

            (A::Bits(..), _) => todo!(),
        }
    }
}

fn gather_strided<T: Copy>(slice: &[T], height: usize, stride: usize, offset: usize) -> Buffer<T> {
    (0..height)
        .map(|y| slice[offset + y * stride as usize])
        .collect()
}
fn gather_offsets<T: Copy>(slice: &[T], offsets: &[u64], offset: usize) -> Buffer<T> {
    offsets
        .iter()
        // @TODO: We are ignoring OOB here.
        .map(|&s| slice[s as usize + offset])
        .collect()
}

impl LazyRunVector {
    pub fn width(&self) -> RunWidth {
        let Type::RunVector(ty) = self.ty.as_ref() else {
            unreachable!()
        };
        ty.width
    }
}

pub struct DslRunVectorExtractOutput(pub DslLazyOutput);
#[derive(PartialEq, Eq, Hash)]
pub struct RunVectorExtractOutput(pub LazyOutputKey);

impl DslRunVectorNode for DslRunVectorExtractOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Extract RunVector")
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn RunVectorNode> {
        let output = converted[&DslPtr::from(&self.0 as &dyn DslNode)].as_output();
        Arc::new(RunVectorExtractOutput(output)) as _
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(&self.0);
    }
}
impl RunVectorNode for RunVectorExtractOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Extract RunVector")
    }
    impl_dyn_eq_hash!(RunVectorNode);
    fn width(&self, _graph: &ComputeGraph) -> RunWidth {
        // @TODO
        RunWidth::Variable
    }

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.outputs.push(self.0);
    }

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<RunVector> {
        match &inputs.outputs[&self.0] {
            Output::RunVector(v) => Ok(v.clone()),
            Output::Plan(_) => panic!("plan!"),
            Output::Array(_) => panic!("array!"),
            Output::Value(_) => panic!("value!"),
        }
    }
}
