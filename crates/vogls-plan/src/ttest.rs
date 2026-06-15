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

fn size_mean_var(v: &[u64]) -> (f64, f64, f64) {
    // @TODO: Better summation strategy.

    let sum = v.iter().map(|v| *v as f64).sum::<f64>();
    let size = v.len() as f64;
    let mean = sum / size;
    let var = v.iter().map(|&v| ((v as f64) - mean).powi(2)).sum::<f64>() / size;

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

        let (Array::UInts(ldata), Array::UInts(rdata)) = (&lhs.data, &rhs.data) else {
            return Err(ComputeError::InvalidTypes);
        };

        let RunWidth::Constant(lwidth) = lhs.offsets.width() else {
            unreachable!();
        };
        let RunWidth::Constant(rwidth) = rhs.offsets.width() else {
            unreachable!();
        };

        assert_eq!(lwidth, rwidth);

        let lsize = lhs.offsets.num_runs() as f64;
        let rsize = rhs.offsets.num_runs() as f64;

        let mut lmean = vec![0f64; lwidth as usize];
        let mut rmean = vec![0f64; lwidth as usize];

        for (o, w) in lhs.offsets.iter() {
            let data = &ldata[o as usize..][..w as usize];
            for (i, &x) in data.iter().enumerate() {
                lmean[i] += x as f64;
            }
        }
        for (o, w) in rhs.offsets.iter() {
            let data = &rdata[o as usize..][..w as usize];
            for (i, &x) in data.iter().enumerate() {
                rmean[i] += x as f64;
            }
        }

        lmean.iter_mut().for_each(|v| *v = *v / lsize);
        rmean.iter_mut().for_each(|v| *v = *v / rsize);

        let mut lvar = vec![0f64; lwidth as usize];
        let mut rvar = vec![0f64; lwidth as usize];

        for (o, w) in lhs.offsets.iter() {
            let data = &ldata[o as usize..][..w as usize];
            for (i, &x) in data.iter().enumerate() {
                lvar[i] += x as f64 - lmean[i];
            }
        }
        for (o, w) in rhs.offsets.iter() {
            let data = &rdata[o as usize..][..w as usize];
            for (i, &x) in data.iter().enumerate() {
                rvar[i] += (x as f64 - rmean[i]).powi(2);
            }
        }

        let tvalue = (0..lwidth as usize)
            .map(|i| {
                let numerator = lmean[i] - rmean[i];
                let denumerator = (lvar[i] / lsize + rvar[i] / rsize).sqrt();

                let tvalue = numerator / denumerator;
                tvalue
            })
            .collect();

        Ok(Array::Floats(tvalue))
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
