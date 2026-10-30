from abc import ABCMeta, abstractmethod
from temper_core import LoggingConsole


class _Greeter(metaclass=ABCMeta):
    __slots__ = ()

    @abstractmethod
    def greet(self, /) -> None:
        ...


class _HiGreeter(_Greeter):
    __slots__ = ()

    def greet(self, /) -> None:
        _console.log('Hi, virtually!')


_console: LoggingConsole = LoggingConsole(__name__)
_greeter: _Greeter = _HiGreeter()
_greeter.greet()
