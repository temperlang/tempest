from abc import ABCMeta, abstractmethod
from collections.abc import MutableSequence, Sequence
from temper_core import (
    LoggingConsole,
    Pair,
    boolean_to_string,
    int_add,
    list_builder_add,
    list_get,
    list_get_or,
    list_slice,
    map_builder_set,
    map_constructor,
    mapped_has,
)
from types import MappingProxyType
from typing import Generic, TypeVar

T = TypeVar('T')
T_2 = TypeVar('T_2', bound='Named')


class Holder(Generic[T], metaclass=ABCMeta):
    __slots__ = ()

    @property
    @abstractmethod
    def held(self, /) -> T:
        ...


class Box(Holder[T], Generic[T]):
    __slots__ = ('_value',)
    _value: T

    def __init__(self, /, value: T) -> None:
        self._value = value

    @property
    def value(self, /) -> T:
        return self._value

    @property
    def held(self, /) -> T:
        return self._value

    def pair_with(self, other: int, /) -> Pair[T, int]:
        return Pair(self._value, other)


class Named:
    __slots__ = ('_name',)
    _name: str

    def __init__(self, /, name: str) -> None:
        self._name = name

    @property
    def name(self, /) -> str:
        return self._name


class Shelf(Generic[T_2]):
    __slots__ = ('_items',)
    _items: Sequence[T_2]

    def __init__(self, /, items: Sequence[T_2]) -> None:
        self._items = items

    @property
    def items(self, /) -> Sequence[T_2]:
        return self._items

    def first_name(self, /) -> str | None:
        if not self._items:
            return None
        else:
            return list_get(self._items, 0).name


def first(items: Sequence[T], fallback: T, /) -> T:
    if not items:
        return fallback
    else:
        return list_get(items, 0)


def _held(h: Holder[T], /) -> T:
    return h.held


def _total(ns: Sequence[int], /) -> int:
    sum_ = 0
    i = 0
    while i < len(ns):
        sum_ = int_add(sum_, list_get(ns, i))
        i = int_add(i, 1)
    return sum_


_console: LoggingConsole = LoggingConsole(__name__)
_ns: Sequence[int] = (1, 2, 3)
_none: Sequence[int] = ()
_console.log(f'{_total(_ns)} {first(_ns, 0)} {first(_none, -1)}')
_console.log(first(('a', 'b'), 'z'))
_box: Box[str] = Box('boxed')
_console.log(_held(_box))
_pair: Pair[str, int] = _box.pair_with(7)
_console.log(f'{_pair.key} {_pair.value}')
_shelf: Shelf[Named] = Shelf((Named('ann'), Named('bo')))
_console.log('none' if (_subject := _shelf.first_name()) is None else _subject)
_no_names: Sequence[Named] = ()
_empty: Shelf[Named] = Shelf(_no_names)
_console.log('none' if (_subject2 := _empty.first_name()) is None else _subject2)
_builder: MutableSequence[str] = []
_builder.append('x')
_builder.append('y')
list_builder_add(_builder, 'w', 0)
_console.log(f'{len(_builder)} {list_get(_builder, 0)} {_builder.pop()}')
_built: Sequence[str] = tuple(_builder)
_console.log(f"{len(_built)} {list_get_or(_built, 5, '?')} {list_get(list_slice(_built, 1, 2), 0)}")
_m: MappingProxyType[str, int] = map_constructor((Pair('a', 1), Pair('b', 2)))
_t_0: int
try:
    _t_0 = _m['b']
except Exception:
    _t_0 = -1
_t_1: int
try:
    _t_1 = _m['c']
except Exception:
    _t_1 = -1
_console.log(f"{_t_0} {_t_1} {boolean_to_string(mapped_has(_m, 'a'))} {len(_m)}")
_mb: dict[str, int | None] = {}
map_builder_set(_mb, 'q', 5)
map_builder_set(_mb, 'n', None)
_console.log(f"{-2 if (_subject3 := _mb.get('q', 0)) is None else _subject3} {-2 if (_subject4 := _mb.get('n', 0)) is None else _subject4} {boolean_to_string(mapped_has(_mb, 'n'))}")
_t_2: int | None
try:
    _t_2 = _mb.pop('q')
except Exception:
    _t_2 = 0
_console.log('null' if (_subject5 := None if (_subject6 := _t_2) is None else str(_subject6)) is None else _subject5)
_console.log(str(len(list(_mb.keys()))))
