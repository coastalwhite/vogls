use std::fmt;
use std::sync::Arc;

use pyo3::types::PyAnyMethods;
use pyo3::{Py, PyAny};
use vogls_plan::array::Array;
use vogls_plan::compute::{ComputeContext, ComputeResult};
use vogls_plan::typing::{ArrayType, DataType};

use crate::vogls::PyArray;

#[derive(Clone)]
pub struct PyAnonymousMap {
    pub(crate) f: Arc<Py<PyAny>>,
}

impl std::hash::Hash for PyAnonymousMap {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}
impl PartialEq for PyAnonymousMap {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
impl Eq for PyAnonymousMap {}

impl vogls_plan::map::Map for PyAnonymousMap {
    // @TODO: Allow multiple arguments.
    type Inputs<Input>
        = [Input; 1]
    where
        Input: Send + Sync;
    type Scratches = ();

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Anonymous Python Function")
    }

    fn output_type(&self, _inputs: Self::Inputs<ArrayType>) -> ComputeResult<ArrayType> {
        // @TODO: Make these configurable.
        Ok(ArrayType {
            data: DataType::UInt,
            length: None,
        })
    }

    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        _scratch: &mut Self::Scratches,
    ) -> ComputeResult<Array> {
        pyo3::Python::attach(|py| {
            let [input] = inputs;
            let f = self.f.bind(py);
            let result = f.call1((PyArray(input),))?;
            let result = result
                .extract::<Py<PyArray>>()
                .map_err(<_ as Into<pyo3::PyErr>>::into)?;
            Ok(result.get().0.clone())
        })
    }
}
