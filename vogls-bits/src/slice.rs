use crate::VectorSize;

pub fn tv_slice(dst: &mut [u8], src: &[u8], out_size: VectorSize) {
    let width = out_size.get() as usize;

    for i in 0..width.div_ceil(8) {
        dst[i] = src[i];
    }
    let woff = width % 8;
    if woff != 0 {
        dst[width / 8] &= 1u8.unbounded_shl(woff as u32).wrapping_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_disjoint_dst_src;
    use crate::load::load_partial_u64;
    use crate::store::store_partial_u64;

    #[test]
    fn test_slice_u16() {
        let mut stack = [0u8; 16];
        for size in 1..=32 {
            let size = VectorSize::new(size).unwrap();
            let mask = (1u64 << size.get()).wrapping_sub(1);
            for value in [0x0000, 0xFFFF, 0xABCD, 0x8181] {
                let value = value & mask;
                for width in 1..=size.get() {
                    let width = VectorSize::new(width).unwrap();
                    store_partial_u64(&mut stack, value, size);

                    let (dst, src) = get_disjoint_dst_src(
                        &mut stack,
                        8,
                        width.get().div_ceil(8) as usize,
                        0,
                        size.get().div_ceil(8) as usize,
                    );
                    tv_slice(dst, src, width);
                    let result = load_partial_u64(dst, width);
                    let expected = value & (1u64 << width.get()).wrapping_sub(1);
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
}
