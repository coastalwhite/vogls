use crate::VectorSize;

pub fn load_full_u32(slice: &[u8]) -> u32 {
    u32::from_le_bytes(slice[..4].try_into().unwrap())
}

pub fn load_partial_u64(slice: &[u8], size: VectorSize) -> u64 {
    assert!(size.get() <= 64 && slice.len() >= size.get().div_ceil(8) as usize);
    if slice.len() >= 8 {
        return u64::from_le_bytes(slice[..8].try_into().unwrap());
    }

    let mut value = 0u64;
    for (i, &b) in slice.iter().enumerate() {
        value |= (b as u64) << (8 * i);
    }
    value
}
