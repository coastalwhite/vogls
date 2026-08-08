use vogls_ir::{SCALAR_VSIZE, VectorSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VType {
    SignedNet(VectorSize),
    UnsignedNet(VectorSize),
}

impl VType {
    pub const SCALAR_NET: Self = Self::UnsignedNet(SCALAR_VSIZE);

    pub const fn bit_length(&self) -> VectorSize {
        match self {
            Self::SignedNet(size) => *size,
            Self::UnsignedNet(size) => *size,
        }
    }

    pub fn net(width: VectorSize, signed: bool) -> VType {
        if signed {
            Self::SignedNet(width)
        } else {
            Self::UnsignedNet(width)
        }
    }

    pub fn is_signed(self) -> bool {
        match self {
            VType::SignedNet(_) => true,
            VType::UnsignedNet(_) => false,
        }
    }

    pub fn to_signed(self) -> VType {
        match self {
            VType::SignedNet(_) => self,
            VType::UnsignedNet(width) => VType::SignedNet(width),
        }
    }

    pub fn to_unsigned(self) -> VType {
        match self {
            VType::UnsignedNet(_) => self,
            VType::SignedNet(width) => VType::UnsignedNet(width),
        }
    }

    pub fn truncate_or_extend(self, width: VectorSize) -> VType {
        Self::net(width, self.is_signed())
    }
    pub fn zero_or_sign_extend(self, width: VectorSize) -> VType {
        assert!(width >= self.bit_length());
        self.truncate_or_extend(width)
    }
    pub fn truncate(self, width: VectorSize) -> VType {
        assert!(width <= self.bit_length());
        self.truncate_or_extend(width)
    }
}
