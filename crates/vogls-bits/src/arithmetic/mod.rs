use std::cell::Cell;

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
    Z = 0b10,
    /// Logical zero
    L0 = 0b01,
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
        Self::from_repr(((val as u8) << 1) | (spc as u8))
    }

    pub const fn from_repr(repr: u8) -> Self {
        match repr & 0b11 {
            0b01 => Self::L0,
            0b10 => Self::Z,
            0b11 => Self::L1,
            _ => Self::X,
        }
    }

    pub fn spc(self) -> bool {
        (self as u8 & 1) != 0
    }
    pub fn val(self) -> bool {
        (self as u8 >> 1) != 0
    }

    pub fn to_char(self) -> char {
        match self {
            FvLogicValue::X => 'x',
            FvLogicValue::Z => 'z',
            FvLogicValue::L0 => '0',
            FvLogicValue::L1 => '1',
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
        let (spc, val) = fv_bitwise_and_elem(
            (self as u8 & 1) as u64,
            (self as u8 >> 1) as u64,
            (rhs as u8 & 1) as u64,
            (rhs as u8 >> 1) as u64,
        );
        let spc = spc as u8 & 1;
        let val = val as u8 & 1;
        Self::from_repr(spc | (val << 1))
    }
}
impl std::ops::BitOr<Self> for FvLogicValue {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        let (spc, val) = fv_bitwise_or_elem(
            (self as u8 & 1) as u64,
            (self as u8 >> 1) as u64,
            (rhs as u8 & 1) as u64,
            (rhs as u8 >> 1) as u64,
        );
        let spc = spc as u8 & 1;
        let val = val as u8 & 1;
        Self::from_repr(spc | (val << 1))
    }
}
impl std::ops::BitXor<Self> for FvLogicValue {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        let (spc, val) = fv_bitwise_xor_elem(
            (self as u8 & 1) as u64,
            (self as u8 >> 1) as u64,
            (rhs as u8 & 1) as u64,
            (rhs as u8 >> 1) as u64,
        );
        let spc = spc as u8 & 1;
        let val = val as u8 & 1;
        Self::from_repr(spc | (val << 1))
    }
}
impl std::ops::Not for FvLogicValue {
    type Output = Self;
    fn not(self) -> Self::Output {
        let (spc, val) = fv_bitwise_inv_elem((self as u8 & 1) as u64, (self as u8 >> 1) as u64);
        let spc = spc as u8 & 1;
        let val = val as u8 & 1;
        Self::from_repr(spc | (val << 1))
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

pub fn tv_bin_u64_cell_bitwise_op(
    dst: &[Cell<u64>],
    lhs: &[Cell<u64>],
    rhs: &[Cell<u64>],
    op: impl Fn(u64, u64) -> u64,
) {
    assert!(dst.len() == lhs.len() && dst.len() == rhs.len());
    let nwords = dst.len();
    for i in 0..nwords {
        dst[i].set(op(lhs[i].get(), rhs[i].get()));
    }
}

pub fn tv_bin_u64_cell_bitwise_mask_last_op(
    dst: &[Cell<u64>],
    lhs: &[Cell<u64>],
    rhs: &[Cell<u64>],
    op: impl Fn(u64, u64) -> u64,
    size: VectorSize,
) {
    assert!(dst.len() == lhs.len() && dst.len() == rhs.len());
    let nwords = dst.len();
    for i in 0..nwords {
        dst[i].set(op(lhs[i].get(), rhs[i].get()));
    }
    if let Some(d) = dst.last() {
        if size.get() % 64 != 0 {
            d.set(d.get() & (1u64 << (size.get() % 64)).wrapping_sub(1));
        }
    }
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

pub fn fv_bin_u64_cell_bitwise_op(
    dst: &[Cell<u64>],
    lhs: &[Cell<u64>],
    rhs: &[Cell<u64>],
    op: impl Fn(u64, u64, u64, u64) -> (u64, u64),
) {
    assert!(dst.len() == lhs.len() && dst.len() == rhs.len());
    let nwords = dst.len() / 2;
    for i in 0..nwords {
        let (spc, val) = op(
            lhs[i].get(),
            lhs[nwords + i].get(),
            rhs[i].get(),
            rhs[nwords + i].get(),
        );
        dst[i].set(spc);
        dst[nwords + i].set(val);
    }
}

pub fn fv_s_select_bit(src: &[u8], idx: u32, size: VectorSize) -> FvLogicValue {
    if idx >= size.get() {
        return FvLogicValue::X;
    }

    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let x = load_partial_u64(src, dsize);
    let (spc, val) = fv_unpack_u64(x, size);
    let spc = ((spc >> idx) & 1) != 0;
    let val = ((val >> idx) & 1) != 0;
    FvLogicValue::from_spc_and_val(spc, val)
}
pub fn fv_l_select_bit(src: &[u64], idx: u32, size: VectorSize) -> FvLogicValue {
    if idx >= size.get() {
        return FvLogicValue::X;
    }

    let nwords = src.len() / 2;
    let spc = ((src[(idx / 64) as usize] >> (idx % 64)) & 1) != 0;
    let val = ((src[nwords + (idx / 64) as usize] >> (idx % 64)) & 1) != 0;
    FvLogicValue::from_spc_and_val(spc, val)
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
    // 1 | 0  0  1  0               1 | 0  0  1  1
    // 0 | 0  0  0  0               0 | 1  1  1  1
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
    // 1 | 1  1  1  1               1 | 1  1  1  1
    // 0 | 0  0  1  0               0 | 0  0  1  1
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
    // 1 | 0  0  0  1               1 | 0  0  1  1
    // 0 | 0  0  1  0               0 | 0  0  1  1
    let zspc = xspc & yspc;
    let zvalue = xspc & yspc & (x ^ y);
    (zspc, zvalue)
}
#[inline(always)]
pub fn fv_bitwise_andnot_elem(xspc: u64, x: u64, yspc: u64, y: u64) -> (u64, u64) {
    // & | x  z  1  0
    // --+-----------
    // x | x  x  0  x
    // z | x  x  0  x
    // 1 | x  x  0  1
    // 0 | 0  0  0  0
    //
    // z1z0 = fv.andnot(x1x0, y1y0)
    //
    // z0 = x1 x0 y1 ~y0            z1 = x1 x0b + y1 y0b + x1 y1
    // &0| x  z  1  0               &1| x  z  1  0
    // --+-----------               --+-----------
    // x | 0  0  0  0               x | 0  0  1  0
    // z | 0  0  0  0               z | 0  0  1  0
    // 1 | 0  0  0  1               1 | 0  0  1  1
    // 0 | 0  0  0  0               0 | 1  1  1  1
    let zvalue = xspc & x & yspc & !y;
    let zspc = (xspc & !x) | (yspc & y) | zvalue;
    (zspc, zvalue)
}
#[inline(always)]
pub fn fv_bitwise_ornot_elem(xspc: u64, x: u64, yspc: u64, y: u64) -> (u64, u64) {
    // | | x  z  1  0
    // --+-----------
    // x | x  x  x  1
    // z | x  x  x  1
    // 1 | 1  1  1  1
    // 0 | x  x  0  1
    //
    // z1z0 = fv.ornot(x1x0, y1y0)
    //
    // z0 = ?                       z1 = x1 x0 + y1 y0 + x1 y1
    // |0| x  z  1  0               |1| x  z  1  0
    // --+-----------               --+-----------
    // x | 0  0  0  1               x | 0  0  0  1
    // z | 0  0  0  1               z | 0  0  0  1
    // 1 | 1  1  1  1               1 | 1  1  1  1
    // 0 | 0  0  0  1               0 | 0  0  1  1
    let zvalue = (xspc & x) | (yspc & !y);
    let zspc = zvalue | (xspc & yspc);
    (zspc, zvalue)
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
pub fn fv_cell_contains_special(src: &[Cell<u64>], size: VectorSize) -> bool {
    assert!(src.len() > 0 && src.len() == 2 * (size.get().div_ceil(64) as usize));
    let nwords = src.len() / 2;
    for i in 0..nwords - 1 {
        if !src[i].get() != 0 {
            return true;
        }
    }
    let last_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << size.get() % 64) - 1
    };
    src[nwords - 1].get() & last_mask != last_mask
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
    (v & ((1u64 << size.get()) - 1), v >> size.get())
}
pub fn fv_pack_u64(spc: u64, value: u64, size: VectorSize) -> u64 {
    debug_assert!(size.get() <= 32);
    (value << size.get()) | spc
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
    // @NOTE: We don't have to mask out the most significant bits because the inverted value of
    // unknown is unknown.
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
pub fn fv_cell_set_no_special(slice: &[Cell<u64>], size: VectorSize) {
    assert!(slice.len() > 0 && slice.len() == 2 * (size.get().div_ceil(64) as usize));
    let nwords = slice.len() / 2;
    slice[..nwords].iter().for_each(|v| v.set(u64::MAX));
    if size.get() % 64 != 0 {
        slice[nwords - 1].update(|v| v & (1u64 << size.get() % 64) - 1);
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
    if (l & mask) != mask || (r & mask) != mask {
        dst.fill(0);
        return;
    }

    let out = match op(l >> size.get(), r >> size.get()) {
        None => 0u64,
        Some(out) => mask | ((out & mask) << size.get()),
    };
    store_partial_u64(dst, out, dsize);
}

#[cfg(test)]
mod tests {
    use crate::proptest::any_reasonable_size;
    use crate::reduce::{fv_reduce_and_elem, fv_reduce_or_elem, fv_reduce_xor_elem};

    use super::*;
    use FvLogicValue as L;
    use FvLogicValue::*;
    use proptest::prelude::Just;

    #[rustfmt::skip]
    const FV_BIN_TEST_VECTORS: [[L; 7]; 16] = [
        //  lhs rhs       AND    OR     XOR    ANDNOT  ORNOT
        [   X,  X,        X,     X,     X,     X,      X,   ],
        [   X,  Z,        X,     X,     X,     X,      X,   ],
        [   X,  L0,       L0,    X,     X,     X,      L1,  ],
        [   X,  L1,       X,     L1,    X,     L0,     X,   ],

        [   Z,  X,        X,     X,     X,     X,      X,   ],
        [   Z,  Z,        X,     X,     X,     X,      X,   ],
        [   Z,  L0,       L0,    X,     X,     X,      L1,  ],
        [   Z,  L1,       X,     L1,    X,     L0,     X,   ],

        [   L0, X,        L0,    X,     X,     L0,     X,   ],
        [   L0, Z,        L0,    X,     X,     L0,     X,   ],
        [   L0, L0,       L0,    L0,    L0,    L0,     L1,  ],
        [   L0, L1,       L0,    L1,    L1,    L0,     L0,  ],

        [   L1, X,        X,     L1,    X,     X,      L1,  ],
        [   L1, Z,        X,     L1,    X,     X,      L1,  ],
        [   L1, L0,       L0,    L1,    L1,    L1,     L1,  ],
        [   L1, L1,       L1,    L1,    L0,    L0,     L1,  ],
    ];

    #[rustfmt::skip]
    const FV_BITWISE_INV_LUT: [[L; 2]; 4] = [
        [X,   X,  ],
        [Z,   X,  ],
        [L0,  L1, ],
        [L1,  L0, ],
    ];

    #[test]
    fn test_fv_bitwise() {
        for [
            x,
            y,
            expect_and,
            expect_or,
            expect_xor,
            expect_andnot,
            expect_ornot,
        ] in FV_BIN_TEST_VECTORS
        {
            let (xspc, xval) = (x.spc() as u64, x.val() as u64);
            let (yspc, yval) = (y.spc() as u64, y.val() as u64);

            let (z_andspc, z_and) = fv_bitwise_and_elem(xspc, xval, yspc, yval);
            let (z_orspc, z_or) = fv_bitwise_or_elem(xspc, xval, yspc, yval);
            let (z_xorspc, z_xor) = fv_bitwise_xor_elem(xspc, xval, yspc, yval);
            let (z_andnotspc, z_andnot) = fv_bitwise_andnot_elem(xspc, xval, yspc, yval);
            let (z_ornotspc, z_ornot) = fv_bitwise_ornot_elem(xspc, xval, yspc, yval);

            let z_and = FvLogicValue::from_repr(((z_and << 1) | z_andspc) as u8);
            let z_or = FvLogicValue::from_repr(((z_or << 1) | z_orspc) as u8);
            let z_xor = FvLogicValue::from_repr(((z_xor << 1) | z_xorspc) as u8);
            let z_andnot = FvLogicValue::from_repr(((z_andnot << 1) | z_andnotspc) as u8);
            let z_ornot = FvLogicValue::from_repr(((z_ornot << 1) | z_ornotspc) as u8);

            assert_eq!(
                z_and, expect_and,
                "{x:?} & {y:?}, expected = {expect_and:?}, gotten = {z_and:?}"
            );
            assert_eq!(
                z_or, expect_or,
                "{x:?} | {y:?}, expected = {expect_or:?}, gotten = {z_or:?}"
            );
            assert_eq!(
                z_xor, expect_xor,
                "{x:?} ^ {y:?}, expected = {expect_xor:?}, gotten = {z_xor:?}"
            );
            assert_eq!(
                z_andnot, expect_andnot,
                "{x:?} & !{y:?}, expected = {expect_andnot:?}, gotten = {z_andnot:?}"
            );
            assert_eq!(
                z_ornot, expect_ornot,
                "{x:?} | !{y:?}, expected = {expect_ornot:?}, gotten = {z_ornot:?}"
            );

            let spc = (xspc << 1) | (yspc);
            let value = (xval << 1) | (yval);
            let size = VectorSize::new(2).unwrap();
            let z_redand = fv_reduce_and_elem(spc, value, size);
            let z_redor = fv_reduce_or_elem(spc, value, size);
            let z_redxor = fv_reduce_xor_elem(spc, value, size);

            assert_eq!(
                z_redand, expect_and,
                "& {{ {x:?}, {y:?} }}, expected = {expect_and:?}, gotten = {z_and:?}"
            );
            assert_eq!(
                z_redor, expect_or,
                "| {{ {x:?} | {y:?} }}, expected = {expect_or:?}, gotten = {z_or:?}"
            );
            assert_eq!(
                z_redxor, expect_xor,
                "^ {{ {x:?}, {y:?} }}, expected = {expect_xor:?}, gotten = {z_xor:?}"
            );
        }
    }
    #[test]
    fn test_fv_bitwise_inv() {
        for [a, expect] in FV_BITWISE_INV_LUT {
            let spc = a.spc() as u64;
            let val = a.val() as u64;
            let (spc, val) = fv_bitwise_inv_elem(spc, val);
            let got = FvLogicValue::from_repr(((val << 1) | spc) as u8);

            assert_eq!(
                got, expect,
                "!{a:?}, expected = {expect:?}, gotten = {got:?}"
            );
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
