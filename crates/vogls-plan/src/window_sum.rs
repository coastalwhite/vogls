use std::fmt;

use crate::array::Array;
use crate::buffer::Buffer;
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::map::Map;
use crate::typing::{ArrayType, DataType};

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct WindowSum {
    pub start: u64,
    pub end: u64,
    pub width: u64,
}

impl Map for WindowSum {
    type Inputs<Input>
        = [Input; 2]
    where
        Input: Send + Sync;

    type Scratches = ();

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { start, end, width } = &self;
        write!(
            f,
            "WindowSum {{ width: {width}, start: {start}, end: {end} }}"
        )
    }

    fn output_type(
        &self,
        inputs: Self::Inputs<ArrayType>,
    ) -> ComputeResult<crate::typing::ArrayType> {
        if !matches!(
            (inputs[0].data, inputs[1].data),
            (DataType::UInt, DataType::UInt)
        ) {
            return Err(ComputeError::InvalidTypes);
        }
        let diff = self.end - self.start;
        let width = diff.div_ceil(self.width) as usize;
        Ok(ArrayType {
            data: inputs[0].data,
            length: Some(width),
        })
    }

    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        _scratch: &mut Self::Scratches,
    ) -> ComputeResult<Array> {
        let [on, by] = inputs;
        let (Array::UInts(on), Array::UInts(by)) = (&on, &by) else {
            return Err(ComputeError::InvalidTypes);
        };

        assert!(self.start <= self.end);
        assert!(self.width > 0);

        let diff = self.end - self.start;
        let num_bins = diff.div_ceil(self.width);

        assert_eq!(on.len(), by.len());

        let mut inner_offset = 0;
        let sums = (0..num_bins)
            .map(|i| {
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
            })
            .collect::<Buffer<u64>>();

        Ok(Array::UInts(sums))
    }
}
