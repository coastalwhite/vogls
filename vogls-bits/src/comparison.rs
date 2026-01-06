use std::cmp::Ordering;

use crate::VectorSize;

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
