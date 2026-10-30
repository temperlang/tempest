from temper_core import LoggingConsole
from typing import ClassVar


class _Simple:
    __slots__ = ()
    value: ClassVar[str]
    other: ClassVar[str]


def _read_simple_value() -> str:
    return _Simple.value


def _read_simple_other() -> str:
    return _Simple.other


_console: LoggingConsole = LoggingConsole(__name__)
_Simple.value = 'foo'
_Simple.other = 'bar'
_console.log(f'Simple.value is {_read_simple_value()}')
_console.log(f'Simple.other is {_read_simple_other()}')
