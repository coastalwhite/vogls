use vogls_ir::bits::arithmetic::FvLogicValue;
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

    pub fn coerce(self, ty: &VType) -> VValue {
        use VType as T;
        use VValue as V;
        match (self, ty) {
            (V::SignedNet(v), T::SignedNet(size)) => V::SignedNet(v.truncate_or_sign_extend(*size)),
            (V::SignedNet(v), T::UnsignedNet(size)) => {
                V::UnsignedNet(v.truncate_or_sign_extend(*size))
            }
            (V::UnsignedNet(v), T::SignedNet(size)) => {
                V::SignedNet(v.truncate_or_zero_extend(*size))
            }
            (V::UnsignedNet(v), T::UnsignedNet(size)) => {
                V::UnsignedNet(v.truncate_or_zero_extend(*size))
            }

            (V::String(_), _) | (_, T::String(_)) => todo!(),
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
            // (V::UnsignedNet(l), V::SignedNet(r)) => {
            //     let max_size = l.size().max(r.size());
            //     (
            //         VValue::SignedNet(l.zero_extend(max_size)),
            //         VValue::SignedNet(r.sign_extend(max_size)),
            //     )
            // }
            // (V::SignedNet(l), V::UnsignedNet(r)) => {
            //     let max_size = l.size().max(r.size());
            //     (
            //         VValue::SignedNet(l.sign_extend(max_size)),
            //         VValue::SignedNet(r.zero_extend(max_size)),
            //     )
            // }
            // (V::UnsignedNet(l), V::UnsignedNet(r)) => {
            //     let max_size = l.size().max(r.size());
            //     (
            //         VValue::UnsignedNet(l.zero_extend(max_size)),
            //         VValue::UnsignedNet(r.zero_extend(max_size)),
            //     )
            // }
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

    pub fn logical_equal(self, rhs: VValue) -> FvLogicValue {
        use VValue as V;
        let (slf, rhs) = Self::coerce_max_size(self, rhs);
        match (slf, rhs) {
            (V::SignedNet(l) | V::UnsignedNet(l), V::SignedNet(r) | V::UnsignedNet(r)) => {
                l.logical_equal(&r)
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
    }
    pub fn logical_not_equal(self, rhs: VValue) -> FvLogicValue {
        !self.logical_equal(rhs)
    }

    pub fn case_equal(self, rhs: VValue) -> bool {
        use VValue as V;
        let (slf, rhs) = Self::coerce_max_size(self, rhs);
        match (slf, rhs) {
            (V::SignedNet(l) | V::UnsignedNet(l), V::SignedNet(r) | V::UnsignedNet(r)) => l == r,
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
    }
    pub fn case_not_equal(self, rhs: VValue) -> bool {
        !self.case_equal(rhs)
    }

    pub fn logical_shift_left(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (&mut lhs, rhs) {
            (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                let r = r.truncate(VectorSize::new(32).unwrap());
                *lb = match r.extract_exact_u32() {
                    None => Bits::new_unknown(lb.size()),
                    Some(r) => Bits::logical_shift_left(lb, r),
                };
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
                let r = r.truncate(VectorSize::new(32).unwrap());
                *lb = match r.extract_exact_u32() {
                    None => Bits::new_unknown(lb.size()),
                    Some(r) => Bits::logical_shift_right(lb, r),
                };
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
        lhs
    }
    pub fn arithmetic_shift_right(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (&mut lhs, &rhs) {
            (V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                let r = r.truncate(VectorSize::new(32).unwrap());
                *lb = match r.extract_exact_u32() {
                    None => Bits::new_unknown(lb.size()),
                    Some(r) => Bits::arithmetic_shift_right(lb, r),
                };
            }
            _ => return Self::logical_shift_right(lhs, rhs),
        }
        lhs
    }

    pub fn bitwise_invert(self) -> VValue {
        use VValue as V;
        match self {
            V::SignedNet(v) => V::SignedNet(v.bitwise_negate()),
            V::UnsignedNet(v) => V::UnsignedNet(v.bitwise_negate()),
            V::String(_) => todo!(),
        }
    }

    pub fn sign_invert(self) -> VValue {
        use VValue as V;
        match self {
            V::SignedNet(v) => V::SignedNet(v.sign_invert()),
            V::UnsignedNet(v) => V::UnsignedNet(v.sign_invert()),
            V::String(_) => todo!(),
        }
    }

    pub fn bitwise_xnor(lhs: VValue, rhs: VValue) -> VValue {
        use VValue as V;
        let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (&mut lhs, rhs) {
            (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                *lb = Bits::bitwise_or(lb, &r).bitwise_negate();
            }
            (V::String(_), _) | (_, V::String(_)) => todo!(),
        }
        lhs
    }

    pub fn less_than(lhs: VValue, rhs: VValue) -> FvLogicValue {
        !Self::less_than_equal(rhs, lhs)
    }

    pub fn less_than_equal(lhs: VValue, rhs: VValue) -> FvLogicValue {
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
    pub fn greater_than(lhs: VValue, rhs: VValue) -> FvLogicValue {
        !Self::less_than_equal(lhs, rhs)
    }
    pub fn greater_than_equal(lhs: VValue, rhs: VValue) -> FvLogicValue {
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

    pub fn clog2(&self) -> Option<u32> {
        match self {
            Self::SignedNet(bits) => bits.clog2(),
            Self::UnsignedNet(bits) => bits.clog2(),
            Self::String(_) => todo!(),
        }
    }

    pub fn zero_or_sign_extend(self, new_size: VectorSize) -> Self {
        match self {
            Self::SignedNet(bits) => Self::SignedNet(bits.zero_extend(new_size)),
            Self::UnsignedNet(bits) => Self::SignedNet(bits.sign_extend(new_size)),
            Self::String(_) => todo!(),
        }
    }
}

impl From<FvLogicValue> for VValue {
    fn from(value: FvLogicValue) -> Self {
        Self::UnsignedNet(Bits::from(value))
    }
}

macro_rules! impl_arithmetic {
    ($(($f:ident, $op:tt),)+) => {
        impl VValue {
        $(
        #[allow(clippy::should_implement_trait)]
        pub fn $f(lhs: VValue, rhs: VValue) -> VValue {
            use VValue as V;
            let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
            match (&mut lhs, rhs) {
                (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                    *lb = Bits::$op(lb, &r);
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
    (multiply, multiply),
    (power, power),
    (divide, divide_x),
    (remainder, remainder_x),
    (add, add),
    (sub, subtract),
    (bitwise_and, bitwise_and),
    (bitwise_xor, bitwise_xor),
    (bitwise_or, bitwise_or),
}
