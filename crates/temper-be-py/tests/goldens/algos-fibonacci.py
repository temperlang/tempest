from temper_core import (
    LoggingConsole as _LoggingConsole,
    int_add as _int_add,
    int_sub as _int_sub,
)

__all__ = ()


def _fib(i: int, /) -> int:
    a = 0
    b = 1
    while i > 0:
        c = _int_add(a, b)
        a = b
        b = c
        i = _int_sub(i, 1)
    return a


_console: _LoggingConsole = _LoggingConsole(__name__)
_zero: int = 0
_zero = _zero
_console.log(f'fib(1)={_fib(_int_add(_zero, 1))}')
_console.log(f'fib(10)={_fib(_int_add(_zero, 10))}')
_console.log(f'fib(20)={_fib(_int_add(_zero, 20))}')
