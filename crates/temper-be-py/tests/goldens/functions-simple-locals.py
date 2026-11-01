from collections.abc import Callable
from temper_core import LoggingConsole, int_add, int_mul


def _call_it(i: int, f: Callable[[int], int], /) -> int:
    return int_mul(3, f(i))


def _enclose(i: int, /) -> None:
    result = _call_it(2, lambda j: int_add(i, j))
    _console.log(str(result))


_console: LoggingConsole = LoggingConsole(__name__)
_enclose(1)
