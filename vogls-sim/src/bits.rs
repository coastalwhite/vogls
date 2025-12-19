use vogls_ir::VectorSize;

pub fn load_from_u64(stack: &mut [u8], offset: usize, size: VectorSize, value: u64) {
    let size = size as usize;

    if size == 0 {
        return;
    }

    let nbytes = size.div_ceil(8);
    let noffbits = size % 8;
    if noffbits == 0 {
        for i in 0..nbytes {
            stack[offset + i] = ((value >> (nbytes - i - 1) * 8) & 0xFF) as u8;
        }
        return;
    }

    stack[offset] = ((value >> (size - noffbits)) & 0xFF) as u8;
    for i in 0..nbytes - 1 {
        stack[offset + 1 + i] = ((value >> 8 * (nbytes - 2 - i)) & 0xFF) as u8;
    }
}

pub fn store_to_u64(stack: &[u8], offset: usize, size: VectorSize) -> u64 {
    let size = size as usize;
    if size == 0 {
        return 0;
    }

    let mut value = 0u64;
    for &b in &stack[offset..][..size.div_ceil(8)] {
        value <<= 8;
        value |= b as u64;
    }
    value
}

pub fn concat(
    stack: &mut [u8],
    dst: usize,
    lhs: usize,
    rhs: usize,
    lhs_size: VectorSize,
    rhs_size: VectorSize,
) {
    let (lhs_size, rhs_size) = (lhs_size as usize, rhs_size as usize);

    let lbytes = lhs_size.div_ceil(8);
    let rbytes = rhs_size.div_ceil(8);
    let dbytes = (lhs_size + rhs_size).div_ceil(8);

    // No aliasing.
    debug_assert!(lhs + lbytes <= dst || lhs >= dst + dbytes);
    debug_assert!(rhs + rbytes <= dst || rhs >= dst + dbytes);

    let roff = rhs_size % 8;

    // Fast path: left side is empty or right side is aligned.
    if lhs_size == 0 || roff == 0 {
        for i in 0..lbytes {
            stack[dst + i] = stack[lhs + i];
        }
        for i in 0..rbytes {
            stack[dst + lbytes + i] = stack[rhs + i];
        }
        return;
    }

    let mut residual_offset = 0;
    if dbytes == lbytes + rbytes {
        // Two residuals overflow into another byte.
        stack[dst] = stack[lhs] >> (8 - roff);
        residual_offset = 1;
    }

    for i in 0..lbytes - 1 {
        stack[dst + residual_offset + i] =
            (stack[lhs + i] << roff) | (stack[lhs + i + 1] >> (8 - roff));
    }
    stack[dst + lbytes - 1 + residual_offset] = (stack[lhs + lbytes - 1] << roff) | stack[rhs];
    for i in 1..rbytes {
        stack[dst + dbytes - rbytes + i] = stack[rhs + i];
    }
}

pub fn slice(stack: &mut [u8], dst: usize, src: usize, width: VectorSize, n: VectorSize) {
    let (width, n) = (width as usize, n as usize);

    if width == 0 {
        return;
    }

    // Fast path: input is output width.
    if width == n {
        for i in 0..n.div_ceil(8) {
            stack[dst + i] = stack[src + i];
        }
        return;
    }

    let lhs_start = src + n.div_ceil(8) - width.div_ceil(8);
    for i in 0..width.div_ceil(8) {
        stack[dst + i] = stack[lhs_start + i];
    }
    if width % 8 > 0 {
        stack[dst] &= (1u8 << (width % 8)).wrapping_sub(1);
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
    for i in 0..shift / 8 {
        stack[dst + i] = 0;
    }

    let soff = shift % 8;
    if soff == 0 {
        for i in shift / 8..width.div_ceil(8) {
            stack[dst + i] = stack[src + i - shift / 8];
        }
    } else {
        stack[dst + shift / 8] = stack[src] >> soff;
        for i in shift / 8 + 1..width.div_ceil(8) {
            stack[dst + i] = (stack[src + i - shift / 8 - 1] << (8 - soff))
                | (stack[src + i - shift / 8] >> soff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_u16() {
        let mut stack = [0u8; 8];
        for size in 0..=16 {
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
        for size in 0..=32 {
            let mask = (1u64 << size).wrapping_sub(1);
            for value in [0x0000, 0xFFFF, 0xABCD, 0x8181] {
                let value = value & mask;
                for width in 0..=size {
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
        for size in 0..=16 {
            for value in 0..=(1u64 << size).wrapping_sub(1) {
                for shift in 0..=size {
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
        for lhs_size in 0..=16 {
            let lhs_mask = (1u64 << lhs_size).wrapping_sub(1);
            for rhs_size in 0..=16 {
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
