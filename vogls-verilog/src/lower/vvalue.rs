use vogls_ir::Bits;

type ArraySize = u32;

#[derive(Clone)]
pub enum VValue {
    Integer(i64),
    ScalarNet(bool),
    VectorNet(Bits),
    Array(Box<VValueArray>),
}

#[derive(Clone)]
pub struct VValueArray {
    dims: Box<[ArraySize]>,
    leaf: VValueArrayLeaf,
}

#[derive(Clone)]
pub enum VValueArrayLeaf {
    Integer(Box<[i64]>),
    ScalarNet(Box<[bool]>),
    VectorNet(Box<[Bits]>),
}

impl VValue {
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(v) => Some(*v),
            Self::ScalarNet(_) | Self::VectorNet(_) | Self::Array(_) => None,
        }
    }
}
