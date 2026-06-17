from typing import Callable, TypeVar, Generic, Any, Union, Self, overload, List
import vogls.vogls as vgr

T = TypeVar("T")
U = TypeVar("U")


class Lazy(Generic[T]):
    """A lazy class that will materialize into a T when computed."""

    def __init__(self, producer: Any) -> None:
        self._producer = producer

    def compute(self) -> T:
        return self._producer.compute()

    def to_dot_graph(self) -> str:
        return self._producer.to_dot_graph()


class _LazyLambda:
    inner: Lazy[T]
    f: Callable[[T], U]

    def __init__(self, producer: Lazy[T], f: Callable[[T], U]) -> None:
        self.inner = producer
        self.f = f

    def compute(self) -> U:
        return self.f(self.inner.compute())


class Run:
    _inner: vgr.PyRun

    @classmethod
    def _from_py(cls, py: vgr.PyRun) -> Self:
        slf = cls()
        slf._inner = py
        return slf

    def run_for(self, time: int) -> Self:
        return Run._from_py(self._inner.run_for(time))

    def repeat(self, n: int) -> Self:
        return Run._from_py(self._inner.repeat(n))

    def trace_start(self) -> Self:
        return Run._from_py(self._inner.trace_start())

    def trace_stop(self) -> Self:
        return Run._from_py(self._inner.trace_stop())

    def set_signal(self, name: str | List[str], array: "LazyArray") -> Self:
        names_list: List[str]
        if isinstance(name, str):
            names_list = [name]
        else:
            names_list = name
        return Run._from_py(self._inner.set_signal(names_list, array._producer))

    def hamming_distance(self, name: str = "hd") -> Self:
        return Run._from_py(self._inner.hamming_distance(name=name))

    def finish(self) -> "LazyPlan":
        return LazyPlan._from_py(self._inner.finish())


class Array:
    _inner: vgr.PyArray

    def __init__(self, v: list[float]) -> None:
        self._inner = vgr.PyArray.from_f64s(v)

    @classmethod
    def _from_py(cls, py: vgr.PyArray) -> Self:
        instance = cls.__new__(cls)
        instance._inner = py
        return instance

    def lazy(self) -> "LazyValue":
        return LazyArray._from_py(self._inner.lazy())


class LazyArray(Lazy[Array]):
    @classmethod
    def _from_py(cls, py: vgr.PyLazyArray) -> Self:
        instance = cls.__new__(cls)
        super(LazyArray, instance).__init__(py)
        return instance

    @staticmethod
    def random_bits(length: int, width: int, seed: int | None = None) -> Self:
        return LazyArray._from_py(vgr.PyLazyArray.random_bits(length, width, seed))

    def min(self) -> "LazyValue":
        self._producer.min()


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

    def run(self) -> Run:
        return Run._from_py(self._inner.run())

    @classmethod
    def _from_py(cls, py: vgr.PyLazyDesign) -> Self:
        slf = cls()
        slf._inner = py
        return slf


class Plan:
    _inner: vgr.PyPlan

    @classmethod
    def _from_py(cls, py: vgr.PyPlan) -> Self:
        instance = cls.__new__(cls)
        instance._inner = py
        return instance

    def lazy(self) -> "LazyPlan":
        return LazyPlan._from_py(self._inner.lazy())

    @overload
    def get(self, key: str, kind: type["RunVector"]) -> "RunVector": ...
    @overload
    def get(self, key: str, kind: type["Array"]) -> "Array": ...
    @overload
    def get(self, key: str, kind: type["Value"]) -> "Value": ...
    @overload
    def get(self, key: str, kind: type["Plan"]) -> "Plan": ...

    def get(self, key: str, kind: type | None = None) -> "Output":
        output = _output_from_py(self._inner.get(key))
        if kind is not None:
            assert isinstance(output, kind)
        return output


class LazyPlan(Lazy[Plan]):
    @classmethod
    def _from_py(cls, py: vgr.PyLazyPlan) -> Self:
        instance = cls.__new__(cls)
        super(LazyPlan, instance).__init__(py)
        return instance

    @overload
    def get(self, key: str, kind: type["LazyRunVector"]) -> "LazyRunVector": ...
    @overload
    def get(self, key: str, kind: type["LazyArray"]) -> "LazyArray": ...
    @overload
    def get(self, key: str, kind: type["LazyValue"]) -> "LazyValue": ...
    @overload
    def get(self, key: str, kind: type["LazyPlan"]) -> "LazyPlan": ...

    def get(self, key: str, kind: type | None = None) -> "LazyOutput":
        output = _lazyoutput_from_py(self._producer.get(key))
        if kind is not None:
            assert isinstance(output, kind)
        return output


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

    def repeat(self, n: int) -> Array:
        return self.lazy().repeat(n).compute()


class LazyValue(Lazy[Value]):
    @classmethod
    def _from_py(cls, py: vgr.PyLazyValue) -> Self:
        instance = cls.__new__(cls)
        super(LazyValue, instance).__init__(py)
        return instance

    def repeat(self, n: int) -> LazyArray:
        return self._producer.repeat(n)


class RunVector:
    __slots__ = ("_inner",)
    _inner: vgr.PyRunVector

    @classmethod
    def _from_py(cls, py: vgr.PyRunVector) -> Self:
        instance = cls.__new__(cls)
        instance._inner = py
        return instance

    def lazy(self) -> "LazyRunVector":
        return LazyRunVector._from_py(self._inner.lazy())

    def window_sum(self, *, by: Self, width: int, start: int, end: int) -> Self:
        return (
            self.lazy().window_sum(by=by, width=width, start=start, end=end).compute()
        )

    def entropy(self) -> Array:
        return self.lazy().entropy().compute()


class LazyRunVector(Lazy[RunVector]):
    @classmethod
    def _from_py(cls, py: vgr.PyLazyRunVector) -> Self:
        instance = cls.__new__(cls)
        super(LazyRunVector, instance).__init__(py)
        return instance

    def window_sum(self, *, by: Self, width: int, start: int, end: int) -> Self:
        return LazyRunVector._from_py(
            self._producer.window_sum(
                by=by._producer, width=width, start=start, end=end
            )
        )

    def entropy(self) -> LazyArray:
        return LazyArray._from_py(self._producer.entropy())


Output = Union[RunVector, Array, Value, Plan]
LazyOutput = Union[LazyRunVector, LazyArray, LazyValue, LazyPlan]


def _output_from_py(v: Any) -> Output:
    if isinstance(v, vgr.PyRunVector):
        return RunVector._from_py(v)
    elif isinstance(v, vgr.PyArray):
        return Array._from_py(v)
    elif isinstance(v, vgr.PyValue):
        return Value._from_py(v)
    elif isinstance(v, vgr.PyPlan):
        return Plan._from_py(v)

    v_type = v if type(v) is type else f"of type {type(v).__name__!r}"
    raise ValueError(f"cannot turn {v_type} into Output")


def _lazyoutput_from_py(v: Any) -> LazyOutput:
    if isinstance(v, vgr.PyLazyRunVector):
        return LazyRunVector._from_py(v)
    elif isinstance(v, vgr.PyLazyArray):
        return LazyArray._from_py(v)
    elif isinstance(v, vgr.PyLazyValue):
        return LazyValue._from_py(v)
    elif isinstance(v, vgr.PyLazyPlan):
        return LazyPlan._from_py(v)

    v_type = v if type(v) is type else f"of type {type(v).__name__!r}"
    raise ValueError(f"cannot turn {v_type} into Output")


def welch_t_test(lhs: LazyRunVector, rhs: LazyRunVector) -> LazyArray:
    return LazyArray._from_py(vgr.PyLazyArray.ttest(lhs._producer, rhs._producer))


def mutual_information(lhs: LazyRunVector, rhs: LazyRunVector) -> LazyArray:
    return LazyArray._from_py(
        vgr.PyLazyArray.mutual_information(lhs._producer, rhs._producer)
    )
