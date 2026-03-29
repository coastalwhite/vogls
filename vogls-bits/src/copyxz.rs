use crate::arithmetic::{fv_pack_u64, fv_unpack_u64};
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;
use crate::{VectorSize, util};

pub fn copy_x(src_spc: u64, src_val: u64, mask_spc: u64, mask_val: u64) -> (u64, u64) {
    let copy_mask = !mask_spc & !mask_val;
    (src_spc & !copy_mask, src_val & !copy_mask)
}
pub fn copy_z(src_spc: u64, src_val: u64, mask_spc: u64, mask_val: u64) -> (u64, u64) {
    let copy_mask = !mask_spc & mask_val;
    (src_spc & !copy_mask, src_val | copy_mask)
}

pub fn fv_l_copy_x(dst: &mut [u64], src: &[u64], mask: &[u64]) {
    let (src_spc, src_val) = src.split_at(src.len() / 2);
    let (mask_spc, mask_val) = mask.split_at(mask.len() / 2);
    fv_l_copy_x_sep(dst, src_spc, src_val, mask_spc, mask_val);
}
pub fn fv_l_copy_z(dst: &mut [u64], src: &[u64], mask: &[u64]) {
    let (src_spc, src_val) = src.split_at(src.len() / 2);
    let (mask_spc, mask_val) = mask.split_at(mask.len() / 2);
    fv_l_copy_z_sep(dst, src_spc, src_val, mask_spc, mask_val);
}

pub fn fv_l_copy_x_sep(
    dst: &mut [u64],
    src_spc: &[u64],
    src_val: &[u64],
    mask_spc: &[u64],
    mask_val: &[u64],
) {
    let nwords = dst.len() / 2;
    assert!(
        nwords == src_spc.len()
            && nwords == src_val.len()
            && nwords == mask_spc.len()
            && nwords == mask_val.len()
    );
    for i in 0..nwords {
        (dst[i], dst[nwords + i]) = copy_x(src_spc[i], src_val[i], mask_spc[i], mask_val[i]);
    }
}
pub fn fv_l_copy_z_sep(
    dst: &mut [u64],
    src_spc: &[u64],
    src_val: &[u64],
    mask_spc: &[u64],
    mask_val: &[u64],
) {
    let nwords = dst.len() / 2;
    assert!(
        nwords == src_spc.len()
            && nwords == src_val.len()
            && nwords == mask_spc.len()
            && nwords == mask_val.len()
    );
    for i in 0..nwords {
        (dst[i], dst[nwords + i]) = copy_z(src_spc[i], src_val[i], mask_spc[i], mask_val[i]);
    }
}
pub fn fv_tv_l_copy_x(
    dst: &mut [u64],
    src: &[u64],
    mask_spc: &[u64],
    mask_val: &[u64],
    _size: VectorSize,
) {
    let nwords = dst.len() / 2;
    assert!(nwords == src.len() && nwords == mask_spc.len() && nwords == mask_val.len());
    for i in 0..nwords {
        (dst[i], dst[nwords + i]) = copy_x(u64::MAX, src[i], mask_spc[i], mask_val[i]);
    }
}
pub fn fv_tv_l_copy_z(
    dst: &mut [u64],
    src: &[u64],
    mask_spc: &[u64],
    mask_val: &[u64],
    size: VectorSize,
) {
    let nwords = dst.len() / 2;
    assert!(nwords == src.len() && nwords == mask_spc.len() && nwords == mask_val.len());
    for i in 0..nwords - 1 {
        (dst[i], dst[nwords + i]) = copy_z(u64::MAX, src[i], mask_spc[i], mask_val[i]);
    }
    let rem_size = util::saturating_rem(size.get(), 64);
    (dst[nwords - 1], dst[2 * nwords - 1]) = copy_z(
        1u64.unbounded_shl(rem_size).wrapping_sub(1),
        src[nwords - 1],
        mask_spc[nwords - 1],
        mask_val[nwords - 1],
    );
}

pub fn fv_s_copy_x(dst: &mut [u8], src: &[u8], mask: &[u8], size: VectorSize) {
    let dsize = VectorSize::new(2 * size.get()).unwrap();
    let src = load_partial_u64(src, dsize);
    let mask = load_partial_u64(mask, dsize);

    let (src_spc, src_val) = fv_unpack_u64(src, size);
    let (mask_spc, mask_val) = fv_unpack_u64(mask, size);

    let (result_spc, result_val) = copy_x(src_spc, src_val, mask_spc, mask_val);
    let result = fv_pack_u64(result_spc, result_val, size);
    store_partial_u64(dst, result, dsize);
}
pub fn fv_s_copy_z(dst: &mut [u8], src: &[u8], mask: &[u8], size: VectorSize) {
    let dsize = VectorSize::new(2 * size.get()).unwrap();
    let src = load_partial_u64(src, dsize);
    let mask = load_partial_u64(mask, dsize);

    let (src_spc, src_val) = fv_unpack_u64(src, size);
    let (mask_spc, mask_val) = fv_unpack_u64(mask, size);

    let (result_spc, result_val) = copy_z(src_spc, src_val, mask_spc, mask_val);
    let result = fv_pack_u64(result_spc, result_val, size);
    store_partial_u64(dst, result, dsize);
}
