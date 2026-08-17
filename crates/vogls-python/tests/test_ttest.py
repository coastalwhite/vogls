import numpy as np
import pytest

import vogls as vg


def _reference_ttest(a: np.ndarray, b: np.ndarray, order: int) -> float:
    a = a.astype(float)
    b = b.astype(float)

    def preprocess(x: np.ndarray) -> np.ndarray:
        mu = x.mean()
        if order == 1:
            return x
        if order == 2:
            return (x - mu) ** 2
        sigma = x.std()
        return ((x - mu) / sigma) ** order

    pa, pb = preprocess(a), preprocess(b)
    numerator = pa.mean() - pb.mean()
    denominator = np.sqrt(pa.var(ddof=0) / len(a) + pb.var(ddof=0) / len(b))
    return numerator / denominator


def _vogls_ttest(a: np.ndarray, b: np.ndarray, order: int = 1) -> float:
    la = vg.Array(a.astype(np.uint64)).lazy()
    lb = vg.Array(b.astype(np.uint64)).lazy()
    return vg.welch_t_test(la, lb, order=order).compute().extract(float)


A = np.array([1, 2, 3, 4, 5, 6, 7, 8], dtype=np.uint64)
B = np.array([2, 2, 3, 5, 4, 9, 4, 6], dtype=np.uint64)


@pytest.mark.parametrize("order", [1, 2, 3, 4])
def test_ttest_matches_reference(order: int) -> None:
    got = _vogls_ttest(A, B, order=order)
    expected = _reference_ttest(A, B, order)
    assert got == pytest.approx(expected, rel=1e-9, abs=1e-9)


def test_ttest_default_is_first_order() -> None:
    la, lb = vg.Array(A).lazy(), vg.Array(B).lazy()
    default = vg.welch_t_test(la, lb).compute().extract(float)
    first = vg.welch_t_test(la, lb, order=1).compute().extract(float)
    assert default == first


def test_ttest_orders_differ() -> None:
    t1 = _vogls_ttest(A, B, order=1)
    t2 = _vogls_ttest(A, B, order=2)
    t3 = _vogls_ttest(A, B, order=3)
    assert t1 != t2
    assert t2 != t3


def test_ttest_known_first_order_value() -> None:
    a = np.array([0, 2, 0, 2], dtype=np.uint64)
    b = np.array([1, 3, 1, 3], dtype=np.uint64)
    assert _vogls_ttest(a, b, order=1) == pytest.approx(-1.0 / np.sqrt(0.5))


def test_ttest_rejects_order_below_one() -> None:
    la, lb = vg.Array(A).lazy(), vg.Array(B).lazy()
    with pytest.raises(ValueError):
        vg.welch_t_test(la, lb, order=0)
