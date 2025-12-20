use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::Index;

use crate::VectorSize;

pub struct TypeTable {
    content: Vec<Type>,
    lut: HashMap<Type, TypeKey>,
}

impl Default for TypeTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeKey(NonZeroU32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Bits(VectorSize),
    Decimal,
    Array(TypeKey, u32),
}

impl Index<TypeKey> for TypeTable {
    type Output = Type;

    fn index(&self, index: TypeKey) -> &Self::Output {
        &self.content[(index.0.get() - 1) as usize]
    }
}

impl TypeTable {
    pub fn new() -> Self {
        let mut slf = Self {
            content: Vec::new(),
            lut: HashMap::new(),
        };
        slf.insert(Type::Decimal);
        slf.insert(Type::Bits(1));
        slf
    }

    pub const INT64: TypeKey = TypeKey(NonZeroU32::new(1).unwrap());
    pub const SCALAR_BIT: TypeKey = TypeKey(NonZeroU32::new(2).unwrap());

    pub fn insert(&mut self, ty: Type) -> TypeKey {
        *self.lut.entry(ty).or_insert_with(|| {
            self.content.push(ty);
            TypeKey(NonZeroU32::new(self.content.len() as u32).expect("TypeKey overflow"))
        })
    }
}

impl Type {
    pub fn to_net_width(self) -> Option<VectorSize> {
        match self {
            Type::Bits(n) => Some(n),
            Type::Decimal => None,
            Type::Array(_, _) => None,
        }
    }

    pub fn try_net_width(self) -> Result<VectorSize, ()> {
        self.to_net_width().ok_or(())
    }

    pub fn net(width: Option<VectorSize>) -> Self {
        Self::Bits(width.unwrap_or(1))
    }
}
