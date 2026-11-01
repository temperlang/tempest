from collections.abc import MutableSequence, Sequence
from temper_core import (
    LoggingConsole,
    float64_sign,
    float64_to_int_unsafe,
    float64_to_string,
    int_sub,
    list_builder_sort,
    list_get,
    list_join,
    listed_sorted,
)
from typing import Any, cast


class _Entry:
    __slots__ = ('_id_', '_group')
    _id_: int
    _group: int

    def __init__(self, /, id_: int, group: int) -> None:
        self._id_ = id_
        self._group = group

    @property
    def id_(self, /) -> int:
        return self._id_

    @property
    def group(self, /) -> int:
        return self._group

    def to_string(self, /) -> str:
        return f'{{ id: {self._id_}, group: {self._group} }}'

    def __str__(self, /) -> str:
        return self.to_string()


def _munge_ints(ints: Sequence[int], /) -> None:
    builder = list(ints)
    list_builder_sort(cast('list[Any]', builder), lambda a, b: int_sub(a, b))
    _console.log(list_join(listed_sorted(ints, lambda a, b: int_sub(a, b)), ', ', lambda a: str(a)))
    _console.log(list_join(builder, ', ', lambda a: str(a)))


def _munge_floats(floats: Sequence[float], /) -> None:
    builder = list(floats)
    def compare_floats(a: float, b: float, /) -> int:
        return float64_to_int_unsafe(float64_sign(a - b))
    list_builder_sort(cast('list[Any]', builder), compare_floats)
    _console.log(list_join(listed_sorted(floats, compare_floats), ', ', lambda a: float64_to_string(a)))
    _console.log(list_join(builder, ', ', lambda a: float64_to_string(a)))


_console: LoggingConsole = LoggingConsole(__name__)
_entries: MutableSequence[_Entry] = list((_Entry(0, 0), _Entry(1, 1), _Entry(2, 2), _Entry(3, 0), _Entry(4, 1), _Entry(5, 2), _Entry(6, 0), _Entry(7, 1), _Entry(8, 2)))
_sorted_entries: Sequence[_Entry] = listed_sorted(_entries, lambda a, b: int_sub(a.group, b.group))
_console.log(list_join(_sorted_entries, '\n', lambda a: a.to_string()))
_console.log(list_get(_entries, 1).to_string())
list_builder_sort(cast('list[Any]', _entries), lambda a, b: int_sub(a.group, b.group))
_console.log(list_join(_entries, '\n', lambda a: a.to_string()))
_munge_ints((2, 1))
_munge_floats((3.4, 1.2))
