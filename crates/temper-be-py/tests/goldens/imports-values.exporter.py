from collections.abc import Sequence
from temper_core import int_add, list_get


def _foo() -> Sequence[int]:
    return (int_add(21, 21), 42)


_a: int = list_get(_foo(), 0)
_a = _a
magic_number: int = _a
