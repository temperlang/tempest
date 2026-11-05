from temper_core import LoggingConsole as _LoggingConsole, int_mul as _int_mul
from values.exporter import magic_number as _magic_number

__all__ = ('times_magic',)


def times_magic(n: int, /) -> int:
    return _int_mul(n, _magic_number)


_console: _LoggingConsole = _LoggingConsole(__name__)
_a: int = 1
_a = _a
_c: int = times_magic(_a)
_console.log(f'number = {_c}')
