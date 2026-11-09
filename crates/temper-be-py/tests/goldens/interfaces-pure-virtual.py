from abc import ABCMeta as _ABCMeta, abstractmethod as _abstractmethod
from temper_core import LoggingConsole as _LoggingConsole, int_add as _int_add
from typing import Generic as _Generic, TypeVar as _TypeVar

__all__ = ()

_T = _TypeVar('_T')


class _Greeter(metaclass=_ABCMeta):
    __slots__ = ()

    @_abstractmethod
    def greet(self, /) -> None:
        ...


class _HiGreeter(_Greeter):
    __slots__ = ()

    def greet(self, /) -> None:
        _console.log('Hi, virtually!')


class _Wrapper(_Generic[_T], metaclass=_ABCMeta):
    __slots__ = ()

    @_abstractmethod
    def wrap(self, item: _T, /) -> _T:
        ...


class _Box:
    __slots__ = ('_label',)
    _label: str

    def __init__(self, /, label: str) -> None:
        self._label = label

    @property
    def label(self, /) -> str:
        return self._label


class _BoxWrapper(_Wrapper[_Box]):
    __slots__ = ()

    def wrap(self, item: _Box, /) -> _Box:
        item = _Box(f'[{item.label}]')
        return item


class _IntWrapper(_Wrapper[int]):
    __slots__ = ()

    def wrap(self, item: int, /) -> int:
        return _int_add(item, 1)


_console: _LoggingConsole = _LoggingConsole(__name__)
_greeter: _Greeter = _HiGreeter()
_greeter.greet()
_box_wrapper: _Wrapper[_Box] = _BoxWrapper()
_int_wrapper: _Wrapper[int] = _IntWrapper()
_console.log(_box_wrapper.wrap(_Box('box')).label)
_console.log(str(_int_wrapper.wrap(41)))
