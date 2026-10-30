from abc import ABCMeta
from temper_core import LoggingConsole, cast_by_type


class _Greeter(metaclass=ABCMeta):
    __slots__ = ()


class _HiGreeter(_Greeter):
    __slots__ = ()

    def greet(self, /) -> None:
        _console.log('Hi!')


_console: LoggingConsole = LoggingConsole(__name__)
_greeter: _Greeter = _HiGreeter()
cast_by_type(_greeter, _HiGreeter).greet()
