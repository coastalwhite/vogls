use crate::VectorSize;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

pub fn tv_logical_shift_left(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
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
        for i in 0..shift / 8 {
            dst[i] = 0;
        }
        for i in 0..nbytes - shift / 8 {
            dst[i + shift / 8] = src[i];
        }
    } else {
        // X = [ A ]
        // LSL (X, 7)
        //
        // [ A << 7 ]
        //
        // X = [ A B C D ]
        // LSL (X, 7)
        // [
        //     A << 7
        //    (B << 7) | (A >> 1)
        //    (C << 7) | (B >> 1)
        //    (D << 7) | (C >> 1)
        // ]
        //
        // LSL (X, 15)
        // [
        //     0
        //     A << 7
        //    (B << 7) | (A >> 1)
        //    (C << 7) | (B >> 1)
        // ]
        let sbytes = shift.div_ceil(8);
        for i in 0..sbytes - 1 {
            dst[i] = 0;
        }
        dst[sbytes - 1] = src[0] << soff;
        for i in 0..nbytes - sbytes {
            dst[i + sbytes] = (src[i + 1] << soff) | (src[i] >> (8 - soff));
        }
    }
    if size.get() % 8 != 0 {
        dst[nbytes - 1] &= (1u8 << (size.get() % 8)) - 1;
    }
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
        // X = [ A ]
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

pub fn tv_gtu64_logical_shift_left(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    let nwords = size.get().div_ceil(64) as usize;
    if shift == 0 {
        dst[..nwords].copy_from_slice(&src[..nwords]);
        return;
    }
    if shift >= size.get() {
        for i in 0..nwords {
            dst[i] = 0;
        }
        return;
    }
    let shift = shift as usize;
    let soff = shift % 64;
    if soff == 0 {
        for i in 0..shift / 64 {
            dst[i] = 0;
        }
        for i in 0..nwords - shift / 64 {
            dst[i + shift / 64] = src[i];
        }
    } else {
        // X = [ A ]
        // LSL (X, 7)
        //
        // [ A << 7 ]
        //
        // X = [ A B C D ]
        // LSL (X, 7)
        // [
        //     A << 7
        //    (B << 7) | (A >> 1)
        //    (C << 7) | (B >> 1)
        //    (D << 7) | (C >> 1)
        // ]
        //
        // LSL (X, 15)
        // [
        //     0
        //     A << 7
        //    (B << 7) | (A >> 1)
        //    (C << 7) | (B >> 1)
        // ]
        let swords = shift.div_ceil(64);
        for i in 0..swords - 1 {
            dst[i] = 0;
        }
        dst[swords - 1] = src[0] << soff;
        for i in 0..nwords - swords {
            dst[i + swords] = (src[i + 1] << soff) | (src[i] >> (64 - soff));
        }
    }
    if size.get() % 64 != 0 {
        dst[nwords - 1] &= (1u64 << (size.get() % 64)) - 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::get_disjoint_dst_src;
    use crate::load::load_partial_u64;
    use crate::store::store_partial_u64;

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

    #[test]
    fn lsl_u16() {
        let mut stack = [0u8; 16];
        for size in 1..=16 {
            let size = VectorSize::new(size).unwrap();
            let nbytes = size.get().div_ceil(8) as usize;
            for value in 0..=(1u64 << size.get()).wrapping_sub(1) {
                for shift in 0..=size.get() {
                    store_partial_u64(&mut stack, value, size);

                    let (dst, src) = get_disjoint_dst_src(&mut stack, 8, nbytes, 0, nbytes);

                    tv_logical_shift_left(dst, src, shift, size);
                    let result = load_partial_u64(dst, size);
                    let expected =
                        (value << shift) & 1u64.unbounded_shl(size.get()).wrapping_sub(1);
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
