use vogls_ir::VectorSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VType {
    Net(VectorSize),
    Integer,
    String(u32),
}

impl VType {
    pub const SCALAR_NET: Self = Self::Net(1);

    pub const fn net_size(self) -> Option<VectorSize> {
        match self {
            Self::Net(n) => Some(n),
            Self::Integer => None,
            Self::String(_) => None,
        }
    }

    pub fn force_net_width(&self) -> VectorSize {
        match self {
            VType::Net(size) => *size,
            VType::Integer => 32,
            VType::String(n) => *n * 8,
        }
    }
}
