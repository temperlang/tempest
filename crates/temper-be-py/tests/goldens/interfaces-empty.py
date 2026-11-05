from abc import ABCMeta as _ABCMeta
from temper_core import (
    LoggingConsole as _LoggingConsole,
    cast_by_type as _cast_by_type,
)

__all__ = ()


class _Greeter(metaclass=_ABCMeta):
    __slots__ = ()


class _HiGreeter(_Greeter):
    __slots__ = ()

    def greet(self, /) -> None:
        _console.log('Hi!')


_console: _LoggingConsole = _LoggingConsole(__name__)
_greeter: _Greeter = _HiGreeter()
_cast_by_type(_greeter, _HiGreeter).greet()
