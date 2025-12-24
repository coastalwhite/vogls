use std::ops::{BitAnd as _, BitOr as _, BitXor as _};

use vogls_ir::Bits;

use super::VType;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VValue {
    Integer(i64),
    Net(Bits),
}

impl VValue {
    pub fn default_value(ty: VType) -> Self {
        // @TODO: Use X
        match ty {
            VType::Net(n) => Self::Net(Bits::new_zeroed(n)),
            VType::Integer => Self::Integer(-1337),
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(v) => Some(*v),
            Self::Net(_) => None,
        }
    }

    pub fn into_ir(self) -> vogls_ir::Value {
        match self {
            Self::Integer(v) => vogls_ir::Value::Decimal(v),
            Self::Net(v) => vogls_ir::Value::Bits(v),
        }
    }

    pub fn ty(&self) -> VType {
        match self {
            VValue::Integer(_) => VType::Integer,
            VValue::Net(bits) => VType::Net(bits.size()),
        }
    }

    pub fn coerce_max_size(l: VValue, r: VValue) -> (VValue, VValue) {
        use VValue as V;
        match (l, r) {
            (l @ V::Integer(_), r @ V::Integer(_)) => (l, r),
            (V::Net(l), V::Net(r)) if l.size() == r.size() => (V::Net(l), V::Net(r)),
            (V::Integer(l), V::Net(r)) => {
                (V::Net(Bits::from_i64_truncated(l, r.size())), V::Net(r))
            }
            (V::Net(l), V::Integer(r)) => {
                let size = l.size();
                (V::Net(l), V::Net(Bits::from_i64_truncated(r, size)))
            }
            (V::Net(l), V::Net(r)) => {
                let max_size = l.size().max(r.size());

                (
                    V::Net(l.sign_extend(max_size)),
                    V::Net(r.sign_extend(max_size)),
                )
            }
        }
    }

    pub fn logical_equal(self, rhs: VValue) -> bool {
        use VValue as V;
        let (slf, rhs) = Self::coerce_max_size(self, rhs);
        match (slf, rhs) {
            (V::Integer(l), V::Integer(r)) => l == r,
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => unreachable!(),
            (V::Net(l), V::Net(r)) => l.as_slice() == r.as_slice(),
        }
    }

    pub fn logical_shift_left(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::Integer(l), V::Integer(r)) => VValue::Integer(l.wrapping_shl(r as u32)),
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
            (V::Net(_), V::Net(_)) => todo!(),
        }
    }
    pub fn logical_shift_right(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::Integer(l), V::Integer(r)) => VValue::Integer(l.wrapping_shr(r as u32)),
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
            (V::Net(_), V::Net(_)) => todo!(),
        }
    }
    pub fn bitwise_xnor(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::Integer(l), V::Integer(r)) => VValue::Integer(!(l ^ r)),
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
            (V::Net(_), V::Net(_)) => todo!(),
        }
    }
    pub fn less_than(lhs: VValue, rhs: VValue) -> bool {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::Integer(l), V::Integer(r)) => l < r,
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
            (V::Net(_), V::Net(_)) => todo!(),
        }
    }
    pub fn less_than_equal(lhs: VValue, rhs: VValue) -> bool {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::Integer(l), V::Integer(r)) => l <= r,
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
            (V::Net(_), V::Net(_)) => todo!(),
        }
    }
    pub fn greater_than(lhs: VValue, rhs: VValue) -> bool {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::Integer(l), V::Integer(r)) => l > r,
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
            (V::Net(_), V::Net(_)) => todo!(),
        }
    }
    pub fn greater_than_equal(lhs: VValue, rhs: VValue) -> bool {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::Integer(l), V::Integer(r)) => l >= r,
            (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
            (V::Net(_), V::Net(_)) => todo!(),
        }
    }

    pub fn scalar_from_bool(value: bool) -> VValue {
        VValue::Net(Bits::Small(u64::from(value), 1))
    }

    pub fn to_logical(&self) -> bool {
        use VValue as V;
        match self {
            V::Integer(v) => *v != 0,
            V::Net(v) => v.not_eq_zero(),
        }
    }

    pub fn logical_and(lhs: VValue, rhs: VValue) -> bool {
        lhs.to_logical() && rhs.to_logical()
    }
    pub fn logical_or(lhs: VValue, rhs: VValue) -> bool {
        lhs.to_logical() || rhs.to_logical()
    }

    pub fn into_bits(self) -> Bits {
        match self {
            VValue::Integer(v) => Bits::from_i64_truncated(v, (64 - v.leading_zeros()).max(1)),
            VValue::Net(bits) => bits,
        }
    }

    pub fn concatenate(lhs: VValue, rhs: VValue) -> VValue {
        let lhs = lhs.into_bits();
        let rhs = rhs.into_bits();

        VValue::Net(Bits::concatenate(lhs, rhs))
    }
}

macro_rules! impl_arithmetic {
    ($(($f:ident, $op:tt),)+ : $(($ft:ident, $opt:tt),)+) => {
        impl VValue {
        $(
        pub fn $f(lhs: VValue, rhs: VValue) -> VValue {
            use VValue as V;
            let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
            match (lhs, rhs) {
                (V::Integer(l), V::Integer(r)) => VValue::Integer(l.$op(r)),
                (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
                (V::Net(l), V::Net(r)) => VValue::Net(Bits::$f(l, r)),
            }
        }
        )+
        $(
        pub fn $ft(lhs: VValue, rhs: VValue) -> VValue {
            use VValue as V;
            let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
            match (lhs, rhs) {
                (V::Integer(l), V::Integer(r)) => VValue::Integer(l.$opt(r)),
                (V::Integer(_), V::Net(_)) | (V::Net(_), V::Integer(_)) => todo!(),
                (V::Net(_), V::Net(_)) => todo!(),
            }
        }
        )+
        }
    };
}

impl_arithmetic! {
    (multiply, wrapping_mul),
    :
    (divide, wrapping_div),
    (remainder, wrapping_rem),
    (add, wrapping_add),
    (sub, wrapping_sub),
    (bitwise_and, bitand),
    (bitwise_xor, bitxor),
    (bitwise_or, bitor),
}
