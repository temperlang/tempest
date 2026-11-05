from abc import ABCMeta as _ABCMeta, abstractmethod as _abstractmethod
from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()


class _Greeter(metaclass=_ABCMeta):
    __slots__ = ()

    @_abstractmethod
    def greet(self, /) -> None:
        ...


class _HiGreeter(_Greeter):
    __slots__ = ()

    def greet(self, /) -> None:
        _console.log('Hi, virtually!')


_console: _LoggingConsole = _LoggingConsole(__name__)
_greeter: _Greeter = _HiGreeter()
_greeter.greet()
