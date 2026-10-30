from abc import ABCMeta
from temper_core import LoggingConsole


class _I(metaclass=ABCMeta):
    __slots__ = ()

    @property
    def place(self, /) -> str:
        return 'Byron Bay'


class _C(_I):
    __slots__ = ()


_console: LoggingConsole = LoggingConsole(__name__)
_c: _C = _C()
_console.log(f'Goodbye, gotta run, {_c.place}!')
