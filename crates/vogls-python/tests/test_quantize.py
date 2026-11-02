"""`quantize()`: linear remap of an integer array onto a narrower unsigned width."""

import numpy as np
import pytest

import vogls as vg

BITS = {"u8": 8, "u16": 16, "u32": 32, "u64": 64}
DTYPE = {"u8": np.uint8, "u16": np.uint16, "u32": np.uint32, "u64": np.uint64}


def _reference(values, lb: int, ub: int, output_dtype: str) -> list[int]:
    """The plan's formula in exact Python integer arithmetic."""
    maxv = (1 << BITS[output_dtype]) - 1
    den = ub - lb
    if den == 0:
        return [0] * len(values)
    out = []
    for x in values:
        clamped = min(max(int(x), lb), ub) - lb
        out.append((clamped * maxv + den // 2) // den)
    return out


def _quantized(src: np.ndarray, **kwargs) -> np.ndarray:
    return np.asarray(vg.Array(src).quantize(**kwargs))


# --------------------------------------------------------------------------- #
# Output dtype and the formula itself
# --------------------------------------------------------------------------- #


@pytest.mark.parametrize("output_dtype", list(BITS))
def test_output_dtype_and_values(output_dtype) -> None:
    src = np.asarray([3, 7, 11, 100, 250], dtype=np.uint64)
    out = _quantized(src, output_dtype=output_dtype, lowerbound=3, upperbound=250)

    assert out.dtype == np.dtype(DTYPE[output_dtype])
    assert out.tolist() == _reference(src, 3, 250, output_dtype)


@pytest.mark.parametrize("output_dtype", list(BITS))
def test_both_endpoints_are_attained(output_dtype) -> None:
    src = np.asarray([10, 20], dtype=np.uint64)
    out = _quantized(src, output_dtype=output_dtype, lowerbound=10, upperbound=20)

    assert out[0] == 0
    assert out[-1] == (1 << BITS[output_dtype]) - 1


def test_rounds_half_up() -> None:
    # ub - lb = 2, so the midpoint lands exactly on MAX/2 and rounds up.
    out = _quantized(
        np.asarray([0, 1, 2], dtype=np.uint64),
        output_dtype="u8",
        lowerbound=0,
        upperbound=2,
    )
    assert out.tolist() == [0, 128, 255]


def test_values_outside_the_bounds_clamp() -> None:
    src = np.asarray([0, 5, 10, 15, 20], dtype=np.uint64)
    out = _quantized(src, output_dtype="u16", lowerbound=5, upperbound=15)

    assert out.tolist() == _reference(src, 5, 15, "u16")
    assert out[0] == out[1] == 0  # below lb
    assert out[3] == out[4] == 65535  # above ub


def test_exact_beyond_f64_mantissa() -> None:
    # Two u64 values that are indistinguishable once passed through an f64, so an
    # f64 implementation of the formula would map them to the same bucket.
    lo = 2**64 - 2049
    hi = 2**64 - 1
    src = np.asarray([0, lo, hi], dtype=np.uint64)
    out = _quantized(src, output_dtype="u64", lowerbound=0, upperbound=hi)

    assert out.tolist() == _reference(src, 0, hi, "u64")
    assert out[1] != out[2]


def test_upscaling_is_a_linear_remap() -> None:
    src = np.asarray([0, 128, 255], dtype=np.uint8)
    out = _quantized(src, output_dtype="u16", lowerbound=0, upperbound=255)

    assert out.dtype == np.dtype(np.uint16)
    assert out.tolist() == [0, 32896, 65535]


# --------------------------------------------------------------------------- #
# Bound resolution
# --------------------------------------------------------------------------- #

SRC = np.asarray([4, 9, 15, 22, 40], dtype=np.uint64)


def test_bounds_default_to_the_data() -> None:
    out = _quantized(SRC, output_dtype="u16")
    assert out.tolist() == _reference(SRC, 4, 40, "u16")


def test_lowerbound_zero_with_upper_from_data() -> None:
    # The common case, and the one that `Value(0)` being signed used to break.
    out = _quantized(SRC, output_dtype="u16", lowerbound=0)
    assert out.tolist() == _reference(SRC, 0, 40, "u16")


def test_upperbound_only_with_lower_from_data() -> None:
    out = _quantized(SRC, output_dtype="u16", upperbound=100)
    assert out.tolist() == _reference(SRC, 4, 100, "u16")


def test_both_bounds_explicit() -> None:
    out = _quantized(SRC, output_dtype="u16", lowerbound=0, upperbound=100)
    assert out.tolist() == _reference(SRC, 0, 100, "u16")


# --------------------------------------------------------------------------- #
# Degenerate cases: all reachable from Python, none of them an assert
# --------------------------------------------------------------------------- #


def test_upper_below_lower_is_an_error() -> None:
    with pytest.raises(ValueError, match="below lowerbound"):
        _quantized(SRC, output_dtype="u16", lowerbound=30, upperbound=10)


def test_explicit_lower_above_data_max_is_an_error() -> None:
    # Mixed resolution can also invert the bounds.
    with pytest.raises(ValueError, match="below lowerbound"):
        _quantized(SRC, output_dtype="u16", lowerbound=1000)


def test_equal_bounds_give_all_zeros() -> None:
    out = _quantized(SRC, output_dtype="u16", lowerbound=7, upperbound=7)
    assert out.tolist() == [0] * len(SRC)


def test_constant_input_under_data_bounds_gives_all_zeros() -> None:
    src = np.asarray([12, 12, 12], dtype=np.uint64)
    assert _quantized(src, output_dtype="u16").tolist() == [0, 0, 0]


def test_empty_input_gives_empty_output() -> None:
    src = np.asarray([], dtype=np.uint64)
    out = _quantized(src, output_dtype="u16")

    assert len(out) == 0
    assert out.dtype == np.dtype(np.uint16)


def test_empty_input_with_explicit_bounds_gives_empty_output() -> None:
    src = np.asarray([], dtype=np.uint64)
    assert len(_quantized(src, output_dtype="u8", lowerbound=0, upperbound=9)) == 0


# --------------------------------------------------------------------------- #
# Input and bound typing
# --------------------------------------------------------------------------- #


@pytest.mark.parametrize("dtype", [np.uint8, np.uint16, np.uint32, np.uint64])
def test_accepts_every_uint_input_width(dtype) -> None:
    src = np.asarray([0, 1, 2, 3], dtype=dtype)
    out = _quantized(src, output_dtype="u16", lowerbound=0, upperbound=3)
    assert out.tolist() == _reference(src, 0, 3, "u16")


def test_accepts_signed_input_with_negative_bounds() -> None:
    src = np.asarray([-100, -50, 0, 50, 100], dtype=np.int64)
    out = _quantized(src, output_dtype="u16", lowerbound=-100, upperbound=100)

    assert out.tolist() == _reference(src, -100, 100, "u16")
    assert out[0] == 0
    assert out[-1] == 65535


def test_signed_input_with_data_bounds() -> None:
    src = np.asarray([-7, 0, 9], dtype=np.int64)
    assert _quantized(src, output_dtype="u8").tolist() == _reference(src, -7, 9, "u8")


def test_negative_bound_on_unsigned_input_raises_at_build_time() -> None:
    # Raised by the bound coercion on the `quantize()` call itself, not during
    # `.compute()`, because the input's dtype is already known there.
    lazy = vg.Array(SRC).lazy()
    with pytest.raises((OverflowError, ValueError, TypeError)):
        lazy.quantize(output_dtype="u16", lowerbound=-1)


def test_bound_too_large_for_the_input_width_raises_at_build_time() -> None:
    lazy = vg.Array(np.asarray([1, 2, 3], dtype=np.uint8)).lazy()
    with pytest.raises((OverflowError, ValueError, TypeError)):
        lazy.quantize(output_dtype="u16", upperbound=300)


def test_unknown_output_dtype_is_rejected() -> None:
    with pytest.raises(ValueError, match="output_dtype"):
        vg.Array(SRC).lazy().quantize(output_dtype="u12")

def test_float_input_quantizes() -> None:
    src = np.asarray([0.0, 0.25, 0.5, 1.0])
    out = _quantized(src, output_dtype="u8", lowerbound=0.0, upperbound=1.0)

    assert out.dtype == np.dtype(np.uint8)
    # 0.25 * 255 = 63.75 -> 64, and 0.5 * 255 = 127.5 rounds away from zero.
    assert out.tolist() == [0, 64, 128, 255]


def test_float_accepts_integral_bounds() -> None:
    # A Python `int` bound on a float array widens rather than erroring.
    out = _quantized(np.asarray([0.0, 1.0]), output_dtype="u8", lowerbound=0, upperbound=1)
    assert out.tolist() == [0, 255]


def test_float_negative_bounds() -> None:
    src = np.asarray([-1.0, 0.0, 1.0])
    out = _quantized(src, output_dtype="u16", lowerbound=-1.0, upperbound=1.0)
    assert out.tolist() == [0, 32768, 65535]


def test_float_nan_quantizes_to_zero() -> None:
    out = _quantized(np.asarray([1.0, np.nan, 3.0]), output_dtype="u8")
    # The NaN maps to 0 and is left out of the data-derived bounds.
    assert out.tolist() == [0, 0, 255]


def test_float_nan_bound_is_an_error() -> None:
    with pytest.raises(ValueError, match="NaN"):
        _quantized(np.asarray([1.0, 2.0]), output_dtype="u8", lowerbound=float("nan"))


def test_float_data_bounds() -> None:
    src = np.asarray([2.5, 5.0, 7.5])
    out = _quantized(src, output_dtype="u8")
    assert out.tolist() == [0, 128, 255]


# --------------------------------------------------------------------------- #
# Bit-vector input
# --------------------------------------------------------------------------- #


def test_bit_vector_output_scales_consistently_across_widths() -> None:
    bits = vg.LazyArray.random_bits(64, 8, seed=11)
    narrow = np.asarray(
        bits.quantize(output_dtype="u8", lowerbound=0, upperbound=255).compute()
    )
    wide = np.asarray(
        bits.quantize(output_dtype="u16", lowerbound=0, upperbound=255).compute()
    )

    assert narrow.dtype == np.dtype(np.uint8)
    assert wide.dtype == np.dtype(np.uint16)
    # 65535 == 255 * 257, so the u16 scale is exactly 257x the u8 one. This pins
    # the values without needing to read the bit-vectors back out.
    np.testing.assert_array_equal(
        wide.astype(np.uint32), narrow.astype(np.uint32) * 257
    )


def test_wide_bit_vector_input_takes_the_slow_path() -> None:
    # 128-bit elements exceed every primitive, so these go through `Bits`
    # arithmetic rather than the u128 kernel.
    bits = vg.LazyArray.random_bits(16, 128, seed=3)
    out = np.asarray(bits.quantize(output_dtype="u16").compute())

    assert out.dtype == np.dtype(np.uint16)
    assert len(out) == 16
    # Data-derived bounds put the extremes at the ends of the scale.
    assert out.min() == 0
    assert out.max() == 65535


def test_bit_vector_bound_must_fit_the_input_width() -> None:
    bits = vg.LazyArray.random_bits(8, 4, seed=1)
    with pytest.raises(ValueError, match="does not fit"):
        bits.quantize(output_dtype="u8", upperbound=100)


# --------------------------------------------------------------------------- #
# RunVector
# --------------------------------------------------------------------------- #


def test_run_vector_bounds_are_global_across_runs() -> None:
    # `repeat(2)` gives one run per element, so every run is constant. Per-run
    # bounds would make lb == ub inside each run and flatten everything to zero;
    # only a global min/max keeps the runs comparable.
    rv = vg.Array(np.asarray([0, 10, 20, 30], dtype=np.uint64)).lazy().repeat(2)
    out = rv.quantize(output_dtype="u16").compute().as_list()

    expected = _reference([0, 10, 20, 30], 0, 30, "u16")
    assert out == [[v, v] for v in expected]
    assert out != [[0, 0]] * 4


def test_run_vector_respects_explicit_bounds() -> None:
    rv = vg.Array(np.asarray([0, 10, 20, 30], dtype=np.uint64)).lazy().repeat(1)
    out = rv.quantize(output_dtype="u8", lowerbound=0, upperbound=60).compute()

    expected = _reference([0, 10, 20, 30], 0, 60, "u8")
    assert out.as_list() == [[v] for v in expected]


def test_run_vector_upper_below_lower_is_an_error() -> None:
    rv = vg.Array(SRC).lazy().repeat(1)
    with pytest.raises(ValueError, match="below lowerbound"):
        rv.quantize(output_dtype="u16", lowerbound=30, upperbound=10).compute()


# --------------------------------------------------------------------------- #
# Downstream consumers
# --------------------------------------------------------------------------- #


def test_quantized_output_feeds_the_aggregators() -> None:
    a = vg.Array(np.asarray([1, 2, 3, 4, 5, 6, 7, 8], dtype=np.uint64))
    b = vg.Array(np.asarray([2, 2, 3, 5, 4, 9, 4, 6], dtype=np.uint64))
    qa = a.quantize(output_dtype="u16").lazy()
    qb = b.quantize(output_dtype="u16").lazy()

    assert isinstance(vg.welch_t_test(qa, qb).compute().extract(float), float)
    assert isinstance(vg.pearson_corr(qa, qb).compute().extract(float), float)
    assert isinstance(vg.mutual_information(qa, qb).compute().extract(float), float)
    assert isinstance(a.quantize(output_dtype="u8").entropy().extract(float), float)


def test_quantize_is_a_terminal_dtype_for_accumulating_nodes() -> None:
    # `quantize(...).expand()` is an error by choice: a clean error beats a
    # wrong number. Quantizing to u64 is the escape hatch.
    with pytest.raises(ValueError):
        vg.Array(SRC).lazy().quantize(output_dtype="u16").expand()

    assert vg.Array(SRC).lazy().quantize(output_dtype="u64").expand() is not None
