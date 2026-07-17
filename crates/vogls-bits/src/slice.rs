use crate::VectorSize;
use crate::arithmetic::fv_set_no_special;
use crate::util::last_word_mask;

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
            if num_valid_bits % 64 != 0 {
                dst[(num_valid_bits / 64) as usize] = (1u64 << (num_valid_bits % 64)) - 1;
            }
            dst[(num_valid_bits / 64) as usize + usize::from(num_valid_bits % 64 != 0)..].fill(0);
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

pub fn tv_s_truncate(dst: &mut [u8], src: &[u8], out_size: VectorSize) {
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
                    tv_s_truncate(dst, src, width);
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
