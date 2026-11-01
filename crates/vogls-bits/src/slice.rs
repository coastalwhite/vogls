use crate::VectorSize;
use crate::arithmetic::fv_set_no_special;
use crate::util::{last_word_mask, mask_size_0to63};

pub fn tv_s_slice(src: u64, offset: u32, dst_size: VectorSize, src_size: VectorSize) -> (u64, u64) {
    assert!(dst_size <= src_size);
    let dst_mask = 1u64.unbounded_shl(dst_size.get()).wrapping_sub(1);
    if offset == 0 {
        return (dst_mask, src & dst_mask);
    }
    if offset >= src_size.get() {
        return (0, 0);
    }

    (
        dst_mask >> offset.saturating_sub(src_size.get() - dst_size.get()),
        (src >> offset) & dst_mask,
    )
}
pub fn tv_ls_slice(
    src: &[u64],
    offset: u32,
    dst_size: VectorSize,
    src_size: VectorSize,
) -> (u64, u64) {
    assert!(dst_size <= src_size);
    let dst_mask = 1u64.unbounded_shl(dst_size.get()).wrapping_sub(1);
    if offset == 0 {
        return (dst_mask, src[0] & dst_mask);
    }
    if offset >= src_size.get() {
        return (0, 0);
    }

    let wi = (offset / 64) as usize;
    let bi = offset % 64;
    let spc = dst_mask >> offset.saturating_sub(src_size.get() - dst_size.get());
    let mut val = src[wi] >> bi;
    if wi < src.len() - 1 && bi > 0 {
        val |= src[wi + 1] << (64 - bi);
    }
    (spc, val & dst_mask)
}
pub fn tv_part_ll_slice(
    dst: &mut [u64],
    src: &[u64],
    offset: u32,
    dst_size: VectorSize,
    src_size: VectorSize,
    shiftin_value: bool,
) {
    assert!(dst_size <= src_size);
    let dst_words = dst.len();
    if offset == 0 {
        dst.copy_from_slice(&src[..dst_words]);
        dst[dst.len() - 1] &= last_word_mask(dst_size);
        return;
    }
    let shiftin_mask = u64::from(!shiftin_value).wrapping_sub(1);
    if offset >= src_size.get() {
        dst.fill(shiftin_mask);
        dst[dst.len() - 1] &= last_word_mask(dst_size);
        return;
    }

    let swords = offset.div_ceil(64) as usize;
    let soff = offset % 64;
    let num_copy_words = (src.len() - swords).min(dst.len());
    if soff == 0 {
        dst[..num_copy_words].copy_from_slice(&src[swords..][..num_copy_words]);
        dst[num_copy_words..].fill(shiftin_mask);
    } else {
        for i in 0..num_copy_words {
            dst[i] = (src[i + swords] << (64 - soff)) | (src[i + swords - 1] >> soff);
        }
        if num_copy_words < dst.len() {
            dst[num_copy_words] = (shiftin_mask << (64 - soff)) | (src[src.len() - 1] >> soff);
            dst[num_copy_words + 1..].fill(shiftin_mask);
        }
    }
    dst[dst.len() - 1] &= last_word_mask(dst_size);
}
pub fn tv_ll_slice(
    dst: &mut [u64],
    src: &[u64],
    offset: u32,
    dst_size: VectorSize,
    src_size: VectorSize,
    fill_with_x: bool,
) {
    assert!(dst_size <= src_size);
    let mut dst_words = dst.len();
    if fill_with_x {
        dst_words /= 2;
    }
    if offset == 0 {
        if fill_with_x {
            fv_set_no_special(dst, dst_size);
            dst[dst_words..].copy_from_slice(&src[..dst_words]);
        } else {
            dst.copy_from_slice(&src[..dst_words]);
        }
        dst[dst.len() - 1] &= last_word_mask(dst_size);
        return;
    }
    if offset >= src_size.get() {
        dst.fill(0);
        return;
    }

    // Fill valid bits.
    if fill_with_x {
        let num_x_bits = offset.saturating_sub(src_size.get() - dst_size.get());
        if num_x_bits == 0 {
            fv_set_no_special(dst, dst_size);
        } else {
            let num_valid_bits = dst_size.get() - num_x_bits;
            dst[..(num_valid_bits / 64) as usize].fill(u64::MAX);
            if !num_valid_bits.is_multiple_of(64) {
                dst[(num_valid_bits / 64) as usize] = (1u64 << (num_valid_bits % 64)) - 1;
            }
            dst[(num_valid_bits / 64) as usize + usize::from(!num_valid_bits.is_multiple_of(64))..]
                .fill(0);
        }
        tv_part_ll_slice(
            &mut dst[dst_words..],
            src,
            offset,
            dst_size,
            src_size,
            false,
        );
    } else {
        tv_part_ll_slice(dst, src, offset, dst_size, src_size, false);
    }
}

pub fn fv_s_slice(
    spc: u64,
    val: u64,
    offset: u32,
    dst_size: VectorSize,
    src_size: VectorSize,
    fill_with_x: bool,
) -> (u64, u64) {
    assert!(dst_size <= src_size);
    let dst_mask = 1u64.unbounded_shl(dst_size.get()).wrapping_sub(1);
    if offset == 0 {
        return (spc & dst_mask, val & dst_mask);
    }
    let num_fill_bits = (dst_size.get() + offset)
        .saturating_sub(src_size.get())
        .min(dst_size.get());
    let spc_fill = if fill_with_x {
        0u64
    } else {
        1u64.unbounded_shl(
            (dst_size.get() + offset)
                .saturating_sub(src_size.get())
                .min(dst_size.get()),
        )
        .wrapping_sub(1)
    };
    if offset >= src_size.get() {
        return (spc_fill, 0);
    }
    (
        ((spc >> offset) & dst_mask) | spc_fill.unbounded_shl(dst_size.get() - num_fill_bits),
        (val >> offset) & dst_mask,
    )
}
pub fn fv_ls_slice(
    src: &[u64],
    offset: u32,
    dst_size: VectorSize,
    src_size: VectorSize,
    fill_with_x: bool,
) -> (u64, u64) {
    assert!(dst_size <= src_size);
    let dst_mask = 1u64.unbounded_shl(dst_size.get()).wrapping_sub(1);
    let src_nwords = src.len() / 2;
    if offset == 0 {
        return (src[0] & dst_mask, src[src_nwords] & dst_mask);
    }
    let num_fill_bits = (dst_size.get() + offset)
        .saturating_sub(src_size.get())
        .min(dst_size.get());
    let spc_fill = if fill_with_x {
        0u64
    } else {
        1u64.unbounded_shl(
            (dst_size.get() + offset)
                .saturating_sub(src_size.get())
                .min(dst_size.get()),
        )
        .wrapping_sub(1)
    };
    if offset >= src_size.get() {
        return (spc_fill, 0);
    }

    let wi = (offset / 64) as usize;
    let bi = offset % 64;

    let mut spc = src[wi] >> bi;
    let mut val = src[src_nwords + wi] >> bi;
    if wi < src_nwords - 1 && bi > 0 {
        spc |= src[wi + 1] << (64 - bi);
        val |= src[src_nwords + wi + 1] << (64 - bi);
    }
    (
        (spc & dst_mask) | spc_fill.unbounded_shl(dst_size.get() - num_fill_bits),
        val & dst_mask,
    )
}
pub fn fv_ll_slice(
    dst: &mut [u64],
    src: &[u64],
    offset: u32,
    dst_size: VectorSize,
    src_size: VectorSize,
    fill_with_x: bool,
) {
    assert!(dst_size <= src_size);
    let src_words = src.len() / 2;
    let dst_words = dst.len() / 2;
    tv_part_ll_slice(
        &mut dst[..dst_words],
        &src[..src_words],
        offset,
        dst_size,
        src_size,
        !fill_with_x,
    );
    tv_part_ll_slice(
        &mut dst[dst_words..],
        &src[src_words..],
        offset,
        dst_size,
        src_size,
        false,
    );
}

/// Copy bits `start..end` from `src` to `dst` starting at bit offset `dst_start`.
pub fn copy_shifted_bits(dst: &mut [u64], shift: u64, src: &[u64], start: u64, end: u64) {
    if shift.is_multiple_of(64) && start.is_multiple_of(64) {
        let start_dst_word = (shift / 64) as usize;
        let start_src_word = (start / 64) as usize;
        let nwords = ((end - start) / 64) as usize;
        dst[start_dst_word..start_dst_word + nwords]
            .copy_from_slice(&src[start_src_word..start_src_word + nwords]);
        if !end.is_multiple_of(64) {
            dst[start_dst_word + nwords] =
                src[start_src_word + nwords] & mask_size_0to63((end % 64) as u32);
        }

        return;
    }

    let mut pos = start;
    while pos < end {
        let nbits = (end - pos).min(64);
        let mask = u64::MAX >> (64 - nbits);

        let woff = (pos / 64) as usize;
        let boff = pos % 64;
        let mut w = src[woff] >> boff;
        if boff + nbits > 64 {
            w |= src[woff + 1] << (64 - boff);
        }
        w &= mask;

        let dst_pos = shift + (pos - start);
        let woff = (dst_pos / 64) as usize;
        let boff = dst_pos % 64;
        dst[woff] = (dst[woff] & !(mask << boff)) | (w << boff);
        if boff + nbits > 64 {
            let hi_mask = mask >> (64 - boff);
            dst[woff + 1] = (dst[woff + 1] & !hi_mask) | (w >> (64 - boff));
        }

        pos += nbits;
    }
}

/// Set all bits of `words` in the range `start..end`.
pub fn set_bit_range(words: &mut [u64], start: u32, end: u32) {
    for (i, word) in words.iter_mut().enumerate() {
        let base = i as u32 * 64;
        let lo = start.saturating_sub(base).min(64);
        let hi = end.saturating_sub(base).min(64);
        if lo < hi {
            *word |= (u64::MAX >> (64 - (hi - lo))) << lo;
        }
    }
}

/// Slice `dst_size` bits starting at bit `offset` out of `src` into `dst`.
///
/// Only the source bits inside the inclusive range `lower..=upper` and inside the source are
/// read. All other bits of the slice are X when `fill_with_x`, and 0 otherwise. A four-value
/// source or `fill_with_x` gives a SPC and VAL plane in `dst`, each `dst_size` bits.
#[allow(clippy::too_many_arguments)]
pub fn ll_slice_bounded(
    dst: &mut [u64],
    src: &[u64],
    offset: u32,
    dst_size: VectorSize,
    src_size: VectorSize,
    src_is_fv: bool,
    fill_with_x: bool,
    lower: u32,
    upper: u32,
) {
    let src_plane_words = src_size.get().div_ceil(64) as usize;
    let dst_plane_words = dst_size.get().div_ceil(64) as usize;

    // The bits of the source which can be read, as the range `start..end`. They land at bit
    // `shift` of the slice.
    let start = u32::max(offset, lower);
    let end = offset
        .saturating_add(dst_size.get())
        .min(upper.saturating_add(1))
        .min(src_size.get())
        .max(start);
    let shift = start - offset;

    dst.fill(0);
    let (dst_spc, dst_val) = if src_is_fv | fill_with_x {
        dst.split_at_mut(dst_plane_words)
    } else {
        (&mut [][..], dst)
    };

    if src_is_fv {
        let (src_spc, src_val) = src.split_at(src_plane_words);
        if !fill_with_x {
            // A SPC bit without a VAL bit is a 0.
            set_bit_range(dst_spc, 0, dst_size.get());
        }
        copy_shifted_bits(dst_spc, shift.into(), src_spc, start.into(), end.into());
        copy_shifted_bits(dst_val, shift.into(), src_val, start.into(), end.into());
    } else {
        if fill_with_x {
            set_bit_range(dst_spc, shift, shift + (end - start));
        }
        copy_shifted_bits(dst_val, shift.into(), src, start.into(), end.into());
    }
}
