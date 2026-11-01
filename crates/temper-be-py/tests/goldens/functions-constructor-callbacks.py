from collections.abc import Callable
from temper_core import int_add
from typing import Generic, TypeVar

T = TypeVar('T')


class Hi(Generic[T]):
    __slots__ = ('_ha',)
    _ha: Callable[[T], T]

    def __init__(self, /, ha: Callable[[T], T]) -> None:
        self._ha = ha

    @property
    def ha(self, /) -> Callable[[T], T]:
        return self._ha


def make_hi(ha: Callable[[T], T], /) -> Hi[T]:
    return Hi(ha)


def new_int_hi(j: int, /) -> Hi[int]:
    return Hi(lambda i: int_add(i, j))


def make_int_hi(j: int, /) -> Hi[int]:
    return make_hi(lambda i: int_add(i, j))
