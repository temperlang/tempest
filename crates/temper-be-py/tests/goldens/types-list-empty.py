from collections.abc import Sequence
from temper_core import LoggingConsole


def _f(b: Sequence[int], /) -> None:
    if not b:
        _console.log('empty')
    else:
        _console.log('not empty')


_console: LoggingConsole = LoggingConsole(__name__)
_f((2, 3))
_f(())
