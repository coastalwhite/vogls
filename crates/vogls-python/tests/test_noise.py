import math
import re

import numpy as np
import pytest

import vogls as vg

# --------------------------------------------------------------------------- #
# A pure-Python ChaCha12 + Box-Muller reference
#
# The point of `noise()` is an SNR curve that can be regenerated, so the whole
# seed-to-sample mapping is part of the contract rather than an implementation
# detail. This reference is what gives the pinned values below their authority:
# they agree with an implementation written from the algorithm, not just with
# whatever the kernel happened to emit.
# --------------------------------------------------------------------------- #

M32 = 0xFFFFFFFF
CONSTANTS = (0x61707865, 0x3320646E, 0x79622D32, 0x6B206574)  # "expand 32-byte k"


def _rotl(v: int, n: int) -> int:
    return ((v << n) | (v >> (32 - n))) & M32


def _quarter_round(s: list[int], a: int, b: int, c: int, d: int) -> None:
    s[a] = (s[a] + s[b]) & M32
    s[d] = _rotl(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotl(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b]) & M32
    s[d] = _rotl(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & M32
    s[b] = _rotl(s[b] ^ s[c], 7)


def _keystream_words(seed: int, stream: int):
    """ChaCha12 keystream as 32-bit words, under vogls' own seed expansion."""
    # Bytes 0..8 are the seed little-endian, the rest zero.
    key = seed.to_bytes(8, "little") + bytes(24)
    base = list(CONSTANTS)
    base += [int.from_bytes(key[i : i + 4], "little") for i in range(0, 32, 4)]
    base += [0, 0, stream & M32, (stream >> 32) & M32]

    counter = 0
    while True:
        state = list(base)
        state[12] = counter & M32
        state[13] = (counter >> 32) & M32
        work = list(state)
        for _ in range(6):  # ChaCha12 is 6 double rounds
            _quarter_round(work, 0, 4, 8, 12)
            _quarter_round(work, 1, 5, 9, 13)
            _quarter_round(work, 2, 6, 10, 14)
            _quarter_round(work, 3, 7, 11, 15)
            _quarter_round(work, 0, 5, 10, 15)
            _quarter_round(work, 1, 6, 11, 12)
            _quarter_round(work, 2, 7, 8, 13)
            _quarter_round(work, 3, 4, 9, 14)
        yield from [(w + s) & M32 for w, s in zip(work, state)]
        counter += 1


def _u64s(seed: int, stream: int):
    """The keystream as `u64`s: consecutive word pairs, low word first."""
    words = _keystream_words(seed, stream)
    while True:
        lo = next(words)
        hi = next(words)
        yield lo | (hi << 32)


def _samples(seed: int, stream: int, stddev: float, mean: float = 0.0, n: int = 16):
    """`n` Gaussian samples, discarding Box-Muller's second normal each time."""
    words = _u64s(seed, stream)
    out = []
    for _ in range(n):
        x1 = next(words)
        x2 = next(words)
        u1 = ((x1 >> 11) + 1) * 2.0**-53  # (0, 1]
        u2 = (x2 >> 11) * 2.0**-53  # [0, 1)
        z = math.sqrt(-2.0 * math.log(u1)) * math.cos(math.tau * u2)
        out.append(mean + stddev * z)
    return out


def _reference_uint(base, seed, stddev, mean=0.0, bits=8, stream=0) -> list[int]:
    """The integer kernel: round the *sample*, add and clamp as an integer."""
    maxv = (1 << bits) - 1
    noise = _samples(seed, stream, stddev, mean, n=len(base))
    return [
        min(max(int(x) + math.floor(n + 0.5), 0), maxv) for x, n in zip(base, noise)
    ]


# --------------------------------------------------------------------------- #
# 1. Pinned bytes
#
# Everything else in this file rests on these. They fail loudly if the RNG is
# swapped, Box-Muller is reordered, or the u64 -> f64 conversion changes.
# --------------------------------------------------------------------------- #

# `seed = 0xC0FFEE`, `stddev = 1`, `mean = 0`, added to an array of zeros -- so
# these *are* the raw Gaussian samples.
PINNED_FLOATS = [
    0.9509212623164226,
    0.14761179600715443,
    -0.12410517671152343,
    0.5075870177462608,
    -1.086672786806295,
    0.11701741401400406,
    -0.16422312002928663,
    1.1176248306554004,
]

# `seed = 7`, `stddev = 8`, `mean = 0`, on a `u8` array of 128s.
PINNED_U8 = [132, 109, 126, 129, 121, 138, 125, 135, 129, 116, 139, 151]


def test_pinned_float_samples() -> None:
    out = np.asarray(vg.Array(np.zeros(len(PINNED_FLOATS))).noise(1.0, seed=0xC0FFEE))

    # An RNG or Box-Muller change moves these by order 1, so a tight relative
    # tolerance still catches it while leaving room for libm's last bit.
    np.testing.assert_allclose(out, PINNED_FLOATS, rtol=1e-12, atol=0.0)


def test_pinned_float_samples_match_the_reference() -> None:
    assert _samples(0xC0FFEE, 0, 1.0, n=len(PINNED_FLOATS)) == pytest.approx(
        PINNED_FLOATS, rel=1e-12
    )


def test_pinned_uint_samples() -> None:
    src = np.full(len(PINNED_U8), 128, dtype=np.uint8)
    out = np.asarray(vg.Array(src).noise(8.0, seed=7))

    assert out.dtype == np.dtype(np.uint8)
    assert out.tolist() == PINNED_U8
    assert out.tolist() == _reference_uint(src, seed=7, stddev=8.0)


def test_a_longer_run_still_tracks_the_reference() -> None:
    # The pins above are short enough to read; this one crosses the 64-word
    # keystream buffer boundary, where the block counter has to advance.
    src = np.full(64, 1000, dtype=np.uint32)
    out = np.asarray(vg.Array(src).noise(30.0, mean=-5.0, seed=1234))

    assert out.tolist() == _reference_uint(
        src, seed=1234, stddev=30.0, mean=-5.0, bits=32
    )


# --------------------------------------------------------------------------- #
# Dtypes
# --------------------------------------------------------------------------- #


@pytest.mark.parametrize("dtype", [np.uint8, np.uint16, np.uint32, np.uint64])
def test_output_dtype_is_the_input_dtype(dtype) -> None:
    # Nothing widens: a u8 trace set stays u8 rather than costing 8x memory.
    src = np.full(16, 100, dtype=dtype)
    out = np.asarray(vg.Array(src).noise(5.0, seed=1))

    assert out.dtype == np.dtype(dtype)
    assert out.tolist() == _reference_uint(
        src, seed=1, stddev=5.0, bits=8 * np.dtype(dtype).itemsize
    )


def test_signed_input_stays_signed_and_may_go_negative() -> None:
    src = np.zeros(32, dtype=np.int64)
    out = np.asarray(vg.Array(src).noise(50.0, seed=2))

    assert out.dtype == np.dtype(np.int64)
    assert out.min() < 0 < out.max()


def test_float_input_stays_float() -> None:
    out = np.asarray(vg.Array(np.zeros(16)).noise(1.0, seed=3))
    assert out.dtype == np.dtype(np.float64)


# --------------------------------------------------------------------------- #
# 2. Per-run independence
# --------------------------------------------------------------------------- #


def _ragged(widths: list[int], value: int = 100):
    """A ragged `RunVector`: run `i` holds `widths[i]` copies of `value`."""
    src = vg.Array(np.arange(len(widths), dtype=np.uint64)).lazy().repeat(1)

    def one_run(run: vg.Array) -> vg.Array:
        i = int(np.asarray(run)[0])
        return vg.Array(np.full(widths[i], value, dtype=np.uint64))

    return src.map(one_run)


def test_runs_get_different_noise() -> None:
    rv = vg.Array(np.full(4, 1000, dtype=np.uint64)).lazy().repeat(8)
    runs = rv.noise(40.0, seed=17).compute().as_list()

    assert len(runs) == 4
    # Noise shared between traces is a *signal*, and the statistics in this
    # crate would find it.
    assert len({tuple(r) for r in runs}) == 4


def test_a_later_run_widening_does_not_move_an_earlier_one() -> None:
    # The raggedness property the per-run substreams exist to provide: without
    # them, "I removed a trace and all my SNR numbers moved".
    wide = _ragged([4, 4, 6]).noise(4.0, seed=5).compute().as_list()
    narrow = _ragged([4, 4, 2]).noise(4.0, seed=5).compute().as_list()

    assert [len(r) for r in wide] == [4, 4, 6]
    assert [len(r) for r in narrow] == [4, 4, 2]
    assert wide[0] == narrow[0]
    assert wide[1] == narrow[1]


def test_run_noise_is_a_function_of_seed_run_and_element_only() -> None:
    # Run `i` uses substream `i` from element 0, independent of its own width.
    runs = _ragged([2, 5]).noise(4.0, seed=5).compute().as_list()

    for i, run in enumerate(runs):
        assert run == _reference_uint(
            [100] * len(run), seed=5, stddev=4.0, bits=64, stream=i
        )


# --------------------------------------------------------------------------- #
# 3. Loop-order safety
#
# `map.rs` invokes `compute` once per run in a sequential loop, and no run index
# reaches the kernel -- it is reconstructed by counting calls. Parallelizing that
# loop has to break a test rather than silently move numbers.
# --------------------------------------------------------------------------- #

# `seed = 99`, `stddev = 4`, `mean = 0`, on three runs of four zeros each.
PINNED_RUNS = [[3, -2, -9, -2], [-1, -7, 2, 3], [-5, 0, -5, -7]]


def test_pinned_per_run_values() -> None:
    rv = vg.Array(np.zeros(3, dtype=np.int64)).lazy().repeat(4)
    assert rv.noise(4.0, seed=99).compute().as_list() == PINNED_RUNS


def test_pinned_per_run_values_match_the_reference_substreams() -> None:
    for i, run in enumerate(PINNED_RUNS):
        assert run == [math.floor(n + 0.5) for n in _samples(99, i, 4.0, n=len(run))]


def test_the_array_path_is_the_first_substream() -> None:
    # One `compute` call, so `call_index == 0`: the same bytes run 0 gets.
    flat = np.asarray(vg.Array(np.zeros(4, dtype=np.int64)).noise(4.0, seed=99))
    assert flat.tolist() == PINNED_RUNS[0]


# --------------------------------------------------------------------------- #
# 4. Saturation boundaries
# --------------------------------------------------------------------------- #


def test_uint_output_saturates_at_both_rails() -> None:
    # Wrapping would turn a near-zero sample into a near-maximum one and inject
    # enormous data-dependent variance at the low end of the trace.
    src = np.asarray([0] * 64 + [255] * 64, dtype=np.uint8)
    out = np.asarray(vg.Array(src).noise(60.0, seed=4))

    assert out.min() >= 0
    assert out.max() <= 255
    # The clipping is actually exercised, so this is not a vacuous range check.
    assert (out[:64] == 0).any()
    assert (out[64:] == 255).any()


def test_an_enormous_stddev_still_stays_in_range() -> None:
    src = np.full(32, 7, dtype=np.uint16)
    out = np.asarray(vg.Array(src).noise(1e30, seed=6))

    assert out.min() >= 0
    assert out.max() <= 65535
    assert set(out.tolist()) == {0, 65535}


def test_signed_output_saturates_at_the_i64_rails() -> None:
    src = np.asarray([np.iinfo(np.int64).min, np.iinfo(np.int64).max], dtype=np.int64)
    out = np.asarray(vg.Array(src).noise(1e18, seed=8))

    assert out[0] >= np.iinfo(np.int64).min
    assert out[1] <= np.iinfo(np.int64).max


# --------------------------------------------------------------------------- #
# 5. Dedup semantics
# --------------------------------------------------------------------------- #


def test_the_same_seed_is_bit_identical() -> None:
    src = np.zeros(32)
    a = np.asarray(vg.Array(src).noise(1.0, seed=42))
    b = np.asarray(vg.Array(src).noise(1.0, seed=42))

    # Same input, same `Map` -- one node. Any correlation between `a` and `b`
    # reads 1.0, which looks like a result.
    np.testing.assert_array_equal(a, b)


def test_a_drawn_seed_differs_between_calls() -> None:
    src = np.zeros(32)
    c = np.asarray(vg.Array(src).noise(1.0))
    d = np.asarray(vg.Array(src).noise(1.0))

    # Genuinely independent realizations: "two noisy measurements of the same
    # operation" is what `seed=None` is for.
    assert not np.array_equal(c, d)


def test_mean_and_stddev_participate_in_node_identity() -> None:
    src = np.zeros(32)
    assert not np.array_equal(
        np.asarray(vg.Array(src).noise(1.0, seed=42)),
        np.asarray(vg.Array(src).noise(2.0, seed=42)),
    )


def test_the_drawn_seed_is_recoverable_from_the_plan_graph() -> None:
    # `-> Self` stays intact, so `fmt` is the only way back to an unrepeatable
    # realization -- and the SCA flow already dumps plan graphs.
    lazy = vg.Array(np.zeros(16)).lazy().noise(3.0, mean=1.0)
    dot = lazy.to_dot_graph()

    match = re.search(r"Noise \{ stddev: 3, mean: 1, seed: (\d+) \}", dot)
    assert match is not None, dot

    drawn = int(match.group(1))
    np.testing.assert_array_equal(
        np.asarray(lazy.compute()),
        np.asarray(vg.Array(np.zeros(16)).noise(3.0, mean=1.0, seed=drawn)),
    )


def test_an_explicit_seed_is_used_verbatim() -> None:
    dot = vg.Array(np.zeros(4)).lazy().noise(1.0, seed=12345).to_dot_graph()
    assert "seed: 12345" in dot


# --------------------------------------------------------------------------- #
# 6. Degenerate and invalid parameters
# --------------------------------------------------------------------------- #


def test_zero_stddev_is_the_identity() -> None:
    # The noiseless baseline every SNR sweep starts from, so `for s in [0, ...]`
    # must not fail on its first iteration.
    src = np.asarray([-1.5, 0.0, 2.25, 1e300])
    out = np.asarray(vg.Array(src).noise(0.0, seed=1))
    np.testing.assert_array_equal(out, src)


def test_zero_stddev_adds_the_mean() -> None:
    src = np.asarray([0.0, 1.0, 2.0])
    out = np.asarray(vg.Array(src).noise(0, mean=2.5, seed=1))
    np.testing.assert_array_equal(out, src + 2.5)


def test_zero_stddev_rounds_the_mean_on_integer_input() -> None:
    src = np.full(4, 10, dtype=np.uint8)
    assert np.asarray(vg.Array(src).noise(0, mean=2.5, seed=1)).tolist() == [13] * 4
    assert np.asarray(vg.Array(src).noise(0, mean=-2.5, seed=1)).tolist() == [8] * 4


def test_a_sub_lsb_stddev_on_integer_input_is_accepted() -> None:
    # Degenerate, not an error: at stddev = 0.3 most samples round to no change
    # and the rest to +-1, so `stddev` stops describing the output. The user who
    # wants a sub-LSB floor promotes to `Float` themselves.
    src = np.full(256, 100, dtype=np.uint8)
    out = np.asarray(vg.Array(src).noise(0.3, seed=1))

    assert set(out.tolist()) <= {99, 100, 101}
    assert (out == 100).sum() > len(out) // 2


def test_a_negative_stddev_raises_on_the_noise_call() -> None:
    # Box-Muller is symmetric, so a negative stddev would be silently
    # indistinguishable from its absolute value.
    with pytest.raises(ValueError, match="non-negative"):
        vg.Array(np.zeros(4)).lazy().noise(-1.0)


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), float("-inf")])
def test_a_non_finite_stddev_raises_on_the_noise_call(bad) -> None:
    with pytest.raises(ValueError, match="finite"):
        vg.Array(np.zeros(4)).lazy().noise(bad)


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), float("-inf")])
def test_a_non_finite_mean_raises_on_the_noise_call(bad) -> None:
    # A NaN parameter would poison the entire output array.
    with pytest.raises(ValueError, match="mean must be finite"):
        vg.Array(np.zeros(4)).lazy().noise(1.0, mean=bad)


def test_an_empty_input_gives_an_empty_output() -> None:
    out = np.asarray(vg.Array(np.zeros(0)).noise(1.0, seed=1))
    assert out.tolist() == []


def test_float_propagates_nan_and_infinities() -> None:
    # A deliberate divergence from `quantize`, which maps NaN -> 0. Rewriting a
    # NaN here would hide corrupt input inside a plausible-looking measurement.
    src = np.asarray([float("nan"), float("inf"), float("-inf"), 0.0])
    out = np.asarray(vg.Array(src).noise(1.0, seed=1))

    assert math.isnan(out[0])
    assert out[1] == float("inf")
    assert out[2] == float("-inf")
    assert math.isfinite(out[3])


# --------------------------------------------------------------------------- #
# 7. Statistical sanity
#
# The only test here that catches Box-Muller being wrong by a factor of sqrt(2).
# --------------------------------------------------------------------------- #


def test_sample_mean_and_variance_converge() -> None:
    n = 200_000
    out = np.asarray(vg.Array(np.zeros(n)).noise(3.0, mean=1.0, seed=20250101))

    # The standard error of the mean is 3/sqrt(n) ~= 0.0067, so 0.05 is ~7
    # sigma: tight enough to catch a scale error, loose enough not to flake.
    assert out.mean() == pytest.approx(1.0, abs=0.05)
    assert out.var() == pytest.approx(9.0, rel=0.02)


def test_the_noise_is_zero_mean_per_run() -> None:
    # Noise that is not centred shifts every trace.
    rv = vg.Array(np.zeros(8)).lazy().repeat(20_000)
    runs = np.asarray(rv.noise(2.0, seed=777).compute().as_list())

    assert runs.shape == (8, 20_000)
    np.testing.assert_allclose(runs.mean(axis=1), 0.0, atol=0.1)


def test_different_runs_are_uncorrelated() -> None:
    rv = vg.Array(np.zeros(2)).lazy().repeat(50_000)
    a, b = np.asarray(rv.noise(1.0, seed=31337).compute().as_list())

    # Cross-run correlation is the failure mode that reads downstream as
    # leakage, so it is worth asserting on directly.
    assert abs(np.corrcoef(a, b)[0, 1]) < 0.02


# --------------------------------------------------------------------------- #
# 8. Bits input
# --------------------------------------------------------------------------- #


def test_bit_vector_input_is_rejected() -> None:
    # `Array::slice` is `todo!()` for `Bits`, so a `Bits` run-vector panics
    # rather than errors inside any `Map`. `output_type` converts that into a
    # diagnosable error -- revisit once `Array::slice` handles `Bits`.
    with pytest.raises(ValueError):
        vg.LazyArray.random_bits(8, 8, seed=1).noise(1.0)


def test_wide_bit_vector_input_is_rejected_too() -> None:
    with pytest.raises(ValueError):
        vg.LazyArray.random_bits(8, 128, seed=1).noise(1.0)


def test_quantizing_first_is_the_way_in_from_bits() -> None:
    bits = vg.LazyArray.random_bits(32, 8, seed=1)
    out = np.asarray(bits.quantize(output_dtype="u8").noise(10.0, seed=1).compute())

    assert out.dtype == np.dtype(np.uint8)
    assert len(out) == 32


# --------------------------------------------------------------------------- #
# Downstream consumers
# --------------------------------------------------------------------------- #


def test_noisy_output_feeds_the_aggregators() -> None:
    # noise -> quantize -> statistic is the SNR-sweep chain this node exists for.
    a = vg.Array(np.zeros(64)).noise(1.0, seed=1).lazy().quantize(output_dtype="u16")
    b = vg.Array(np.zeros(64)).noise(1.0, seed=2).lazy().quantize(output_dtype="u16")

    assert isinstance(vg.welch_t_test(a, b).compute().extract(float), float)
    assert isinstance(vg.pearson_corr(a, b).compute().extract(float), float)


def test_noise_is_not_a_terminal_dtype() -> None:
    # Unlike `quantize`, the output dtype is the input dtype, so the nodes that
    # refuse a narrowed width are still reachable afterwards.
    src = np.full(8, 3, dtype=np.uint64)
    assert vg.Array(src).lazy().noise(1.0, seed=1).expand() is not None


def test_the_eager_and_lazy_array_surfaces_agree() -> None:
    src = np.full(16, 500, dtype=np.uint32)

    eager = np.asarray(vg.Array(src).noise(20.0, mean=1.0, seed=9))
    lazy = np.asarray(vg.Array(src).lazy().noise(20.0, mean=1.0, seed=9).compute())
    np.testing.assert_array_equal(eager, lazy)
