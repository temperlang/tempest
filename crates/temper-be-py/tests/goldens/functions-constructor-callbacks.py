from collections.abc import Callable as _Callable
from temper_core import int_add as _int_add
from typing import Generic as _Generic, TypeVar as _TypeVar

__all__ = ('Hi', 'make_hi', 'new_int_hi', 'make_int_hi')

_T = _TypeVar('_T')


class Hi(_Generic[_T]):
    __slots__ = ('_ha',)
    _ha: _Callable[[_T], _T]

    def __init__(self, /, ha: _Callable[[_T], _T]) -> None:
        self._ha = ha

    @property
    def ha(self, /) -> _Callable[[_T], _T]:
        return self._ha


def make_hi(ha: _Callable[[_T], _T], /) -> Hi[_T]:
    return Hi(ha)


def new_int_hi(j: int, /) -> Hi[int]:
    return Hi(lambda i: _int_add(i, j))


def make_int_hi(j: int, /) -> Hi[int]:
    return make_hi(lambda i: _int_add(i, j))
