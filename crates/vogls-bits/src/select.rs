use crate::VectorSize;

pub fn tv_select_bit(src: &[u8], idx: u32, size: VectorSize) -> bool {
    if idx > size.get() {
        todo!()
    }
    ((src[(idx / 8) as usize] >> (idx % 8)) & 1) != 0
}

pub fn tv_gtu64_select_bit(src: &[u64], idx: u32, size: VectorSize) -> bool {
    if idx >= size.get() {
        return false;
    }
    ((src[(idx / 64) as usize] >> (idx % 64)) & 1) != 0
}
