from modules.helpers.deep import greet
from temper_core import LoggingConsole, int_add, int_mul


def twice(x: int, /) -> int:
    return int_mul(x, 2)


def bump() -> int:
    global calls
    calls = int_add(calls, 1)
    return calls


_console: LoggingConsole = LoggingConsole(__name__)
base: int = 10
calls: int = 0
_console.log(greet('helpers'))
