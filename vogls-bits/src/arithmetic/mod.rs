use crate::VectorSize;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

mod add_sub;
mod division;
mod multiplication;
mod power;

pub use add_sub::{
    fv_addition, fv_ltu32_addition, fv_ltu32_subtraction, fv_subtraction, tv_addition,
    tv_ltu64_addition, tv_ltu64_subtraction, tv_subtraction,
};
pub use division::{
    fv_division, fv_ltu32_division, fv_ltu32_modulus, tv_division, tv_ltu64_division,
    tv_ltu64_modulus,
};
pub use multiplication::{
    fv_ltu32_multiplication, fv_multiplication, tv_ltu64_multiplication, tv_multiplication,
};
pub use power::{fv_ltu32_power, fv_power, tv_ltu64_power, tv_power};

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

    #[inline(always)]
    pub const fn from_spc_and_val(spc: bool, val: bool) -> Self {
        Self::from_repr(((spc as u8) << 1) | (val as u8))
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

pub fn tv_bin_bitwise_op(dst: &mut [u8], lhs: &[u8], rhs: &[u8], op: impl Fn(u8, u8) -> u8) {
    for i in 0..dst.len() {
        dst[i] = op(lhs[i], rhs[i]);
    }
}
pub fn tv_bin_mut_bitwise_op(dst: &mut [u8], other: &[u8], op: impl Fn(u8, u8) -> u8) {
    for i in 0..dst.len() {
        dst[i] = op(dst[i], other[i]);
    }
}
pub fn fv_bin_bitwise_op(
    dst: &mut [u8],
    lhs: &[u8],
    rhs: &[u8],
    size: VectorSize,
    op: impl Fn(u64, u64, u64, u64) -> (u64, u64),
) {
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let x = load_partial_u64(lhs, dsize);
    let y = load_partial_u64(rhs, dsize);
    let (xspc, xvalue) = fv_unpack_u64(x, size);
    let (yspc, yvalue) = fv_unpack_u64(y, size);
    let (spc, value) = op(xspc, xvalue, yspc, yvalue);
    let result = fv_pack_u64(spc, value, size);
    store_partial_u64(dst, result, dsize);
}
pub fn fv_bin_mut_bitwise_op(
    dst: &mut [u8],
    rhs: &[u8],
    size: VectorSize,
    op: impl Fn(u64, u64, u64, u64) -> (u64, u64),
) {
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let x = load_partial_u64(dst, dsize);
    let y = load_partial_u64(rhs, dsize);
    let (xspc, xvalue) = fv_unpack_u64(x, size);
    let (yspc, yvalue) = fv_unpack_u64(y, size);
    let (spc, value) = op(xspc, xvalue, yspc, yvalue);
    let result = fv_pack_u64(spc, value, size);
    store_partial_u64(dst, result, dsize);
}

pub fn tv_bin_u64_bitwise_op(
    dst: &mut [u64],
    lhs: &[u64],
    rhs: &[u64],
    op: impl Fn(u64, u64) -> u64,
) {
    assert!(dst.len() == lhs.len() && dst.len() == rhs.len());
    let nwords = dst.len();
    for i in 0..nwords {
        dst[i] = op(lhs[i], rhs[i]);
    }
}
pub fn tv_bin_u64_mut_bitwise_op(dst_lhs: &mut [u64], rhs: &[u64], op: impl Fn(u64, u64) -> u64) {
    assert!(dst_lhs.len() == rhs.len());
    let nwords = dst_lhs.len();
    for i in 0..nwords {
        dst_lhs[i] = op(dst_lhs[i], rhs[i]);
    }
}
pub fn fv_bin_u64_bitwise_op(
    dst: &mut [u64],
    lhs: &[u64],
    rhs: &[u64],
    op: impl Fn(u64, u64, u64, u64) -> (u64, u64),
) {
    assert!(dst.len() == lhs.len() && dst.len() == rhs.len());
    let nwords = dst.len() / 2;
    for i in 0..nwords {
        (dst[i], dst[nwords + i]) = op(lhs[i], lhs[nwords + i], rhs[i], rhs[nwords + i]);
    }
}
pub fn fv_bin_u64_mut_bitwise_op(
    dst_lhs: &mut [u64],
    rhs: &[u64],
    op: impl Fn(u64, u64, u64, u64) -> (u64, u64),
) {
    assert!(dst_lhs.len() == rhs.len());
    let nwords = dst_lhs.len() / 2;
    for i in 0..nwords {
        (dst_lhs[i], dst_lhs[nwords + i]) =
            op(dst_lhs[i], dst_lhs[nwords + i], rhs[i], rhs[nwords + i]);
    }
}

pub fn fv_reduce_bitwise_op(
    src: &[u64],
    size: VectorSize,
    unit: FvLogicValue,
    op: impl Fn(u64, u64, VectorSize) -> FvLogicValue,
) -> FvLogicValue {
    let mut gspc = unit as u64 >> 1;
    let mut gvalue = unit as u64 & 1;
    let nwords = src.len() / 2;
    let mut i = 0;
    let mut shift = 1;
    let mut size = size.get();
    while let Some(s) = VectorSize::new(size) {
        let r = op(src[i], src[nwords + i], s);
        gspc |= (r as u64 >> 1) << shift;
        gvalue |= (r as u64 & 1) << shift;
        if shift == 63 {
            let subresult = op(gspc, gvalue, VectorSize::new(64).unwrap());
            gspc = subresult as u64 >> 1;
            gvalue = subresult as u64 & 1;
            shift = 1;
        } else {
            shift += 1;
        }
        size = size.saturating_sub(64);
        i += 1;
    }
    op(gspc, gvalue, VectorSize::new(shift).unwrap())
}
pub fn fv_s_reduce_bitwise_op(
    src: &[u8],
    size: VectorSize,
    op: impl Fn(u64, u64, VectorSize) -> FvLogicValue,
) -> FvLogicValue {
    assert!(size.get() <= 32);
    let x = load_partial_u64(src, VectorSize::new(size.get() * 2).unwrap());
    let (spc, value) = fv_unpack_u64(x, size);
    op(spc, value, size)
}

pub fn fv_l_reduce_or(src: &[u64], size: VectorSize) -> FvLogicValue {
    fv_reduce_bitwise_op(src, size, FvLogicValue::L0, fv_reduce_or_elem)
}
pub fn fv_l_reduce_and(src: &[u64], size: VectorSize) -> FvLogicValue {
    fv_reduce_bitwise_op(src, size, FvLogicValue::L1, fv_reduce_and_elem)
}
pub fn fv_l_reduce_xor(src: &[u64], size: VectorSize) -> FvLogicValue {
    fv_reduce_bitwise_op(src, size, FvLogicValue::L0, fv_reduce_xor_elem)
}
pub fn fv_s_reduce_or(src: &[u8], size: VectorSize) -> FvLogicValue {
    fv_s_reduce_bitwise_op(src, size, fv_reduce_or_elem)
}
pub fn fv_s_reduce_and(src: &[u8], size: VectorSize) -> FvLogicValue {
    fv_s_reduce_bitwise_op(src, size, fv_reduce_and_elem)
}
pub fn fv_s_reduce_xor(src: &[u8], size: VectorSize) -> FvLogicValue {
    fv_s_reduce_bitwise_op(src, size, fv_reduce_xor_elem)
}

pub fn fv_s_select_bit(src: &[u8], idx: u32, size: VectorSize) -> FvLogicValue {
    if idx >= size.get() {
        return FvLogicValue::X;
    }

    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let x = load_partial_u64(src, dsize);
    let spc = (x >> (size.get() + idx)) & 1;
    let val = (x >> idx) & 1;
    FvLogicValue::from_repr(((spc as u8) << 1) | (val as u8))
}
pub fn fv_l_select_bit(src: &[u64], idx: u32, size: VectorSize) -> FvLogicValue {
    if idx >= size.get() {
        return FvLogicValue::X;
    }

    let nwords = src.len() / 2;
    let spc = (src[(idx / 64) as usize] >> (idx % 64)) & 1;
    let val = (src[nwords + (idx / 64) as usize] >> (idx % 64)) & 1;
    FvLogicValue::from_repr(((spc as u8) << 1) | (val as u8))
}

/// Does the `value` have a `Unknown` or `High Impedance` value?
#[inline(always)]
pub fn has_fv_non_logical(value: u64, size: VectorSize) -> bool {
    (value >> 32).count_ones() != size.get().min(32)
}

#[inline(always)]
pub fn fv_bitwise_inv_elem(spc: u64, value: u64) -> (u64, u64) {
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
    (spc, spc & !value)
}
#[inline(always)]
pub fn fv_bitwise_and_elem(xspc: u64, x: u64, yspc: u64, y: u64) -> (u64, u64) {
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
    let zvalue = xspc & x & yspc & y;
    let zspc = (xspc & !x) | (yspc & !y) | zvalue;
    (zspc, zvalue)
}
#[inline(always)]
pub fn fv_bitwise_or_elem(xspc: u64, x: u64, yspc: u64, y: u64) -> (u64, u64) {
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
    let zvalue = (xspc & x) | (yspc & y);
    let zspc = zvalue | (xspc & yspc);
    (zspc, zvalue)
}
#[inline(always)]
pub fn fv_bitwise_xor_elem(xspc: u64, x: u64, yspc: u64, y: u64) -> (u64, u64) {
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
    let zspc = xspc & yspc;
    let zvalue = xspc & yspc & (x ^ y);
    (zspc, zvalue)
}
#[inline(always)]
pub fn fv_equality(xspc: u64, x: u64, yspc: u64, y: u64, size: VectorSize) -> FvLogicValue {
    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    if (xspc & mask != mask) | (yspc & mask != mask) {
        return FvLogicValue::X;
    }

    FvLogicValue::from_bool(x == y)
}
#[inline(always)]
pub fn fv_reduce_and_elem(spc: u64, value: u64, size: VectorSize) -> FvLogicValue {
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
    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    let z1 = (spc == mask) | (spc & !value != 0);
    let z0 = (spc == mask) & (value == mask);
    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}
#[inline(always)]
pub fn fv_reduce_or_elem(spc: u64, value: u64, size: VectorSize) -> FvLogicValue {
    // | | x  z  1  0
    // --+-----------
    // x | x  x  1  x
    // z | x  x  1  x
    // 1 | 1  1  1  1
    // 0 | x  x  1  0

    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    let z0 = (spc & value) != 0;
    let z1 = (spc == mask) | z0;
    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}
#[inline(always)]
pub fn fv_reduce_xor_elem(spc: u64, value: u64, size: VectorSize) -> FvLogicValue {
    // ^ | x  z  1  0
    // --+-----------
    // x | x  x  x  x
    // z | x  x  x  x
    // 1 | x  x  0  1
    // 0 | x  x  1  0

    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    let z1 = spc == mask;
    let z0 = z1 & ((spc & value).count_ones() % 2 == 1);
    FvLogicValue::from_repr((u8::from(z1) << 1) | u8::from(z0))
}

pub fn fv_contains_special(src: &[u64], size: VectorSize) -> bool {
    assert!(src.len() > 0 && src.len() == 2 * (size.get().div_ceil(64) as usize));
    let nwords = src.len() / 2;
    for i in 0..nwords - 1 {
        if !src[i] != 0 {
            return true;
        }
    }
    let last_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << size.get() % 64) - 1
    };
    src[nwords - 1] & last_mask != last_mask
}

pub fn fv_contains_unknown(src: &[u64], size: VectorSize) -> bool {
    assert!(src.len() > 0 && src.len() == 2 * (size.get().div_ceil(64) as usize));
    let nwords = src.len() / 2;
    for i in 0..nwords - 1 {
        if !src[i] & !src[nwords + i] != 0 {
            return true;
        }
    }
    let last_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << size.get() % 64) - 1
    };
    !src[nwords - 1] & !src[2 * nwords - 1] & last_mask != 0
}

pub fn fv_contains_high_impedance(src: &[u64], size: VectorSize) -> bool {
    assert!(src.len() > 0 && src.len() == 2 * (size.get().div_ceil(64) as usize));
    let nwords = src.len() / 2;
    for i in 0..nwords - 1 {
        if !src[i] & src[nwords + i] != 0 {
            return true;
        }
    }
    let last_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << size.get() % 64) - 1
    };
    !src[nwords - 1] & src[2 * nwords - 1] & last_mask != 0
}

pub fn fv_s_contains_unknown(src: &[u8], size: VectorSize) -> bool {
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let x = load_partial_u64(src, dsize);
    let (spc, val) = fv_unpack_u64(x, size);
    !spc & !val & 1u64.unbounded_shl(size.get()).wrapping_sub(1) != 0
}

pub fn fv_unpack_u64(v: u64, size: VectorSize) -> (u64, u64) {
    debug_assert!(size.get() <= 32);
    (v >> size.get(), v & ((1u64 << size.get()) - 1))
}
pub fn fv_pack_u64(spc: u64, value: u64, size: VectorSize) -> u64 {
    debug_assert!(size.get() <= 32);
    (spc << size.get()) | value
}

pub fn fv_leu32_bitwise_inv(dst: &mut [u8], src: &[u8], size: VectorSize) {
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let src = load_partial_u64(&src, dsize);
    let (spc, value) = fv_unpack_u64(src, size);
    let (spc, value) = fv_bitwise_inv_elem(spc, value);
    let result = fv_pack_u64(spc, value, size);
    store_partial_u64(dst, result, dsize);
}

pub fn fv_gtu32_bitwise_inv(dst: &mut [u64], src: &[u64], size: VectorSize) {
    assert!(dst.len() == src.len() && dst.len() == 2 * size.get().div_ceil(64) as usize);
    let nwords = dst.len() / 2;
    for i in 0..nwords {
        (dst[i], dst[nwords + i]) = fv_bitwise_inv_elem(src[i], src[nwords + i]);
    }
}

pub fn fv_set_no_special(slice: &mut [u64], size: VectorSize) {
    assert!(slice.len() > 0 && slice.len() == 2 * (size.get().div_ceil(64) as usize));
    let nwords = slice.len() / 2;
    slice[..nwords].fill(u64::MAX);
    if size.get() % 64 != 0 {
        slice[nwords - 1] &= (1u64 << size.get() % 64) - 1;
    }
}

#[inline(always)]
pub fn fv_fixup_last_u64(v: u64, size: VectorSize) -> u64 {
    debug_assert!(size.get() <= 32);
    let mask = (1u64 << size.get()) - 1;
    ((v & !mask) << (32 - size.get())) | (v & mask)
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
                let x_u8 = x as u8;
                let y_u8 = y as u8;

                let xspc = (x_u8 >> 1) as u64;
                let xvalue = (x_u8 & 1) as u64;
                let yspc = (y_u8 >> 1) as u64;
                let yvalue = (y_u8 & 1) as u64;

                let (z_andspc, z_and) = fv_bitwise_and_elem(xspc, xvalue, yspc, yvalue);
                let (z_orspc, z_or) = fv_bitwise_or_elem(xspc, xvalue, yspc, yvalue);
                let (z_xorspc, z_xor) = fv_bitwise_xor_elem(xspc, xvalue, yspc, yvalue);

                let z_and = FvLogicValue::from_repr(((z_andspc << 1) | z_and) as u8);
                let z_or = FvLogicValue::from_repr(((z_orspc << 1) | z_or) as u8);
                let z_xor = FvLogicValue::from_repr(((z_xorspc << 1) | z_xor) as u8);

                let spc = (xspc << 1) | (yspc);
                let value = (xvalue << 1) | (yvalue);
                let size = VectorSize::new(2).unwrap();
                let z_redand = fv_reduce_and_elem(spc, value, size);
                let z_redor = fv_reduce_or_elem(spc, value, size);
                let z_redxor = fv_reduce_xor_elem(spc, value, size);

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
                let x_u8 = x as u8;
                let y_u8 = y as u8;

                let xspc = (x_u8 >> 1) as u64;
                let xvalue = (x_u8 & 1) as u64;
                let yspc = (y_u8 >> 1) as u64;
                let yvalue = (y_u8 & 1) as u64;

                let z = fv_equality(xspc, xvalue, yspc, yvalue, VectorSize::new(1).unwrap());

                let idx = (((y as u8) << 2) | (x as u8)) as usize;
                assert_eq!(z, FV_EQUALITY_LUT[idx], "{z:?} != ({x:?} == {y:?})");
            }
        }
    }
    #[test]
    fn test_fv_bitwise_inv() {
        for &x in L::VALUES {
            let x_u8 = x as u8;
            let xspc = (x_u8 >> 1) as u64;
            let xvalue = (x_u8 & 1) as u64;
            let (spc, value) = fv_bitwise_inv_elem(xspc, xvalue);
            let z = FvLogicValue::from_repr(((spc << 1) | value) as u8);
            let idx = (x as u8) as usize;
            assert_eq!(z, FV_BITWISE_INV_LUT[idx], "{z:?} = !{x:?}");
        }
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
