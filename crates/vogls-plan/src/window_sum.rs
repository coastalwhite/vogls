use std::any::Any;
use std::hash::Hasher;
use std::sync::Arc;

use vogls::utils::VgHashMap;

use crate::array::Array;
use crate::buffer::Buffer;
use crate::compute::{
    ComputeContext, ComputeDependencies, ComputeError, ComputeInputs, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::run_vector::{
    DslRunVector, DslRunVectorNode, LazyRunVectorKey, RunOffsets, RunVector, RunVectorNode,
};
use crate::typing::{RunVectorType, RunWidth, Type};

pub struct WindowSum {
    pub on: DslRunVector,
    pub by: DslRunVector,

    pub start: u64,
    pub end: u64,
    pub width: u64,
}

impl WindowSum {
    pub fn build(self) -> DslRunVector {
        let diff = self.end - self.start;
        let width = RunWidth::Constant(diff.div_ceil(self.width));
        DslRunVector {
            ty: Arc::new(Type::RunVector(RunVectorType {
                data: self.on.ty().data,
                length: self.on.ty().length,
                width,
            })),
            f: Arc::new(self),
        }
    }
}

#[derive(PartialEq, Hash)]
pub struct LazyWindowSum {
    on: LazyRunVectorKey,
    by: LazyRunVectorKey,

    start: u64,
    end: u64,
    width: u64,
}

impl RunVectorNode for LazyWindowSum {
    fn csp_eq(&self, other: &dyn RunVectorNode) -> bool {
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
    }

    fn csp_hash(&self, mut state: &mut dyn Hasher) {
        use std::hash::Hash;
        self.hash(&mut state);
    }

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.run_vectors.extend([self.on, self.by]);
    }

    fn width(&self, _graph: &crate::compute::ComputeGraph) -> RunWidth {
        assert!(self.start <= self.end);
        assert!(self.width > 0);

        let diff = self.end - self.start;
        RunWidth::Constant(diff.div_ceil(self.width))
    }

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<RunVector> {
        let on = &inputs.run_vectors[&self.on];
        let by = &inputs.run_vectors[&self.by];

        assert_eq!(on.offsets.num_runs(), by.offsets.num_runs());

        let on_offsets = &on.offsets;
        let by_offsets = &by.offsets;

        let num_runs = on.offsets.num_runs();
        let (Array::UInts(on), Array::UInts(by)) = (&on.data, &by.data) else {
            return Err(ComputeError::InvalidTypes);
        };

        assert!(self.start <= self.end);
        assert!(self.width > 0);

        let diff = self.end - self.start;
        let num_bins = diff.div_ceil(self.width);

        let mut sums = Vec::with_capacity(num_bins as usize * num_runs);

        for ((on_offset, on_width), (by_offset, by_width)) in
            on_offsets.iter().zip(by_offsets.iter())
        {
            assert_eq!(on_width, by_width);
            let by = &by[by_offset as usize..][..by_width as usize];
            let on = &on[on_offset as usize..][..on_width as usize];

            let mut inner_offset = 0;
            sums.extend((0..num_bins).into_iter().map(|i| {
                let start = i * self.width;
                let end = ((i + 1) * self.width).min(self.end);
                let mut sum = 0u64;

                while let Some(&by) = by.get(inner_offset)
                    && by < start
                {
                    inner_offset += 1;
                }
                while let Some(&by) = by.get(inner_offset)
                    && by < end
                {
                    sum += on[inner_offset];
                    inner_offset += 1;
                }
                sum
            }));
        }

        Ok(RunVector {
            offsets: RunOffsets::Constant(num_bins, num_runs),
            data: Array::UInts(Buffer::from_vec(sums)),
        })
    }
}

impl DslRunVectorNode for WindowSum {
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn RunVectorNode> {
        let on = converted[&DslPtr::from(&self.on as &dyn DslNode)].as_run_vector();
        let by = converted[&DslPtr::from(&self.by as &dyn DslNode)].as_run_vector();
        Arc::new(LazyWindowSum {
            on,
            by,
            start: self.start,
            end: self.end,
            width: self.width,
        })
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(&self.on as _);
        f.push(&self.by as _);
    }
}
