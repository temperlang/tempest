from collections import deque as _deque_
from temper_core import (
    LoggingConsole as _LoggingConsole,
    deque_remove_first as _deque_remove_first,
)

__all__ = ()


def _check_empty() -> None:
    _console.log('Empty' if not _deque else 'Not empty')


_console: _LoggingConsole = _LoggingConsole(__name__)
_deque: _deque_[str] = _deque_()
_check_empty()
if not not _deque:
    _console.log(_deque_remove_first(_deque))
_deque.append('foo')
_deque.append('bar')
_console.log(_deque_remove_first(_deque))
_deque.append('baz')
_console.log(_deque_remove_first(_deque))
_deque.append('boo')
_check_empty()
while not not _deque:
    _console.log(_deque_remove_first(_deque))
_check_empty()
