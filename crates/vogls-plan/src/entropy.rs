use std::fmt;

use vogls::utils::IndexMap;

use crate::agg::Agg;
use crate::array::Array;
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::typing::DataType;
use crate::value::Value;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Entropy;

#[derive(Default)]
pub struct EntropyScratches {
    unique_counts: IndexMap<u64, u64>,
}

pub(crate) fn prenormalized_entropy(iter: impl Iterator<Item = f64>) -> f64 {
    -iter.map(|v| v * v.log2()).sum::<f64>()
}

impl Agg for Entropy {
    type Inputs<Input>
        = [Input; 1]
    where
        Input: Send + Sync;

    type Scratches = EntropyScratches;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Entropy")
    }

    fn output_type(&self, inputs: Self::Inputs<DataType>) -> ComputeResult<DataType> {
        let [DataType::UInt] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };
        Ok(DataType::Float)
    }
    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        scratch: &mut Self::Scratches,
    ) -> ComputeResult<Value> {
        let [Array::UInts(data)] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };
        scratch.unique_counts.clear();

        for &v in data.iter() {
            *scratch.unique_counts.entry(v).or_default() += 1;
        }

        Ok(Value::Float(prenormalized_entropy(
            scratch
                .unique_counts
                .iter_values()
                .map(|&v| v as f64 / data.len() as f64),
        )))
    }
}
