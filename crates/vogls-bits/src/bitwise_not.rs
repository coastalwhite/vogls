use crate::VectorSize;
use crate::arithmetic::fv_bitwise_inv_elem;
use crate::util::last_word_mask;
use std::cell::Cell;

pub fn tv_cell_not(dst: &[Cell<u64>], src: &[Cell<u64>], size: VectorSize) {
    assert!(dst.len() == src.len() && dst.len() == size.get().div_ceil(64) as usize);
    dst.iter().zip(src).for_each(|(d, s)| d.set(!s.get()));
    dst.last().unwrap().update(|v| v & last_word_mask(size));
}

pub fn fv_cell_not(dst: &[Cell<u64>], src: &[Cell<u64>], size: VectorSize) {
    assert!(dst.len() == src.len() && dst.len() == 2 * size.get().div_ceil(64) as usize);
    let offset = dst.len() / 2;
    for i in 0..offset {
        let (spc, val) = fv_bitwise_inv_elem(src[i].get(), src[offset + i].get());
        dst[i].set(spc);
        dst[offset + i].set(val);
    }
}
