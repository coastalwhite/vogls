use crate::VectorSize;
use crate::arithmetic::{fv_pack_u64, fv_separate_packed_u64};
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

pub fn tv_concat(
    dst: &mut [u8],
    lhs: &[u8],
    rhs: &[u8],
    lhs_size: VectorSize,
    rhs_size: VectorSize,
) {
    let (lhs_size, rhs_size) = (lhs_size.get() as usize, rhs_size.get() as usize);

    let lbytes = lhs_size.div_ceil(8);
    let rbytes = rhs_size.div_ceil(8);
    let dbytes = (lhs_size + rhs_size).div_ceil(8);

    for i in 0..rbytes {
        dst[i] = rhs[i];
    }

    let roff = rhs_size % 8;

    // Fast path: left side is empty or right side is aligned.
    if roff == 0 {
        for i in 0..lbytes {
            dst[rbytes + i] = lhs[i];
        }
        return;
    }

    dst[rbytes - 1] |= lhs[0] << roff;
    let mut s = lhs_size.saturating_sub(8 - roff);
    let mut i = 0;
    while s > roff {
        dst[rbytes + i] = (lhs[i] >> (8 - roff)) | (lhs[i + 1] << roff);
        s = s.saturating_sub(8);
        i += 1;
    }
    if s > 0 {
        dst[dbytes - 1] = lhs[lbytes - 1] >> (8 - roff);
    }
}
pub fn tv_l_concat(
    dst: &mut [u64],
    lhs: &[u64],
    rhs: &[u64],
    lhs_size: VectorSize,
    rhs_size: VectorSize,
) {
    let (lhs_size, rhs_size) = (lhs_size.get() as usize, rhs_size.get() as usize);

    let lwords = lhs_size.div_ceil(64);
    let rwords = rhs_size.div_ceil(64);
    let dwords = (lhs_size + rhs_size).div_ceil(64);

    for i in 0..rwords {
        dst[i] = rhs[i];
    }

    let roff = rhs_size % 64;

    // Fast path: left side is empty or right side is aligned.
    if roff == 0 {
        for i in 0..lwords {
            dst[rwords + i] = lhs[i];
        }
        return;
    }

    dst[rwords - 1] |= lhs[0] << roff;
    let mut s = lhs_size.saturating_sub(64 - roff);
    let mut i = 0;
    while s > roff {
        dst[rwords + i] = (lhs[i] >> (64 - roff)) | (lhs[i + 1] << roff);
        s = s.saturating_sub(64);
        i += 1;
    }
    if s > 0 {
        dst[dwords - 1] = lhs[lwords - 1] >> (64 - roff);
    }
}
pub fn fv_l_concat(
    dst: &mut [u64],
    lhs: &[u64],
    rhs: &[u64],
    lhs_size: VectorSize,
    rhs_size: VectorSize,
) {
    let lwords = lhs.len() / 2;
    let rwords = rhs.len() / 2;
    let dwords = dst.len() / 2;
    tv_l_concat(
        &mut dst[..dwords],
        &lhs[..lwords],
        &rhs[..rwords],
        lhs_size,
        rhs_size,
    );
    tv_l_concat(
        &mut dst[dwords..],
        &lhs[lwords..],
        &rhs[rwords..],
        lhs_size,
        rhs_size,
    );
}

pub fn fv_s_concat(
    dst: &mut [u8],
    lhs: &[u8],
    rhs: &[u8],
    lhs_size: VectorSize,
    rhs_size: VectorSize,
) {
    let x = load_partial_u64(lhs, lhs_size);
    let y = load_partial_u64(rhs, rhs_size);
    let (xspc, xvalue) = fv_separate_packed_u64(x, lhs_size);
    let (yspc, yvalue) = fv_separate_packed_u64(y, rhs_size);
    let spc = (xspc << rhs_size.get()) | yspc;
    let value = (xvalue << rhs_size.get()) | yvalue;
    let dsize = VectorSize::new(lhs_size.get() + rhs_size.get()).unwrap();
    let result = fv_pack_u64(spc, value, dsize);
    store_partial_u64(dst, result, dsize);
}

#[cfg(test)]
mod tests {
    use crate::get_disjoint_dst_s1_s2;
    use crate::load::load_partial_u64;
    use crate::store::store_partial_u64;

    use super::*;

    #[test]
    fn test_concat_u16() {
        let mut stack = [0u8; 8 * 3];
        for lhs_size in 1..=32 {
            let lhs_size = VectorSize::new(lhs_size).unwrap();
            let lhs_mask = (1u64 << lhs_size.get()).wrapping_sub(1);
            for rhs_size in 1..=32 {
                let rhs_size = VectorSize::new(rhs_size).unwrap();
                let rhs_mask = (1u64 << rhs_size.get()).wrapping_sub(1);
                for lhs in [0x00000000, 0xFFFFFFFF, 0xABCDEF01, 0x81818181] {
                    let lhs = lhs & lhs_mask;
                    for rhs in [0x00000000, 0xFFFFFFFF, 0xABCDEF01, 0x81818181] {
                        let rhs = rhs & rhs_mask;

                        let lhs_offset = 0;
                        let rhs_offset = lhs_size.get().div_ceil(8) as usize;
                        let dst_offset =
                            (lhs_size.get().div_ceil(8) + rhs_size.get().div_ceil(8)) as usize;

                        let lbytes = lhs_size.get().div_ceil(8) as usize;
                        let rbytes = rhs_size.get().div_ceil(8) as usize;
                        let dbytes = (lhs_size.get() + rhs_size.get()).div_ceil(8) as usize;

                        store_partial_u64(&mut stack[lhs_offset..], lhs, lhs_size);
                        store_partial_u64(&mut stack[rhs_offset..], rhs, rhs_size);

                        let (dst, lhs_slice, rhs_slice) = get_disjoint_dst_s1_s2(
                            &mut stack, dst_offset, dbytes, lhs_offset, lbytes, rhs_offset, rbytes,
                        );

                        tv_concat(dst, lhs_slice, rhs_slice, lhs_size, rhs_size);

                        let expected = (lhs << rhs_size.get()) | rhs;
                        let result = load_partial_u64(
                            dst,
                            VectorSize::new(lhs_size.get() + rhs_size.get()).unwrap(),
                        );

                        if result != expected {
                            eprintln!("lhs    = {lhs:08X} ({lhs_size})");
                            eprintln!("rhs    = {rhs:08X} ({rhs_size})");
                            eprintln!("result = {result:08X}");
                            assert_eq!(result, expected);
                        }
                    }
                }
            }
        }
    }
}
