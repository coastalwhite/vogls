use crate::VectorSize;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

pub fn tv_logical_shift_left(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let src = load_partial_u64(&src, size);
    let out = src.unbounded_shl(shift);
    store_partial_u64(dst, out, size);
}

pub fn tv_logical_shift_right(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    let nbytes = size.get().div_ceil(8) as usize;
    if shift == 0 {
        dst[..nbytes].copy_from_slice(&src[..nbytes]);
        return;
    }
    if shift >= size.get() {
        for i in 0..nbytes {
            dst[i] = 0;
        }
        return;
    }

    let shift = shift as usize;
    let soff = shift % 8;
    if soff == 0 {
        for i in 0..nbytes - shift / 8 {
            dst[i] = src[i + shift / 8];
        }
        for i in nbytes - shift / 8..nbytes {
            dst[i] = 0;
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
            dst[i] = (src[i + sbytes - 1] >> soff) | (src[i + sbytes] << (8 - soff));
        }
        dst[nbytes - sbytes] = src[nbytes - 1] >> soff;
        for i in nbytes - sbytes + 1..nbytes {
            dst[i] = 0;
        }
    }
}

pub fn tv_arithmetic_shift_right(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let src = load_partial_u64(&src, size);
    let unused_bits = 64 - size.get();
    let out = src << unused_bits;
    let out = out as i64;
    let out = out.unbounded_shr(unused_bits + shift);
    store_partial_u64(dst, out as u64, size);
}

#[cfg(test)]
mod tests {
    use crate::get_disjoint_dst_src;
    use crate::store::store_partial_u64;
    use crate::load::load_partial_u64;

    use super::*;

    #[test]
    fn lsr_u16() {
        let mut stack = [0u8; 16];
        for size in 1..=16 {
            let size = VectorSize::new(size).unwrap();
            let nbytes = size.get().div_ceil(8) as usize;
            for value in 0..=(1u64 << size.get()).wrapping_sub(1) {
                for shift in 0..=size.get() {
                    store_partial_u64(&mut stack, value, size);

                    let (dst, src) = get_disjoint_dst_src(&mut stack, 8, nbytes, 0, nbytes);
                    
                    tv_logical_shift_right(dst, src, shift, size);
                    let result = load_partial_u64(dst, size);
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
}
