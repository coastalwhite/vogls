use crate::VectorSize;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

pub fn tv_bitwise_and(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    for i in 0..size.get().div_ceil(8) as usize {
        dst[i] = lhs[i] & rhs[i];
    }
}

pub fn tv_bitwise_or(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    for i in 0..size.get().div_ceil(8) as usize {
        dst[i] = lhs[i] | rhs[i];
    }
}

pub fn tv_bitwise_xor(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    for i in 0..size.get().div_ceil(8) as usize {
        dst[i] = lhs[i] ^ rhs[i];
    }
}

pub fn tv_addition(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_add(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_subtraction(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_sub(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_multiplication(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_mul(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_division(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    // @TODO: Deal with r = 0
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_div(r);
    store_partial_u64(dst, out, size);
}

pub fn tv_modulus(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    if size.get() > 64 {
        todo!()
    }
    // @TODO: Deal with r = 0
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.wrapping_rem(r);
    store_partial_u64(dst, out, size);
}
