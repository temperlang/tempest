from abc import ABCMeta as _ABCMeta, abstractmethod as _abstractmethod
from temper_core import (
    LoggingConsole as _LoggingConsole,
    boolean_to_string as _boolean_to_string,
    cast_by_type as _cast_by_type,
    int_add as _int_add,
    int_mul as _int_mul,
    int_negate as _int_negate,
    int_sub as _int_sub,
)
from typing import ClassVar as _ClassVar

__all__ = ('Shape', 'Rect', 'Square')


class Shape(metaclass=_ABCMeta):
    __slots__ = ()

    @property
    @_abstractmethod
    def area(self, /) -> int:
        ...

    def describe(self, /) -> str:
        return f'shape of area {self.area}'


class Rect(Shape):
    __slots__ = ('_w', '_h')
    _w: int
    _h: int
    unit: _ClassVar['Rect']

    def __init__(self, /, w: int, h: int) -> None:
        self._w = w
        self._h = h

    @property
    def w(self, /) -> int:
        return self._w

    @property
    def h(self, /) -> int:
        return self._h

    @property
    def area(self, /) -> int:
        return _int_mul(self._w, self._h)

    def to_string(self, /) -> str:
        return f'Rect({self._w}, {self._h})'

    def __str__(self, /) -> str:
        return self.to_string()

    def scaled(self, k: int, /) -> 'Rect':
        return Rect(_int_mul(self._w, k), _int_mul(self._h, k))


class Square(Shape):
    __slots__ = ('_side',)
    _side: int

    def __init__(self, /, side: int) -> None:
        self._side = side

    @property
    def area(self, /) -> int:
        return _int_mul(self._side, self._side)

    def describe(self, /) -> str:
        return f'square of side {self._side}'


class _Counter:
    __slots__ = ('_count', '_hidden')
    _count: int
    _hidden: int

    def __init__(self, /) -> None:
        self._count = 0
        self._hidden = 0

    @property
    def count(self, /) -> int:
        return self._count

    @count.setter
    def count(self, value: int, /) -> None:
        self._count = value

    @property
    def doubled(self, /) -> int:
        return _int_mul(self._count, 2)

    @doubled.setter
    def doubled(self, value: int, /) -> None:
        self._count = _int_sub(value, self._count)

    def bump(self, /) -> '_Counter':
        self._count = _int_add(self._count, 1)
        self._hidden = _int_add(self._hidden, 1)
        return self

    def same(self, other: '_Counter', /) -> bool:
        return self._hidden == other._hidden

    @staticmethod
    def zero() -> '_Counter':
        return _Counter()


class _Point:
    __slots__ = ('_x', '_y')
    _x: int
    _y: int

    def __init__(self, /, x: int, y: int) -> None:
        self._x = x
        self._y = y

    @property
    def x(self, /) -> int:
        return self._x

    @property
    def y(self, /) -> int:
        return self._y

    def eq(self, other: '_Point', /) -> bool:
        if self._x == other._x:
            return self._y == other._y
        else:
            return False

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, _Point) and self.eq(other)


class _Loose:
    __slots__ = ('_n',)
    _n: int

    def __init__(self, /, n: int) -> None:
        self._n = n

    @property
    def n(self, /) -> int:
        return self._n

    def eq(self, other: '_Loose', /) -> bool:
        if self._n == other._n:
            return True
        else:
            return self._n == _int_negate(other._n)

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, _Loose) and self.eq(other)


class _Holder:
    __slots__ = ('_loose', '_tag')
    _loose: _Loose
    _tag: str

    def __init__(self, /, loose: _Loose, tag: str) -> None:
        self._loose = loose
        self._tag = tag

    @property
    def loose(self, /) -> _Loose:
        return self._loose

    @property
    def tag(self, /) -> str:
        return self._tag

    def eq(self, other: '_Holder', /) -> bool:
        if self._loose == other._loose:
            return self._tag == other._tag
        else:
            return False

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, _Holder) and self.eq(other)


class _Version:
    __slots__ = ('_major', '_minor')
    _major: int
    _minor: int

    def __init__(self, /, major: int, minor: int) -> None:
        self._major = major
        self._minor = minor

    @property
    def major(self, /) -> int:
        return self._major

    @property
    def minor(self, /) -> int:
        return self._minor

    def compare_to(self, other: '_Version', /) -> int:
        if self._major != other._major:
            return self._major - other._major
        else:
            return self._minor - other._minor


def _shape(x: Shape, /) -> Shape:
    return x


_console: _LoggingConsole = _LoggingConsole(__name__)
Rect.unit = Rect(1, 1)
_r: Rect = Rect(2, 3)
_console.log(_r.to_string())
_console.log(str(_r.area))
_s: Shape = _shape(_r)
_console.log(_s.describe())
_q: Shape = _shape(Square(4))
_console.log(_q.describe())
_console.log(str(_q.area))
_console.log(str(_r.scaled(2).scaled(3).area))
_console.log(Rect.unit.scaled(5).to_string())
_c: _Counter = _Counter.zero()
_c.bump().bump()
_console.log(str(_c.count))
_c.doubled = 10
_console.log(str(_c.doubled))
_console.log(_boolean_to_string(_c.same(_Counter().bump().bump())))
_console.log(f'{_boolean_to_string(_Point(1, 2) == _Point(1, 2))} {_boolean_to_string(_Point(1, 2) != _Point(2, 1))}')
_console.log(_boolean_to_string(_Holder(_Loose(3), 'a') == _Holder(_Loose(-3), 'a')))
_console.log(f'{_boolean_to_string(_Version(1, 2).compare_to(_Version(1, 3)) < 0)} {_boolean_to_string(_Version(2, 0).compare_to(_Version(1, 9)) >= 0)}')
_console.log(f'{_boolean_to_string(isinstance(_q, Square))} {_boolean_to_string(isinstance(_s, Square))}')
_sq: Square = _cast_by_type(_q, Square)
_console.log(str(_sq.area))
if isinstance(_s, Rect):
    _console.log(str(_s.w))
