use std::any::Any;
use std::hash::Hasher;
use std::sync::Arc;

use vogls::utils::VgHashMap;

use crate::array::{Array, ArrayNode, DslArrayNode};
use crate::buffer::Buffer;
use crate::compute::{
    ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs, ComputeResult,
    Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::run_vector::{DslRunVector, LazyRunVectorKey, RunWidth};

pub struct TTest {
    pub lhs: DslRunVector,
    pub rhs: DslRunVector,
}

#[derive(PartialEq, Hash)]
pub struct LazyTTest {
    lhs: LazyRunVectorKey,
    rhs: LazyRunVectorKey,
}

fn size_mean_var(v: &[f64]) -> (f64, f64, f64) {
    // @TODO: Better summation strategy.

    let sum = v.iter().sum::<f64>();
    let size = v.len() as f64;
    let mean = sum / size;
    let var = v.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / size;

    (size, mean, var)
}

impl ArrayNode for LazyTTest {
    fn csp_eq(&self, other: &dyn ArrayNode) -> bool {
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };

        self == other
    }

    fn csp_hash(&self, mut state: &mut dyn Hasher) {
        use std::hash::Hash;
        self.hash(&mut state);
    }

    fn len(&self, graph: &ComputeGraph) -> Option<usize> {
        let lwidth = graph.run_vectors[self.lhs].width(graph);
        let rwidth = graph.run_vectors[self.rhs].width(graph);

        let (RunWidth::Constant(lwidth), RunWidth::Constant(rwidth)) = (lwidth, rwidth) else {
            return None;
        };

        assert_eq!(lwidth, rwidth);
        Some(lwidth as usize)
    }

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.run_vectors.extend([self.lhs, self.rhs]);
    }

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Array> {
        let lhs = &inputs.run_vectors[&self.lhs];
        let rhs = &inputs.run_vectors[&self.rhs];

        assert_eq!(lhs.offsets.num_runs(), rhs.offsets.num_runs());
        let (Array::Floats(ldata), Array::Floats(rdata)) = (&lhs.data, &rhs.data) else {
            return Err(ComputeError::InvalidTypes);
        };

        let mut output = Vec::with_capacity(lhs.offsets.num_runs());
        for ((lo, lw), (ro, rw)) in lhs.offsets.iter().zip(rhs.offsets.iter()) {
            let (lsize, lmean, lvar) = size_mean_var(&ldata[lo as usize..][..lw as usize]);
            let (rsize, rmean, rvar) = size_mean_var(&rdata[ro as usize..][..rw as usize]);

            let numerator = lmean - rmean;
            let denumerator = (lvar / lsize + rvar / rsize).sqrt();

            let tvalue = numerator / denumerator;
            output.push(tvalue);
        }

        Ok(Array::Floats(Buffer::from_vec(output)))
    }
}

impl DslArrayNode for TTest {
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn ArrayNode> {
        let lhs = converted[&DslPtr::from(&self.lhs as &dyn DslNode)].as_run_vector();
        let rhs = converted[&DslPtr::from(&self.rhs as &dyn DslNode)].as_run_vector();
        Arc::new(LazyTTest { lhs, rhs })
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(&self.lhs as _);
        f.push(&self.rhs as _);
    }
}
