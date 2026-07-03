use std::cell::Cell;

use crate::VectorSize;
use crate::arithmetic::{fv_pack_u64, fv_unpack_u64};
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

pub fn tv_s_logical_shift_left(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    assert!(size.get() <= 64);
    let src = load_partial_u64(&src, size);
    let out = src.unbounded_shl(shift);
    store_partial_u64(dst, out as u64, size);
}

pub fn tv_s_logical_shift_right(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    assert!(size.get() <= 64);
    let src = load_partial_u64(&src, size);
    let out = src.unbounded_shr(shift);
    store_partial_u64(dst, out as u64, size);
}

pub fn tv_s_arithmetic_shift_right(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    assert!(size.get() <= 64);
    let src = load_partial_u64(&src, size);
    let unused_bits = 64 - size.get();
    let out = src << unused_bits;
    let out = out as i64;
    let out = out.unbounded_shr(unused_bits + shift);
    store_partial_u64(dst, out as u64, size);
}

pub fn tv_l_logical_shift_left(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    tv_l_logical_shift_left_with(dst, src, shift, size, false);
}
pub fn tv_cell_logical_shift_left(dst: &[Cell<u64>], src: &[Cell<u64>], shift: u32, size: VectorSize) {
    tv_cell_logical_shift_left_with(dst, src, shift, size, false);
}
pub fn tv_l_logical_shift_left_with(
    dst: &mut [u64],
    src: &[u64],
    shift: u32,
    size: VectorSize,
    shiftin_value: bool,
) {
    let nwords = size.get().div_ceil(64) as usize;
    if shift == 0 {
        dst[..nwords].copy_from_slice(&src[..nwords]);
        return;
    }

    let shiftin_mask = u64::from(!shiftin_value).wrapping_sub(1);
    if shift >= size.get() {
        dst.fill(shiftin_mask);
        if size.get() % 64 != 0 {
            dst[nwords - 1] &= (1u64 << (size.get() % 64)) - 1;
        }
        return;
    }

    let shift = shift as usize;
    let soff = shift % 64;
    if soff == 0 {
        dst[..shift / 64].fill(shiftin_mask);
        dst[shift / 64..].copy_from_slice(&src[..nwords - shift / 64]);
    } else {
        let swords = shift.div_ceil(64);
        dst[..swords - 1].fill(shiftin_mask);
        dst[swords - 1] = (src[0] << soff) | (shiftin_mask >> (64 - soff));
        for i in 0..nwords - swords {
            dst[i + swords] = (src[i + 1] << soff) | (src[i] >> (64 - soff));
        }
    }
    if size.get() % 64 != 0 {
        dst[nwords - 1] &= (1u64 << (size.get() % 64)) - 1;
    }
}
pub fn tv_cell_logical_shift_left_with(
    dst: &[Cell<u64>],
    src: &[Cell<u64>],
    shift: u32,
    size: VectorSize,
    shiftin_value: bool,
) {
    let nwords = size.get().div_ceil(64) as usize;
    if shift == 0 {
        dst[..nwords]
            .iter()
            .zip(&src[..nwords])
            .for_each(|(d, s)| d.set(s.get()));
        return;
    }

    let shiftin_mask = u64::from(!shiftin_value).wrapping_sub(1);
    if shift >= size.get() {
        dst.iter().for_each(|v| v.set(shiftin_mask));
        if size.get() % 64 != 0 {
            dst[nwords - 1].update(|v| v & (1u64 << (size.get() % 64)) - 1);
        }
        return;
    }

    let shift = shift as usize;
    let soff = shift % 64;
    if soff == 0 {
        dst[..shift / 64].iter().for_each(|v| v.set(shiftin_mask));
        dst[shift / 64..]
            .iter()
            .zip(&src[..nwords - shift / 64])
            .for_each(|(d, s)| d.set(s.get()));
    } else {
        let swords = shift.div_ceil(64);
        dst[..swords - 1].iter().for_each(|v| v.set(shiftin_mask));
        dst[swords - 1].set((src[0].get() << soff) | (shiftin_mask >> (64 - soff)));
        for i in 0..nwords - swords {
            dst[i + swords].set((src[i + 1].get() << soff) | (src[i].get() >> (64 - soff)));
        }
    }
    if size.get() % 64 != 0 {
        dst[nwords - 1].update(|v| v & (1u64 << (size.get() % 64)) - 1);
    }
}
pub fn tv_l_logical_shift_right(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    tv_l_logical_shift_right_with(dst, src, shift, size, false);
}
pub fn tv_cell_logical_shift_right(
    dst: &[Cell<u64>],
    src: &[Cell<u64>],
    shift: u32,
    size: VectorSize,
) {
    tv_cell_logical_shift_right_with(dst, src, shift, size, false);
}
pub fn tv_l_logical_shift_right_with(
    dst: &mut [u64],
    src: &[u64],
    shift: u32,
    size: VectorSize,
    shiftin_value: bool,
) {
    let shiftin_mask = u64::from(!shiftin_value).wrapping_sub(1);
    let nwords = size.get().div_ceil(64) as usize;
    if shift == 0 {
        dst[..nwords].copy_from_slice(&src[..nwords]);
        return;
    }
    if shift >= size.get() {
        dst.fill(shiftin_mask);
        if size.get() % 64 != 0 {
            dst[nwords - 1] &= (1u64 << (size.get() % 64)) - 1;
        }
        return;
    }

    //       >> 54                      >> 68                      >> 129
    // [     [                          [                          [
    // I0    (I1 << 8) | (I0 >> 54)     (I2 << 60) | (I1 >> 4)     (I3 << 63) | (I2 >> 1)
    // I1    (I2 << 8) | (I1 >> 54)     (I3 << 60) | (I2 >> 4)     (SM << 63) | (I3 >> 1)
    // I2    (I3 << 8) | (I2 >> 54)     (SM << 60) | (I3 >> 4)     (SM << 63) | (SM >> 1)
    // I3    (SM << 8) | (I3 >> 54)      SM                         SM
    // ]     ]                          ]                          ]

    let shift = shift as usize;
    let swords = shift.div_ceil(64);
    let soff = shift % 64;
    if soff == 0 {
        dst[..nwords - swords].copy_from_slice(&src[swords..]);
        dst[nwords - swords..].fill(shiftin_mask);
    } else {
        for i in 0..nwords - swords {
            dst[i] = (src[i + swords] << (64 - soff)) | (src[i + swords - 1] >> soff);
        }
        dst[nwords - swords] = (shiftin_mask << (64 - soff)) | (src[nwords - 1] >> soff);
        dst[nwords - swords + 1..].fill(shiftin_mask);
    }
    if size.get() % 64 != 0 {
        let mask = shiftin_mask << (size.get() % 64);
        if shiftin_value {
            dst[nwords - shift / 64 - 1] |= mask >> soff;
            if soff != 0 && nwords >= shift / 64 + 2 {
                dst[nwords - shift / 64 - 2] |= mask << (64 - soff);
            }
        }
        dst[nwords - 1] &= (1u64 << (size.get() % 64)) - 1;
    }
}
pub fn tv_cell_logical_shift_right_with(
    dst: &[Cell<u64>],
    src: &[Cell<u64>],
    shift: u32,
    size: VectorSize,
    shiftin_value: bool,
) {
    let shiftin_mask = u64::from(!shiftin_value).wrapping_sub(1);
    let nwords = size.get().div_ceil(64) as usize;
    if shift == 0 {
        dst[..nwords]
            .iter()
            .zip(&src[..nwords])
            .for_each(|(d, s)| d.set(s.get()));
        return;
    }
    if shift >= size.get() {
        dst.iter().for_each(|v| v.set(shiftin_mask));
        if size.get() % 64 != 0 {
            dst[nwords - 1].update(|v| v & (1u64 << (size.get() % 64)) - 1);
        }
        return;
    }

    //       >> 54                      >> 68                      >> 129
    // [     [                          [                          [
    // I0    (I1 << 8) | (I0 >> 54)     (I2 << 60) | (I1 >> 4)     (I3 << 63) | (I2 >> 1)
    // I1    (I2 << 8) | (I1 >> 54)     (I3 << 60) | (I2 >> 4)     (SM << 63) | (I3 >> 1)
    // I2    (I3 << 8) | (I2 >> 54)     (SM << 60) | (I3 >> 4)     (SM << 63) | (SM >> 1)
    // I3    (SM << 8) | (I3 >> 54)      SM                         SM
    // ]     ]                          ]                          ]

    let shift = shift as usize;
    let swords = shift.div_ceil(64);
    let soff = shift % 64;
    if soff == 0 {
        dst[..nwords - swords]
            .iter()
            .zip(&src[swords..])
            .for_each(|(d, s)| d.set(s.get()));
        dst[nwords - swords..]
            .iter()
            .for_each(|v| v.set(shiftin_mask));
    } else {
        for i in 0..nwords - swords {
            dst[i]
                .set((src[i + swords].get() << (64 - soff)) | (src[i + swords - 1].get() >> soff));
        }
        dst[nwords - swords].set((shiftin_mask << (64 - soff)) | (src[nwords - 1].get() >> soff));
        dst[nwords - swords + 1..]
            .iter()
            .for_each(|v| v.set(shiftin_mask));
    }
    if size.get() % 64 != 0 {
        let mask = shiftin_mask << (size.get() % 64);
        if shiftin_value {
            dst[nwords - shift / 64 - 1].update(|v| v | (mask >> soff));
            if soff != 0 && nwords >= shift / 64 + 2 {
                dst[nwords - shift / 64 - 2].update(|v| v | (mask << (64 - soff)));
            }
        }
        dst[nwords - 1].update(|v| v & ((1u64 << (size.get() % 64)) - 1));
    }
}
pub fn tv_l_arithmetic_shift_right(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    let msb_idx = (size.get() - 1) as usize;
    let msb_val = (src[msb_idx / 64] >> (msb_idx % 64)) & 1 != 0;
    tv_l_logical_shift_right_with(dst, src, shift, size, msb_val);
}
pub fn tv_cell_arithmetic_shift_right(
    dst: &[Cell<u64>],
    src: &[Cell<u64>],
    shift: u32,
    size: VectorSize,
) {
    let msb_idx = (size.get() - 1) as usize;
    let msb_val = (src[msb_idx / 64].get() >> (msb_idx % 64)) & 1 != 0;
    tv_cell_logical_shift_right_with(dst, src, shift, size, msb_val);
}

pub fn fv_s_logical_shift_left(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    if shift == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let mask = (1u64 << size.get()) - 1;
    if shift >= size.get() {
        store_partial_u64(dst, mask, dsize);
        return;
    }

    let src = load_partial_u64(&src, dsize);
    let (spc, val) = fv_unpack_u64(src, size);
    let spc = ((spc << shift) & mask) | ((1u64 << shift) - 1);
    let val = (val << shift) & mask;
    let result = fv_pack_u64(spc, val, size);
    store_partial_u64(dst, result, dsize);
}
pub fn fv_s_logical_shift_right(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    if shift == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let mask = (1u64 << size.get()) - 1;
    if shift >= size.get() {
        store_partial_u64(dst, mask, dsize);
        return;
    }

    let src = load_partial_u64(&src, dsize);
    let (spc, val) = fv_unpack_u64(src, size);
    let spc = (spc >> shift) | (((1u64 << shift) - 1) << (size.get() - shift));
    let val = val >> shift;
    let result = fv_pack_u64(spc, val, size);
    store_partial_u64(dst, result, dsize);
}
pub fn fv_s_arithmetic_shift_right(dst: &mut [u8], src: &[u8], shift: u32, size: VectorSize) {
    if shift == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let dsize = VectorSize::new(size.get() * 2).unwrap();
    let mask = (1u64 << size.get()) - 1;

    let src = load_partial_u64(&src, dsize);
    let (spc, val) = fv_unpack_u64(src, size);
    let spc = (((spc as i64) << (64 - size.get())) >> (64 - size.get())).unbounded_shr(shift);
    let val = (((val as i64) << (64 - size.get())) >> (64 - size.get())).unbounded_shr(shift);
    let result = fv_pack_u64(spc as u64 & mask, val as u64 & mask, size);
    store_partial_u64(dst, result, dsize);
}

pub fn tv_shift_arith_right(val: u64, shift: u32, size: VectorSize) -> u64 {
    assert!(size.get() <= 64);
    let unused_bits = 64 - size.get();
    let val = ((val as i64) << unused_bits).unbounded_shr(unused_bits + shift);
    val as u64
}
pub fn fv_shift_arith_right(spc: u64, val: u64, shift: u32, size: VectorSize) -> (u64, u64) {
    assert!(size.get() <= 64);
    let unused_bits = 64 - size.get();
    let spc = ((spc as i64) << unused_bits).unbounded_shr(unused_bits + shift);
    let val = ((val as i64) << unused_bits).unbounded_shr(unused_bits + shift);
    (spc as u64, val as u64)
}

pub fn fv_l_logical_shift_left(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    let nwords = dst.len() / 2;
    tv_l_logical_shift_left_with(&mut dst[..nwords], &src[..nwords], shift, size, true);
    tv_l_logical_shift_left(&mut dst[nwords..], &src[nwords..], shift, size);
}
pub fn fv_l_logical_shift_right(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    let nwords = dst.len() / 2;
    tv_l_logical_shift_right_with(&mut dst[..nwords], &src[..nwords], shift, size, true);
    tv_l_logical_shift_right(&mut dst[nwords..], &src[nwords..], shift, size);
}
pub fn fv_l_arithmetic_shift_right(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    let nwords = dst.len() / 2;
    let msb_idx = (size.get() - 1) as usize;
    let msb_spc = (src[msb_idx / 64] >> (msb_idx % 64)) & 1 != 0;
    let msb_val = (src[nwords + msb_idx / 64] >> (msb_idx % 64)) & 1 != 0;
    tv_l_logical_shift_right_with(&mut dst[..nwords], &src[..nwords], shift, size, msb_spc);
    tv_l_logical_shift_right_with(&mut dst[nwords..], &src[nwords..], shift, size, msb_val);
}

pub fn fv_cell_logical_shift_left(dst: &[Cell<u64>], src: &[Cell<u64>], shift: u32, size: VectorSize) {
    let nwords = dst.len() / 2;
    tv_cell_logical_shift_left_with(&dst[..nwords], &src[..nwords], shift, size, true);
    tv_cell_logical_shift_left(&dst[nwords..], &src[nwords..], shift, size);
}
pub fn fv_cell_logical_shift_right(dst: &[Cell<u64>], src: &[Cell<u64>], shift: u32, size: VectorSize) {
    let nwords = dst.len() / 2;
    tv_cell_logical_shift_right_with(&dst[..nwords], &src[..nwords], shift, size, true);
    tv_cell_logical_shift_right(&dst[nwords..], &src[nwords..], shift, size);
}
pub fn fv_cell_arithmetic_shift_right(dst: &[Cell<u64>], src: &[Cell<u64>], shift: u32, size: VectorSize) {
    let nwords = dst.len() / 2;
    let msb_idx = (size.get() - 1) as usize;
    let msb_spc = (src[msb_idx / 64].get() >> (msb_idx % 64)) & 1 != 0;
    let msb_val = (src[nwords + msb_idx / 64].get() >> (msb_idx % 64)) & 1 != 0;
    tv_cell_logical_shift_right_with(&dst[..nwords], &src[..nwords], shift, size, msb_spc);
    tv_cell_logical_shift_right_with(&dst[nwords..], &src[nwords..], shift, size, msb_val);
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

                    tv_s_logical_shift_right(dst, src, shift, size);
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

                    tv_s_logical_shift_left(dst, src, shift, size);
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
