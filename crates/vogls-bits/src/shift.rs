use crate::VectorSize;
use crate::util::last_word_mask;

pub fn tv_l_logical_shift_left(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    tv_l_logical_shift_left_with(dst, src, shift, size, false);
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
        dst[nwords - 1] &= last_word_mask(size);
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
    dst[nwords - 1] &= last_word_mask(size);
}
pub fn tv_l_logical_shift_right(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    tv_l_logical_shift_right_with(dst, src, shift, size, false);
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
        dst[nwords - 1] &= last_word_mask(size);
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
    if !size.get().is_multiple_of(64) {
        let mask = shiftin_mask << (size.get() % 64);
        if shiftin_value {
            dst[nwords - shift / 64 - 1] |= mask >> soff;
            if soff != 0 && nwords >= shift / 64 + 2 {
                dst[nwords - shift / 64 - 2] |= mask << (64 - soff);
            }
        }
        dst[nwords - 1] &= last_word_mask(size);
    }
}
pub fn tv_l_arithmetic_shift_right(dst: &mut [u64], src: &[u64], shift: u32, size: VectorSize) {
    let msb_idx = (size.get() - 1) as usize;
    let msb_val = (src[msb_idx / 64] >> (msb_idx % 64)) & 1 != 0;
    tv_l_logical_shift_right_with(dst, src, shift, size, msb_val);
}

pub fn tv_shift_arith_right(val: u64, shift: u32, size: VectorSize) -> u64 {
    assert!(size.get() <= 64);
    let unused_bits = 64 - size.get();
    let val = ((val as i64) << unused_bits).unbounded_shr(unused_bits + shift);
    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    val as u64 & mask
}
pub fn fv_shift_arith_right(spc: u64, val: u64, shift: u32, size: VectorSize) -> (u64, u64) {
    assert!(size.get() <= 64);
    let unused_bits = 64 - size.get();
    let spc = ((spc as i64) << unused_bits).unbounded_shr(unused_bits + shift);
    let val = ((val as i64) << unused_bits).unbounded_shr(unused_bits + shift);
    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    (spc as u64 & mask, val as u64 & mask)
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
