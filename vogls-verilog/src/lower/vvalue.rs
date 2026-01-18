use std::ops::{BitAnd as _, BitOr as _, BitXor as _};

use vogls_ir::{Bits, VectorSize};

use super::VType;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VValue {
    SignedNet(Bits),
    UnsignedNet(Bits),
    String(Box<str>),
}

impl VValue {
    pub fn default_value(ty: VType) -> Self {
        // @TODO: Use X
        match ty {
            VType::SignedNet(n) => Self::SignedNet(Bits::new_zeroed(n)),
            VType::UnsignedNet(n) => Self::UnsignedNet(Bits::new_zeroed(n)),
            VType::String(n) => Self::String(std::iter::repeat_n('\0', n as usize).collect()),
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::SignedNet(bits) | Self::UnsignedNet(bits) => bits.as_i64(),
            Self::String(_) => None,
        }
    }

    pub fn ty(&self) -> VType {
        match self {
            VValue::SignedNet(bits) => VType::SignedNet(bits.size()),
            VValue::UnsignedNet(bits) => VType::UnsignedNet(bits.size()),
            VValue::String(s) => VType::String(s.len() as u32),
        }
    }

    pub fn coerce_max_size(l: VValue, r: VValue) -> (VValue, VValue) {
        use VValue as V;
        match (l, r) {
            (V::SignedNet(l), V::SignedNet(r)) if l.size() == r.size() => {
                (V::SignedNet(l), V::SignedNet(r))
            }
            (V::UnsignedNet(l) | V::SignedNet(l), V::UnsignedNet(r) | V::SignedNet(r))
                if l.size() == r.size() =>
            {
                (V::SignedNet(l), V::SignedNet(r))
            }
            (l @ V::SignedNet(_), r @ V::SignedNet(_)) => {
                let max_size = l.ty().force_net_width().max(r.ty().force_net_width());

                (l.sign_extend(max_size), r.sign_extend(max_size))
            }
            (
                l @ (V::UnsignedNet(_) | V::SignedNet(_)),
                r @ (V::UnsignedNet(_) | V::SignedNet(_)),
            ) => {
                let max_size = l.ty().force_net_width().max(r.ty().force_net_width());

                (l.sign_extend(max_size), r.sign_extend(max_size))
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
    }

    pub fn logical_equal(self, rhs: VValue) -> bool {
        use VValue as V;
        let (slf, rhs) = Self::coerce_max_size(self, rhs);
        match (slf, rhs) {
            (V::SignedNet(l) | V::UnsignedNet(l), V::SignedNet(r) | V::UnsignedNet(r)) => {
                l.as_slice() == r.as_slice()
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
    }

    pub fn logical_shift_left(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (&mut lhs, rhs) {
            (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                let r = r.truncate(VectorSize::new(32).unwrap()).extract_exact_u32();
                *lb = Bits::logical_shift_left(lb, r);
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
        lhs
    }
    pub fn logical_shift_right(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (&mut lhs, rhs) {
            (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                let r = r.truncate(VectorSize::new(32).unwrap()).extract_exact_u32();
                *lb = Bits::logical_shift_right(lb, r);
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
        lhs
    }
    pub fn bitwise_xnor(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (&mut lhs, rhs) {
            (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(rb) | V::SignedNet(rb)) => {
                *lb = Bits::tv_bitwise_op(lb, &rb, |l, r| !(l ^ r));
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
        lhs
    }

    pub fn less_than(lhs: VValue, rhs: VValue) -> bool {
        !Self::less_than_equal(rhs, lhs)
    }

    pub fn less_than_equal(lhs: VValue, rhs: VValue) -> bool {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::UnsignedNet(lb), V::UnsignedNet(rb)) => Bits::is_unsigned_leq(&lb, &rb),
            (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(rb) | V::SignedNet(rb)) => {
                Bits::is_signed_leq(&lb, &rb)
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
    }
    pub fn greater_than(lhs: VValue, rhs: VValue) -> bool {
        !Self::less_than_equal(lhs, rhs)
    }
    pub fn greater_than_equal(lhs: VValue, rhs: VValue) -> bool {
        Self::less_than(rhs, lhs)
    }

    pub fn scalar_from_bool(value: bool) -> VValue {
        VValue::UnsignedNet(Bits::from(value))
    }

    pub fn to_logical(&self) -> bool {
        use VValue as V;
        match self {
            V::SignedNet(v) => v.not_eq_zero(),
            V::UnsignedNet(v) => v.not_eq_zero(),
            V::String(v) => v.as_bytes().iter().any(|b| *b != 0),
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
            VValue::SignedNet(bits) => bits,
            VValue::UnsignedNet(bits) => bits,
            VValue::String(v) => {
                Bits::load_from_slice(v.as_bytes(), VectorSize::new((v.len() * 8) as u32).unwrap())
            }
        }
    }

    pub fn concatenate(lhs: VValue, rhs: VValue) -> VValue {
        let lhs = lhs.into_bits();
        let rhs = rhs.into_bits();

        VValue::UnsignedNet(Bits::concatenate(&lhs, &rhs))
    }

    pub fn to_vector_size(self) -> Option<VectorSize> {
        match self {
            VValue::SignedNet(_) | VValue::UnsignedNet(_) => todo!(),
            VValue::String(_) => todo!(),
        }
    }

    pub fn net(bits: Bits, signed: bool) -> VValue {
        if signed {
            Self::SignedNet(bits)
        } else {
            Self::UnsignedNet(bits)
        }
    }

    fn sign_extend(self, extended_size: VectorSize) -> VValue {
        if self.ty().force_net_width() == extended_size {
            return self;
        }

        use VValue as V;
        match self {
            V::SignedNet(bits) => Self::SignedNet(bits.sign_extend(extended_size)),
            V::UnsignedNet(bits) => Self::UnsignedNet(bits.sign_extend(extended_size)),
            V::String(_) => todo!(),
        }
    }

    pub fn truncate_or_extend(self, new_size: VectorSize) -> Self {
        match self {
            Self::SignedNet(bits) => Self::SignedNet(bits.truncate_or_sign_extend(new_size)),
            Self::UnsignedNet(bits) => Self::SignedNet(bits.truncate_or_zero_extend(new_size)),
            Self::String(_) => todo!(),
        }
    }

    pub fn clog2(&self) -> u32 {
        match self {
            Self::SignedNet(bits) => bits.size().get() - bits.leading_zeroes(),
            Self::UnsignedNet(bits) => bits.size().get() - bits.leading_zeroes(),
            Self::String(_) => todo!(),
        }
    }
}

macro_rules! impl_arithmetic {
    ($(($f:ident, $op:tt),)+ : $(($ft:ident, $opt:tt),)+) => {
        impl VValue {
        $(
        pub fn $f(lhs: VValue, rhs: VValue) -> VValue {
            use VValue as V;
            let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
            match (&mut lhs, rhs) {
                (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                    let size = lb.size();
                    let (Some(l), Some(r)) = (lb.as_u64(), r.as_u64()) else {
                        todo!();
                    };

                    *lb = Bits::from_u64(size, l.$op(r));
                }
                (V::String(_), _) | (_, V::String(_)) => todo!(),
            }
            lhs
        }
        )+
        $(
        pub fn $ft(lhs: VValue, rhs: VValue) -> VValue {
            use VValue as V;
            let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
            match (&mut lhs, rhs) {
                (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                    let size = lb.size();
                    let (Some(l), Some(r)) = (lb.as_u64(), r.as_u64()) else {
                        todo!();
                    };

                    *lb = Bits::from_u64(size, l.$opt(r));
                }
                (V::String(_), _) | (_, V::String(_)) => todo!(),
            }
            lhs
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
