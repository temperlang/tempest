from collections.abc import Sequence as _Sequence
from temper_core import (
    LoggingConsole as _LoggingConsole,
    int_add as _int_add,
    list_filter as _list_filter,
    list_get as _list_get,
)

__all__ = ()


def _total(xs: _Sequence[int], /) -> int:
    t = 0
    length = len(xs)
    i = 0
    while i < length:
        x = _list_get(xs, i)
        i = _int_add(i, 1)
        t = _int_add(t, x)
    return t


def _total_for_each(xs: _Sequence[int], /) -> int:
    t = 0
    length = len(xs)
    i = 0
    while i < length:
        x = _list_get(xs, i)
        i = _int_add(i, 1)
        t = _int_add(t, x)
    return t


def _total_filtered(xs: _Sequence[int], /) -> int:
    t = 0
    def fn(x: int, /) -> bool:
        nonlocal t
        t = _int_add(t, x)
        return True
    _list_filter(xs, fn)
    return t


def _total_nested(xss: _Sequence[_Sequence[int]], /) -> int:
    t = 0
    length = len(xss)
    i = 0
    while i < length:
        xs = _list_get(xss, i)
        i = _int_add(i, 1)
        length2 = len(xs)
        i2 = 0
        while i2 < length2:
            x = _list_get(xs, i2)
            i2 = _int_add(i2, 1)
            t = _int_add(t, x)
    return t


def _first_over(xs: _Sequence[int], n: int, /) -> int:
    length = len(xs)
    i = 0
    while i < length:
        x = _list_get(xs, i)
        i = _int_add(i, 1)
        if x > n:
            return x
    return -1


_console: _LoggingConsole = _LoggingConsole(__name__)
_console.log(f'total {_total((1, 2, 3))}')
_console.log(f'forEach {_total_for_each((1, 2, 3))}')
_console.log(f'filter {_total_filtered((1, 2, 3))}')
_console.log(f'nested {_total_nested(((1, 2), (3,), ()))}')
_console.log(f'firstOver {_first_over((1, 2, 3), 1)} {_first_over((1, 2, 3), 5)}')
