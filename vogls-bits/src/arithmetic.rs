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
    pub const VALUES: &[FvLogicValue] = &[Self::X, Self::L0, Self::L1, Self::Z];

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
    + std::fmt::Debug
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
    fn mask(size: VectorSize) -> Self;
    fn count_ones(self) -> u32;
    fn as_u64(self) -> u64;
    fn from_u64(w: u64) -> Self;
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
            #[inline(always)]
            fn mask(size: VectorSize) -> Self {
                let one: Self = 1;
                one.unbounded_shl(size.get()).wrapping_sub(1)
            }
            #[inline(always)]
            fn count_ones(self) -> u32 {
                Self::count_ones(self)
            }
            #[inline(always)]
            fn as_u64(self) -> u64 {
                self as u64
            }
            #[inline(always)]
            fn from_u64(w: u64) -> Self {
                let zero: Self = 0;
                (w & (!zero) as u64) as Self
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
    #[inline(always)]
    fn mask(size: VectorSize) -> Self {
        Self::from_repr(1u8.unbounded_shl(size.get()).wrapping_sub(1))
    }
    #[inline(always)]
    fn count_ones(self) -> u32 {
        (self as u8).count_ones()
    }
    #[inline(always)]
    fn as_u64(self) -> u64 {
        self as u64
    }
    #[inline(always)]
    fn from_u64(w: u64) -> Self {
        Self::from_repr((w & 0b11) as u8)
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
    let odd_bits = T::splat_byte(ODD_BITS_BYTE);
    (value ^ (value >> 1)) & odd_bits != odd_bits
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
    let z0 = !x0 & (x1 >> 1);
    let z1 = (x0 << 1) & !x1;
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
    // z0 = x0 x1b + y0 y1b         z1 = x1 y1 x0b y0b
    //    x  0  1  z                   x  0  1  z
    // x  0  1  0  0                x  0  0  0  0
    // 0  1  1  1  1                0  0  0  0  0
    // 1  0  1  0  0                1  0  0  1  0
    // z  0  1  0  0                z  0  0  0  0
    let x0 = x & T::splat_byte(ODD_BITS_BYTE);
    let x1 = x & T::splat_byte(EVEN_BITS_BYTE);
    let y0 = y & T::splat_byte(ODD_BITS_BYTE);
    let y1 = y & T::splat_byte(EVEN_BITS_BYTE);
    let z0 = (x0 & !(x1 >> 1)) | (y0 & !(y1 >> 1));
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
    // z0 = x1b y1b x0 y0         z1 = x0b x1 + y0b y1
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
    let z1 = (!(x0 << 1) & x1) | (!(y0 << 1) & y1);
    z1 | z0
}
#[inline(always)]
pub fn fv_bitwise_xor<T: BitwisePart>(x: T, y: T) -> T {
    // &  x  0  1  z
    // x  x  x  x  x
    // 0  x  0  1  x
    // 1  x  1  0  x
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
    //
    // Components:
    // x1^x0:              y1^y0:              x0^y0:
    //    x  0  1  z          x  0  1  z          x  0  1  z
    // x  0  1  1  0       x  0  0  0  0       x  0  1  0  1
    // 0  0  1  1  0       0  1  1  1  1       0  1  0  1  0
    // 1  0  1  1  0       1  1  1  1  1       1  0  1  0  1
    // z  0  1  1  0       z  0  0  0  0       z  1  0  1  0
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
#[inline(always)]
pub fn fv_reduce_and<T: BitwisePart>(x: T, size: VectorSize) -> FvLogicValue {
    // &  x  0  1  z
    // x  x  0  x  x
    // 0  0  0  0  0
    // 1  x  0  1  x
    // z  x  0  x  x
    let zero = T::splat_byte(0);
    let mask = T::mask(VectorSize::new(size.get() * 2).unwrap());
    let odd_bits = T::splat_byte(ODD_BITS_BYTE);
    let even_bits = T::splat_byte(EVEN_BITS_BYTE);

    let x0 = x & odd_bits;
    let x1 = x & even_bits;

    let z0 = (!(x1 >> 1) & x0) != zero;
    let z1 = (x1 == even_bits & mask) & (x0 == zero);

    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}
#[inline(always)]
pub fn fv_reduce_or<T: BitwisePart>(x: T, size: VectorSize) -> FvLogicValue {
    // &  x  0  1  z
    // x  x  x  1  x
    // 0  x  0  1  x
    // 1  1  1  1  1
    // z  x  x  1  x
    let zero = T::splat_byte(0);
    let mask = T::mask(VectorSize::new(size.get() * 2).unwrap());
    let odd_bits = T::splat_byte(ODD_BITS_BYTE);
    let even_bits = T::splat_byte(EVEN_BITS_BYTE);

    let x0 = x & odd_bits;
    let x1 = x & even_bits;

    let z0 = (x1 == zero) & (x0 == odd_bits & mask);
    let z1 = x1 & !(x0 << 1) != zero;

    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}
#[inline(always)]
pub fn fv_reduce_xor<T: BitwisePart>(x: T, size: VectorSize) -> FvLogicValue {
    // &  x  0  1  z
    // x  x  x  x  x
    // 0  x  0  1  x
    // 1  x  1  0  x
    // z  x  x  x  x
    let odd_bits = T::splat_byte(ODD_BITS_BYTE);
    let even_bits = T::splat_byte(EVEN_BITS_BYTE);

    let x0 = x & odd_bits;
    let x1 = x & even_bits;

    let t0 = !(x1 >> 1) & x0;
    let t1 = x1 & !(x0 << 1);

    let num_l0 = t0.count_ones();
    let num_l1 = t1.count_ones();

    let t2 = num_l1 % 2 == 0;
    let t3 = num_l0 + num_l1 == size.get();

    let z0 = t2 & t3;
    let z1 = !t2 & t3;

    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}

pub fn fv_addition<T: BitwisePart>(x: T, y: T, carry_in: T, size: VectorSize) -> (T, T) {
    if has_fv_non_logical(x) | has_fv_non_logical(y) | has_fv_non_logical(carry_in) {
        return (T::splat_byte(0u8), T::splat_byte(0u8));
    }

    let x = extract_tv_u64(x.as_u64()) as u64;
    let y = extract_tv_u64(y.as_u64()) as u64;
    let carry_in = extract_tv_u64(carry_in.as_u64()) as u64;

    let sum = x.wrapping_add(y).wrapping_add(carry_in);
    let sum = sum & 1u64.unbounded_shl(size.get()).wrapping_sub(1);

    let carry_out = T::from_u64(encode_fv_u64((sum >> 32) as u32));
    let sum = T::from_u64(encode_fv_u64((sum & 0xFFFF_FFFF) as u32));

    (carry_out, sum)
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

#[inline(always)]
fn extract_tv_u64(w: u64) -> u32 {
    if is_x86_feature_detected!("bmi2") {
        (unsafe { ::std::arch::x86_64::_pext_u64(w, 0xAAAA_AAAA_AAAA_AAAA) }) as u32
    } else {
        !morton1(w)
    }
}
#[inline(always)]
fn encode_fv_u64(w: u32) -> u64 {
    if is_x86_feature_detected!("bmi2") {
        let w = unsafe { ::std::arch::x86_64::_pdep_u64(w as u64, 0xAAAA_AAAA_AAAA_AAAA) };
        w | (!w >> 1)
    } else {
        // Adapted from https://stackoverflow.com/questions/30539347/2d-morton-code-encode-decode-64bits
        let mut w = w as u64;
        w = (w | (w << 16)) & 0x0000FFFF0000FFFF;
        w = (w | (w << 8)) & 0x00FF00FF00FF00FF;
        w = (w | (w << 4)) & 0x0F0F0F0F0F0F0F0F;
        w = (w | (w << 2)) & 0x3333333333333333;
        w = (w | (w << 1)) & 0x5555555555555555;
        w |= !w << 1;
        !w
    }
}

// Adapted from https://stackoverflow.com/questions/30539347/2d-morton-code-encode-decode-64bits
// Extracts the odd bits
#[inline(always)]
fn morton1(w: u64) -> u32 {
    let w = w & 0x5555_5555_5555_5555;
    let w = (w | (w >> 1)) & 0x3333_3333_3333_3333;
    let w = (w | (w >> 2)) & 0x0F0F_0F0F_0F0F_0F0F;
    let w = (w | (w >> 4)) & 0x00FF_00FF_00FF_00FF;
    let w = (w | (w >> 8)) & 0x0000_FFFF_0000_FFFF;
    let w = (w | (w >> 16)) & 0x0000_0000_FFFF_FFFF;
    w as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use FvLogicValue as L;

    #[rustfmt::skip]
    const FV_BITWISE_AND_LUT: [L; 16] = [
        // x       0     1      z      &
        L::X,  L::L0, L::X,  L::X,  // x
        L::L0, L::L0, L::L0, L::L0, // 0
        L::X,  L::L0, L::L1, L::X,  // 1
        L::X,  L::L0, L::X,  L::X,  // z
    ];
    #[rustfmt::skip]
    const FV_BITWISE_OR_LUT: [L; 16] = [
        // x       0     1      z      &
        L::X,  L::X,  L::L1, L::X,  // x
        L::X,  L::L0, L::L1, L::X,  // 0
        L::L1, L::L1, L::L1, L::L1, // 1
        L::X,  L::X,  L::L1, L::X,  // z
    ];
    #[rustfmt::skip]
    const FV_BITWISE_XOR_LUT: [L; 16] = [
        // x       0     1      z      &
        L::X,  L::X,  L::X,  L::X,  // x
        L::X,  L::L0, L::L1, L::X,  // 0
        L::X,  L::L1, L::L0, L::X,  // 1
        L::X,  L::X,  L::X,  L::X,  // z
    ];
    #[rustfmt::skip]
    const FV_BITWISE_INV_LUT: [L; 4] = [
        // x       0      1      z
        L::X,  L::L1, L::L0,  L::X,
    ];
    #[rustfmt::skip]
    const FV_EQUALITY_LUT: [L; 16] = [
        // x       0     1      z      &
        L::X,  L::X,  L::X,  L::X,  // x
        L::X,  L::L1, L::L0, L::X,  // 0
        L::X,  L::L0, L::L1, L::X,  // 1
        L::X,  L::X,  L::X,  L::X,  // z
    ];

    #[test]
    fn test_fv_bitwise() {
        for &y in L::VALUES {
            for &x in L::VALUES {
                let z_and = fv_bitwise_and(x, y);
                let z_or = fv_bitwise_or(x, y);
                let z_xor = fv_bitwise_xor(x, y);

                let concat = ((x as u8) << 2) | (y as u8);
                let size = VectorSize::new(2).unwrap();
                let z_redand = fv_reduce_and(concat, size);
                let z_redor = fv_reduce_or(concat, size);
                let z_redxor = fv_reduce_xor(concat, size);

                let idx = (((y as u8) << 2) | (x as u8)) as usize;
                assert_eq!(z_and, FV_BITWISE_AND_LUT[idx], "{z_and:?} != {x:?} & {y:?}");
                assert_eq!(z_or, FV_BITWISE_OR_LUT[idx], "{z_or:?} != {x:?} | {y:?}");
                assert_eq!(z_xor, FV_BITWISE_XOR_LUT[idx], "{z_xor:?} != {x:?} ^ {y:?}");

                assert_eq!(
                    z_redand, FV_BITWISE_AND_LUT[idx],
                    "{z_redand:?} != &{{ {x:?}, {y:?} }}"
                );
                assert_eq!(
                    z_redor, FV_BITWISE_OR_LUT[idx],
                    "{z_redor:?} != | {{ {x:?}, {y:?} }}"
                );
                assert_eq!(
                    z_redxor, FV_BITWISE_XOR_LUT[idx],
                    "{z_redxor:?} != ^ {{ {x:?}, {y:?} }}"
                );
            }
        }
    }
    #[test]
    fn test_fv_equality() {
        for &y in L::VALUES {
            for &x in L::VALUES {
                let z = fv_equality(x, y);

                let idx = (((y as u8) << 2) | (x as u8)) as usize;
                assert_eq!(z, FV_EQUALITY_LUT[idx], "{z:?} != ({x:?} == {y:?})");
            }
        }
    }
    #[test]
    fn test_fv_bitwise_inv() {
        for &x in L::VALUES {
            let z = fv_bitwise_inv(x);
            let idx = (x as u8) as usize;
            assert_eq!(z, FV_BITWISE_INV_LUT[idx], "{z:?} = !{x:?}");
        }
    }
    #[test]
    fn test_fv_has_non_logical() {
        assert!(has_fv_non_logical(L::X));
        assert!(!has_fv_non_logical(L::L0));
        assert!(!has_fv_non_logical(L::L1));
        assert!(has_fv_non_logical(L::Z));
    }
}
