from collections.abc import Callable as _Callable
from temper_core import (
    LoggingConsole as _LoggingConsole,
    int_add as _int_add,
    int_mul as _int_mul,
)

__all__ = ()


def _call_it(i: int, f: _Callable[[int], int], /) -> int:
    return _int_mul(3, f(i))


def _enclose(i: int, /) -> None:
    result = _call_it(2, lambda j: _int_add(i, j))
    _console.log(str(result))


_console: _LoggingConsole = _LoggingConsole(__name__)
_enclose(1)
