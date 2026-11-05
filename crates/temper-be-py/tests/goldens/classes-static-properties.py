from abc import ABCMeta as _ABCMeta
from temper_core import LoggingConsole as _LoggingConsole
from typing import ClassVar as _ClassVar

__all__ = ()


class _Simple:
    __slots__ = ('_name',)
    _name: str
    simon: _ClassVar['_Simple']
    value: _ClassVar[str]
    other: _ClassVar[int]

    def __init__(self, /, name: str) -> None:
        self._name = name

    @property
    def name(self, /) -> str:
        return self._name


class _C:
    __slots__ = ()

    @staticmethod
    def say_hi() -> None:
        _console.log(_C._compose('C', "'hi.'"))

    @staticmethod
    def _compose(subject: str, message: str, /) -> str:
        return f'{subject} says {message}'


class _I(metaclass=_ABCMeta):
    __slots__ = ()

    @staticmethod
    def say_bye() -> None:
        _console.log("I says 'bye.'")


_console: _LoggingConsole = _LoggingConsole(__name__)
_Simple.simon = _Simple('foo')
_Simple.value = _Simple.simon.name
_Simple.other = 22
_console.log(f'Simple.simon.name is {_Simple.simon.name}')
_console.log(f'Simple.value is {_Simple.value}')
_console.log(f'Simple.other is {_Simple.other}')
_C.say_hi()
_I.say_bye()
