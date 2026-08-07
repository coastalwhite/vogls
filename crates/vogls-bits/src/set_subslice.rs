use std::cell::Cell;

use crate::VectorSize;
use crate::util::mask_size_1to64;

pub fn set_subslice(
    mut dst: &mut [u8],
    src: &[u8],
    dst_size: VectorSize,
    offset: u32,
    src_size: VectorSize,
) -> bool {
    assert!(offset + src_size.get() <= dst_size.get());

    let mut offset = offset;
    dst = &mut dst[(offset / 8) as usize..];
    offset %= 8;

    // @Performance: Please do something better.
    let mut updated = false;
    for i in 0..src_size.get() {
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

/// Updates dst[offset +: src_size] to src and returns whether changes were made.
pub fn tv_s_set(
    dst: u64,
    src: u64,
    dst_size: VectorSize,
    offset: u32,
    src_size: VectorSize,
) -> u64 {
    assert!(dst_size >= src_size);

    if dst_size == src_size && offset == 0 {
        return src;
    }
    if offset >= dst_size.get() {
        return dst;
    }

    let update_size = u32::min(src_size.get(), dst_size.get() - offset);
    let mask = (1u64 << update_size) - 1;
    let src = src & mask;
    let mask = mask << offset;
    (src << offset) | (dst & !mask)
}

/// Updates dst[offset +: src_size] to src and returns whether changes were made.
pub fn tv_l_set(
    dst: &mut [u64],
    src: &[u64],
    dst_size: VectorSize,
    offset: u32,
    src_size: VectorSize,
) -> bool {
    assert!(
        dst_size >= src_size
            && dst.len() == dst_size.get().div_ceil(64) as usize
            && src.len() == src_size.get().div_ceil(64) as usize
    );

    // Fast path: offset >= dst_size.
    let Some(max_src_size) = dst_size.get().checked_sub(offset).and_then(VectorSize::new) else {
        return false;
    };

    // Fast path: dst = src.
    if dst_size == src_size && offset == 0 {
        let updated = dst == src;
        dst.copy_from_slice(src);
        return updated;
    }

    // Truncate `src << offset` to fit in `dst`.
    let src_size = src_size.min(max_src_size);
    let src = &src[..src_size.get().div_ceil(64) as usize];

    let sh_words = (offset / 64) as usize;
    let sh_offset = offset % 64;
    let mut i = 0;
    let mut bits_consumed = 0u32;
    let mut updated = false;

    // Least-Significant Word
    if sh_offset > 0 {
        let old = dst[sh_words];
        let mask = 1u64.unbounded_shl(src_size.get()).wrapping_sub(1);
        let value = src[0] & 1u64.unbounded_shl(src_size.get()).wrapping_sub(1);
        let new = (value << sh_offset) | (old & !(mask << sh_offset));
        dst[sh_words] = new;
        updated |= old != new;
        i += 1;
        bits_consumed += 64 - sh_offset;
        while bits_consumed + 64 <= src_size.get() {
            let value = (src[(bits_consumed / 64) as usize] >> (64 - sh_offset))
                | src[(bits_consumed / 64) as usize + 1] << sh_offset;
            updated |= value != std::mem::replace(&mut dst[i + sh_words], value);
            bits_consumed += 64;
            i += 1;
        }
    } else {
        while bits_consumed + 64 <= src_size.get() {
            updated |= src[(bits_consumed / 64) as usize]
                != std::mem::replace(&mut dst[i + sh_words], src[(bits_consumed / 64) as usize]);
            bits_consumed += 64;
            i += 1;
        }
    }

    // Most-Significant Word
    if bits_consumed < src_size.get() {
        let old = dst[i + sh_words];
        let num_rem_bits = src_size.get() - bits_consumed;
        let mask = (1u64 << num_rem_bits) - 1;
        let mut new_src = src[(bits_consumed / 64) as usize] >> ((64 - sh_offset) % 64);
        bits_consumed += sh_offset;
        if bits_consumed < src_size.get() {
            new_src |= src[(bits_consumed / 64) as usize] << sh_offset;
        }
        new_src &= mask;
        let new = new_src | (old & !mask);
        dst[i + sh_words] = new;
        updated |= old != new;
    }

    updated
}

/// Updates dst[offset +: src_size] to src and returns whether changes were made.
pub fn tv_cell_set(
    dst: &[Cell<u64>],
    src: &[Cell<u64>],
    dst_size: VectorSize,
    offset: u32,
    src_size: VectorSize,
) -> bool {
    assert!(
        dst_size >= src_size
            && dst.len() == dst_size.get().div_ceil(64) as usize
            && src.len() == src_size.get().div_ceil(64) as usize
    );

    // Fast path: offset >= dst_size.
    let Some(max_src_size) = dst_size.get().checked_sub(offset).and_then(VectorSize::new) else {
        return false;
    };

    // Fast path: dst = src.
    if dst_size == src_size && offset == 0 {
        let updated = dst == src;
        dst.iter().zip(src).for_each(|(d, s)| d.set(s.get()));
        return updated;
    }

    // Truncate `src << offset` to fit in `dst`.
    let src_size = src_size.min(max_src_size);
    let src = &src[..src_size.get().div_ceil(64) as usize];

    let sh_words = (offset / 64) as usize;
    let sh_offset = offset % 64;
    let mut i = 0;
    let mut bits_consumed = 0u32;
    let mut updated = false;

    // Least-Significant Word
    if sh_offset > 0 {
        let old = dst[sh_words].get();
        let mask = 1u64.unbounded_shl(src_size.get()).wrapping_sub(1);
        let value = src[0].get() & 1u64.unbounded_shl(src_size.get()).wrapping_sub(1);
        let new = (value << sh_offset) | (old & !(mask << sh_offset));
        dst[sh_words].set(new);
        updated |= old != new;
        i += 1;
        bits_consumed += 64 - sh_offset;
        while bits_consumed + 64 <= src_size.get() {
            let value = (src[(bits_consumed / 64) as usize].get() >> (64 - sh_offset))
                | src[(bits_consumed / 64) as usize + 1].get() << sh_offset;
            updated |= value != dst[i + sh_words].replace(value);
            bits_consumed += 64;
            i += 1;
        }
    } else {
        while bits_consumed + 64 <= src_size.get() {
            updated |= src[(bits_consumed / 64) as usize].get()
                != dst[i + sh_words].replace(src[(bits_consumed / 64) as usize].get());
            bits_consumed += 64;
            i += 1;
        }
    }

    // Most-Significant Word
    if bits_consumed < src_size.get() {
        let old = dst[i + sh_words].get();
        let num_rem_bits = src_size.get() - bits_consumed;
        let mask = (1u64 << num_rem_bits) - 1;
        let mut new_src = src[(bits_consumed / 64) as usize].get() >> ((64 - sh_offset) % 64);
        bits_consumed += sh_offset;
        if bits_consumed < src_size.get() {
            new_src |= src[(bits_consumed / 64) as usize].get() << sh_offset;
        }
        new_src &= mask;
        let new = new_src | (old & !mask);
        dst[i + sh_words].set(new);
        updated |= old != new;
    }

    updated
}

/// Update dst[offset +: src_size] to src[0 +: src_size] and keep a mask of which bits were
/// updated.
///
/// Bit `i` in `update_mask` is OR-ed with `1` i.f.f. the bit `offset+i` in `dst` was changed by
/// bit `i` in the `src`.
///
/// The `base_offset` and `base_size` serve as a limiting range and any bits in `dst` outside
/// `base_offset..base_offset + base_size` are never changed. If `offset..offset + src_size`
/// includes bits outside of the base range, this writes are no-ops and the `update_mask` is
/// uneffected for those bits.
///
/// The `src` argument may contains any bit-pattern and are not necessarily masked according to
/// `src_size`. The `dst` should fit `base_offset..base_offset + base_size`.
pub fn set_with_mask(
    update_mask: &mut [u64],
    dst: &mut [u64],
    src: &[u64],
    offset: u64,
    src_size: VectorSize,
    base_offset: u64,
    base_size: VectorSize,
) {
    assert_eq!(update_mask.len(), src.len());
    assert_eq!(src.len(), src_size.get().div_ceil(64) as usize);

    let src_size = src_size.get() as u64;
    let base_end = base_offset + base_size.get() as u64;
    let boff = (offset % 64) as u32;

    // Iterate over source words and write them into the destination.
    for (wi, w) in src.iter().enumerate() {
        let word_start_bit = offset + (wi as u64) * 64;

        // Calculate an effective mask for this word, taking into account the base offset and size.
        let lo = base_offset.saturating_sub(word_start_bit).min(64);
        let hi = base_end
            .saturating_sub(word_start_bit)
            .min(src_size.saturating_sub((wi as u64) * 64))
            .min(64);
        if lo >= hi {
            continue;
        }
        let eff_mask = mask_size_1to64((hi - lo) as u32) << lo;

        let word_dst_idx = (word_start_bit / 64) as usize;

        // Gather the old value.
        let mut old = dst[word_dst_idx] >> boff;
        if boff > 0 && word_dst_idx + 1 < dst.len() {
            old |= dst[word_dst_idx + 1] << (64 - boff);
        }
        update_mask[wi] |= (w ^ old) & eff_mask;

        // Update the destination.
        dst[word_dst_idx] = (dst[word_dst_idx] & !(eff_mask << boff)) | ((w & eff_mask) << boff);
        if boff > 0 {
            let clear_hi = eff_mask >> (64 - boff);
            if clear_hi != 0 {
                dst[word_dst_idx + 1] =
                    (dst[word_dst_idx + 1] & !clear_hi) | ((w & eff_mask) >> (64 - boff));
            }
        }
    }
}
