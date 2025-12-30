use crate::VectorSize;

pub fn tv_negate(dst: &mut [u8], src: &[u8], size: VectorSize) {
    for i in 0..size.get().div_ceil(8) as usize {
        dst[i] = !src[i];
    }
}

pub fn tv_negate_mut(s: &mut [u8], size: VectorSize) {
    for i in 0..size.get().div_ceil(8) as usize {
        s[i] = !s[i];
    }
}
