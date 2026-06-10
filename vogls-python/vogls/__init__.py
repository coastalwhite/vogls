from typing import Callable, Self, TypeVar, Generic, Any, List
import vogls.vogls as vgr

T = TypeVar("T")
U = TypeVar("U")


class Lazy(Generic[T]):
    """A lazy class that will materialize into a T when computed."""

    def __init__(self, producer: Any) -> None:
        self._producer = producer

    def compute(self) -> T:
        return self._producer.compute()


class Array:
    _inner: vgr.PyArray

    def __init__(self, v: list[float]) -> None:
        self._inner = vgr.PyArray.from_f64s(v)

    def lazy(self) -> "LazyValue":
        return LazyArray._from_py(self._inner.lazy())


class Value:
    _inner: vgr.PyValue

    def __init__(self, v: int | float) -> None:
        if isinstance(v, int) and v > 0:
            self._inner = vgr.PyValue.from_unsigned_int(v)
        elif isinstance(v, int):
            self._inner = vgr.PyValue.from_signed_int(v)
        elif isinstance(v, float):
            self._inner = vgr.PyValue.from_float(v)
        else:
            v_type = v if type(v) is type else f"of type {type(v).__name__!r}"
            raise ValueError(f"cannot turn {v_type} into Value")

    @classmethod
    def _from_py(cls, py: vgr.PyValue) -> Self:
        slf = cls()
        slf._inner = py
        return slf

    def lazy(self) -> "LazyValue":
        return LazyValue._from_py(self._inner.lazy())


class Plan:
    _inner: vgr.PyPlan

    def lazy(self) -> "LazyPlan":
        return LazyPlan._from_py(self._inner.lazy())


class LazyDesign:
    _inner: vgr.PyLazyDesign

    def __init__(
        self, sources: str | List[str], *, top_level_module: str | None = None
    ) -> None:
        sources_paths: List[str]
        if isinstance(sources, str):
            sources_paths = [sources]
        else:
            sources_paths = sources
        self._inner = vgr.PyLazyDesign(sources_paths, top_level_module)

    def run(self) -> "LazyRun":
        return LazyRun._from_py(self._inner.run())

    @classmethod
    def _from_py(cls, py: vgr.PyLazyDesign) -> Self:
        slf = cls()
        slf._inner = py
        return slf


class LazyRun:
    _inner: vgr.PyLazyRun

    @classmethod
    def _from_py(cls, py: vgr.PyLazyRun) -> Self:
        slf = cls()
        slf._inner = py
        return slf

    def run_for(self, time: int) -> Self:
        return LazyRun._from_py(self._inner.run_for(time))

    def repeat(self, n: int) -> Self:
        return LazyRun._from_py(self._inner.repeat(n))

    def trace_start(self) -> Self:
        return LazyRun._from_py(self._inner.trace_start())

    def trace_stop(self) -> Self:
        return LazyRun._from_py(self._inner.trace_stop())

    def set_signal(self, name: str | List[str], array: "LazyArray") -> Self:
        names_list: List[str]
        if isinstance(name, str):
            names_list = [name]
        else:
            names_list = name
        return LazyRun._from_py(self._inner.set_signal(names_list, array._inner))

    def hamming_distance(self) -> Any:
        return self._inner.hamming_distance()


class LazyValue(Lazy[Value]):
    @classmethod
    def _from_py(cls, py: vgr.PyLazyValue) -> Self:
        instance = cls.__new__(cls)
        super(LazyValue, instance).__init__(py)
        return instance

    def repeat(self, n: int) -> "LazyArray":
        return self._producer.repeat(n)


class LazyArray(Lazy[Array]):
    @classmethod
    def _from_py(cls, py: vgr.PyLazyArray) -> Self:
        instance = cls.__new__(cls)
        super(LazyArray, instance).__init__(py)
        return instance

    def min(self) -> LazyValue:
        self._producer.min()


class LazyPlan(Lazy[Plan]):
    @classmethod
    def _from_py(cls, py: vgr.PyLazyPlan) -> Self:
        instance = cls.__new__(cls)
        super(LazyPlan, instance).__init__(py)
        return instance

class _LazyLambda:
    inner: Lazy[T]
    f: Callable[[T], U]
    
    def __init__(self, producer: Lazy[T], f: Callable[[T], U]) -> None:
        self.inner = producer
        self.f = f

    def compute(self) -> U:
        return self.f(self.inner.compute())


def t_test(lhs: LazyArray, rhs: LazyArray) -> Lazy[float]:
    return _LazyLambda(
        Lazy(vgr.PyLazyOutput.ttest(lhs._producer, rhs._producer)),
        lambda output: output.extract_value().extract_float(),
    )
