from collections.abc import Sequence as _Sequence
from functions.exporter import norm_squared as _norm_squared
from math import sqrt as _sqrt
from temper_core import (
    LoggingConsole as _LoggingConsole,
    float64_to_string as _float64_to_string,
    list_join as _list_join,
    listed_reduce_from as _listed_reduce_from,
)

__all__ = ('norm', 'something')


def norm(x: float, y: float, /) -> float:
    return _sqrt(_norm_squared(x, y))


def something(vals: _Sequence[float], /) -> float:
    return _listed_reduce_from(vals, 0.0, _norm_squared)


_console: _LoggingConsole = _LoggingConsole(__name__)
_a: float = 3.0
_a = _a
_b: float = _a + 1.0
_values: str = _list_join((_a, _b, norm(_a, _b)), ', ', lambda it: _float64_to_string(it))
_console.log(f'{_values} triangle')
_console.log(_float64_to_string(something((_a, _b))))
