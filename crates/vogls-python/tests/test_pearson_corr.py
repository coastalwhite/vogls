import numpy as np
import vogls as vg


def test_pearson_corr() -> None:
    a1 = vg.Array(np.array([1, 2, 3, 4], dtype=np.uint64))
    a2 = vg.Array(np.array([5, 6, 7, 8], dtype=np.uint64))

    v = vg.pearson_corr(a1.lazy(), a2.lazy())

    assert round(v.compute().extract(float), 3) == 1.0
