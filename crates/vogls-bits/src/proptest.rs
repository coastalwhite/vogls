use std::ops::RangeInclusive;

use crate::VectorSize;
use crate::util::mask_size_0to63;
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
        if size.get().is_multiple_of(8) {
            *v.last_mut().unwrap() &= mask_size_0to63(size.get() % 64);
        }
        v
    })
}
