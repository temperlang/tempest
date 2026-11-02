from collections import deque
from temper_core import LoggingConsole, deque_remove_first


def _check_empty() -> None:
    _console.log('Empty' if not _deque else 'Not empty')


_console: LoggingConsole = LoggingConsole(__name__)
_deque: deque[str] = deque()
_check_empty()
if not not _deque:
    _console.log(deque_remove_first(_deque))
_deque.append('foo')
_deque.append('bar')
_console.log(deque_remove_first(_deque))
_deque.append('baz')
_console.log(deque_remove_first(_deque))
_deque.append('boo')
_check_empty()
while not not _deque:
    _console.log(deque_remove_first(_deque))
_check_empty()
