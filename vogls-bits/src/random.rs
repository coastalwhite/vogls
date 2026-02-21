use std::cell::RefCell;

use crate::{Bits, Mode, VectorSize};
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};

thread_local! {
    static THREAD_LOCAL_RNG_STATE: RefCell<SmallRng> = RefCell::new(SmallRng::seed_from_u64(0));
}

fn next(size: VectorSize, mode: Mode, rng: &mut SmallRng) -> Bits {
    match mode {
        Mode::TwoValue => {
            if size <= Mode::TwoValue.max_inline_size() {
                Bits::from_u64(size, rng.next_u64())
            } else {
                let mut vs = std::iter::repeat_n(0, size.get().div_ceil(64) as usize)
                    .collect::<Box<[u64]>>();
                rng.fill(&mut vs);
                Bits::from_boxed_slice(Mode::TwoValue, size, vs)
            }
        }
        Mode::FourValue => {
            if size <= Mode::FourValue.max_inline_size() {
                let v = rng.next_u64();
                Bits::from_four_value_u64(size, (v >> 32) as u32, v as u32)
            } else {
                let mut vs = std::iter::repeat_n(0, size.get().div_ceil(64) as usize * 2)
                    .collect::<Box<[u64]>>();
                rng.fill(&mut vs);
                Bits::from_boxed_slice(Mode::FourValue, size, vs)
            }
        }
    }
}

pub fn rand_bits_from_seed(size: VectorSize, mode: Mode, seed: u64) -> Bits {
    next(size, mode, &mut SmallRng::seed_from_u64(seed))
}

pub fn rand_bits(size: VectorSize, mode: Mode) -> Bits {
    THREAD_LOCAL_RNG_STATE.with_borrow_mut(|rng| next(size, mode, rng))
}
