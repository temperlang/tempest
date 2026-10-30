from temper_core import LoggingConsole, int_add, int_sub


def _fib(i: int, /) -> int:
    a = 0
    b = 1
    while i > 0:
        c = int_add(a, b)
        a = b
        b = c
        i = int_sub(i, 1)
    return a


_console: LoggingConsole = LoggingConsole(__name__)
_zero: int = 0
_zero = _zero
_console.log(f'fib(1)={_fib(int_add(_zero, 1))}')
_console.log(f'fib(10)={_fib(int_add(_zero, 10))}')
_console.log(f'fib(20)={_fib(int_add(_zero, 20))}')
