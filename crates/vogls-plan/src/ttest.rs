use std::fmt;

use crate::agg::Agg;
use crate::array::Array;
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::typing::DataType;
use crate::value::Value;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TTest {
    pub order: u32,
}

impl TTest {
    pub fn new(order: u32) -> Self {
        TTest { order }
    }
}

impl Default for TTest {
    fn default() -> Self {
        TTest { order: 1 }
    }
}

fn moment_stats(data: &[u64], order: u32) -> (f64, f64) {
    let n = data.len() as f64;
    let mean = data.iter().map(|&v| v as f64).sum::<f64>() / n;

    // Standard deviation is only needed to standardise orders >= 3.
    let std = if order >= 3 {
        let var = data.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
        var.sqrt()
    } else {
        1.0
    };

    // Pre-process each raw sample into the statistic whose group means we
    // ultimately compare with the t-test.
    let preprocess = |v: u64| -> f64 {
        let centered = v as f64 - mean;
        match order {
            1 => v as f64,
            2 => centered * centered,
            d => (centered / std).powi(d as i32),
        }
    };

    let preprocessed: Vec<f64> = data.iter().map(|&v| preprocess(v)).collect();
    let pmean = preprocessed.iter().sum::<f64>() / n;
    let psumsq = preprocessed
        .iter()
        .map(|&t| (t - pmean).powi(2))
        .sum::<f64>();
    (pmean, psumsq)
}

impl Agg for TTest {
    type Inputs<Input>
        = [Input; 2]
    where
        Input: Send + Sync;
    type Scratches = ();

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TTest(order={})", self.order)
    }

    fn output_type(&self, inputs: Self::Inputs<DataType>) -> ComputeResult<DataType> {
        let [DataType::UInt, DataType::UInt] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };
        Ok(DataType::Float)
    }

    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        _scratch: &mut Self::Scratches,
    ) -> ComputeResult<crate::value::Value> {
        let [Array::UInts(ldata), Array::UInts(rdata)] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };

        let (lmean, lsumsq) = moment_stats(&ldata, self.order);
        let (rmean, rsumsq) = moment_stats(&rdata, self.order);

        let numerator = lmean - rmean;
        let denumerator =
            (lsumsq / (ldata.len() as f64).powi(2) + rsumsq / (rdata.len() as f64).powi(2)).sqrt();

        let tvalue = numerator / denumerator;
        Ok(Value::Float(tvalue))
    }
}
