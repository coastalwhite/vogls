import numpy as np
import pytest

import vogls as vg


def _array(*values: int) -> vg.Array:
    return vg.Array(np.array(values, dtype=np.uint64))


def test_lazy_plan_from_dict_round_trips() -> None:
    plan = vg.LazyPlan.from_dict(
        {
            "a": _array(1, 2, 3).lazy(),
            "v": vg.Value(7).lazy(),
        }
    ).compute()

    assert plan.get("a", vg.Array).as_list() == [1.0, 2.0, 3.0]
    assert plan.get("v", vg.Value).extract(int) == 7


def test_plan_from_dict_round_trips() -> None:
    plan = vg.Plan.from_dict({"a": _array(4, 5), "v": vg.Value(1)})

    assert plan.get("a", vg.Array).as_list() == [4.0, 5.0]
    assert plan.get("v", vg.Value).extract(int) == 1


def test_plan_from_dict_nests_and_lazifies() -> None:
    inner = vg.Plan.from_dict({"v": vg.Value(3)})
    outer = vg.Plan.from_dict({"inner": inner}).lazy().compute()

    assert outer.get("inner", vg.Plan).get("v", vg.Value).extract(int) == 3


def test_lazy_plan_from_dict_nests() -> None:
    inner = vg.LazyPlan.from_dict({"v": vg.Value(3).lazy()})
    outer = vg.LazyPlan.from_dict({"inner": inner}).compute()

    assert outer.get("inner", vg.Plan).get("v", vg.Value).extract(int) == 3


def test_from_dict_empty() -> None:
    assert vg.LazyPlan.from_dict({}).compute() is not None
    assert vg.Plan.from_dict({}) is not None


def test_from_dict_rejects_wrong_laziness() -> None:
    with pytest.raises(ValueError):
        vg.LazyPlan.from_dict({"v": vg.Value(1)})  # type: ignore[dict-item]
    with pytest.raises(ValueError):
        vg.Plan.from_dict({"v": vg.Value(1).lazy()})  # type: ignore[dict-item]
