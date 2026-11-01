from collections.abc import Sequence
from temper_core import LoggingConsole, int_add, list_filter, list_get


def _total(xs: Sequence[int], /) -> int:
    t = 0
    length = len(xs)
    i = 0
    while i < length:
        x = list_get(xs, i)
        i = int_add(i, 1)
        t = int_add(t, x)
    return t


def _total_for_each(xs: Sequence[int], /) -> int:
    t = 0
    length = len(xs)
    i = 0
    while i < length:
        x = list_get(xs, i)
        i = int_add(i, 1)
        t = int_add(t, x)
    return t


def _total_filtered(xs: Sequence[int], /) -> int:
    t = 0
    def fn(x: int, /) -> bool:
        nonlocal t
        t = int_add(t, x)
        return True
    list_filter(xs, fn)
    return t


def _total_nested(xss: Sequence[Sequence[int]], /) -> int:
    t = 0
    length = len(xss)
    i = 0
    while i < length:
        xs = list_get(xss, i)
        i = int_add(i, 1)
        length2 = len(xs)
        i2 = 0
        while i2 < length2:
            x = list_get(xs, i2)
            i2 = int_add(i2, 1)
            t = int_add(t, x)
    return t


def _first_over(xs: Sequence[int], n: int, /) -> int:
    length = len(xs)
    i = 0
    while i < length:
        x = list_get(xs, i)
        i = int_add(i, 1)
        if x > n:
            return x
    return -1


_console: LoggingConsole = LoggingConsole(__name__)
_console.log(f'total {_total((1, 2, 3))}')
_console.log(f'forEach {_total_for_each((1, 2, 3))}')
_console.log(f'filter {_total_filtered((1, 2, 3))}')
_console.log(f'nested {_total_nested(((1, 2), (3,), ()))}')
_console.log(f'firstOver {_first_over((1, 2, 3), 1)} {_first_over((1, 2, 3), 5)}')
