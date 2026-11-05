from collections.abc import Sequence as _Sequence
from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()


def _f(b: _Sequence[int], /) -> None:
    if not b:
        _console.log('empty')
    else:
        _console.log('not empty')


_console: _LoggingConsole = _LoggingConsole(__name__)
_f((2, 3))
_f(())
