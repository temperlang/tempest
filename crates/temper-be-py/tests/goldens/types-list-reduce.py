from collections.abc import Callable, Sequence
from temper_core import (
    LoggingConsole,
    arith_int_mod,
    boolean_to_string,
    int_add,
    listed_reduce,
    listed_reduce_from,
)
from typing import TypeVar

T = TypeVar('T')


def _all(items: Sequence[T], check: Callable[[T], bool], /) -> bool:
    def fn(result: bool, it: T, /) -> bool:
        if result:
            return check(it)
        else:
            return False
    return listed_reduce_from(items, True, fn)


_console: LoggingConsole = LoggingConsole(__name__)
_vals: Sequence[int] = (2, 3, 4)


def fn(sum_: int, n: int, /) -> int:
    return int_add(sum_, n)


_sum: int = listed_reduce(_vals, fn)
_poor_reverse_join: str = listed_reduce_from(_vals, '', lambda text, n: str(n) + text)
_console.log(f'Reductions: {_sum} {_poor_reverse_join}')
_console.log(f'All even: {boolean_to_string(_all(_vals, lambda n: arith_int_mod(n, 2) == 0))}')
_console.log(f'All positive: {boolean_to_string(_all(_vals, lambda n: n > 0))}')
