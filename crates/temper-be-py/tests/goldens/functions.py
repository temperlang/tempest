from collections.abc import (
    Callable as _Callable,
    MutableSequence as _MutableSequence,
    Sequence as _Sequence,
)
from concurrent.futures import Future as _Future
from temper_core import (
    LoggingConsole as _LoggingConsole,
    adapt_generator_factory as _adapt_generator_factory,
    int_add as _int_add,
    int_mul as _int_mul,
    int_sub as _int_sub,
    list_filter as _list_filter,
    list_get as _list_get,
    list_join as _list_join,
    list_map as _list_map,
    listed_sorted as _listed_sorted,
)
from typing import Any as _Any, Generator as _Generator

__all__ = ()


def _make_counter(start: int, /) -> _Callable[[], int]:
    count = start
    def fn() -> int:
        nonlocal count
        count = _int_add(count, 1)
        return count
    return fn


def _twice(f: _Callable[[int], int], /) -> _Callable[[int], int]:
    return lambda x: f(f(x))


def _apply_to(f: _Callable[[int], int], x: int, /) -> int:
    return f(x)


def _add_one(x: int, /) -> int:
    return _int_add(x, 1)


class _Scaler:
    __slots__ = ()

    @staticmethod
    def triple(x: int, /) -> int:
        return _int_mul(x, 3)


class _Button:
    __slots__ = ('_label', '_on_press')
    _label: str
    _on_press: _Callable[[], int]

    def __init__(self, /, label: str, on_press: _Callable[[], int]) -> None:
        self._label = label
        self._on_press = on_press

    @property
    def label(self, /) -> str:
        return self._label

    @property
    def on_press(self, /) -> _Callable[[], int]:
        return self._on_press

    def press(self, /) -> str:
        return f'{self._label} {self._on_press()}'


def _drive(steps: int, body: _Callable[[], _Generator[_Any, _Any, None]], /) -> None:
    g = body()
    s = 0
    while s < steps:
        next(g, None)
        s = _int_add(s, 1)


_console: _LoggingConsole = _LoggingConsole(__name__)
_counter: _Callable[[], int] = _make_counter(10)
_counter()
_console.log(f'count {_counter()}')
_other: _Callable[[], int] = _make_counter(0)
_console.log(f'other {_other()} count {_counter()}')
_add_two: _Callable[[int], int] = _twice(_add_one)
_console.log(f'twice {_add_two(5)} {_apply_to(_add_one, 1)}')
_console.log(f'triple {_apply_to(_Scaler.triple, 4)} {_twice(_Scaler.triple)(2)}')
_ns: _Sequence[int] = (5, 2, 8, 1, 4)
_doubled: _Sequence[int] = _list_map(_ns, lambda n: _int_mul(n, 2))
_big: _Sequence[int] = _list_filter(_doubled, lambda n: n > 4)
_console.log(_list_join(_big, ', ', lambda n: str(n)))
_order: _Sequence[int] = _listed_sorted(_ns, lambda a, b: _int_sub(a, b))
_console.log(_list_join(_order, ' ', lambda n: str(n)))
_total: int = 0
_length = len(_ns)
_i2 = 0
while _i2 < _length:
    _n = _list_get(_ns, _i2)
    _i2 = _int_add(_i2, 1)
    _total = _int_add(_total, _n)
_console.log(f'total {_total}')
_button: _Button = _Button('hits', _make_counter(0))
_button.press()
_console.log(_button.press())
_makers: _MutableSequence[_Callable[[], int]] = []
_i = 0
while _i < 3:
    _k = _int_mul(_i, 10)
    def _make_fn6(_k: int, /) -> _Callable[[], int]:
        return lambda: _k
    _makers.append(_make_fn6(_k))
    _i = _int_add(_i, 1)
_console.log(_list_join(_makers, ' ', lambda c: str(c())))


@_adapt_generator_factory
def _fn8(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
    fs: _MutableSequence[_Callable[[], int]] = []
    i = 0
    while i < 3:
        j = _int_mul(i, 10)
        def make_fn9(j: int, /) -> _Callable[[], int]:
            return lambda: j
        fs.append(make_fn9(j))
        yield
        def make_fn10(j: int, /) -> _Callable[[], int]:
            return lambda: _int_add(j, 1)
        fs.append(make_fn10(j))
        i = _int_add(i, 1)
    _console.log(_list_join(fs, ' ', lambda c: str(c())))
    yield


_drive(5, _fn8)
