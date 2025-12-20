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

#[derive(Debug, Clone)]
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
    pub const fn to_ir(self) -> vogls_ir::Type {
        match self {
            VType::ScalarNet => vogls_ir::Type::Bits(1),
            VType::VectorNet(n) => vogls_ir::Type::Bits(n),
            VType::Integer => vogls_ir::Type::Decimal,
            VType::Array(_) => todo!(),
        }
    }

    pub const fn net(width: Option<VectorSize>) -> VType {
        match width {
            None => Self::ScalarNet,
            Some(v) => Self::VectorNet(v),
        }
    }

    pub const fn net_width(self) -> Option<VectorSize> {
        match self {
            Self::ScalarNet => Some(1),
            Self::VectorNet(n) => Some(n),
            Self::Integer => None,
            Self::Array(_) => None,
        }
    }

    pub const fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
}

impl VTypeTable {
    pub fn new() -> Self {
        let mut slf = Self {
            content: Vec::new(),
            lut: HashMap::new(),
        };
        slf.insert(VType::Integer);
        slf.insert(VType::ScalarNet);
        slf
    }

    pub const fn integer(&self) -> VTypeKey {
        VTypeKey(NonZeroU32::new(1).unwrap())
    }

    pub const fn scalar_net(&self) -> VTypeKey {
        VTypeKey(NonZeroU32::new(2).unwrap())
    }

    pub fn insert(&mut self, ty: VType) -> VTypeKey {
        *self.lut.entry(ty).or_insert_with(|| {
            self.content.push(ty);
            VTypeKey(NonZeroU32::new(self.content.len() as u32).expect("VTypeKey overflow"))
        })
    }
}
