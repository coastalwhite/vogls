use std::cmp::Ordering;

use crate::VectorSize;
use crate::arithmetic::{FvLogicValue, fv_contains_special, fv_separate_packed_u64};
use crate::load::load_partial_u64;
use crate::select::tv_gtu64_select_bit;

pub fn tv_unsigned_leq(lhs: &[u8], rhs: &[u8], size: VectorSize) -> bool {
    for i in (0..size.get().div_ceil(8) as usize).rev() {
        let value = match lhs[i].cmp(&rhs[i]) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => continue,
        };
        return value;
    }
    true
}

pub fn tv_gtu64_unsigned_leq(lhs: &[u64], rhs: &[u64], size: VectorSize) -> bool {
    for i in (0..size.get().div_ceil(64) as usize).rev() {
        let value = match lhs[i].cmp(&rhs[i]) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => continue,
        };
        return value;
    }
    true
}

pub fn tv_gtu64_signed_leq(lhs: &[u64], rhs: &[u64], size: VectorSize) -> bool {
    if tv_gtu64_select_bit(lhs, size.get() - 1, size)
        && !tv_gtu64_select_bit(rhs, size.get() - 1, size)
    {
        return true;
    }

    tv_gtu64_unsigned_leq(lhs, rhs, size)
}

pub fn fv_l_unsigned_leq(lhs: &[u64], rhs: &[u64], size: VectorSize) -> FvLogicValue {
    if fv_contains_special(lhs, size) || fv_contains_special(rhs, size) {
        return FvLogicValue::X;
    }

    for i in (0..size.get().div_ceil(8) as usize).rev() {
        let value = match lhs[i].cmp(&rhs[i]) {
            Ordering::Less => FvLogicValue::L1,
            Ordering::Greater => FvLogicValue::L0,
            Ordering::Equal => continue,
        };
        return value;
    }
    FvLogicValue::L0
}
pub fn fv_s_unsigned_leq(lhs: &[u8], rhs: &[u8], size: VectorSize) -> FvLogicValue {
    let lhs = load_partial_u64(lhs, VectorSize::new(2 * size.get()).unwrap());
    let rhs = load_partial_u64(rhs, VectorSize::new(2 * size.get()).unwrap());

    let (lspc, lval) = fv_separate_packed_u64(lhs, size);
    let (rspc, rval) = fv_separate_packed_u64(rhs, size);

    let mask = (1u64 << size.get()) - 1;
    if lspc != mask || rspc != mask {
        return FvLogicValue::X;
    }

    FvLogicValue::from_bool(lval <= rval)
}
