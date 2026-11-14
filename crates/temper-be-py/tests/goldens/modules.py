import modules.helpers as _m_modules_helpers
from modules.core import share as _share
from modules.helpers import (
    base as _base_,
    bump as _bump,
    int_doubled as _int_doubled,
    twice as _twice,
)
from modules.helpers.deep import greet as _greet
from modules.machine import describe as _describe
from temper_core import LoggingConsole as _LoggingConsole, int_add as _int_add

__all__ = ('sum_',)


class _Engine:
    __slots__ = ('_name',)
    _name: str

    def __init__(self, /, name: str) -> None:
        self._name = name

    @property
    def name(self, /) -> str:
        return self._name


def sum_(n: int, /) -> int:
    return _int_add(_int_add(_twice(n), _base_), _base)


_console: _LoggingConsole = _LoggingConsole(__name__)
_calls: int = _m_modules_helpers.calls
_base: int = 1
_console.log(_greet('modules'))
_console.log(f'sum {sum_(3)}')
_console.log(f'share {_share(7, 2)} {_int_doubled(21)}')
_console.log(f"{_Engine('own').name}, {_describe()}")
_bump()
_bump()
_console.log(f'calls {_calls}')
_console.log(f'calls now {_bump()}')
