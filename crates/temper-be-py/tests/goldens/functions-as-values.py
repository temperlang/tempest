from collections.abc import Callable as _Callable
from temper_core import (
    LoggingConsole as _LoggingConsole,
    boolean_to_string as _boolean_to_string,
    int_sub as _int_sub,
)

__all__ = ()


def _hello(how: str, /) -> None:
    _console.log(f'Called {how}!')


def _call_it(f: _Callable[[str], None], /) -> None:
    f('by a function')


class _DelayedFunction:
    __slots__ = ('_f',)
    _f: _Callable[[str], None]

    def __init__(self, /, f: _Callable[[str], None]) -> None:
        self._f = f

    def call_it(self, /) -> None:
        self._f('from a method')


def _is_odd(i: int, /) -> bool:
    if i == 0:
        return False
    else:
        return _is_even(_int_sub(i, 1))


def _is_even(i: int, /) -> bool:
    if i == 0:
        return True
    else:
        return _is_odd(_int_sub(i, 1))


_console: _LoggingConsole = _LoggingConsole(__name__)
_hello('directly')
_call_it(_hello)
_DelayedFunction(_hello).call_it()
_console.log(f'Is 5 even? {_boolean_to_string(_is_even(5))}')
_console.log(f'Is 6 even? {_boolean_to_string(_is_even(6))}')
