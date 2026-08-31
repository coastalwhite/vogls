use vogls_ir::{INTEGER_VSIZE, SCALAR_VSIZE, TIME_VSIZE, VSIZE_64, VectorSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VType {
    SignedNet(VectorSize),
    UnsignedNet(VectorSize),
    Real,
}

impl VType {
    pub const SCALAR_NET: Self = Self::UnsignedNet(SCALAR_VSIZE);
    pub const TIME: Self = Self::UnsignedNet(TIME_VSIZE);
    pub const INTEGER: Self = Self::UnsignedNet(INTEGER_VSIZE);

    pub const fn bit_length(&self) -> VectorSize {
        match self {
            Self::SignedNet(size) => *size,
            Self::UnsignedNet(size) => *size,
            Self::Real => VSIZE_64,
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
            VType::SignedNet(_) | VType::Real => true,
            VType::UnsignedNet(_) => false,
        }
    }

    pub fn to_signed(self) -> VType {
        match self {
            VType::SignedNet(_) | VType::Real => self,
            VType::UnsignedNet(width) => VType::SignedNet(width),
        }
    }

    pub fn to_unsigned(self) -> VType {
        match self {
            VType::UnsignedNet(_) | VType::Real => self,
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

    pub fn is_unsigned_net(&self) -> bool {
        matches!(self, Self::UnsignedNet(_))
    }
    pub fn is_signed_net(&self) -> bool {
        matches!(self, Self::SignedNet(_))
    }
    pub fn is_real(&self) -> bool {
        matches!(self, Self::Real)
    }

    pub fn resize_net_to(&self, bit_length: VectorSize) -> VType {
        match self {
            Self::SignedNet(_) => Self::SignedNet(bit_length),
            Self::UnsignedNet(_) => Self::UnsignedNet(bit_length),
            Self::Real => Self::Real,
        }
    }
}
