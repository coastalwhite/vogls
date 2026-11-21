use std::ops::{BitOr, BitOrAssign, RangeInclusive};

use crate::util::mask_size_0to63;
use crate::{Bits, Mode, VectorSize};
use proptest::arbitrary::Arbitrary;
use proptest::collection::vec;
use proptest::prelude::{BoxedStrategy, Just, Strategy, any, any_with, prop_oneof};
use proptest::sample::{SizeRange, select};

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

/// A set of [`Mode`]s, as bit flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modes(u8);

impl Modes {
    pub const NONE: Self = Self(0);
    pub const TWO_VALUE: Self = Self(1 << 0);
    pub const FOUR_VALUE: Self = Self(1 << 1);
    pub const ALL: Self = Self(Self::TWO_VALUE.0 | Self::FOUR_VALUE.0);

    pub const fn contains(self, mode: Mode) -> bool {
        let flag = match mode {
            Mode::TwoValue => Self::TWO_VALUE,
            Mode::FourValue => Self::FOUR_VALUE,
        };
        self.0 & flag.0 != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl From<Mode> for Modes {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::TwoValue => Self::TWO_VALUE,
            Mode::FourValue => Self::FOUR_VALUE,
        }
    }
}

impl BitOr for Modes {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Modes {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// How often four-value [`Bits`] contain x/z bits. Has no effect on two-value [`Bits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Specials {
    /// Four-value mode, but every bit is a known 0/1.
    Never,
    /// About 1 in 8 bits is x/z.
    Sparse,
    /// About 1 in 2 bits is x/z.
    Dense,
    /// Picks one of the above per generated value.
    #[default]
    Mixed,
}

/// Parameters for generating arbitrary [`Bits`] with `any_with::<Bits>(params)`.
///
/// ```ignore
/// any_with::<Bits>(BitsParams::default().size_range(1..=64).modes(Modes::FOUR_VALUE))
/// ```
#[derive(Debug, Clone)]
pub struct BitsParams {
    /// Bit lengths to generate. Must be non-empty and start at 1 or more.
    pub size_range: RangeInclusive<u32>,
    /// Modes to pick from, uniformly. Must be non-empty.
    pub modes: Modes,
    pub specials: Specials,
    /// Over-sample sizes at representation boundaries (inline/boxed, word edges, the range
    /// ends) that fall inside `size_range`.
    pub bias_boundaries: bool,
}

impl Default for BitsParams {
    fn default() -> Self {
        Self {
            size_range: 1..=300,
            modes: Modes::ALL,
            specials: Specials::Mixed,
            bias_boundaries: true,
        }
    }
}

impl BitsParams {
    pub fn size_range(mut self, size_range: RangeInclusive<u32>) -> Self {
        self.size_range = size_range;
        self
    }

    pub fn modes(mut self, modes: Modes) -> Self {
        self.modes = modes;
        self
    }

    pub fn specials(mut self, specials: Specials) -> Self {
        self.specials = specials;
        self
    }

    pub fn bias_boundaries(mut self, bias_boundaries: bool) -> Self {
        self.bias_boundaries = bias_boundaries;
        self
    }
}

/// Sizes where the representation of [`Bits`] (or of containers built on it) changes.
const SIZE_BOUNDARIES: &[u32] = &[
    1, 2, 3, 4, 5, 8, 9, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
];

/// Arbitrary bit lengths following `params.size_range` and `params.bias_boundaries`.
pub fn any_bits_size(params: &BitsParams) -> BoxedStrategy<u32> {
    let range = params.size_range.clone();
    assert!(*range.start() >= 1, "size_range must start at 1 or more");
    assert!(!range.is_empty(), "size_range must be non-empty");

    let uniform = range.clone().boxed();
    if !params.bias_boundaries {
        return uniform;
    }

    let mut boundaries: Vec<u32> = SIZE_BOUNDARIES
        .iter()
        .copied()
        .chain([*range.start(), *range.end()])
        .filter(|s| range.contains(s))
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();

    prop_oneof![3 => uniform, 1 => select(boundaries)].boxed()
}

/// One spc word, where a 1 bit is a known 0/1 and a 0 bit is x/z.
fn any_spc_word(specials: Specials) -> BoxedStrategy<u64> {
    match specials {
        Specials::Never => Just(u64::MAX).boxed(),
        Specials::Sparse => any::<[u64; 3]>().prop_map(|[a, b, c]| !(a & b & c)).boxed(),
        Specials::Dense => any::<u64>().boxed(),
        Specials::Mixed => unreachable!("resolved per value in `any_bits`"),
    }
}

fn bits_from_words(size: VectorSize, mode: Mode, spc: Vec<u64>, val: Vec<u64>) -> Bits {
    match mode {
        Mode::TwoValue if size <= mode.max_inline_size() => Bits::from_u64(size, val[0]),
        Mode::TwoValue => Bits::from_boxed_slice(mode, size, val.into()),
        Mode::FourValue if size <= mode.max_inline_size() => {
            Bits::from_four_value_u64(size, spc[0] as u32, val[0] as u32)
        }
        Mode::FourValue => Bits::from_boxed_slice(mode, size, spc.into_iter().chain(val).collect()),
    }
}

/// Arbitrary [`Bits`] following `params`. Same as `any_with::<Bits>(params)`.
pub fn any_bits(params: BitsParams) -> impl Strategy<Value = Bits> {
    assert!(!params.modes.is_empty(), "modes must be non-empty");

    let specials = match params.specials {
        Specials::Mixed => select(vec![Specials::Never, Specials::Sparse, Specials::Dense]).boxed(),
        s => Just(s).boxed(),
    };

    let modes: &[Mode] = match (
        params.modes.contains(Mode::TwoValue),
        params.modes.contains(Mode::FourValue),
    ) {
        (true, true) => &[Mode::TwoValue, Mode::FourValue],
        (true, false) => &[Mode::TwoValue],
        (false, true) => &[Mode::FourValue],
        (false, false) => &[Mode::FourValue],
    };

    (any_bits_size(&params), select(modes), specials)
        .prop_flat_map(|(size, mode, specials)| {
            let nwords = size.div_ceil(64) as usize;
            let spc = match mode {
                Mode::TwoValue => Just(Vec::new()).boxed(),
                Mode::FourValue => vec(any_spc_word(specials), nwords).boxed(),
            };
            (Just(size), Just(mode), spc, vec(any::<u64>(), nwords))
        })
        .prop_map(|(size, mode, spc, val)| {
            bits_from_words(VectorSize::new(size).unwrap(), mode, spc, val)
        })
}

impl Arbitrary for Bits {
    type Parameters = BitsParams;
    type Strategy = BoxedStrategy<Bits>;

    fn arbitrary_with(params: Self::Parameters) -> Self::Strategy {
        any_bits(params).boxed()
    }
}
