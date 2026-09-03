use crate::VectorSize;
use crate::arithmetic::multiplication::tv_multiplication;
use crate::arithmetic::{fv_contains_special, fv_set_no_special};

/// Two-value logic arbitrary precision exponentiation.
pub fn tv_power(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
    assert!(!dst.is_empty() && dst.len() == lhs.len() && dst.len() == rhs.len());

    let nwords = dst.len();
    let bits = size.get() as usize;

    let mut scratch = vec![0u64; nwords * 2];
    let (scratch, base) = scratch.split_at_mut(nwords);
    base.copy_from_slice(lhs);

    dst.fill(0);
    dst[0] = 1;

    for bit in 0..bits {
        if (rhs[bit / 64] >> (bit % 64)) & 1 == 1 {
            tv_multiplication(scratch, dst, base, size);
            dst.copy_from_slice(scratch);
        }
        // Squaring the base is only needed for the remaining higher bits.
        if bit + 1 < bits {
            tv_multiplication(scratch, base, base, size);
            base.copy_from_slice(scratch);
        }
    }
}

/// Four-value logic arbitrary precision exponentiation.
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

#[cfg(test)]
mod tests {
    use super::tv_power;
    use crate::arithmetic::tests::{
        u64x2_to_slice, u64x2_to_slice_mut, u128_arith_target, u128_to_u64x2,
    };
    use proptest::proptest;

    proptest! {
        #[test]
        fn proptest_tv_power
            ((size, lhs, rhs) in u128_arith_target())
        {
            let mask = (1u128.unbounded_shl(size.get())).wrapping_sub(1);
            // Reference: exponentiation by squaring in u128, modulo 2**size.
            let mut expected = 1u128 & mask;
            let mut b = lhs & mask;
            let mut e = rhs & mask;
            while e != 0 {
                if e & 1 == 1 {
                    expected = expected.wrapping_mul(b) & mask;
                }
                b = b.wrapping_mul(b) & mask;
                e >>= 1;
            }
            let expected = u128_to_u64x2(expected);
            let mut given = [0u64; 2];

            tv_power(
                u64x2_to_slice_mut(&mut given, size),
                u64x2_to_slice(&u128_to_u64x2(lhs), size),
                u64x2_to_slice(&u128_to_u64x2(rhs), size),
                size
            );

            proptest::prop_assert_eq!(given, expected);
        }
    }
}
