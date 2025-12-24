use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::Index;

use crate::VectorSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArrayWidth(NonZeroU32);


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Bits(VectorSize),
    Decimal,
}

impl ArrayWidth {
    pub fn new(v: u32) -> Self {
        Self(NonZeroU32::new(v + 1).unwrap())
    }

    pub fn get(self) -> u32 {
        self.0.get() - 1
    }
}

impl Type {
    pub const SCALAR_NET: Self = Self::Bits(1);

    pub fn to_net_width(self) -> Option<VectorSize> {
        match self {
            Type::Bits(n) => Some(n),
            Type::Decimal => None,
        }
    }

    pub fn try_net_width(self) -> Result<VectorSize, ()> {
        self.to_net_width().ok_or(())
    }

    pub fn net(width: Option<VectorSize>) -> Self {
        Self::Bits(width.unwrap_or(1))
    }
}
