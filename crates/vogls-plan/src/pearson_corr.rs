use std::fmt;

use crate::agg::Agg;
use crate::array::Array;
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::typing::DataType;
use crate::value::Value;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PearsonCorrelation;

impl Agg for PearsonCorrelation {
    type Inputs<Input>
        = [Input; 2]
    where
        Input: Send + Sync;
    type Scratches = ();

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Mutual Information")
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
    ) -> ComputeResult<Value> {
        let [Array::UInts(ldata), Array::UInts(rdata)] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };

        let lmean = ldata.iter().map(|&v| v as f64).sum::<f64>() / ldata.len() as f64;
        let rmean = rdata.iter().map(|&v| v as f64).sum::<f64>() / rdata.len() as f64;

        let lstd = ldata
            .iter()
            .map(|&v| (v as f64 - lmean).powi(2))
            .sum::<f64>()
            .sqrt();
        let rstd = rdata
            .iter()
            .map(|&v| (v as f64 - rmean).powi(2))
            .sum::<f64>()
            .sqrt();

        let covar = ldata
            .iter()
            .zip(rdata.iter())
            .map(|(&l, &r)| (l as f64 - lmean) * (r as f64 - rmean))
            .sum::<f64>();

        let numerator = covar;
        let denumerator = lstd * rstd;

        let tvalue = numerator / denumerator;
        Ok(Value::Float(tvalue))
    }
}
