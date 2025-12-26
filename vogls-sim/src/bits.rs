use vogls_ir::VectorSize;

pub fn load_from_u64(stack: &mut [u8], offset: usize, size: VectorSize, value: u64) {
    assert!(size > 0);
    assert!(size <= 64);
    let nbytes = size.div_ceil(8) as usize;
    stack[offset..][..nbytes].copy_from_slice(&value.to_le_bytes()[..nbytes]);
    stack[offset + nbytes - 1] &= 1u8.unbounded_shl(size).wrapping_sub(1);
}

pub fn store_to_u64(stack: &[u8], offset: usize, size: VectorSize) -> u64 {
    assert!(size > 0);
    assert!(size <= 64);
    if let Ok(bs) = stack[offset..][..(stack.len() - offset).min(8)].try_into() {
        let value = u64::from_le_bytes(bs);
        let value = value & 1u64.unbounded_shl(size).wrapping_sub(1);
        value
    } else {
        let mut value = 0u64;
        for (i, &b) in stack[offset..][..size.div_ceil(8) as usize]
            .iter()
            .enumerate()
        {
            value |= (b as u64) << (8 * i);
        }
        value
    }
}

pub fn concat(
    stack: &mut [u8],
    dst: usize,
    lhs: usize,
    rhs: usize,
    lhs_size: VectorSize,
    rhs_size: VectorSize,
) {
    assert!(lhs_size > 0 && rhs_size > 0);

    let (lhs_size, rhs_size) = (lhs_size as usize, rhs_size as usize);

    let lbytes = lhs_size.div_ceil(8);
    let rbytes = rhs_size.div_ceil(8);
    let dbytes = (lhs_size + rhs_size).div_ceil(8);

    // No aliasing.
    debug_assert!(lhs + lbytes <= dst || lhs >= dst + dbytes);
    debug_assert!(rhs + rbytes <= dst || rhs >= dst + dbytes);

    for i in 0..rbytes {
        stack[dst + i] = stack[rhs + i];
    }

    let roff = rhs_size % 8;

    // Fast path: left side is empty or right side is aligned.
    if roff == 0 {
        for i in 0..lbytes {
            stack[dst + rbytes + i] = stack[lhs + i];
        }
        return;
    }

    stack[dst + rbytes - 1] |= stack[lhs] << roff;
    let s = lhs_size.saturating_sub(8 - roff);
    for i in 0..s / 8 {
        stack[dst + rbytes + i] = (stack[lhs + i] << roff) | (stack[lhs + i] >> (8 - roff));
    }
    if s % 8 > 0 {
        stack[dst + dbytes - 1] = stack[lhs + lbytes - 1] >> (8 - roff);
    }
}

pub fn slice(stack: &mut [u8], dst: usize, src: usize, width: VectorSize, n: VectorSize) {
    assert!(width > 0 && n > 0 && width <= n, "width = {width}, n = {n}");
    let width = width as usize;

    for i in 0..width.div_ceil(8) {
        stack[dst + i] = stack[src + i];
    }
    let woff = width % 8;
    if woff != 0 {
        stack[dst + width / 8] &= 1u8.unbounded_shl(woff as u32).wrapping_sub(1);
    }
}

pub fn logical_shift_right(
    stack: &mut [u8],
    dst: usize,
    src: usize,
    shift: VectorSize,
    width: VectorSize,
) {
    let (shift, width) = (shift as usize, width as usize);
    assert!(shift <= width);

    let nbytes = width.div_ceil(8);
    let soff = shift % 8;
    if shift == width {
        for i in 0..nbytes {
            stack[dst + i] = 0;
        }
    } else if soff == 0 {
        for i in 0..nbytes - shift / 8 {
            stack[dst + i] = stack[src + i + shift / 8];
        }
        for i in nbytes - shift / 8..nbytes {
            stack[dst + i] = 0;
        }
    } else {
        // X = [ A B C D ]
        //
        // LSR (X, 7)
        //
        // [ A >> 7 ]
        //
        // X = [ A B C D ]
        // LSR (X, 7)
        // [
        //    (A >> 7) | (B << 1)
        //    (B >> 7) | (C << 1)
        //    (C >> 7) | (D << 1)
        //     D >> 7
        // ]
        //
        // LSR (X, 15)
        // [
        //    (B >> 7) | (C << 1)
        //    (C >> 7) | (D << 1)
        //     D >> 7
        //    0
        // ]
        let sbytes = shift.div_ceil(8);
        for i in 0..nbytes - sbytes {
            stack[dst + i] =
                (stack[src + i + sbytes - 1] >> soff) | (stack[src + i + sbytes] << (8 - soff));
        }
        stack[dst + nbytes - sbytes] = stack[src + nbytes - 1] >> soff;
        for i in nbytes - sbytes + 1..nbytes {
            stack[dst + i] = 0;
        }
    }
}

pub fn set_subslice(
    mut dst: &mut [u8],
    src: &[u8],
    dst_size: VectorSize,
    offset: VectorSize,
    src_size: VectorSize,
) -> bool {
    assert!(offset + src_size <= dst_size);

    let mut offset = offset;
    dst = &mut dst[(offset / 8) as usize..];
    offset = offset % 8;

    // @Performance: Please do something better.
    let mut updated = false;
    for i in 0..src_size {
        let dst_idx = offset + i;
        let src_idx = i;

        let dst_byte = dst[(dst_idx / 8) as usize];
        let src_current = (src[(src_idx / 8) as usize] >> (src_idx % 8)) & 1;
        let new_dst_byte = dst_byte & !(1u8 << (dst_idx % 8));
        let new_dst_byte = new_dst_byte | (src_current << (dst_idx % 8));

        updated |= dst_byte != new_dst_byte;
        dst[(dst_idx / 8) as usize] = new_dst_byte;
    }
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_u16() {
        let mut stack = [0u8; 8];
        for size in 1..=16 {
            let mask = (1u64 << size).wrapping_sub(1);
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

    #[test]
    fn test_slice_u16() {
        let mut stack = [0u8; 16];
        for size in 1..=32 {
            let mask = (1u64 << size).wrapping_sub(1);
            for value in [0x0000, 0xFFFF, 0xABCD, 0x8181] {
                let value = value & mask;
                for width in 1..=size {
                    load_from_u64(&mut stack, 0, size, value);
                    slice(&mut stack, 8, 0, width, size);
                    let result = store_to_u64(&stack, 8, width);
                    let expected = value & (1u64 << width).wrapping_sub(1);
                    if result != expected {
                        eprintln!("value    = {value:08X}");
                        eprintln!("result   = {result:08X}");
                        eprintln!("expected = {expected:08X}");
                        eprintln!("size  = {size}");
                        eprintln!("width = {width}");

                        assert_eq!(result, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn test_lsr_u16() {
        let mut stack = [0u8; 16];
        for size in 1..=16 {
            for value in 0..=(1u64 << size).wrapping_sub(1) {
                for shift in 1..=size {
                    load_from_u64(&mut stack, 0, size, value);
                    logical_shift_right(&mut stack, 8, 0, shift, size);
                    let result = store_to_u64(&stack, 8, size);
                    let expected = value >> shift;
                    if result != expected {
                        eprintln!("value    = {value:04X}");
                        eprintln!("result   = {result:04X}");
                        eprintln!("expected = {expected:04X}");
                        eprintln!("size  = {size}");
                        eprintln!("shift = {shift}");

                        assert_eq!(result, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn test_concat_u16() {
        let mut stack = [0u8; 8 * 3];
        for lhs_size in 1..=16 {
            let lhs_mask = (1u64 << lhs_size).wrapping_sub(1);
            for rhs_size in 1..=16 {
                let rhs_mask = (1u64 << rhs_size).wrapping_sub(1);
                for lhs in [0x0000, 0xFFFF, 0xABCD, 0x8181] {
                    let lhs = lhs & lhs_mask;
                    for rhs in [0x0000, 0xFFFF, 0xABCD, 0x8181] {
                        let rhs = rhs & rhs_mask;
                        load_from_u64(&mut stack, 0, lhs_size, lhs);
                        load_from_u64(&mut stack, 8, rhs_size, rhs);

                        concat(&mut stack, 16, 0, 8, lhs_size, rhs_size);

                        let expected = (lhs << rhs_size) | rhs;
                        let result = store_to_u64(&stack, 16, lhs_size + rhs_size);

                        if result != expected {
                            eprintln!("lhs    = {lhs:04X} ({lhs_size})");
                            eprintln!("rhs    = {rhs:04X} ({rhs_size})");
                            eprintln!("result = {result:04X}");
                            assert_eq!(result, expected);
                        }
                    }
                }
            }
        }
    }
}
