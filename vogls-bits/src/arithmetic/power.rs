use crate::VectorSize;
use crate::arithmetic::{fv_contains_special, fv_set_no_special};
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

use super::fv_ltu32_arith_op;

pub fn tv_ltu64_power(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 32 {
        todo!();
    }

    assert!(size.get() <= 64);
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_pow(r as u32);
    store_partial_u64(dst, out, size);
}
pub fn fv_ltu32_power(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    fv_ltu32_arith_op(dst, lhs, rhs, size, |l, r| Some(l.wrapping_pow(r as u32)));
}

/// Two-value logic arbitrary precision multiplication.
pub fn tv_power(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
    if size.get() > 32 {
        todo!()
    }
    dst[0] = lhs[0].wrapping_pow(rhs[0] as u32);
    dst[0] &= (1u64 << size.get()) - 1;
}
pub fn fv_power(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
    assert!(
        dst.len() > 0
            && dst.len() == lhs.len()
            && dst.len() == rhs.len()
            && dst.len() == 2 * size.get().div_ceil(64) as usize
    );

    if fv_contains_special(lhs, size) || fv_contains_special(rhs, size) {
        dst.fill(0);
        return;
    }

    fv_set_no_special(dst, size);
    let nwords = dst.len() / 2;
    tv_power(&mut dst[nwords..], &lhs[nwords..], &rhs[nwords..], size);
}
