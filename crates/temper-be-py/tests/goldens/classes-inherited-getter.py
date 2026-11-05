from abc import ABCMeta as _ABCMeta
from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()


class _I(metaclass=_ABCMeta):
    __slots__ = ()

    @property
    def place(self, /) -> str:
        return 'Byron Bay'


class _C(_I):
    __slots__ = ()


_console: _LoggingConsole = _LoggingConsole(__name__)
_c: _C = _C()
_console.log(f'Goodbye, gotta run, {_c.place}!')
