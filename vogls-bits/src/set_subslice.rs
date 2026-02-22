use crate::VectorSize;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

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
    offset = offset % 8;

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
    dst: &mut [u8],
    src: &[u8],
    dst_size: VectorSize,
    offset: u32,
    src_size: VectorSize,
) -> bool {
    assert!(dst_size >= src_size);

    if dst_size == src_size && offset == 0 {
        let updated = dst == src;
        dst.copy_from_slice(src);
        return updated;
    }

    let dst_v = load_partial_u64(dst, dst_size);
    let src_v = load_partial_u64(src, src_size);

    let mask = (1u64 << src_size.get()) - 1;
    let mask = mask << offset;
    let new = (src_v << offset) | (dst_v & !mask);
    store_partial_u64(dst, new, dst_size);
    dst_v != new
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
    let mut updated = false;
    let mut src_rem_size = src_size.get();

    // Least-Significant Word
    if sh_offset > 0 {
        let old = dst[i + sh_words];
        let mask = 1u64.unbounded_shl(src_rem_size).wrapping_sub(1);
        let value = src[i] & 1u64.unbounded_shl(src_rem_size).wrapping_sub(1);
        let new = (value << sh_offset) | (old & !(mask << sh_offset));
        dst[i + sh_words] = new;
        updated |= old != new;
        i += 1;

        src_rem_size = src_rem_size.saturating_sub(sh_offset);
        while src_rem_size >= 64 {
            let value = (src[i - 1] >> sh_offset) | src[i] << (64 - sh_offset);
            updated |= value != std::mem::replace(&mut dst[i + sh_words], value);
            i += 1;
            src_rem_size -= 64;
        }
    } else {
        while src_rem_size >= 64 {
            updated |= src[i] != std::mem::replace(&mut dst[i + sh_words], src[i]);
            i += 1;
            src_rem_size -= 64;
        }
    }

    // Most-Significant Word
    if let Some(src_rem_size) = VectorSize::new(src_rem_size) {
        let old = dst[i + sh_words];
        let mask = 1u64.unbounded_shl(src_rem_size.get()).wrapping_sub(1);
        let new = ((src[i] >> sh_offset) & mask) | (old & !mask);
        dst[i + sh_words] = new;
        updated |= old != new;
    }

    updated
}
