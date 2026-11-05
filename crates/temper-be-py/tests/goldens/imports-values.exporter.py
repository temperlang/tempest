from collections.abc import Sequence as _Sequence
from temper_core import int_add as _int_add, list_get as _list_get

__all__ = ('magic_number',)


def _foo() -> _Sequence[int]:
    return (_int_add(21, 21), 42)


_a: int = _list_get(_foo(), 0)
_a = _a
magic_number: int = _a
