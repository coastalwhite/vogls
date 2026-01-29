use crate::VectorSize;
use crate::arithmetic::{fv_pack_u64, fv_unpack_u64};
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

pub fn tv_l_extend_with(
    dst: &mut [u64],
    src: &[u64],
    dst_size: VectorSize,
    src_size: VectorSize,
    fill: bool,
) {
    let fill_mask = u64::from(!fill).wrapping_sub(1);
    dst[..src.len()].copy_from_slice(src);
    if src_size.get() % 64 != 0 {
        dst[src.len() - 1] |= fill_mask << src_size.get() % 64;
    };

    dst[src.len()..].fill(fill_mask);
    if dst_size.get() % 64 != 0 {
        dst[dst.len() - 1] &= (1u64 << (dst_size.get() % 64)) - 1;
    }
}
pub fn tv_l_zero_extend(dst: &mut [u64], src: &[u64], dst_size: VectorSize, src_size: VectorSize) {
    tv_l_extend_with(dst, src, dst_size, src_size, false)
}
pub fn tv_l_sign_extend(dst: &mut [u64], src: &[u64], dst_size: VectorSize, src_size: VectorSize) {
    let sign = (src[(src_size.get() / 64) as usize] >> (src_size.get() % 64)) & 1 != 0;
    tv_l_extend_with(dst, src, dst_size, src_size, sign)
}
pub fn fv_l_zero_extend(dst: &mut [u64], src: &[u64], dst_size: VectorSize, src_size: VectorSize) {
    let dwords = dst.len() / 2;
    let swords = src.len() / 2;
    tv_l_extend_with(&mut dst[..dwords], &src[..swords], dst_size, src_size, true);
    tv_l_extend_with(
        &mut dst[dwords..],
        &src[swords..],
        dst_size,
        src_size,
        false,
    );
}
pub fn fv_l_sign_extend(dst: &mut [u64], src: &[u64], dst_size: VectorSize, src_size: VectorSize) {
    let dwords = dst.len() / 2;
    let swords = src.len() / 2;
    tv_l_sign_extend(&mut dst[..dwords], &src[..swords], dst_size, src_size);
    tv_l_sign_extend(&mut dst[dwords..], &src[swords..], dst_size, src_size);
}

pub fn tv_s_zero_extend(dst: &mut [u8], src: &[u8], _dst_size: VectorSize, _src_size: VectorSize) {
    dst.fill(0);
    dst.copy_from_slice(src);
}
pub fn tv_s_sign_extend(dst: &mut [u8], src: &[u8], dst_size: VectorSize, src_size: VectorSize) {
    let sign = (src[(src_size.get() / 8) as usize] >> (src_size.get() % 8)) & 1 != 0;
    let mask = u8::from(!sign).wrapping_sub(1);
    dst.fill(mask);
    if dst_size.get() % 8 != 0 {
        *dst.last_mut().unwrap() &= (1u8 << (dst_size.get() % 8)) - 1;
    }
    dst.copy_from_slice(src);
    if src_size.get() % 8 != 0 {
        dst[src.len() - 1] |= 0xFFu8 << (src_size.get() % 8);
    }
}
pub fn fv_s_zero_extend(dst: &mut [u8], src: &[u8], dst_size: VectorSize, src_size: VectorSize) {
    assert!(dst_size >= src_size);

    if dst_size == src_size {
        dst.copy_from_slice(src);
        return;
    }

    let src = load_partial_u64(src, VectorSize::new(2 * src_size.get()).unwrap());
    let (spc, val) = fv_unpack_u64(src, src_size);
    let spc = spc | (((1u64 << (dst_size.get() - src_size.get())) - 1) << src_size.get());
    let result = fv_pack_u64(spc, val, dst_size);
    store_partial_u64(dst, result, VectorSize::new(2 * dst_size.get()).unwrap());
}
pub fn fv_s_sign_extend(dst: &mut [u8], src: &[u8], dst_size: VectorSize, src_size: VectorSize) {
    let src = load_partial_u64(src, VectorSize::new(2 * src_size.get()).unwrap());
    let (spc, val) = fv_unpack_u64(src, src_size);
    let s = 64 - src_size.get();
    let spc = (spc << s) as i64 >> s;
    let val = (val << s) as i64 >> s;
    let mask = 1u64.unbounded_shl(dst_size.get()).wrapping_sub(1);
    let result = fv_pack_u64(spc as u64 & mask, val as u64 & mask, dst_size);
    store_partial_u64(dst, result, VectorSize::new(2 * dst_size.get()).unwrap());
}
