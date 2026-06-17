use std::fmt;
use std::sync::Arc;

use vogls::utils::VgHashMap;

use crate::array::{Array, ArrayNode, DslArrayNode, DslLazyArray, LazyArrayKey};
use crate::compute::{
    ComputeContext, ComputeDependencies, ComputeError, ComputeInputs, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::run_vector::{DslRunVector, LazyRunVectorKey, RunVector};
use crate::typing::{ArrayType, DataType, RunWidth, Type, ValueType};
use crate::value::{DslLazyValue, DslValueNode, Value, ValueNode};

pub struct AggNode<N: Send + Sync, T: Agg> {
    inputs: T::Inputs<N>,
    inner: T,
}

pub fn build_run_vector_agg<T: Agg>(
    agg: T,
    inputs: T::Inputs<DslRunVector>,
) -> ComputeResult<DslLazyArray>
where
    T::Inputs<LazyRunVectorKey>: std::hash::Hash + Eq,
{
    let mut width = None;
    for input in inputs.iter() {
        let RunWidth::Constant(input_width) = input.ty().width else {
            return Err(ComputeError::NumTracesMismatch);
        };
        if width.is_some_and(|w| w != input_width) {
            return Err(ComputeError::NumTracesMismatch);
        }
        width = Some(input_width);
    }
    let width = width.unwrap();
    let input_dtypes =
        <T::Inputs<DataType> as InputCollection>::from_iter(inputs.iter().map(|v| v.ty().data));
    let output_dtype = agg.output_type(input_dtypes)?;
    let ty = Arc::new(Type::Array(ArrayType {
        data: output_dtype,
        length: Some(width as usize),
    }));
    Ok(DslLazyArray {
        ty,
        f: Arc::new(AggNode { inputs, inner: agg }),
    })
}

pub fn build_array_agg<T: Agg>(
    agg: T,
    inputs: T::Inputs<DslLazyArray>,
) -> ComputeResult<DslLazyValue>
where
    T::Inputs<LazyArrayKey>: std::hash::Hash + Eq,
{
    let input_dtypes =
        <T::Inputs<DataType> as InputCollection>::from_iter(inputs.iter().map(|v| v.ty().data));
    let output_dtype = agg.output_type(input_dtypes)?;
    let ty = Arc::new(Type::Value(ValueType { data: output_dtype }));
    Ok(DslLazyValue {
        ty,
        f: Arc::new(AggNode { inputs, inner: agg }),
    })
}

pub trait Agg: std::hash::Hash + Eq + Clone + Send + Sync + 'static {
    type Inputs<Input>: InputCollection<Item = Input>
    where
        Input: Send + Sync;
    type Scratches: Default;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn output_type(&self, inputs: Self::Inputs<DataType>) -> ComputeResult<DataType>;
    fn eval(
        &self,
        scratch: &mut Self::Scratches,
        inputs: Self::Inputs<Array>,
    ) -> ComputeResult<Value>;
}

pub trait InputCollection: Send + Sync {
    type Item: Send + Sync;
    fn iter(&self) -> impl Iterator<Item = &Self::Item>;
    fn from_iter<I: Iterator<Item = Self::Item>>(iter: I) -> Self;
}

impl<const N: usize, T: Send + Sync> InputCollection for [T; N] {
    type Item = T;
    fn iter(&self) -> impl Iterator<Item = &Self::Item> {
        self.as_slice().iter()
    }

    fn from_iter<I: Iterator<Item = Self::Item>>(mut iter: I) -> Self {
        let slf = std::array::from_fn(|_| iter.by_ref().next().unwrap());
        assert!(iter.next().is_none());
        slf
    }
}

impl<T: Agg> DslArrayNode for AggNode<DslRunVector, T>
where
    T::Inputs<LazyRunVectorKey>: std::hash::Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(&self.inner, f)
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode> {
        let inputs = <T::Inputs<_> as InputCollection>::from_iter(self.inputs.iter().map(|i| {
            let ptr = DslPtr::from(i as &dyn DslNode);
            converted[&ptr].as_run_vector()
        }));
        Arc::new(AggNode {
            inputs,
            inner: self.inner.clone(),
        })
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.extend(self.inputs.iter().map(|v| v as &dyn DslNode))
    }
}
impl<T: Agg> ArrayNode for AggNode<LazyRunVectorKey, T>
where
    T::Inputs<LazyRunVectorKey>: std::hash::Hash + Eq,
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
        deps.run_vectors.extend(self.inputs.iter().copied());
    }

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Array> {
        let inputs = <T::Inputs<RunVector> as InputCollection>::from_iter(
            self.inputs
                .iter()
                .map(|key| inputs.run_vectors[key].clone()),
        );

        let mut width = None;
        for input in inputs.iter() {
            let RunWidth::Constant(input_width) = input.offsets.width() else {
                return Err(ComputeError::NumTracesMismatch);
            };
            if width.is_some_and(|w| w != input_width) {
                return Err(ComputeError::NumTracesMismatch);
            }
            width = Some(input_width);
        }
        let width = width.unwrap();

        let input_dtypes =
            <T::Inputs<DataType> as InputCollection>::from_iter(inputs.iter().map(|v| v.ty().data));
        let output_dtype = self.inner.output_type(input_dtypes)?;
        let arr_type = ArrayType {
            data: output_dtype,
            length: Some(width as usize),
        };

        let mut scratch = T::Scratches::default();
        Array::try_from_value_iter(
            &arr_type,
            // @Performance: Parallelize this.
            (0..width).map(|x| {
                let inputs = <T::Inputs<Array> as InputCollection>::from_iter(
                    inputs
                        .iter()
                        .map(|input| input.gather_array_at_x(x as usize)),
                );
                T::eval(&self.inner, &mut scratch, inputs)
            }),
        )
    }
}

impl<T: Agg> DslValueNode for AggNode<DslLazyArray, T>
where
    T::Inputs<LazyArrayKey>: std::hash::Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(&self.inner, f)
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ValueNode> {
        let inputs = <T::Inputs<_> as InputCollection>::from_iter(self.inputs.iter().map(|i| {
            let ptr = DslPtr::from(i as &dyn DslNode);
            converted[&ptr].as_array()
        }));
        Arc::new(AggNode {
            inputs,
            inner: self.inner.clone(),
        })
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.extend(self.inputs.iter().map(|v| v as &dyn DslNode))
    }
}
impl<T: Agg> ValueNode for AggNode<LazyArrayKey, T>
where
    T::Inputs<LazyArrayKey>: std::hash::Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(&self.inner, f)
    }

    fn csp_eq(&self, other: &dyn ValueNode) -> bool {
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

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Value> {
        let mut scratch = T::Scratches::default();
        let inputs = <T::Inputs<Array> as InputCollection>::from_iter(
            self.inputs.iter().map(|key| inputs.arrays[key].clone()),
        );
        T::eval(&self.inner, &mut scratch, inputs)
    }
}
