use vogls_ir::VectorSize;

pub fn load_from_u64(stack: &mut [u8], offset: usize, size: VectorSize, value: u64) {
    assert!(size.get() <= 64);
    let nbytes = size.get().div_ceil(8) as usize;
    stack[offset..][..nbytes].copy_from_slice(&value.to_le_bytes()[..nbytes]);
    stack[offset + nbytes - 1] &= 1u8.unbounded_shl(size.get()).wrapping_sub(1);
}

pub fn store_to_u64(stack: &[u8], offset: usize, size: VectorSize) -> u64 {
    assert!(size.get() <= 64);
    if let Ok(bs) = stack[offset..][..(stack.len() - offset).min(8)].try_into() {
        let value = u64::from_le_bytes(bs);
        let value = value & 1u64.unbounded_shl(size.get()).wrapping_sub(1);
        value
    } else {
        let mut value = 0u64;
        for (i, &b) in stack[offset..][..size.get().div_ceil(8) as usize]
            .iter()
            .enumerate()
        {
            value |= (b as u64) << (8 * i);
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_u16() {
        let mut stack = [0u8; 8];
        for size in 1..=16 {
            let size = VectorSize::new(size).unwrap();
            let mask = (1u64 << size.get()).wrapping_sub(1);
            for value in [0x0000, 0xFFFF, 0xABCD, 0x8181] {
                let value = value & mask;
                load_from_u64(&mut stack, 2, size, value);
                let result = store_to_u64(&stack, 2, size);
                if result != value {
                    eprintln!("value = {value:04X}");
                    eprintln!("size = {size}");

                    assert_eq!(result, value);
                }
            }
        }
    }
}
