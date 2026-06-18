use crate::array::{Array, Primitive};
use crate::buffer::Buffer;
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::map::Map;
use crate::typing::ArrayType;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Expand;

fn output_size(size: usize) -> Option<usize> {
    if size == 0 {
        return Some(0);
    }

    Some(size.checked_mul(size - 1)? >> 1)
}

fn expand_primitive<T: Primitive>(data: &[T]) -> ComputeResult<Buffer<T>> {
    // @TODO: Generalize across degrees

    let output_length = output_size(data.len()).ok_or_else(|| ComputeError::Overflow)?;
    let mut out = Vec::with_capacity(output_length);

    for (i, &x) in data.iter().enumerate() {
        for &y in &data[i + 1..] {
            out.push(x + y);
        }
    }

    Ok(Buffer::from_vec(out))
}

impl Map for Expand {
    type Inputs<Input>
        = [Input; 1]
    where
        Input: Send + Sync;

    type Scratches = ();

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Expand")
    }

    fn output_type(&self, inputs: Self::Inputs<ArrayType>) -> ComputeResult<ArrayType> {
        let [input] = inputs;
        let length = match input.length {
            None => None,
            Some(length) => Some(output_size(length).ok_or_else(|| ComputeError::Overflow)?),
        };
        Ok(ArrayType {
            data: input.data,
            length,
        })
    }

    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        _scratch: &mut Self::Scratches,
    ) -> ComputeResult<Array> {
        let [input] = inputs;
        use Array as A;
        let data = match input {
            A::Floats(vs) => A::Floats(expand_primitive(vs.as_slice())?),
            A::Ints(vs) => A::Ints(expand_primitive(vs.as_slice())?),
            A::UInts(vs) => A::UInts(expand_primitive(vs.as_slice())?),
            A::Bits(..) => todo!(),
        };
        Ok(data)
    }
}
