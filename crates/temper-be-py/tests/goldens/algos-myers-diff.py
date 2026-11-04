from collections import deque
from collections.abc import Callable, MutableSequence, Sequence
from temper_core import (
    DenseBitVector,
    LoggingConsole,
    cast_by_type,
    dense_bit_vector_set,
    deque_remove_first,
    int_add,
    int_mul,
    int_sub,
    list_get,
    list_slice,
    string_split,
)
from typing import Generic, TypeVar

T = TypeVar('T')


class Change(Generic[T]):
    __slots__ = ('_type_', '_left_index', '_right_index', '_items')
    _type_: str
    _left_index: int
    _right_index: int
    _items: Sequence[T]

    def __init__(self, /, type_: str, left_index: int, right_index: int, items: Sequence[T]) -> None:
        self._type_ = type_
        self._left_index = left_index
        self._right_index = right_index
        self._items = items

    @property
    def type_(self, /) -> str:
        return self._type_

    @property
    def left_index(self, /) -> int:
        return self._left_index

    @property
    def right_index(self, /) -> int:
        return self._right_index

    @property
    def items(self, /) -> Sequence[T]:
        return self._items


class Patch(Generic[T]):
    __slots__ = ('_changes',)
    _changes: Sequence[Change[T]]

    def __init__(self, /, changes: Sequence[Change[T]]) -> None:
        self._changes = changes

    @property
    def changes(self, /) -> Sequence[Change[T]]:
        return self._changes


def format_patch(patch: Patch[str], /) -> str:
    formatted = ''
    changes = patch.changes
    n = len(changes)
    i = 0
    while i < n:
        change = list_get(changes, i)
        change_type = change.type_
        items = change.items
        prefix = '+' if change_type == change_type_addition else '-' if change_type == change_type_deletion else ' '
        n_items = len(items)
        j = 0
        while j < n_items:
            item = list_get(items, j)
            formatted = f'{formatted}{prefix}{item}\n'
            j = int_add(j, 1)
        i = int_add(i, 1)
    return formatted


class _DiffPath:
    __slots__ = ('_left_index', '_right_index', '_change_type', '_previous')
    _left_index: int
    _right_index: int
    _change_type: str
    _previous: '_DiffPath | None'

    def __init__(self, /, left_index: int, right_index: int, change_type: str, previous: '_DiffPath | None') -> None:
        self._left_index = left_index
        self._right_index = right_index
        self._change_type = change_type
        self._previous = previous

    @property
    def left_index(self, /) -> int:
        return self._left_index

    @property
    def right_index(self, /) -> int:
        return self._right_index

    @property
    def change_type(self, /) -> str:
        return self._change_type

    @property
    def previous(self, /) -> '_DiffPath | None':
        return self._previous


def diff(left: Sequence[T], right: Sequence[T], eq: Callable[[T, T], bool], /) -> Patch[T]:
    left_length = len(left)
    right_length = len(right)
    reached_row_length = int_add(left_length, 1)
    reached_col_length = int_add(right_length, 1)
    reached = DenseBitVector(int_mul(reached_row_length, reached_col_length))
    cost_zero_edges: deque[_DiffPath | None] = deque()
    cost_zero_edges.append(None)
    cost_one_edges: deque[_DiffPath] = deque()
    while True:
        diff_path: _DiffPath | None = deque_remove_first(cost_zero_edges) if not not cost_zero_edges else deque_remove_first(cost_one_edges) if not not cost_one_edges else None
        left_index = 0 if (subject := None if diff_path is None else diff_path.left_index) is None else subject
        right_index = 0 if (subject := None if diff_path is None else diff_path.right_index) is None else subject
        if left_index == left_length and right_index == right_length:
            path_elements: MutableSequence[_DiffPath] = []
            path_element: _DiffPath | None = diff_path
            while path_element is not None:  # pyright: ignore[reportUnnecessaryComparison]
                path: _DiffPath = cast_by_type(path_element, _DiffPath)
                path_elements.append(path)
                path_element = path.previous
            path_elements.reverse()
            left_patch_index = 0
            right_patch_index = 0
            changes: MutableSequence[Change[T]] = []
            n = len(path_elements)
            i = 0
            end: int
            while i < n:
                change_type = list_get(path_elements, i).change_type
                end = int_add(i, 1)
                while end < n and change_type == list_get(path_elements, end).change_type:
                    end = int_add(end, 1)
                n_items = int_sub(end, i)
                items: Sequence[T]
                n_left_items = 0
                n_right_items = 0
                if change_type == change_type_addition:
                    items = list_slice(right, right_patch_index, int_add(right_patch_index, n_items))
                    n_right_items = n_items
                else:
                    items = list_slice(left, left_patch_index, int_add(left_patch_index, n_items))
                    if change_type == change_type_unchanged:
                        n_right_items = n_items
                    n_left_items = n_items
                changes.append(Change(change_type, left_patch_index, right_patch_index, items))
                left_patch_index = int_add(left_patch_index, n_left_items)
                right_patch_index = int_add(right_patch_index, n_right_items)
                i = end
            return Patch(tuple(changes))
        if left_index < left_length:
            if right_index < right_length:
                same_reached_index = int_add(int_add(left_index, 1), int_mul(int_add(right_index, 1), reached_row_length))
                if not reached.get(same_reached_index) and eq(list_get(left, left_index), list_get(right, right_index)):
                    dense_bit_vector_set(reached, same_reached_index, True)
                    cost_zero_edges.append(_DiffPath(int_add(left_index, 1), int_add(right_index, 1), change_type_unchanged, diff_path))
            del_reached_index = int_add(int_add(left_index, 1), int_mul(right_index, reached_row_length))
            if not reached.get(del_reached_index):
                dense_bit_vector_set(reached, del_reached_index, True)
                cost_one_edges.append(_DiffPath(int_add(left_index, 1), right_index, change_type_deletion, diff_path))
        if right_index < right_length:
            add_reached_index = int_add(left_index, int_mul(int_add(right_index, 1), reached_row_length))
            if not reached.get(add_reached_index):
                dense_bit_vector_set(reached, add_reached_index, True)
                cost_one_edges.append(_DiffPath(left_index, int_add(right_index, 1), change_type_addition, diff_path))


_console: LoggingConsole = LoggingConsole(__name__)
change_type_addition: str = 'Addition'
change_type_deletion: str = 'Deletion'
change_type_unchanged: str = 'Unchanged'
_console.log(format_patch(diff(string_split('Hello,\nWorld!', '\n'), string_split('Hello,\nCosmos!', '\n'), lambda a, b: a == b)))
