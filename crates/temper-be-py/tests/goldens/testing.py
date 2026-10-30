from temper_core import LoggingConsole, int_mul


def twice(n: int, /) -> int:
    return int_mul(n, 2)


_console: LoggingConsole = LoggingConsole(__name__)
_runs: int = 0
_base: int = 10
_console.log(f'init {twice(1)}')
