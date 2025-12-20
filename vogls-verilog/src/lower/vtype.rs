use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::Index;

use vogls_ir::VectorSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VType {
    ScalarNet,
    VectorNet(VectorSize),
    Integer,
    Array(VTypeKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VTypeKey(NonZeroU32);

#[derive(Debug, Default, Clone)]
pub struct VTypeTable {
    content: Vec<VType>,
    lut: HashMap<VType, VTypeKey>,
}

impl Index<VTypeKey> for VTypeTable {
    type Output = VType;

    fn index(&self, index: VTypeKey) -> &Self::Output {
        &self.content[(index.0.get() - 1) as usize]
    }
}

impl VType {
    pub fn to_ir(self) -> vogls_ir::Type {
        match self {
            VType::ScalarNet => vogls_ir::Type::Bits(1),
            VType::VectorNet(n) => vogls_ir::Type::Bits(n),
            VType::Integer => vogls_ir::Type::Decimal,
            VType::Array(_) => todo!(),
        }
    }

    pub fn net(width: Option<VectorSize>) -> VType {
        match width {
            None => Self::ScalarNet,
            Some(v) => Self::VectorNet(v),
        }
    }
}

impl VTypeTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, ty: VType) -> VTypeKey {
        *self.lut.entry(ty).or_insert_with(|| {
            self.content.push(ty);
            VTypeKey(NonZeroU32::new(self.content.len() as u32).expect("VTypeKey overflow"))
        })
    }
}
