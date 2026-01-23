use crate::VectorSize;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

mod add_sub;
mod division;
mod multiplication;

const SIZE32: VectorSize = VectorSize::new(32).unwrap();

pub use add_sub::{tv_addition, tv_ltu64_addition, tv_ltu64_subtraction, tv_subtraction};
pub use division::{tv_division, tv_ltu64_division, tv_ltu64_modulus};
pub use multiplication::{tv_ltu64_multiplication, tv_multiplication};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum FvLogicValue {
    /// Unknown value
    X = 0b00,
    /// High impedance
    Z = 0b01,
    /// Logical zero
    L0 = 0b10,
    /// Logical one
    L1 = 0b11,
}

impl FvLogicValue {
    pub const VALUES: &[FvLogicValue] = &[Self::X, Self::Z, Self::L0, Self::L1];

    #[inline(always)]
    pub const fn from_bool(value: bool) -> Self {
        match value {
            false => Self::L0,
            true => Self::L1,
        }
    }

    pub const fn from_repr(repr: u8) -> Self {
        match repr & 0b11 {
            0b01 => Self::Z,
            0b10 => Self::L0,
            0b11 => Self::L1,
            _ => Self::X,
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
    const ZERO: Self;
    const NUM_BITS: u32;
    const SPC_MASK: Self;
    const VAL_MASK: Self;
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
            const ZERO: Self = 0;
            const NUM_BITS: u32 = <$ty>::BITS;
            const SPC_MASK: Self = (!0) << (Self::NUM_BITS / 2);
            const VAL_MASK: Self = (!0) >> (Self::NUM_BITS / 2);

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
    const ZERO: Self = FvLogicValue::X;
    const NUM_BITS: u32 = 2;
    const SPC_MASK: Self = FvLogicValue::L0;
    const VAL_MASK: Self = FvLogicValue::Z;

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

/// Does the `value` have a `Unknown` or `High Impedance` value?
#[inline(always)]
pub fn has_fv_non_logical<T: BitwisePart>(value: T, size: VectorSize) -> bool {
    (value >> (T::NUM_BITS / 2)).count_ones() != size.get().min(T::NUM_BITS / 2)
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
    //   x z 1 0
    // ~ x x 0 1
    //
    // z1z0 = fv.inv(x1x0)
    //
    // z1 = x1            z0 = x1 x0b
    // x  0               x  0
    // z  0               z  0
    // 0  1               0  1
    // 1  1               1  0
    let x1 = value & T::SPC_MASK;
    let x0 = value & T::VAL_MASK;
    let z1 = x1;
    let z0 = (x1 >> (T::NUM_BITS / 2)) & !x0;
    z1 | z0
}
#[inline(always)]
pub fn fv_bitwise_and<T: BitwisePart>(x: T, y: T) -> T {
    // & | x  z  1  0
    // --+-----------
    // x | x  x  x  0
    // z | x  x  x  0
    // 1 | x  x  1  0
    // 0 | 0  0  0  0
    //
    // z1z0 = fv.and(x1x0, y1y0)
    //
    // z0 = x1 x0 y1 y0             z1 = x1 x0b + y1 y0b + x1 y1
    // &0| x  z  1  0               &1| x  z  1  0
    // --+-----------               --+-----------
    // x | 0  0  0  0               x | 0  0  0  1
    // z | 0  0  0  0               z | 0  0  0  1
    // 0 | 0  0  1  0               1 | 0  0  1  1
    // 1 | 0  0  0  0               0 | 1  1  1  1
    let x1 = x & T::SPC_MASK;
    let x0 = x & T::VAL_MASK;
    let y1 = y & T::SPC_MASK;
    let y0 = y & T::VAL_MASK;

    let x1s = x1 >> (T::NUM_BITS / 2);
    let y1s = y1 >> (T::NUM_BITS / 2);

    let z0 = x1s & x0 & y1s & y0;
    let z1 = ((x1s & !x0) | (y1s & !y0) | z0) << (T::NUM_BITS / 2);
    z1 | z0
}
#[inline(always)]
pub fn fv_bitwise_or<T: BitwisePart>(x: T, y: T) -> T {
    // | | x  z  1  0
    // --+-----------
    // x | x  x  1  x
    // z | x  x  1  x
    // 1 | 1  1  1  1
    // 0 | x  x  1  0
    //
    // z1z0 = fv.or(x1x0, y1y0)
    //
    // z0 = x1 x0 + y1 y0           z1 = x1 x0 + y1 y0 + x1 y1
    // |0| x  z  1  0               |1| x  z  1  0
    // --+-----------               --+-----------
    // x | 0  0  1  0               x | 0  0  1  0
    // z | 0  0  1  0               z | 0  0  1  0
    // 0 | 1  1  1  1               1 | 1  1  1  1
    // 1 | 0  0  1  0               0 | 0  0  1  1
    let x1 = x & T::SPC_MASK;
    let x0 = x & T::VAL_MASK;
    let y1 = y & T::SPC_MASK;
    let y0 = y & T::VAL_MASK;

    let x1s = x1 >> (T::NUM_BITS / 2);
    let y1s = y1 >> (T::NUM_BITS / 2);

    let z0 = (x1s & x0) | (y1s & y0);
    let z1 = (z0 << (T::NUM_BITS / 2)) | (x1 & y1);
    z1 | z0
}
#[inline(always)]
pub fn fv_bitwise_xor<T: BitwisePart>(x: T, y: T) -> T {
    // ^ | x  z  1  0
    // --+-----------
    // x | x  x  x  x
    // z | x  x  x  x
    // 1 | x  x  0  1
    // 0 | x  x  1  0
    //
    // z1z0 = fv.xor(x1x0, y1y0)
    //
    // z0 = x1 y1 (x0 ^ y0)         z1 = x1 y1
    // ^0| x  z  1  0               ^1| x  z  1  0
    // --+-----------               --+-----------
    // x | 0  0  0  0               x | 0  0  0  0
    // z | 0  0  0  0               z | 0  0  0  0
    // 0 | 0  0  0  1               1 | 0  0  1  1
    // 1 | 0  0  1  0               0 | 0  0  1  1
    let x1 = x & T::SPC_MASK;
    let x0 = x & T::VAL_MASK;
    let y1 = y & T::SPC_MASK;
    let y0 = y & T::VAL_MASK;

    let x1s = x1 >> (T::NUM_BITS / 2);
    let y1s = y1 >> (T::NUM_BITS / 2);

    let z0 = x1s & y1s & (x0 ^ y0);
    let z1 = x1 & y1;
    z1 | z0
}
#[inline(always)]
pub fn fv_equality<T: BitwisePart>(x: T, y: T, size: VectorSize) -> FvLogicValue {
    if has_fv_non_logical(x, size) | has_fv_non_logical(y, size) {
        return FvLogicValue::X;
    }

    FvLogicValue::from_bool(x == y)
}
#[inline(always)]
pub fn fv_reduce_and<T: BitwisePart>(x: T, size: VectorSize) -> FvLogicValue {
    // & | x  z  1  0
    // --+-----------
    // x | x  x  x  0
    // z | x  x  x  0
    // 1 | x  x  1  0
    // 0 | 0  0  0  0
    //
    // z1z0 = fv.redand(sn vn ... s0 v0)
    //
    // z1 = (&si) | (s & !v != 0)
    let mask = T::mask(size);
    let spc_mask = mask << (T::NUM_BITS / 2);
    let val_mask = mask;
    let x1 = x & spc_mask;
    let x0 = x & val_mask;

    let x1s = x1 >> (T::NUM_BITS / 2);

    let z1 = (x1 == spc_mask) | (x1s & !x0 != T::ZERO);
    let z0 = (x1 == spc_mask) & (x0 == val_mask);
    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}
#[inline(always)]
pub fn fv_reduce_or<T: BitwisePart>(x: T, size: VectorSize) -> FvLogicValue {
    // | | x  z  1  0
    // --+-----------
    // x | x  x  1  x
    // z | x  x  1  x
    // 1 | 1  1  1  1
    // 0 | x  x  1  0
    let mask = T::mask(size);
    let spc_mask = mask << (T::NUM_BITS / 2);
    let val_mask = mask;
    let x1 = x & spc_mask;
    let x0 = x & val_mask;

    let x1s = x1 >> (T::NUM_BITS / 2);

    let z1 = (x1 == spc_mask) | ((x1s & x0) != T::ZERO);
    let z0 = (x1s & x0) != T::ZERO;
    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}
#[inline(always)]
pub fn fv_reduce_xor<T: BitwisePart>(x: T, size: VectorSize) -> FvLogicValue {
    // ^ | x  z  1  0
    // --+-----------
    // x | x  x  x  x
    // z | x  x  x  x
    // 1 | x  x  0  1
    // 0 | x  x  1  0
    let mask = T::mask(size);
    let spc_mask = mask << (T::NUM_BITS / 2);
    let val_mask = mask;
    let x1 = x & spc_mask;
    let x0 = x & val_mask;

    let x1s = x1 >> (T::NUM_BITS / 2);

    let z1 = x1 == spc_mask;
    let z0 = z1 & ((x1s & x0).count_ones() % 2 == 1);
    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}

pub fn fv_contains_special(src: &[u64], size: VectorSize) -> bool {
    assert!(src.len() > 0 && size.get().div_ceil(64) as usize == src.len());
    for v in 0..src.len() - 1 {
        if has_fv_non_logical(src[v], SIZE32) {
            return true;
        }
    }

    let rem_size = VectorSize::new(size.get() % 32).unwrap_or(SIZE32);
    has_fv_non_logical(fv_fixup_last_u64(src[src.len() - 1], rem_size), rem_size)
}

#[inline(always)]
pub fn fv_fixup_last_u64(v: u64, size: VectorSize) -> u64 {
    debug_assert!(size.get() <= 32);
    let mask = (1u64 << size.get()) - 1;
    ((v & !mask) << (size.get() - 32)) | (v & mask)
}

pub fn fv_ltu32_arith_op(
    dst: &mut [u8],
    lhs: &[u8],
    rhs: &[u8],
    size: VectorSize,
    op: impl Fn(u64, u64) -> Option<u64>,
) {
    debug_assert!(
        dst.len() > 0
            && dst.len() == lhs.len()
            && dst.len() == rhs.len()
            && size.get() <= 32
            && size.get().div_ceil(4) as usize == dst.len()
    );
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let mask = (1u64 << size.get()) - 1;
    let l = load_partial_u64(&lhs, dsize);
    let r = load_partial_u64(&rhs, dsize);

    // If has special values, return X.
    if l >> size.get() != mask || r >> size.get() != mask {
        dst.fill(0);
        return;
    }

    let out = match op(l, r) {
        None => 0u64,
        Some(out) => (mask << size.get()) | (out & mask),
    };
    store_partial_u64(dst, out, dsize);
}

#[cfg(test)]
mod tests {
    use crate::proptest::any_reasonable_size;

    use super::*;
    use FvLogicValue as L;
    use proptest::prelude::Just;

    #[rustfmt::skip]
    const FV_BITWISE_AND_LUT: [L; 16] = [
        // x      z       0     1      &
        L::X,  L::X,  L::L0, L::X,  // x
        L::X,  L::X,  L::L0, L::X,  // z
        L::L0, L::L0, L::L0, L::L0, // 0
        L::X,  L::X,  L::L0, L::L1, // 1
    ];
    #[rustfmt::skip]
    const FV_BITWISE_OR_LUT: [L; 16] = [
        // x      z       0     1      &
        L::X,  L::X,  L::X,  L::L1, // x
        L::X,  L::X,  L::X,  L::L1, // z
        L::X,  L::X,  L::L0, L::L1, // 0
        L::L1, L::L1, L::L1, L::L1, // 1
    ];
    #[rustfmt::skip]
    const FV_BITWISE_XOR_LUT: [L; 16] = [
        // x      z       0     1       &
        L::X,  L::X,  L::X,  L::X,   // x
        L::X,  L::X,  L::X,  L::X,   // z
        L::X,  L::X,  L::L0, L::L1,  // 0
        L::X,  L::X,  L::L1, L::L0,  // 1
    ];
    #[rustfmt::skip]
    const FV_BITWISE_INV_LUT: [L; 4] = [
        // x      z       0       1 
        L::X,  L::X,  L::L1,  L::L0,
    ];
    #[rustfmt::skip]
    const FV_EQUALITY_LUT: [L; 16] = [
        // x      z       0     1      &
        L::X,  L::X,  L::X,  L::X,  // x
        L::X,  L::X,  L::X,  L::X,  // z
        L::X,  L::X,  L::L1, L::L0, // 0
        L::X,  L::X,  L::L0, L::L1, // 1
    ];

    #[test]
    fn test_fv_bitwise() {
        for &y in L::VALUES {
            for &x in L::VALUES {
                let z_and = fv_bitwise_and(x, y);
                let z_or = fv_bitwise_or(x, y);
                let z_xor = fv_bitwise_xor(x, y);

                let x_u8 = x as u8;
                let y_u8 = y as u8;
                let concat = ((x_u8 & 0b10) << 4)
                    | ((y_u8 & 0b10) << 3)
                    | ((x_u8 & 0b01) << 1)
                    | (y_u8 & 0b01);
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
                let z = fv_equality(x, y, VectorSize::new(1).unwrap());

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
        const S: VectorSize = VectorSize::new(1).unwrap();
        assert!(has_fv_non_logical(L::X, S));
        assert!(has_fv_non_logical(L::Z, S));
        assert!(!has_fv_non_logical(L::L0, S));
        assert!(!has_fv_non_logical(L::L1, S));
    }

    pub fn u8_slice_to_u64_vec(s: &[u8]) -> Vec<u64> {
        s.chunks(8)
            .map(|c| {
                let mut x = [0u8; 8];
                for (i, b) in c.iter().enumerate() {
                    x[i] = *b;
                }
                u64::from_le_bytes(x)
            })
            .collect()
    }

    pub fn u64_to_fvu64x2(v: u64, size: VectorSize) -> [u64; 2] {
        let bmask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
        let tmask = 1u64
            .unbounded_shl(size.get().saturating_sub(32))
            .wrapping_sub(1);
        [
            (bmask << size.get().min(32)) | (v & 0xFFFF_FFFF_FFFF_FFFF),
            (tmask << size.get().saturating_sub(32).min(32)) | (v >> 32) as u64,
        ]
    }
    pub fn u128_to_u64x2(v: u128) -> [u64; 2] {
        [v as u64, (v >> 64) as u64]
    }
    pub fn u64x2_to_slice(v: &[u64; 2], size: VectorSize) -> &[u64] {
        if size.get() > 64 { &v[..] } else { &v[..1] }
    }
    pub fn u64x2_to_slice_mut(v: &mut [u64; 2], size: VectorSize) -> &mut [u64] {
        if size.get() > 64 {
            &mut v[..]
        } else {
            &mut v[..1]
        }
    }
    pub fn fvu64x2_to_slice(v: &[u64; 2], size: VectorSize) -> &[u64] {
        if size.get() > 32 { &v[..] } else { &v[..1] }
    }
    pub fn fvu64x2_to_slice_mut(v: &mut [u64; 2], size: VectorSize) -> &mut [u64] {
        if size.get() > 32 {
            &mut v[..]
        } else {
            &mut v[..1]
        }
    }

    proptest::prop_compose! {
        pub fn u128_arith_target
            ()
            (size in any_reasonable_size(1..=128))
            (
                size in Just(size),
                lhs in 0..=(1u128.unbounded_shl(size.get())).wrapping_sub(1),
                rhs in 0..=(1u128.unbounded_shl(size.get())).wrapping_sub(1)
            )
                -> (VectorSize, u128, u128) {
                (size, lhs, rhs)
        }
    }
    proptest::prop_compose! {
        pub fn u64_arith_target
            ()
            (size in any_reasonable_size(1..=64))
            (
                size in Just(size),
                lhs in 0..=(1u64.unbounded_shl(size.get())).wrapping_sub(1),
                rhs in 0..=(1u64.unbounded_shl(size.get())).wrapping_sub(1)
            )
                -> (VectorSize, u64, u64) {
                (size, lhs, rhs)
        }
    }
}
