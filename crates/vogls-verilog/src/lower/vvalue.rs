use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_ir::{Bits, VectorSize};

use super::VType;

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum VValue {
    SignedNet(Bits),
    UnsignedNet(Bits),
    Real(Real),
}

impl VValue {
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::SignedNet(bits) | Self::UnsignedNet(bits) => bits.as_i64(),
            Self::Real(_) => None,
        }
    }

    pub fn ty(&self) -> VType {
        match self {
            Self::SignedNet(bits) => VType::SignedNet(bits.size()),
            Self::UnsignedNet(bits) => VType::UnsignedNet(bits.size()),
            Self::Real(_) => VType::Real,
        }
    }

    pub fn into_real(self) -> Real {
        match self {
            VValue::SignedNet(v) => Real::from_signed_bits(&v),
            VValue::UnsignedNet(v) => Real::from_unsigned_bits(&v),
            VValue::Real(v) => v,
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

            (V::Real(v), T::UnsignedNet(size)) => V::UnsignedNet(v.to_bits(*size)),
            (V::Real(v), T::SignedNet(size)) => V::SignedNet(v.to_bits(*size)),

            (v, T::Real) => V::Real(v.into_real()),
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
                let max_size = l.ty().bit_length().max(r.ty().bit_length());

                (l.sign_extend(max_size), r.sign_extend(max_size))
            }
            (
                l @ (V::UnsignedNet(_) | V::SignedNet(_)),
                r @ (V::UnsignedNet(_) | V::SignedNet(_)),
            ) => {
                let max_size = l.ty().bit_length().max(r.ty().bit_length());

                (l.sign_extend(max_size), r.sign_extend(max_size))
            }
            (V::Real(l), r) => (V::Real(l), V::Real(r.into_real())),
            (l, V::Real(r)) => (V::Real(l.into_real()), V::Real(r)),
        }
    }

    pub fn logical_equal(self, rhs: VValue) -> FvLogicValue {
        use VValue as V;
        let (slf, rhs) = Self::coerce_max_size(self, rhs);
        match (slf, rhs) {
            (V::SignedNet(l) | V::UnsignedNet(l), V::SignedNet(r) | V::UnsignedNet(r)) => {
                l.logical_equal(&r)
            }

            (V::Real(l), V::Real(r)) => FvLogicValue::from_bool(l == r),
            (V::Real(l), r) => FvLogicValue::from_bool(l == r.into_real()),
            (l, V::Real(r)) => FvLogicValue::from_bool(l.into_real() == r),
        }
    }
    pub fn logical_not_equal(self, rhs: VValue) -> FvLogicValue {
        !self.logical_equal(rhs)
    }

    pub fn case_equal(self, rhs: VValue) -> Result<bool, ()> {
        use VValue as V;
        let (slf, rhs) = Self::coerce_max_size(self, rhs);
        match (slf, rhs) {
            (V::SignedNet(l) | V::UnsignedNet(l), V::SignedNet(r) | V::UnsignedNet(r)) => {
                Ok(l == r)
            }
            (V::Real(_), _) | (_, V::Real(_)) => Err(()),
        }
    }
    pub fn case_not_equal(self, rhs: VValue) -> Result<bool, ()> {
        self.case_equal(rhs).map(|v| !v)
    }

    pub fn logical_shift_left(lhs: VValue, rhs: VValue) -> Result<VValue, ()> {
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
            (V::Real(_), _) | (_, V::Real(_)) => return Err(()),
        }
        Ok(lhs)
    }
    pub fn logical_shift_right(lhs: VValue, rhs: VValue) -> Result<VValue, ()> {
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
            (V::Real(_), _) | (_, V::Real(_)) => return Err(()),
        }
        Ok(lhs)
    }
    pub fn arithmetic_shift_right(lhs: VValue, rhs: VValue) -> Result<VValue, ()> {
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
            (V::UnsignedNet(_), V::UnsignedNet(_) | V::SignedNet(_)) => {
                return Self::logical_shift_right(lhs, rhs);
            }
            (V::Real(_), _) | (_, V::Real(_)) => return Err(()),
        }
        Ok(lhs)
    }

    pub fn bitwise_invert(self) -> Result<VValue, ()> {
        use VValue as V;
        match self {
            V::SignedNet(v) => Ok(V::SignedNet(v.bitwise_not())),
            V::UnsignedNet(v) => Ok(V::UnsignedNet(v.bitwise_not())),
            V::Real(_) => Err(()),
        }
    }

    pub fn sign_invert(self) -> VValue {
        use VValue as V;
        match self {
            V::SignedNet(v) => V::SignedNet(v.sign_invert()),
            V::UnsignedNet(v) => V::UnsignedNet(v.sign_invert()),
            V::Real(v) => V::Real(v.sign_invert()),
        }
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

            (V::Real(l), r) => FvLogicValue::from_bool(l <= r.into_real()),
            (l, V::Real(r)) => FvLogicValue::from_bool(l.into_real() <= r),
        }
    }
    pub fn greater_than(lhs: VValue, rhs: VValue) -> FvLogicValue {
        !Self::less_than_equal(lhs, rhs)
    }
    pub fn greater_than_equal(lhs: VValue, rhs: VValue) -> FvLogicValue {
        Self::less_than_equal(rhs, lhs)
    }

    pub fn scalar_from_bool(value: bool) -> VValue {
        VValue::UnsignedNet(Bits::from(value))
    }

    pub fn to_logical(&self) -> bool {
        use VValue as V;
        match self {
            V::SignedNet(v) => v.reduce_or() == FvLogicValue::L1,
            V::UnsignedNet(v) => v.reduce_or() == FvLogicValue::L1,
            V::Real(v) => v.as_f64() != 0.0,
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
            VValue::Real(v) => Bits::new_u64(v.as_f64().to_bits()),
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
        if self.ty().bit_length() == extended_size {
            return self;
        }

        use VValue as V;
        match self {
            V::SignedNet(bits) => Self::SignedNet(bits.sign_extend(extended_size)),
            V::UnsignedNet(bits) => Self::UnsignedNet(bits.sign_extend(extended_size)),
            V::Real(v) => Self::SignedNet(v.to_bits(extended_size)),
        }
    }

    pub fn truncate_or_extend(self, new_size: VectorSize) -> Self {
        match self {
            Self::SignedNet(bits) => Self::SignedNet(bits.truncate_or_sign_extend(new_size)),
            Self::UnsignedNet(bits) => Self::SignedNet(bits.truncate_or_zero_extend(new_size)),
            Self::Real(v) => Self::SignedNet(v.to_bits(new_size)),
        }
    }

    pub fn clog2(&self) -> Option<u32> {
        match self {
            Self::SignedNet(bits) => bits.clog2(),
            Self::UnsignedNet(bits) => bits.clog2(),
            Self::Real(_) => None,
        }
    }

    pub fn zero_or_sign_extend(self, new_size: VectorSize) -> Self {
        match self {
            Self::SignedNet(bits) => Self::SignedNet(bits.sign_extend(new_size)),
            Self::UnsignedNet(bits) => Self::UnsignedNet(bits.zero_extend(new_size)),
            Self::Real(v) => Self::SignedNet(v.to_bits(new_size)),
        }
    }

    pub fn power(lhs: VValue, rhs: VValue) -> Self {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(lhs, rhs);
        match (lhs, rhs) {
            (V::UnsignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::UnsignedNet(Bits::power(&lb, &r))
            }
            (V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::SignedNet(Bits::power(&lb, &r))
            }
            (lhs @ V::Real(_), rhs) | (lhs, rhs @ V::Real(_)) => {
                V::Real(Real::power(lhs.into_real(), rhs.into_real()))
            }
        }
    }

    pub fn is_real(&self) -> bool {
        matches!(self, Self::Real(_))
    }
}

impl From<FvLogicValue> for VValue {
    fn from(value: FvLogicValue) -> Self {
        Self::UnsignedNet(Bits::from(value))
    }
}

impl Mul for VValue {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(self, rhs);
        match (lhs, rhs) {
            (V::UnsignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::UnsignedNet(Bits::multiply(&lb, &r))
            }
            (V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::SignedNet(Bits::multiply(&lb, &r))
            }
            (lhs @ V::Real(_), rhs) | (lhs, rhs @ V::Real(_)) => {
                V::Real(lhs.into_real() * rhs.into_real())
            }
        }
    }
}
impl Div for VValue {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(self, rhs);
        match (lhs, rhs) {
            (V::UnsignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::UnsignedNet(Bits::divide_x(&lb, &r))
            }
            (V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::SignedNet(Bits::divide_x(&lb, &r))
            }
            (lhs @ V::Real(_), rhs) | (lhs, rhs @ V::Real(_)) => {
                V::Real(lhs.into_real() / rhs.into_real())
            }
        }
    }
}
impl Rem for VValue {
    type Output = Result<Self, ()>;
    fn rem(self, rhs: Self) -> Self::Output {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(self, rhs);
        match (lhs, rhs) {
            (V::UnsignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                Ok(V::UnsignedNet(Bits::remainder_x(&lb, &r)))
            }
            (V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                Ok(V::SignedNet(Bits::remainder_x(&lb, &r)))
            }
            (V::Real(_), _) | (_, V::Real(_)) => Err(()),
        }
    }
}
impl Add for VValue {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(self, rhs);
        match (lhs, rhs) {
            (V::UnsignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::UnsignedNet(Bits::add(&lb, &r))
            }
            (V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::SignedNet(Bits::add(&lb, &r))
            }
            (lhs @ V::Real(_), rhs) | (lhs, rhs @ V::Real(_)) => {
                V::Real(lhs.into_real() + rhs.into_real())
            }
        }
    }
}
impl Sub for VValue {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        use VValue as V;
        let (lhs, rhs) = Self::coerce_max_size(self, rhs);
        match (lhs, rhs) {
            (V::UnsignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::UnsignedNet(Bits::subtract(&lb, &r))
            }
            (V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                V::SignedNet(Bits::subtract(&lb, &r))
            }
            (lhs @ V::Real(_), rhs) | (lhs, rhs @ V::Real(_)) => {
                V::Real(lhs.into_real() - rhs.into_real())
            }
        }
    }
}

macro_rules! impl_bitwise {
    ($(($f:ident, $op:tt$(, $realop:tt)?),)+) => {
        impl VValue {
        $(
        pub fn $f(lhs: VValue, rhs: VValue) -> Result<VValue, ()> {
            use VValue as V;
            let (mut lhs, rhs) = Self::coerce_max_size(lhs, rhs);
            match (&mut lhs, rhs) {
                (V::UnsignedNet(lb) | V::SignedNet(lb), V::UnsignedNet(r) | V::SignedNet(r)) => {
                    *lb = Bits::$op(lb, &r);
                }
                (V::Real(_), _) | (_, V::Real(_)) => return Err(()),
            }
            Ok(lhs)
        }
        )+
        }
    };
}

impl_bitwise! {
    (bitwise_and, bitwise_and),
    (bitwise_xor, bitwise_xor),
    (bitwise_xnor, bitwise_xnor),
    (bitwise_or, bitwise_or),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Real(pub f64);

impl From<f64> for Real {
    #[inline(always)]
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

impl Real {
    #[inline(always)]
    pub fn from_f64(val: f64) -> Self {
        Self(val)
    }

    #[inline(always)]
    pub fn as_f64(self) -> f64 {
        self.0
    }

    pub fn from_unsigned_bits(bits: &Bits) -> Self {
        Real::from_f64(bits.as_unsigned_f64())
    }

    pub fn from_signed_bits(bits: &Bits) -> Self {
        Real::from_f64(bits.as_signed_f64())
    }

    pub fn to_bits(self, size: VectorSize) -> Bits {
        let v = self.as_f64().round();
        let value = v as i64 as u64;
        let bits = Bits::new_u64(value);
        bits.truncate_or_sign_extend(size)
    }

    #[inline(always)]
    fn sign_invert(self) -> Self {
        Self::from_f64(-self.as_f64())
    }

    pub fn power(lhs: Self, rhs: Self) -> Self {
        Self::from_f64(lhs.as_f64().powf(rhs.as_f64()))
    }
}

impl std::hash::Hash for Real {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_f64().to_bits().hash(state);
    }
}

impl Neg for Real {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self.sign_invert()
    }
}

impl Mul for Real {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::from_f64(self.as_f64() * rhs.as_f64())
    }
}
impl Div for Real {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::from_f64(self.as_f64() / rhs.as_f64())
    }
}
impl Add for Real {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::from_f64(self.as_f64() + rhs.as_f64())
    }
}
impl Sub for Real {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::from_f64(self.as_f64() - rhs.as_f64())
    }
}
