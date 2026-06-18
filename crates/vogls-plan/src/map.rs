use std::fmt;
use std::sync::Arc;

use vogls::utils::VgHashMap;

use crate::agg::InputCollection;
use crate::array::{Array, ArrayNode, DslArrayNode, DslLazyArray, LazyArrayKey, new_array_builder};
use crate::buffer::Buffer;
use crate::compute::{
    ComputeContext, ComputeDependencies, ComputeError, ComputeInputs, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::run_vector::{
    DslRunVector, DslRunVectorNode, LazyRunVectorKey, RunOffsets, RunVector, RunVectorNode,
};
use crate::typing::{ArrayType, RunVectorType, RunWidth, Type};

pub struct MapNode<N: Send + Sync, T: Map> {
    inputs: T::Inputs<N>,
    inner: T,
}

pub fn build_run_vector_map<T: Map>(
    map: T,
    inputs: T::Inputs<DslRunVector>,
) -> ComputeResult<DslRunVector>
where
    T::Inputs<LazyRunVectorKey>: std::hash::Hash + Eq,
{
    let mut length = None;
    for input in inputs.iter() {
        if let Some(input_length) = input.ty().length {
            if length.is_some_and(|l| l as usize != input_length) {
                return Err(ComputeError::NumTracesMismatch);
            }

            length = Some(input_length);
        }
    }
    let input_dtypes =
        <T::Inputs<ArrayType> as InputCollection>::from_iter(inputs.iter().map(|v| ArrayType {
            data: v.ty().data,
            length: v.ty().width.size(),
        }));
    let output_dtype = map.output_type(input_dtypes)?;
    let ty = Arc::new(Type::RunVector(RunVectorType {
        data: output_dtype.data,
        width: match output_dtype.length {
            Some(n) => RunWidth::Constant(n as u64),
            None => RunWidth::Variable,
        },
        length: length,
    }));
    Ok(DslRunVector {
        ty,
        f: Arc::new(MapNode { inputs, inner: map }),
    })
}

pub fn build_array_map<T: Map>(
    map: T,
    inputs: T::Inputs<DslLazyArray>,
) -> ComputeResult<DslLazyArray>
where
    T::Inputs<LazyArrayKey>: std::hash::Hash + Eq,
{
    let input_dtypes =
        <T::Inputs<ArrayType> as InputCollection>::from_iter(inputs.iter().map(|v| v.ty().clone()));
    let output_dtype = map.output_type(input_dtypes)?;
    let ty = Arc::new(Type::Array(output_dtype));
    Ok(DslLazyArray {
        ty,
        f: Arc::new(MapNode { inputs, inner: map }),
    })
}

pub trait Map: std::hash::Hash + Eq + Clone + Send + Sync + 'static {
    /// The inputs to the map.
    ///
    /// Generally, this should be a fixed-size array over the `Input` generic parameter.
    type Inputs<Input>: InputCollection<Item = Input>
    where
        Input: Send + Sync;

    /// Scratchpad structures that can be reused between evaluations.
    type Scratches: Default;

    /// Format the information relevant to the map.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// Get the output type for the map given the input types.
    fn output_type(&self, inputs: Self::Inputs<ArrayType>) -> ComputeResult<ArrayType>;

    /// Compute the map.
    ///
    /// Scratches may not be cleared.
    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        ctx: &ComputeContext,
        scratch: &mut Self::Scratches,
    ) -> ComputeResult<Array>;
}

impl<T: Map> DslRunVectorNode for MapNode<DslRunVector, T>
where
    T::Inputs<LazyRunVectorKey>: std::hash::Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(&self.inner, f)
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn RunVectorNode> {
        let inputs = <T::Inputs<_> as InputCollection>::from_iter(self.inputs.iter().map(|i| {
            let ptr = DslPtr::from(i as &dyn DslNode);
            converted[&ptr].as_run_vector()
        }));
        Arc::new(MapNode {
            inputs,
            inner: self.inner.clone(),
        })
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.extend(self.inputs.iter().map(|v| v as &dyn DslNode))
    }
}
impl<T: Map> RunVectorNode for MapNode<LazyRunVectorKey, T>
where
    T::Inputs<LazyRunVectorKey>: std::hash::Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(&self.inner, f)
    }

    fn csp_eq(&self, other: &dyn RunVectorNode) -> bool {
        let Some(other) = (other as &dyn std::any::Any).downcast_ref::<Self>() else {
            return false;
        };
        self.inputs == other.inputs && self.inner == other.inner
    }
    fn csp_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        std::hash::Hash::hash(&self.inputs, &mut state);
        std::hash::Hash::hash(&self.inner, &mut state);
    }

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.run_vectors.extend(self.inputs.iter().copied());
    }

    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<RunVector> {
        let inputs = <T::Inputs<RunVector> as InputCollection>::from_iter(
            self.inputs
                .iter()
                .map(|key| inputs.run_vectors[key].clone()),
        );

        let mut num_runs = None;
        for input in inputs.iter() {
            let input_num_runs = input.offsets.num_runs();
            if num_runs.is_some_and(|w| w != input_num_runs) {
                return Err(ComputeError::NumTracesMismatch);
            }
            num_runs = Some(input_num_runs);
        }
        let num_runs = num_runs.unwrap();

        let input_dtypes =
            <T::Inputs<ArrayType> as InputCollection>::from_iter(inputs.iter().map(|v| {
                let RunVectorType {
                    data,
                    length: _,
                    width,
                } = v.ty();
                let length = match width {
                    RunWidth::Variable => None,
                    RunWidth::Scalar => Some(1),
                    RunWidth::Constant(n) => Some(n as usize),
                };
                ArrayType { data, length }
            }));
        let output_dtype = self.inner.output_type(input_dtypes)?;

        let mut scratch = T::Scratches::default();
        let mut builder = new_array_builder(&output_dtype.data);
        let mut offsets = match output_dtype.length {
            None => Some(Vec::with_capacity(num_runs)),
            Some(_) => None,
        };

        if let Some(width) = output_dtype.length {
            builder.reserve(width * num_runs);
        }

        for i in 0..num_runs {
            let inputs = <T::Inputs<Array> as InputCollection>::from_iter(
                inputs.iter().map(|rv| rv.get_at_y(i)),
            );
            builder.extend(&T::compute(&self.inner, inputs, ctx, &mut scratch)?);
            offsets.as_mut().map(|v| v.push(builder.len() as u64));
        }

        let offsets = match output_dtype.length {
            None => RunOffsets::Offsets(Buffer::from_vec(offsets.unwrap())),
            Some(w) => RunOffsets::Constant(w as u64, num_runs),
        };
        let data = builder.finish();
        Ok(RunVector { offsets, data })
    }
}

impl<T: Map> DslArrayNode for MapNode<DslLazyArray, T>
where
    T::Inputs<LazyArrayKey>: std::hash::Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(&self.inner, f)
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode> {
        let inputs = <T::Inputs<_> as InputCollection>::from_iter(self.inputs.iter().map(|i| {
            let ptr = DslPtr::from(i as &dyn DslNode);
            converted[&ptr].as_array()
        }));
        Arc::new(MapNode {
            inputs,
            inner: self.inner.clone(),
        })
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.extend(self.inputs.iter().map(|v| v as &dyn DslNode))
    }
}
impl<T: Map> ArrayNode for MapNode<LazyArrayKey, T>
where
    T::Inputs<LazyArrayKey>: std::hash::Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(&self.inner, f)
    }

    fn csp_eq(&self, other: &dyn ArrayNode) -> bool {
        let Some(other) = (other as &dyn std::any::Any).downcast_ref::<Self>() else {
            return false;
        };
        self.inputs == other.inputs && self.inner == other.inner
    }
    fn csp_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        std::hash::Hash::hash(&self.inputs, &mut state);
        std::hash::Hash::hash(&self.inner, &mut state);
    }

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.arrays.extend(self.inputs.iter().copied());
    }

    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Array> {
        let mut scratch = T::Scratches::default();
        let inputs = <T::Inputs<Array> as InputCollection>::from_iter(
            self.inputs.iter().map(|key| inputs.arrays[key].clone()),
        );
        T::compute(&self.inner, inputs, ctx, &mut scratch)
    }
}
