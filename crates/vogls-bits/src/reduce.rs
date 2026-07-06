use crate::VectorSize;
use crate::arithmetic::{FvLogicValue, fv_unpack_u64};
use crate::load::load_partial_u64;

pub fn tv_reduce_or(src: &[u64]) -> bool {
    src.iter().any(|v| *v != 0)
}
pub fn tv_reduce_and(src: &[u64], size: VectorSize) -> bool {
    src.iter().map(|v| v.count_ones()).sum::<u32>() == size.get()
}
pub fn tv_reduce_xor(src: &[u64]) -> bool {
    src.iter().map(|v| v.count_ones()).sum::<u32>() % 2 == 1
}

pub fn fv_reduce_bitwise_op(
    src: &[u64],
    size: VectorSize,
    unit: FvLogicValue,
    op: impl Fn(u64, u64, VectorSize) -> FvLogicValue,
) -> FvLogicValue {
    let mut gspc = unit as u64 & 1;
    let mut gvalue = unit as u64 >> 1;
    let nwords = src.len() / 2;
    let mut i = 0;
    let mut shift = 1;
    let mut size = size.get();
    while let Some(s) = VectorSize::new(size) {
        let r = op(src[i], src[nwords + i], s);
        gspc |= (r as u64 & 1) << shift;
        gvalue |= (r as u64 >> 1) << shift;
        if shift == 63 {
            let subresult = op(gspc, gvalue, VectorSize::new(64).unwrap());
            gspc = subresult as u64 & 1;
            gvalue = subresult as u64 >> 1;
            shift = 1;
        } else {
            shift += 1;
        }
        size = size.saturating_sub(64);
        i += 1;
    }
    op(gspc, gvalue, VectorSize::new(shift).unwrap())
}

pub fn fv_s_reduce_bitwise_op(
    src: &[u8],
    size: VectorSize,
    op: impl Fn(u64, u64, VectorSize) -> FvLogicValue,
) -> FvLogicValue {
    assert!(size.get() <= 32);
    let x = load_partial_u64(src, VectorSize::new(size.get() * 2).unwrap());
    let (spc, value) = fv_unpack_u64(x, size);
    op(spc, value, size)
}

pub fn fv_l_reduce_or(src: &[u64], size: VectorSize) -> FvLogicValue {
    fv_reduce_bitwise_op(src, size, FvLogicValue::L0, fv_reduce_or_elem)
}
pub fn fv_l_reduce_and(src: &[u64], size: VectorSize) -> FvLogicValue {
    fv_reduce_bitwise_op(src, size, FvLogicValue::L1, fv_reduce_and_elem)
}
pub fn fv_l_reduce_xor(src: &[u64], size: VectorSize) -> FvLogicValue {
    fv_reduce_bitwise_op(src, size, FvLogicValue::L0, fv_reduce_xor_elem)
}
pub fn fv_s_reduce_or(src: &[u8], size: VectorSize) -> FvLogicValue {
    fv_s_reduce_bitwise_op(src, size, fv_reduce_or_elem)
}
pub fn fv_s_reduce_and(src: &[u8], size: VectorSize) -> FvLogicValue {
    fv_s_reduce_bitwise_op(src, size, fv_reduce_and_elem)
}
pub fn fv_s_reduce_xor(src: &[u8], size: VectorSize) -> FvLogicValue {
    fv_s_reduce_bitwise_op(src, size, fv_reduce_xor_elem)
}

#[inline(always)]
pub fn fv_reduce_and_elem(spc: u64, value: u64, size: VectorSize) -> FvLogicValue {
    // & | x  z  1  0
    // --+-----------
    // x | x  x  x  0
    // z | x  x  x  0
    // 1 | x  x  1  0
    // 0 | 0  0  0  0
    //
    // z1z0 = fv.redand(sn vn ... s0 v0)
    //
    // z1 = (&si) | (s & !v != 0)
    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    let z1 = (spc == mask) | (spc & !value != 0);
    let z0 = (spc == mask) & (value == mask);
    FvLogicValue::from_repr((u8::from(z0) << 1) | u8::from(z1))
}
#[inline(always)]
pub fn fv_reduce_or_elem(spc: u64, value: u64, size: VectorSize) -> FvLogicValue {
    // | | x  z  1  0
    // --+-----------
    // x | x  x  1  x
    // z | x  x  1  x
    // 1 | 1  1  1  1
    // 0 | x  x  1  0

    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    let z0 = (spc & value) != 0;
    let z1 = (spc == mask) | z0;
    FvLogicValue::from_repr((u8::from(z0) << 1) | u8::from(z1))
}
#[inline(always)]
pub fn fv_reduce_xor_elem(spc: u64, value: u64, size: VectorSize) -> FvLogicValue {
    // ^ | x  z  1  0
    // --+-----------
    // x | x  x  x  x
    // z | x  x  x  x
    // 1 | x  x  0  1
    // 0 | x  x  1  0

    let mask = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
    let z1 = spc == mask;
    let z0 = z1 & (value.count_ones() % 2 == 1);
    FvLogicValue::from_repr((u8::from(z0) << 1) | u8::from(z1))
}
