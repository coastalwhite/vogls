import numpy as np
import pytest

import vogls as vg

UINT_DTYPES = [np.uint8, np.uint16, np.uint32, np.uint64]
NARROW_DTYPES = [np.uint8, np.uint16, np.uint32]
TYPESTR = {
    np.uint8: "|u1",
    np.uint16: "<u2",
    np.uint32: "<u4",
    np.uint64: "<u8",
}


def _data_ptr(obj) -> int:
    return obj.__array_interface__["data"][0]


def _sample(dtype) -> np.ndarray:
    """Small values plus the width's maximum, so a silent narrowing shows up."""
    return np.asarray([0, 1, 2, 3, 7, np.iinfo(dtype).max], dtype=dtype)


@pytest.mark.parametrize("dtype", UINT_DTYPES)
def test_typestr_matches_width(dtype) -> None:
    arr = vg.Array(_sample(dtype))
    assert arr.__array_interface__["typestr"] == TYPESTR[dtype]


@pytest.mark.parametrize("dtype", UINT_DTYPES)
def test_roundtrip_preserves_dtype_and_values(dtype) -> None:
    src = _sample(dtype)
    out = np.asarray(vg.Array(src))

    assert out.dtype == np.dtype(dtype)
    np.testing.assert_array_equal(out, src)


@pytest.mark.parametrize("dtype", UINT_DTYPES)
def test_roundtrip_is_zero_copy(dtype) -> None:
    # Zero-copy export is the point of narrow storage, and a silently-copying
    # import or export still passes a values-only test -- so assert on addresses.
    src = _sample(dtype)
    arr = vg.Array(src)

    assert _data_ptr(arr) == _data_ptr(src)
    assert _data_ptr(np.asarray(arr)) == _data_ptr(src)


@pytest.mark.parametrize("dtype", UINT_DTYPES)
def test_as_list_and_get_preserve_values(dtype) -> None:
    src = _sample(dtype)
    arr = vg.Array(src)

    assert arr.as_list() == src.tolist()
    assert arr.get(1).extract(int) == 1


def test_unsupported_dtype_is_rejected() -> None:
    with pytest.raises(ValueError):
        vg.Array(np.asarray([1, 2, 3], dtype=np.int8))


def test_zero_is_an_unsigned_value() -> None:
    # `0` is the single most likely bound anyone passes, and it belongs on the
    # unsigned side; negatives still land on `Int`.
    assert np.asarray(vg.Value(0).repeat(2)).dtype == np.dtype(np.uint64)
    assert np.asarray(vg.Value(-1).repeat(2)).dtype == np.dtype(np.int64)


@pytest.mark.parametrize("dtype", UINT_DTYPES)
def test_aggregators_accept_every_width(dtype) -> None:
    a = np.asarray([1, 2, 3, 4, 5, 6, 7, 8], dtype=dtype)
    b = np.asarray([2, 2, 3, 5, 4, 9, 4, 6], dtype=dtype)

    narrow = (vg.Array(a).lazy(), vg.Array(b).lazy())
    wide = (
        vg.Array(a.astype(np.uint64)).lazy(),
        vg.Array(b.astype(np.uint64)).lazy(),
    )

    # These four widen to `f64`/`u64` internally, so a narrow input must give a
    # bit-identical result: this stage changes no numbers.
    for agg in (vg.welch_t_test, vg.pearson_corr, vg.mutual_information):
        assert agg(*narrow).compute().extract(float) == (
            agg(*wide).compute().extract(float)
        )

    assert vg.Array(a).entropy().extract(float) == (
        vg.Array(a.astype(np.uint64)).entropy().extract(float)
    )


@pytest.mark.parametrize("ldtype", UINT_DTYPES)
@pytest.mark.parametrize("rdtype", UINT_DTYPES)
def test_two_input_aggregators_accept_mixed_widths(ldtype, rdtype) -> None:
    # The two inputs' widths are independent, so this walks every arm of the
    # 16-way pair dispatch, not just the diagonal.
    a = np.asarray([1, 2, 3, 4, 5, 6, 7, 8])
    b = np.asarray([2, 2, 3, 5, 4, 9, 4, 6])

    mixed = (vg.Array(a.astype(ldtype)).lazy(), vg.Array(b.astype(rdtype)).lazy())
    wide = (
        vg.Array(a.astype(np.uint64)).lazy(),
        vg.Array(b.astype(np.uint64)).lazy(),
    )

    for agg in (vg.welch_t_test, vg.pearson_corr, vg.mutual_information):
        assert agg(*mixed).compute().extract(float) == (
            agg(*wide).compute().extract(float)
        )


@pytest.mark.parametrize("dtype", NARROW_DTYPES)
def test_expand_rejects_narrow_widths(dtype) -> None:
    # `out.push(x + y)` accumulates, so the width is rejected at graph-build time
    # rather than wrapping in a release build.
    with pytest.raises(ValueError):
        vg.Array(_sample(dtype)).lazy().expand()


def test_expand_accepts_u64() -> None:
    out = np.asarray(vg.Array(np.asarray([1, 2, 3], dtype=np.uint64)).expand())
    np.testing.assert_array_equal(np.sort(out), np.asarray([3, 4, 5]))


@pytest.mark.parametrize("dtype", NARROW_DTYPES)
def test_window_sum_rejects_narrow_widths(dtype) -> None:
    # `sum += on[i]` accumulates; same reasoning as `expand`.
    on = vg.Array(_sample(dtype)).lazy().repeat(1)
    by = vg.Array(np.arange(6, dtype=np.uint64)).lazy().repeat(1)

    with pytest.raises(ValueError):
        on.window_sum(by=by, width=2, start=0, end=6)
