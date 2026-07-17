use crate::VectorSize;
use crate::arithmetic::{fv_contains_special, fv_set_no_special};

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
        !dst.is_empty()
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
