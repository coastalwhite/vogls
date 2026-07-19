use std::sync::Arc;

use vogls::VectorSize;
use vogls::utils::VgHashMap;

use crate::array::{Array, ArrayNode, DslArrayNode, DslLazyArray};
use crate::compute::{ComputeContext, ComputeDependencies, ComputeInputs, ComputeResult, Key};
use crate::dsl::{DslNode, DslPtr};
use crate::typing::{ArrayType, DataType, Type};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RandomBits {
    pub length: usize,
    pub width: VectorSize,
    pub seed: u64,
}

impl RandomBits {
    pub fn build(self) -> DslLazyArray {
        DslLazyArray {
            ty: Arc::new(Type::Array(ArrayType {
                data: DataType::Bits(self.width),
                length: Some(self.length),
            })),
            f: Arc::new(self),
        }
    }
}

impl DslArrayNode for RandomBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            length,
            width,
            seed,
        } = &self;
        write!(
            f,
            "Random Bits {{ length: {length}, width: {width}, seed: {seed} }}"
        )
    }
    fn convert_one<'a>(
        &'a self,
        _converted: &'a VgHashMap<DslPtr, Key>,
    ) -> std::sync::Arc<dyn ArrayNode> {
        Arc::new(*self)
    }

    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
impl ArrayNode for RandomBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            length,
            width,
            seed,
        } = &self;
        write!(
            f,
            "Random Bits {{ length: {length}, width: {width}, seed: {seed} }}"
        )
    }
    fn csp_eq(&self, other: &dyn ArrayNode) -> bool {
        let Some(other) = (other as &dyn std::any::Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
    }

    fn csp_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        use std::hash::Hash;
        self.hash(&mut state);
    }

    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {}

    fn compute(&self, _ctx: &ComputeContext, _inputs: &ComputeInputs) -> ComputeResult<Array> {
        let size = VectorSize::new((self.length * self.width.get() as usize) as u32).unwrap();
        Ok(Array::Bits(
            vogls::bits::random::rand_bits_from_seed(size, vogls::ir::Mode::TwoValue, self.seed),
            self.width,
        ))
    }
}
