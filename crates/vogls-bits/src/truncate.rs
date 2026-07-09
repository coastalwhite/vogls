use std::cell::Cell;

use crate::VectorSize;
use crate::arithmetic::{fv_pack_u64, fv_unpack_u64};
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;
use crate::util::CellSlice;

pub fn tv_s_truncate(dst: &mut [u8], src: &[u8], dst_size: VectorSize, src_size: VectorSize) {
    if dst_size.get() == src_size.get() {
        dst.copy_from_slice(src);
        return;
    }

    let src = load_partial_u64(src, src_size);
    let dst_mask = 1u64.unbounded_shl(dst_size.get()).wrapping_sub(1);
    store_partial_u64(dst, src & dst_mask, dst_size);
}
pub fn fv_s_truncate(dst: &mut [u8], src: &[u8], dst_size: VectorSize, src_size: VectorSize) {
    let src = load_partial_u64(src, VectorSize::new(2 * src_size.get()).unwrap());
    let (spc, val) = fv_unpack_u64(src, src_size);
    let dst_mask = 1u64.unbounded_shl(dst_size.get()).wrapping_sub(1);
    let result = fv_pack_u64(spc & dst_mask, val & dst_mask, dst_size);
    store_partial_u64(dst, result, VectorSize::new(2 * dst_size.get()).unwrap());
}

pub fn tv_l_truncate(dst: &mut [u64], src: &[u64], dst_size: VectorSize, src_size: VectorSize) {
    if dst_size.get() == src_size.get() {
        dst.copy_from_slice(src);
        return;
    }
    dst.copy_from_slice(&src[..dst.len()]);
    if dst_size.get() % 64 != 0 {
        dst[dst.len() - 1] &= (1u64 << (dst_size.get() % 64)) - 1;
    }
}
pub fn fv_l_truncate(dst: &mut [u64], src: &[u64], dst_size: VectorSize, src_size: VectorSize) {
    let dwords = dst.len() / 2;
    let swords = src.len() / 2;
    tv_l_truncate(&mut dst[..dwords], &src[..swords], dst_size, src_size);
    tv_l_truncate(&mut dst[dwords..], &src[swords..], dst_size, src_size);
}

pub fn tv_cell_truncate(
    dst: &[Cell<u64>],
    src: &[Cell<u64>],
    dst_size: VectorSize,
    src_size: VectorSize,
) {
    if dst_size.get() == src_size.get() {
        dst.copy_from_slice(src);
        return;
    }
    dst.copy_from_slice(&src[..dst.len()]);
    if dst_size.get() % 64 != 0 {
        dst[dst.len() - 1].update(|v| v & ((1u64 << (dst_size.get() % 64)) - 1));
    }
}
pub fn fv_cell_truncate(
    dst: &[Cell<u64>],
    src: &[Cell<u64>],
    dst_size: VectorSize,
    src_size: VectorSize,
) {
    let dwords = dst.len() / 2;
    let swords = src.len() / 2;
    tv_cell_truncate(&dst[..dwords], &src[..swords], dst_size, src_size);
    tv_cell_truncate(&dst[dwords..], &src[swords..], dst_size, src_size);
}
