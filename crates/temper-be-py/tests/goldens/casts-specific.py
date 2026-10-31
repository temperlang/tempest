from abc import ABCMeta
from collections.abc import Sequence
from temper_core import LoggingConsole, cast_by_type, list_get


class _AlphaInterface(metaclass=ABCMeta):
    __slots__ = ()


class _BravoClass(_AlphaInterface):
    __slots__ = ()


class _CharlieClass(_AlphaInterface):
    __slots__ = ()


def _check_b(instance: _AlphaInterface, /) -> None:
    t_0: str
    try:
        cast_by_type(instance, _BravoClass)
        t_0 = 'yep'
    except Exception:
        t_0 = 'nope'
    _console.log(t_0)


def _check_if_b(instance: _AlphaInterface, /) -> None:
    _console.log('yep' if isinstance(instance, _BravoClass) else 'nope')


_console: LoggingConsole = LoggingConsole(__name__)
_check_b(_BravoClass())
_check_b(_CharlieClass())
_items: Sequence[_AlphaInterface] = (_BravoClass(), _CharlieClass())
_check_b(list_get(_items, 0))
_check_b(list_get(_items, 1))
_check_if_b(list_get(_items, 0))
_check_if_b(list_get(_items, 1))
