use std::fmt;

use vogls::utils::IndexMap;

use crate::agg::Agg;
use crate::array::Array;
use crate::compute::{ComputeError, ComputeResult};
use crate::entropy::prenormalized_entropy;
use crate::typing::DataType;
use crate::value::Value;

const NUM_BREAKS: usize = 64;

fn cut(data: impl Iterator<Item = u64> + Clone, out: &mut Vec<u64>) {
    out.clear();
    let (Some(min), Some(max)) = (data.clone().min(), data.clone().max()) else {
        return;
    };

    let min = min as f64;
    let max = max as f64;
    let width = (max - min) / (NUM_BREAKS as f64);

    out.extend(data.map(|v| {
        let v = v as f64;
        if v.is_nan() || v.is_infinite() {
            return 0;
        }

        let idx = ((v - min) / width).floor() as u64;
        let idx = idx.min(NUM_BREAKS as u64 - 1);
        idx
    }));
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct MutualInformation;
#[derive(Default)]
pub struct MutualInformationScratches {
    bins_a: Vec<u64>,
    bins_b: Vec<u64>,

    da: IndexMap<u64, u64>,
    db: IndexMap<u64, u64>,
    dab: IndexMap<(u64, u64), u64>,
}

impl Agg for MutualInformation {
    type Inputs<Input>
        = [Input; 2]
    where
        Input: Send + Sync;
    type Scratches = MutualInformationScratches;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Mutual Information")
    }
    fn output_type(&self, inputs: Self::Inputs<DataType>) -> ComputeResult<DataType> {
        let [DataType::UInt, DataType::UInt] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };
        Ok(DataType::Float)
    }
    fn eval(
        &self,
        scratch: &mut Self::Scratches,
        inputs: Self::Inputs<Array>,
    ) -> ComputeResult<Value> {
        let [Array::UInts(lhs), Array::UInts(rhs)] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };

        assert_eq!(lhs.len(), rhs.len());

        scratch.bins_a.clear();
        scratch.bins_b.clear();
        scratch.da.clear();
        scratch.db.clear();
        scratch.dab.clear();

        cut(lhs.iter().copied(), &mut scratch.bins_a);
        cut(rhs.iter().copied(), &mut scratch.bins_b);

        for (&a, &b) in scratch.bins_a.iter().zip(&scratch.bins_b) {
            *scratch.da.entry(a).or_default() += 1;
            *scratch.db.entry(b).or_default() += 1;
            *scratch.dab.entry((a, b)).or_default() += 1;
        }

        // Rely on the fact:
        // I(X, Y)
        //   = H(X) - H(X|Y)
        //   = H(X) + H(Y) - H(X,Y)

        let length_f = lhs.len() as f64;
        let dae = prenormalized_entropy(scratch.da.iter_values().map(|&v| v as f64 / length_f));
        let dbe = prenormalized_entropy(scratch.db.iter_values().map(|&v| v as f64 / length_f));
        let dabe = prenormalized_entropy(scratch.dab.iter_values().map(|&v| v as f64 / length_f));

        Ok(Value::Float(dae + dbe - dabe))
    }
}
