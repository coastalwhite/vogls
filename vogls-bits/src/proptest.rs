use std::ops::RangeInclusive;

use crate::VectorSize;
use proptest::prelude::{Strategy, any_with};
use proptest::sample::SizeRange;

proptest::prop_compose! {
    pub fn any_reasonable_size(range: RangeInclusive<u32>) (s in range) -> VectorSize {
        VectorSize::new(s).unwrap()
    }
}

pub fn any_bits_of_size(size: VectorSize) -> impl Strategy<Value = Vec<u64>> {
    let nwords = size.get().div_ceil(64) as usize;
    any_with::<Vec<u64>>(SizeRange::new(nwords..=nwords).lift()).prop_map(move |mut v| {
        if size.get() % 8 != 0 {
            *v.last_mut().unwrap() &= (1u64 << size.get() % 64).wrapping_sub(1);
        }
        v
    })
}
