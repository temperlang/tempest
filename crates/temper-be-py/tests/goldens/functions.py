from collections.abc import Callable, MutableSequence, Sequence
from temper_core import (
    LoggingConsole,
    int_add,
    int_mul,
    int_sub,
    list_filter,
    list_get,
    list_join,
    list_map,
    listed_sorted,
)


def _make_counter(start: int, /) -> Callable[[], int]:
    count = start
    def fn() -> int:
        nonlocal count
        count = int_add(count, 1)
        return count
    return fn


def _twice(f: Callable[[int], int], /) -> Callable[[int], int]:
    return lambda x: f(f(x))


def _apply_to(f: Callable[[int], int], x: int, /) -> int:
    return f(x)


def _add_one(x: int, /) -> int:
    return int_add(x, 1)


class _Scaler:
    __slots__ = ()

    @staticmethod
    def triple(x: int, /) -> int:
        return int_mul(x, 3)


class _Button:
    __slots__ = ('_label', '_on_press')
    _label: str
    _on_press: Callable[[], int]

    def __init__(self, /, label: str, on_press: Callable[[], int]) -> None:
        self._label = label
        self._on_press = on_press

    @property
    def label(self, /) -> str:
        return self._label

    @property
    def on_press(self, /) -> Callable[[], int]:
        return self._on_press

    def press(self, /) -> str:
        return f'{self._label} {self._on_press()}'


_console: LoggingConsole = LoggingConsole(__name__)
_counter: Callable[[], int] = _make_counter(10)
_counter()
_console.log(f'count {_counter()}')
_other: Callable[[], int] = _make_counter(0)
_console.log(f'other {_other()} count {_counter()}')
_add_two: Callable[[int], int] = _twice(_add_one)
_console.log(f'twice {_add_two(5)} {_apply_to(_add_one, 1)}')
_console.log(f'triple {_apply_to(_Scaler.triple, 4)} {_twice(_Scaler.triple)(2)}')
_ns: Sequence[int] = (5, 2, 8, 1, 4)
_doubled: Sequence[int] = list_map(_ns, lambda n: int_mul(n, 2))
_big: Sequence[int] = list_filter(_doubled, lambda n: n > 4)
_console.log(list_join(_big, ', ', lambda n: str(n)))
_order: Sequence[int] = listed_sorted(_ns, lambda a, b: int_sub(a, b))
_console.log(list_join(_order, ' ', lambda n: str(n)))
_total: int = 0
_length = len(_ns)
_i2 = 0
while _i2 < _length:
    _n = list_get(_ns, _i2)
    _i2 = int_add(_i2, 1)
    _total = int_add(_total, _n)
_console.log(f'total {_total}')
_button: _Button = _Button('hits', _make_counter(0))
_button.press()
_console.log(_button.press())
_makers: MutableSequence[Callable[[], int]] = []
_i = 0
while _i < 3:
    _k = int_mul(_i, 10)
    def make_fn(_k: int, /) -> Callable[[], int]:
        return lambda: _k
    _makers.append(make_fn(_k))
    _i = int_add(_i, 1)
_console.log(list_join(_makers, ' ', lambda c: str(c())))
