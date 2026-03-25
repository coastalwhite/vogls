use vogls_ir::{SCALAR_VSIZE, VectorSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VType {
    SignedNet(VectorSize),
    UnsignedNet(VectorSize),
    String(u32),
}

impl VType {
    pub const SCALAR_NET: Self = Self::UnsignedNet(SCALAR_VSIZE);

    pub const fn net_size(self) -> Option<VectorSize> {
        match self {
            Self::SignedNet(n) => Some(n),
            Self::UnsignedNet(n) => Some(n),
            Self::String(_) => None,
        }
    }

    pub fn force_net_width(&self) -> VectorSize {
        match self {
            VType::SignedNet(size) => *size,
            VType::UnsignedNet(size) => *size,
            VType::String(n) => VectorSize::new(*n * 8).unwrap(),
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
            VType::UnsignedNet(_) | VType::String(_) => false,
        }
    }

    pub fn to_signed(self) -> VType {
        match self {
            VType::SignedNet(_) | VType::String(_) => self,
            VType::UnsignedNet(width) => VType::SignedNet(width),
        }
    }

    pub fn to_unsigned(self) -> VType {
        match self {
            VType::UnsignedNet(_) | VType::String(_) => self,
            VType::SignedNet(width) => VType::UnsignedNet(width),
        }
    }

    pub fn zero_or_sign_extend(self, width: VectorSize) -> VType {
        Self::net(width, self.is_signed())
    }
}
