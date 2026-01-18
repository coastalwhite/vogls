use crate::VectorSize;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum FvLogicValue {
    /// Unknown value
    X = 0b00,
    /// Logical zero
    L0 = 0b01,
    /// Logical one
    L1 = 0b10,
    /// High impedance
    Z = 0b11,
}

impl FvLogicValue {
    #[inline(always)]
    pub const fn from_bool(value: bool) -> Self {
        match value {
            false => Self::L0,
            true => Self::L1,
        }
    }

    pub const fn from_repr(repr: u8) -> Self {
        match repr & 0b11 {
            0b00 => Self::X,
            0b01 => Self::L0,
            0b10 => Self::L1,
            _ => Self::Z,
        }
    }
}

impl From<bool> for FvLogicValue {
    #[inline(always)]
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

pub trait BitwisePart:
    Copy
    + std::ops::BitAnd<Output = Self>
    + std::ops::BitOr<Output = Self>
    + std::ops::BitXor<Output = Self>
    + std::ops::Not<Output = Self>
    + std::ops::Shl<u32, Output = Self>
    + std::ops::Shr<u32, Output = Self>
    + Eq
{
    const NUM_BITS: u32;
    fn splat_byte(b: u8) -> Self;
}

macro_rules! impl_bitwise_part {
    ($($ty:ty),+ $(,)?) => {
        $(
        impl BitwisePart for $ty {
            const NUM_BITS: u32 = <$ty>::BITS;
            #[inline(always)]
            fn splat_byte(b: u8) -> Self {
                let mut bs = [0u8; size_of::<Self>()];
                for i in 0..size_of::<Self>() {
                    bs[i] = b;
                }
                Self::from_le_bytes(bs)
            }

        }
        )+
    };
}

impl_bitwise_part!(u8, u64);
impl BitwisePart for FvLogicValue {
    const NUM_BITS: u32 = 2;
    #[inline(always)]
    fn splat_byte(b: u8) -> Self {
        Self::from_repr(b & 0b11)
    }
}

impl std::ops::BitAnd<Self> for FvLogicValue {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self::from_repr((self as u8) & (rhs as u8))
    }
}
impl std::ops::BitOr<Self> for FvLogicValue {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from_repr((self as u8) | (rhs as u8))
    }
}
impl std::ops::BitXor<Self> for FvLogicValue {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self::from_repr((self as u8) ^ (rhs as u8))
    }
}
impl std::ops::Not for FvLogicValue {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self::from_repr(!(self as u8) & 0b11)
    }
}
impl std::ops::Shl<u32> for FvLogicValue {
    type Output = Self;
    fn shl(self, rhs: u32) -> Self::Output {
        Self::from_repr((self as u8) << rhs)
    }
}
impl std::ops::Shr<u32> for FvLogicValue {
    type Output = Self;
    fn shr(self, rhs: u32) -> Self::Output {
        Self::from_repr((self as u8) >> rhs)
    }
}

pub fn bin_bitwise_op<T: BitwisePart>(dst: &mut [T], lhs: &[T], rhs: &[T], op: impl Fn(T, T) -> T) {
    for i in 0..dst.len() {
        dst[i] = op(lhs[i], rhs[i]);
    }
}
pub fn bin_mut_bitwise_op<T: BitwisePart>(dst: &mut [T], other: &[T], op: impl Fn(T, T) -> T) {
    for i in 0..dst.len() {
        dst[i] = op(dst[i], other[i]);
    }
}

pub fn unary_bitwise_op<T: BitwisePart>(dst: &mut [T], src: &[T], op: impl Fn(T) -> T) {
    for i in 0..dst.len() {
        dst[i] = op(src[i]);
    }
}
pub fn unary_mut_bitwise_op<T: BitwisePart>(dst: &mut [T], op: impl Fn(T) -> T) {
    for i in dst.iter_mut() {
        *i = op(*i);
    }
}

const ODD_BITS_BYTE: u8 = 0x55;
const EVEN_BITS_BYTE: u8 = 0xAA;

/// Does the `value` have a `Unknown` or `High Impedance` value?
#[inline(always)]
pub fn has_fv_non_logical<T: BitwisePart>(value: T) -> bool {
    (value ^ (value >> 1)) != T::splat_byte(ODD_BITS_BYTE)
}

#[inline(always)]
pub fn tv_bitwise_and<T: BitwisePart>(l: T, r: T) -> T {
    l & r
}
#[inline(always)]
pub fn tv_bitwise_or<T: BitwisePart>(l: T, r: T) -> T {
    l | r
}
#[inline(always)]
pub fn tv_bitwise_xor<T: BitwisePart>(l: T, r: T) -> T {
    l ^ r
}
#[inline(always)]
pub fn fv_bitwise_inv<T: BitwisePart>(value: T) -> T {
    //    ~
    // x  x
    // 0  1
    // 1  0
    // z  x
    //
    // z1z0 = fv.inv(x1x0)
    //
    // z0 = x0b x1        z1 = x0 x1b
    // x  0               x  0
    // 0  0               0  1
    // 1  1               1  0
    // z  0               z  0
    let x0 = value & T::splat_byte(ODD_BITS_BYTE);
    let x1 = value & T::splat_byte(EVEN_BITS_BYTE);
    let z0 = !x0 & x1;
    let z1 = x0 & !x1;
    z1 | z0
}
#[inline(always)]
pub fn fv_bitwise_and<T: BitwisePart>(x: T, y: T) -> T {
    // &  x  0  1  z
    // x  x  0  x  x
    // 0  0  0  0  0
    // 1  x  0  1  x
    // z  x  0  x  x
    //
    // z1z0 = fv.and(x1x0, y1y0)
    //
    // z0 = x0 y1b + y0 x1b         z1 = x1 y1 x0b y0b
    //    x  0  1  z                   x  0  1  z
    // x  0  1  0  0                x  0  0  0  0
    // 0  1  1  1  1                0  0  0  0  0
    // 1  0  1  0  0                1  0  0  1  0
    // z  0  1  0  0                z  0  0  0  0
    let x0 = x & T::splat_byte(ODD_BITS_BYTE);
    let x1 = x & T::splat_byte(EVEN_BITS_BYTE);
    let y0 = y & T::splat_byte(ODD_BITS_BYTE);
    let y1 = y & T::splat_byte(EVEN_BITS_BYTE);
    let z0 = (x0 & !(y1 >> 1)) | (y0 & !(x1 >> 1));
    let z1 = x1 & y1 & !(x0 << 1) & !(y0 << 1);
    z1 | z0
}
#[inline(always)]
pub fn fv_bitwise_or<T: BitwisePart>(x: T, y: T) -> T {
    // &  x  0  1  z
    // x  x  x  1  x
    // 0  x  0  1  x
    // 1  1  1  1  1
    // z  x  x  1  x
    //
    // z1z0 = fv.or(x1x0, y1y0)
    //
    // z0 = x1b y1b x0 y0         z1 = x0b y1 + y0b x1
    //    x  0  1  z                   x  0  1  z
    // x  0  0  0  0                x  0  0  1  0
    // 0  0  1  0  0                0  0  0  1  0
    // 1  0  0  0  0                1  1  1  1  1
    // z  0  0  0  0                z  0  0  1  0
    let x0 = x & T::splat_byte(ODD_BITS_BYTE);
    let x1 = x & T::splat_byte(EVEN_BITS_BYTE);
    let y0 = y & T::splat_byte(ODD_BITS_BYTE);
    let y1 = y & T::splat_byte(EVEN_BITS_BYTE);
    let z0 = !(x1 >> 1) & !(y1 >> 1) & x0 & y0;
    let z1 = (!(x0 << 1) & y1) | (!(y0 << 1) & x1);
    z1 | z0
}
#[inline(always)]
pub fn fv_bitwise_xor<T: BitwisePart>(x: T, y: T) -> T {
    // &  x  0  1  z
    // x  x  x  x  x
    // 0  x  1  0  x
    // 1  x  0  1  x
    // z  x  x  x  x
    //
    // z1z0 = fv.xor(x1x0, y1y0)
    //
    // z0 = x1^x0 & y1^y0 & (x0^y0)b   z1 = x1^x0 & y1^y0 & x0^y0
    //    x  0  1  z                      x  0  1  z
    // x  0  0  0  0                   x  0  0  0  0
    // 0  0  1  0  0                   0  0  0  1  0
    // 1  0  0  1  0                   1  0  1  0  0
    // z  0  0  0  0                   z  0  0  0  0
    let x0 = x & T::splat_byte(ODD_BITS_BYTE);
    let x1 = x & T::splat_byte(EVEN_BITS_BYTE);
    let y0 = y & T::splat_byte(ODD_BITS_BYTE);
    let y1 = y & T::splat_byte(EVEN_BITS_BYTE);

    let t0 = x0 ^ (x1 >> 1);
    let t1 = y0 ^ (y1 >> 1);
    let t2 = x0 ^ y0;

    let r = t0 & t1;

    let z0 = r & !t2;
    let z1 = (r & t2) << 1;

    z1 | z0
}
#[inline(always)]
pub fn fv_equality<T: BitwisePart>(x: T, y: T) -> FvLogicValue {
    if has_fv_non_logical(x) | has_fv_non_logical(y) {
        return FvLogicValue::X;
    }

    FvLogicValue::from_bool(x == y)
}

pub fn tv_addition(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_add(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_subtraction(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_sub(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_multiplication(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_mul(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_division(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    // @TODO: Deal with r = 0
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_div(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_modulus(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    // @TODO: Deal with r = 0
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_rem(r);
    store_partial_u64(dst, out, size);
}
