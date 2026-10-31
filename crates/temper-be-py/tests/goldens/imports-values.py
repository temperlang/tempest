from temper_core import LoggingConsole, int_mul
from values.exporter import magic_number


def times_magic(n: int, /) -> int:
    return int_mul(n, magic_number)


_console: LoggingConsole = LoggingConsole(__name__)
_a: int = 1
_a = _a
_c: int = times_magic(_a)
_console.log(f'number = {_c}')
