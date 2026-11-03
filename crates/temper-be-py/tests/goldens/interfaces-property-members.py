from abc import ABCMeta, abstractmethod
from collections.abc import Sequence
from temper_core import (
    LoggingConsole,
    Pair,
    boolean_to_string,
    int_add,
    list_get,
)
from typing import Generic, TypeVar

IT = TypeVar('IT', bound='I')
T = TypeVar('T', bound='I')


class I(metaclass=ABCMeta):
    __slots__ = ()

    @property
    @abstractmethod
    def x(self, /) -> str:
        ...


class J(I):
    __slots__ = ()


def _least_x(a: IT, b: IT, /) -> IT:
    if a.x < b.x:
        return a
    else:
        return b


class _C(J):
    __slots__ = ('_x',)
    _x: str

    def __init__(self, /, x: str) -> None:
        self._x = x

    @property
    def x(self, /) -> str:
        return self._x

    @x.setter
    def x(self, value: str, /) -> None:
        self._x = value


class _D(Generic[T]):
    __slots__ = ('_thing',)
    _thing: T

    def __init__(self, /, thing: T) -> None:
        self._thing = thing

    @property
    def thing(self, /) -> T:
        return self._thing


class SI(Generic[T], metaclass=ABCMeta):
    __slots__ = ()

    @property
    @abstractmethod
    def thing(self, /) -> T:
        ...


class _SSub(SI[T], Generic[T]):
    __slots__ = ('_thing',)
    _thing: T

    def __init__(self, /, thing: T) -> None:
        self._thing = thing

    @property
    def thing(self, /) -> T:
        return self._thing


class _AllAlone(I):  # pyright: ignore[reportUnusedClass]
    __slots__ = ()


class _High(metaclass=ABCMeta):
    __slots__ = ()

    @abstractmethod
    def to_string(self, /) -> str:
        ...

    @abstractmethod
    def eq(self, other: '_High', /) -> bool:
        ...


class _Low1(_High):
    __slots__ = ('_i',)
    _i: int

    def __init__(self, /, i: int) -> None:
        self._i = i

    @property
    def i(self, /) -> int:
        return self._i

    def to_string(self, /) -> str:
        return f'Low1({self._i})'

    def __str__(self, /) -> str:
        return self.to_string()

    def eq(self, other: _High, /) -> bool:
        if isinstance(other, _Low1):
            return self._i == other._i
        else:
            return False

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, _High) and self.eq(other)


class _Mid(_High):
    __slots__ = ()


class _Low2(_Mid):
    __slots__ = ('_s',)
    _s: str

    def __init__(self, /, s: str) -> None:
        self._s = s

    @property
    def s(self, /) -> str:
        return self._s

    def to_string(self, /) -> str:
        return f'Low2({self._s})'

    def __str__(self, /) -> str:
        return self.to_string()

    def eq(self, other: _High, /) -> bool:
        if isinstance(other, _Low2):
            return self._s == other._s
        else:
            return False

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, _High) and self.eq(other)


class _Low3(_Mid):
    __slots__ = ()

    def to_string(self, /) -> str:
        return 'Low3'

    def __str__(self, /) -> str:
        return self.to_string()

    def eq(self, other: _High, /) -> bool:
        return isinstance(other, _Low3)

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, _High) and self.eq(other)


_console: LoggingConsole = LoggingConsole(__name__)
_a: _C = _C('foo')
_b: _C = _C('bar')
_console.log(f'leastX of {_a.x} and {_b.x} is {_least_x(_a, _b).x}.')
_i: I = _a
_console.log(f'As an I, a.x is {_i.x}.')
_di: _D[I] = _D(_a)
_dj: _D[J] = _D(_a)
_console.log(f'Generically, x is {_di.thing.x}.')
_console.log(f'Genericallier, x is still {_dj.thing.x}.')
_si: SI[_C] = _SSub(_a)
_console.log(f'Sealed thing is {_si.thing.x}.')
_high1: _High = _Low1(11)
_high1_b: _High = _Low1(12)
_high1_c: _High = _Low1(12)
_high2: _High = _Low2('two')
_high2_b: _High = _Low2('to be')
_high2_c: _High = _Low2('to be')
_high3: _High = _Low3()
_high3_b: _High = _Low3()
_pairs: Sequence[Pair[_High, _High]] = (Pair(_high1, _high1_b), Pair(_high1_b, _high1_c), Pair(_high1, _high2), Pair(_high2, _high2_b), Pair(_high2_b, _high2_c), Pair(_high1, _high3), Pair(_high3, _high3_b))
_length = len(_pairs)
_i2 = 0
while _i2 < _length:
    _pair = list_get(_pairs, _i2)
    _i2 = int_add(_i2, 1)
    _console.log(f'{_pair.key.to_string()} == {_pair.value.to_string()}: {boolean_to_string(_pair.key.eq(_pair.value))}')
