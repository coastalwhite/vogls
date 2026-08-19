use core::fmt;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
use core::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtensionSet(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Isa {
    pub exts: ExtensionSet,
    pub xlen: XLen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XLen {
    Rv32,
    Rv64,
}

macro_rules! define_isas {
    ($(
        ($name:ident, $capname:ident, $var_name:ident, $chr:expr)
    )+) => {

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Ext {
            $($name,)+
        }

        impl ExtensionSet {
            pub const EMPTY: Self = Self(0);
            pub const ALL: Self = {
                let mut s = Self::EMPTY;
                $(s = s.union(Self::$capname);)+
                s
            };

            $(pub const $capname: Self = Self(1 << (Ext::$name as u32));)+
        }

        impl fmt::Debug for ExtensionSet {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct("ExtensionSet")
                    $(.field(stringify!($var_name), &self.contains(Self::$capname)))+
                    .finish()
            }
        }

        impl Iterator for ExtensionSet {
            type Item = Ext;
            fn next(&mut self) -> Option<Self::Item> {
                $(
                if self.contains(Self::$capname) {
                    *self ^= Self::$capname;
                    return Some(Ext::$name);
                }
                )+
                None
            }
        }

        impl TryFrom<char> for Ext {
            type Error = ();
            fn try_from(value: char) -> Result<Self, Self::Error> {
                match value {
                    $($chr => Ok(Self::$name),)+
                    _ => Err(()),
                }
            }
        }
    };
}

define_isas! {
    (Atomic, ATOMIC, atomic, 'a')
    (BitManipulation, BIT_MANIPULATION, bit_manipulation, 'b')
    (Compressed, COMPRESSED, compressed, 'c')
    (DoublePrecisionFp, DOUBLE_PRECISION_FP, double_precision_fp, 'd')
    (Rv32E, RV32E, rv32e, 'e')
    (SinglePrecisionFp, SINGLE_PRECISION_FP, single_precision_fp, 'f')
    // (G, G, g, 'g')
    (Hypervisor, HYPERVISOR, hypervisor, 'h')
    // (J, J, j, 'j')
    // (K, K, k, 'k')
    // (L, L, l, 'l')
    (IntegerMuldiv, INTEGER_MULDIV, integer_muldiv, 'm')
    // (N, N, n, 'n')
    // (O, O, o, 'o')
    (PackedSimd, PACKED_SIMD, packed_simd, 'p')
    (QuadPrecisionFp, QUAD_PRECISION_FP, quad_precision_fp, 'q')
    // (R, R, r, 'r')
    (SupervisorMode, SUPERVISOR_MODE, supervisor_mode, 's')
    // (T, T, t, 't')
    (UserMode, USER_MODE, user_mode, 'u')
    (Vector, VECTOR, vector, 'v')
    // (W, W, w, 'w')
    // (NonStandard, NON_STANDARD, non_standard, 'x')
    // (Y, Y, y, 'y')
    // (Z, Z, z, 'z')
}

impl ExtensionSet {
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub const fn union(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }

    #[inline]
    pub const fn intersect(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl From<Ext> for ExtensionSet {
    fn from(value: Ext) -> Self {
        Self(1 << value as u32)
    }
}

impl BitOr for ExtensionSet {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for ExtensionSet {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
impl BitAnd for ExtensionSet {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for ExtensionSet {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}
impl BitXor for ExtensionSet {
    type Output = Self;
    #[inline]
    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for ExtensionSet {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}
impl Not for ExtensionSet {
    type Output = Self;
    #[inline]
    fn not(self) -> Self {
        Self(!self.0) ^ Self::ALL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsaParseError {
    InvalidExtension(char),
    DuplicateExtension(char),
    InvalidStart,
    InvalidBase,
}

impl fmt::Display for IsaParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExtension(c) => write!(f, "RISC-V ISA specifier has an invalid extension with '{c}'"),
            Self::DuplicateExtension(c) => write!(f, "RISC-V ISA specifier has a duplicate extension with '{c}'"),
            Self::InvalidStart => f.write_str("RISC-V ISA specifier does not start with 'rv32' or 'rv64'"),
            Self::InvalidBase => f.write_str("RISC-V ISA specifier does not have valid base. Valid bases are 'i' and 'g'. 'e' is also valid if mxlen=32."),
        }
    }
}

impl std::error::Error for IsaParseError {}

impl FromStr for Isa {
    type Err = IsaParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let xlen = if s.starts_with("rv32") || s.starts_with("RV32") {
            XLen::Rv32
        } else if s.starts_with("rv64") || s.starts_with("RV64") {
            XLen::Rv64
        } else {
            return Err(IsaParseError::InvalidStart);
        };

        let s = &s[4..];

        let mut chars = s.chars();

        let Some(base) = chars.next() else {
            return Err(IsaParseError::InvalidBase);
        };

        let mut exts = match base {
            'i' => ExtensionSet::EMPTY,
            // @TODO: This should include Zicsr & Zifencei
            'g' => {
                ExtensionSet::INTEGER_MULDIV
                    | ExtensionSet::ATOMIC
                    | ExtensionSet::SINGLE_PRECISION_FP
                    | ExtensionSet::DOUBLE_PRECISION_FP
            }
            _ => return Err(IsaParseError::InvalidBase),
        };

        for c in chars {
            let Ok(ext) = Ext::try_from(c.to_ascii_lowercase()) else {
                return Err(IsaParseError::InvalidExtension(c));
            };
            let ext = ExtensionSet::from(ext);

            if exts.intersect(ext) != ExtensionSet::EMPTY {
                return Err(IsaParseError::DuplicateExtension(c));
            }

            exts |= ext;
        }

        Ok(Isa {
            exts,
            xlen,
        })
    }
}
