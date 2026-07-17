use std::cell::Cell;

use crate::VectorSize;
use crate::util::{CellSlice, last_word_mask};

pub fn tv_l_truncate(dst: &mut [u64], src: &[u64], dst_size: VectorSize, src_size: VectorSize) {
    if dst_size.get() == src_size.get() {
        dst.copy_from_slice(src);
        return;
    }
    dst.copy_from_slice(&src[..dst.len()]);
    dst[dst.len() - 1] &= last_word_mask(dst_size);
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
    dst[dst.len() - 1].update(|v| v & last_word_mask(dst_size));
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
