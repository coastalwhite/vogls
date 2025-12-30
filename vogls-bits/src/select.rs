use crate::VectorSize;

pub fn tv_select_bit(src: &[u8], idx: u32, size: VectorSize) -> bool {
    if idx > size.get() {
        todo!()
    }
    ((src[(idx / 8) as usize] >> (idx % 8)) & 1) != 0
}
