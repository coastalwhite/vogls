//! Additive Gaussian noise on a numeric array.

use std::fmt;
use std::hash::{Hash, Hasher};

use rand::rngs::ChaCha12Rng;
use rand::{Rng, SeedableRng};

use crate::array::{Array, UIntElement};
use crate::buffer::Buffer;
use crate::compute::{ComputeContext, ComputeError, ComputeResult};
use crate::map::Map;
use crate::typing::{ArrayType, DataType};

#[derive(Clone, PartialEq)]
pub struct Noise {
    pub stddev: f64,
    pub mean: f64,
    pub seed: u64,
}

impl Eq for Noise {}

impl Hash for Noise {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.stddev.to_bits().hash(state);
        self.mean.to_bits().hash(state);
        self.seed.hash(state);
    }
}

#[derive(Default)]
pub struct NoiseScratch {
    call_index: u64,
}

/// `2^-53`, the scale that turns 53 random bits into a uniform.
const TWO_POW_NEG_53: f64 = 1.0 / 9007199254740992.0;

/// Expand `seed` into ChaCha's 32-byte seed.
///
/// Written out here rather than deferred to `rand_core`'s `seed_from_u64`, so
/// that the whole seed-to-keystream mapping belongs to this crate.
fn seed_bytes(seed: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    bytes
}

/// Box-Muller, hand-rolled so that the RNG-bytes-to-sample mapping is pinned by
/// this crate's tests rather than by a distribution crate's lookup tables.
#[derive(Clone, Copy)]
struct Gaussian {
    mean: f64,
    stddev: f64,
}

impl Gaussian {
    /// Box-Muller yields two normals per pair of uniforms.
    #[inline(always)]
    fn sample(&self, rng: &mut ChaCha12Rng) -> f64 {
        let x1 = rng.next_u64();
        let x2 = rng.next_u64();
        // `+ 1` puts `u1` in `(0, 1]`, so `ln(u1)` is never `-inf`.
        let u1 = ((x1 >> 11) + 1) as f64 * TWO_POW_NEG_53;
        let u2 = (x2 >> 11) as f64 * TWO_POW_NEG_53;
        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        self.mean + self.stddev * z
    }

    /// The sample rounded half-up, in `i128`.
    #[inline(always)]
    fn sample_rounded(&self, rng: &mut ChaCha12Rng) -> i128 {
        // A float-to-int cast saturates, which is the behaviour an enormous
        // `stddev` should get anyway.
        (self.sample(rng) + 0.5).floor() as i128
    }
}

fn noise_floats(data: &[f64], g: &Gaussian, rng: &mut ChaCha12Rng) -> Buffer<f64> {
    data.iter().map(|&v| v + g.sample(rng)).collect()
}

fn noise_ints(data: &[i64], g: &Gaussian, rng: &mut ChaCha12Rng) -> Buffer<i64> {
    data.iter()
        .map(|&v| {
            let sum = v as i128 + g.sample_rounded(rng);
            sum.clamp(i64::MIN as i128, i64::MAX as i128) as i64
        })
        .collect()
}

fn noise_uints<T: UIntElement>(
    data: &[T],
    max: u64,
    g: &Gaussian,
    rng: &mut ChaCha12Rng,
) -> Buffer<T> {
    data.iter()
        .map(|&v| {
            let sum = v.to_u64() as i128 + g.sample_rounded(rng);
            T::from_u64(sum.clamp(0, max as i128) as u64)
        })
        .collect()
}

impl Map for Noise {
    type Inputs<Input>
        = [Input; 1]
    where
        Input: Send + Sync;

    type Scratches = NoiseScratch;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { stddev, mean, seed } = self;
        write!(
            f,
            "Noise {{ stddev: {stddev}, mean: {mean}, seed: {seed} }}"
        )
    }

    fn output_type(&self, inputs: Self::Inputs<ArrayType>) -> ComputeResult<ArrayType> {
        let [input] = inputs;
        if matches!(input.data, DataType::Bits(_)) {
            return Err(ComputeError::InvalidTypes);
        }
        Ok(ArrayType {
            data: input.data,
            length: input.length,
        })
    }

    fn compute(
        &self,
        inputs: Self::Inputs<Array>,
        _ctx: &ComputeContext,
        scratch: &mut Self::Scratches,
    ) -> ComputeResult<Array> {
        let [input] = inputs;

        let mut rng = ChaCha12Rng::from_seed(seed_bytes(self.seed));
        rng.set_stream(scratch.call_index);
        scratch.call_index += 1;

        let g = Gaussian {
            mean: self.mean,
            stddev: self.stddev,
        };

        Ok(match &input {
            Array::Floats(d) => Array::Floats(noise_floats(d.as_slice(), &g, &mut rng)),
            Array::Ints(d) => Array::Ints(noise_ints(d.as_slice(), &g, &mut rng)),
            Array::UInts(d) => {
                let max = d.ty().max_value();
                Array::UInts(with_match_uint_type!(d.ty(), |$T| {
                    <$T as UIntElement>::into_uint_array(
                        noise_uints::<$T>(d.as_slice::<$T>(), max, &g, &mut rng)
                    )
                }))
            }
            // Rejected in `output_type`.
            Array::Bits(..) => return Err(ComputeError::InvalidTypes),
        })
    }
}
