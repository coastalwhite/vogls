use vogls_ir::VectorSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VType {
    Net(VectorSize),
    Integer,
}

impl VType {
    pub const SCALAR_NET: Self = Self::Net(1);
    pub fn to_ir_info(self) -> vogls_ir::Type {
        match self {
            VType::Net(n) => vogls_ir::Type::Bits(n),
            VType::Integer => vogls_ir::Type::Decimal,
        }
    }

    pub fn net(width: Option<VectorSize>) -> VType {
        Self::Net(width.unwrap_or(1))
    }

    pub const fn net_width(self) -> Option<VectorSize> {
        match self {
            Self::Net(n) => Some(n),
            Self::Integer => None,
        }
    }

    pub fn from_ir(ty: vogls_ir::Type) -> Self {
        match ty {
            vogls_ir::Type::Bits(n) => Self::Net(n),
            vogls_ir::Type::Decimal => Self::Integer,
        }
    }
}
