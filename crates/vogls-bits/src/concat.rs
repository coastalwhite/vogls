use crate::VectorSize;
use crate::arithmetic::{fv_pack_u64, fv_unpack_u64};
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

    dst[..rbytes].copy_from_slice(rhs);
    let roff = rhs_size % 8;

    // Fast path: left side is empty or right side is aligned.
    if roff == 0 {
        dst[rbytes..].copy_from_slice(lhs);
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
    if (lhs_size + rhs_size) % 8 != 0 {
        dst[dbytes - 1] &= (1u8 << ((lhs_size + rhs_size) % 8)) - 1;
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

    dst[..rwords].copy_from_slice(rhs);

    let roff = rhs_size % 64;

    // Fast path: left side is empty or right side is aligned.
    if roff == 0 {
        dst[rwords..].copy_from_slice(lhs);
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

pub fn fv_w_concat(
    xspc: u64,
    xvalue: u64,
    yspc: u64,
    yvalue: u64,
    rhs_size: VectorSize,
) -> (u64, u64) {
    let spc = (xspc << rhs_size.get()) | yspc;
    let value = (xvalue << rhs_size.get()) | yvalue;
    (spc, value)
}
pub fn fv_s_concat(
    dst: &mut [u8],
    lhs: &[u8],
    rhs: &[u8],
    lhs_size: VectorSize,
    rhs_size: VectorSize,
) {
    let x = load_partial_u64(lhs, VectorSize::new(lhs_size.get() * 2).unwrap());
    let y = load_partial_u64(rhs, VectorSize::new(rhs_size.get() * 2).unwrap());
    let (xspc, xvalue) = fv_unpack_u64(x, lhs_size);
    let (yspc, yvalue) = fv_unpack_u64(y, rhs_size);
    let dst_size = VectorSize::new(lhs_size.get() + rhs_size.get()).unwrap();
    let (spc, value) = fv_w_concat(xspc, xvalue, yspc, yvalue, rhs_size);
    let result = fv_pack_u64(spc, value, dst_size);
    store_partial_u64(dst, result, VectorSize::new(dst_size.get() * 2).unwrap());
}
