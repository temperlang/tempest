from temper_core import LoggingConsole as _LoggingConsole, int_mul as _int_mul

__all__ = ('twice',)


def twice(n: int, /) -> int:
    return _int_mul(n, 2)


_console: _LoggingConsole = _LoggingConsole(__name__)
_runs: int = 0
_base: int = 10
_console.log(f'init {twice(1)}')
