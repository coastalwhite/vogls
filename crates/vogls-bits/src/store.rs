use crate::VectorSize;

pub fn store_partial_u64(slice: &mut [u8], value: u64, size: VectorSize) {
    assert!(size.get() <= 64 && slice.len() >= size.get().div_ceil(8) as usize);
    let nbytes = size.get().div_ceil(8) as usize;
    let value = value & 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    slice[..nbytes].copy_from_slice(&value.to_le_bytes()[..nbytes]);
}
