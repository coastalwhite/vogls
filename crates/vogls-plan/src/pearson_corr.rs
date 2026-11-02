use std::fmt;

use crate::agg::Agg;
use crate::array::{Array, UIntElement};
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::typing::DataType;
use crate::value::Value;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PearsonCorrelation;

fn mean<T: UIntElement>(data: &[T]) -> f64 {
    data.iter().map(|&v| v.to_u64() as f64).sum::<f64>() / data.len() as f64
}

fn centered_norm<T: UIntElement>(data: &[T], mean: f64) -> f64 {
    data.iter()
        .map(|&v| (v.to_u64() as f64 - mean).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// The only two-array loop here, hence the one use of the 16-arm pair macro.
fn centered_covariance<T: UIntElement, U: UIntElement>(
    lhs: &[T],
    rhs: &[U],
    lmean: f64,
    rmean: f64,
) -> f64 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(&l, &r)| (l.to_u64() as f64 - lmean) * (r.to_u64() as f64 - rmean))
        .sum::<f64>()
}

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
        // Read-only and widens to `f64` anyway: every width is accepted.
        let [DataType::UInt(_), DataType::UInt(_)] = inputs else {
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

        let lmean = with_match_uint_type!(ldata.ty(), |$T| mean(ldata.as_slice::<$T>()));
        let rmean = with_match_uint_type!(rdata.ty(), |$T| mean(rdata.as_slice::<$T>()));

        let lstd =
            with_match_uint_type!(ldata.ty(), |$T| centered_norm(ldata.as_slice::<$T>(), lmean));
        let rstd =
            with_match_uint_type!(rdata.ty(), |$T| centered_norm(rdata.as_slice::<$T>(), rmean));

        let covar = with_match_uint_type_pair!(ldata.ty(), rdata.ty(), |$T, $U| {
            centered_covariance(
                ldata.as_slice::<$T>(),
                rdata.as_slice::<$U>(),
                lmean,
                rmean,
            )
        });

        let numerator = covar;
        let denumerator = lstd * rstd;

        let tvalue = numerator / denumerator;
        Ok(Value::Float(tvalue))
    }
}
