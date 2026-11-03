from collections.abc import Callable, Sequence
from temper_core import float64_to_string, int_add, list_get
from temper_std.testing import Test


class Vec2:
    __slots__ = ('_x', '_y')
    _x: float
    _y: float

    def __init__(self, /, x: float, y: float) -> None:
        self._x = x
        self._y = y

    @property
    def x(self, /) -> float:
        return self._x

    @property
    def y(self, /) -> float:
        return self._y

    def to_string(self, /) -> str:
        return f'({float64_to_string(self._x)}, {float64_to_string(self._y)}) size squared {float64_to_string(_norm_squared(self))}'

    def __str__(self, /) -> str:
        return self.to_string()


def _norm_squared(vec: Vec2, /) -> float:
    return vec.x * vec.x + vec.y * vec.y


class _TestHelper:
    __slots__ = ('_i', '_j')
    _i: int
    _j: int

    def __init__(self, /, i: int, j: int) -> None:
        self._i = i
        self._j = j

    @property
    def i(self, /) -> int:
        return self._i

    @property
    def j(self, /) -> int:
        return self._j


def _assert_more_things(t: Test, helper: _TestHelper, strings: Sequence[Sequence[str]], /) -> None:  # pyright: ignore[reportUnusedFunction]
    i = 0
    while i < len(strings):
        j = 0
        while j < len(list_get(strings, i)):
            actual = helper.i
            expected = helper.j
            t_0 = actual == expected
            def make_fn(actual: int, expected: int, /) -> Callable[[], str]:
                return lambda: f'expected helper.i == ({expected}) not ({actual})'
            t.assert_(t_0, make_fn(actual, expected))
            s = list_get(list_get(strings, i), j)
            actual2 = s
            expected2 = 'hi'
            t_1 = actual2 == expected2
            def make_fn2(actual2: str, expected2: str, /) -> Callable[[], str]:
                return lambda: f'expected s == ({expected2}) not ({actual2})'
            t.assert_(t_1, make_fn2(actual2, expected2))
            j = int_add(j, 1)
        i = int_add(i, 1)


_name: str = 'case'
