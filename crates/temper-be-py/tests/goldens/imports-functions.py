from collections.abc import Sequence
from functions.exporter import norm_squared
from math import sqrt
from temper_core import (
    LoggingConsole,
    float64_to_string,
    list_join,
    listed_reduce_from,
)


def norm(x: float, y: float, /) -> float:
    return sqrt(norm_squared(x, y))


def something(vals: Sequence[float], /) -> float:
    return listed_reduce_from(vals, 0.0, norm_squared)


_console: LoggingConsole = LoggingConsole(__name__)
_a: float = 3.0
_a = _a
_b: float = _a + 1.0
_values: str = list_join((_a, _b, norm(_a, _b)), ', ', lambda it: float64_to_string(it))
_console.log(f'{_values} triangle')
_console.log(float64_to_string(something((_a, _b))))
