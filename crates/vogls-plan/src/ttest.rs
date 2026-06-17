use std::fmt;

use crate::agg::Agg;
use crate::array::Array;
use crate::compute::{ComputeError, ComputeResult};
use crate::typing::DataType;
use crate::value::Value;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TTest;

impl Agg for TTest {
    type Inputs<Input>
        = [Input; 2]
    where
        Input: Send + Sync;
    type Scratches = ();

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TTest")
    }

    fn output_type(&self, inputs: Self::Inputs<DataType>) -> ComputeResult<DataType> {
        let [DataType::UInt, DataType::UInt] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };
        Ok(DataType::Float)
    }

    fn eval(
        &self,
        _scratch: &mut Self::Scratches,
        inputs: Self::Inputs<Array>,
    ) -> ComputeResult<crate::value::Value> {
        let [Array::UInts(ldata), Array::UInts(rdata)] = inputs else {
            return Err(ComputeError::InvalidTypes);
        };

        let lmean = ldata.iter().map(|&v| v as f64).sum::<f64>() / ldata.len() as f64;
        let rmean = rdata.iter().map(|&v| v as f64).sum::<f64>() / rdata.len() as f64;

        let lvar = ldata
            .iter()
            .map(|&v| (v as f64 - lmean).powi(2))
            .sum::<f64>();
        let rvar = rdata
            .iter()
            .map(|&v| (v as f64 - rmean).powi(2))
            .sum::<f64>();

        let numerator = lmean - rmean;
        let denumerator = (lvar / ldata.len() as f64 + rvar / rdata.len() as f64).sqrt();

        let tvalue = numerator / denumerator;
        Ok(Value::Float(tvalue))
    }
}
