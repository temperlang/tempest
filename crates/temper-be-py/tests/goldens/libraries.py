from collections.abc import Callable as _Callable, Sequence as _Sequence
from temper_core import (
    LoggingConsole as _LoggingConsole,
    boolean_to_string as _boolean_to_string,
    float64_to_string as _float64_to_string,
    float_cmp as _float_cmp,
    int_add as _int_add,
    int_div as _int_div,
    int_mul as _int_mul,
    list_get as _list_get,
    list_join as _list_join,
    list_map as _list_map,
    string_to_float64 as _string_to_float64,
)
from typing import ClassVar as _ClassVar

__all__ = ('Grid', 'parse_grid', 'Casing')


class Grid:
    __slots__ = ('_ncols', '_values')
    _ncols: int
    _values: _Sequence[float]

    def __init__(self, /, ncols: int, values: _Sequence[float]) -> None:
        self._ncols = ncols
        self._values = values

    @property
    def ncols(self, /) -> int:
        return self._ncols

    @property
    def values(self, /) -> _Sequence[float]:
        return self._values

    def get(self, i: int, j: int, /) -> float:
        return _list_get(self._values, _int_add(_int_mul(i, self._ncols), j))

    @staticmethod
    def parse(s: str, /) -> 'Grid | None':
        try:
            return parse_grid(s)
        except Exception:
            return None


def parse_grid(s: str, /) -> Grid:
    if s == '':
        raise RuntimeError()
    return Grid(1, (_string_to_float64(s),))


class Casing:
    __slots__ = ('_name',)
    _name: str
    first: _ClassVar['Casing']

    def __init__(self, /, name: str) -> None:
        self._name = name

    @property
    def name(self, /) -> str:
        return self._name

    def eq(self, other: 'Casing', /) -> bool:
        return self._name == other._name

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, Casing) and self.eq(other)

    def to_string(self, /) -> str:
        return self._name

    def __str__(self, /) -> str:
        return self.to_string()


def _rows(values: _Sequence[float], ncols: int, /) -> int:
    try:
        return _int_div(len(values), ncols)
    except Exception:
        raise BaseException('panic')


def _scaled(grid: Grid, /) -> Grid:
    if grid.ncols == 0:
        raise RuntimeError()
    return grid


def _cubes(grid: Grid, /) -> _Sequence[float]:
    try:
        more = _list_map(_scaled(grid).values, lambda x: x * x * x)
        return more
    except Exception:
        raise BaseException('panic')


def _each(values: _Sequence[float], f: _Callable[[float], None], /) -> None:
    i = 0
    while i < len(values):
        f(_list_get(values, i))
        i = _int_add(i, 1)


def _total(values: _Sequence[float], /) -> float:
    sum_ = 0.0
    def fn(x: float, /) -> None:
        nonlocal sum_
        try:
            part = x / 2.0
            sum_ = sum_ + part
        except Exception:
            raise BaseException('panic')
    _each(values, fn)
    return sum_


def _half(x: float, /) -> float:
    try:
        return x / 2.0
    except Exception:
        raise BaseException('panic')


def _cube(x: float, /) -> float:
    return x * 0.25


def _corner(grid: Grid, /) -> float:
    try:
        return grid.get(1, 1)
    except Exception:
        return -1.0


def _maybe(n: int, /) -> Grid | None:
    if n > 0:
        return Grid(n, (1.0,))
    else:
        return None


def _mapped(grid: Grid, /) -> _Sequence[float]:
    t_0: Grid
    try:
        t_0 = _scaled(grid)
    except Exception:
        raise BaseException('panic')
    t_1 = t_0.values
    def fn(x: float, /) -> float:
        if _float_cmp(x, 1.5) > 0:
            return x * 2.0
        else:
            return x
    return _list_map(t_1, fn)


def _text(n: int, /) -> str | None:
    if n > 0:
        return 'x'
    else:
        return None


def _count(n: int, /) -> int | None:
    if n > 0:
        return n
    else:
        return None


def _casing(n: int, /) -> Casing | None:
    if n > 0:
        return Casing.first
    else:
        return None


def _captured(s: str, /) -> str:
    found: Grid | None
    try:
        found = parse_grid(s)
    except Exception:
        found = None
    return _list_join(_list_map((1, 2), lambda x: found is None), ',', lambda b: _boolean_to_string(b))


def _bidi() -> str:
    return 'look\u202eout'


def _word(n: int, /) -> str:
    if n > 1:
        return 'b'
    else:
        return 'a'


_console: _LoggingConsole = _LoggingConsole(__name__)
Casing.first = Casing('first')
_grid: Grid = Grid(2, (1.0, 2.0, 3.0, 4.0))
_console.log(f'{_rows(_grid.values, 2)} {_float64_to_string(_half(3.0))} {_float64_to_string(_cube(8.0))}')
_console.log(_float64_to_string(_corner(_grid)))
_console.log(_float64_to_string(_total(_grid.values)))
_none: str | None = _text(0)
_some: int | None = _count(3)
_console.log(f"{'null' if (_part := _none) is None else _part} {'null' if (_part2 := _some) is None else str(_part2)} {'null' if (_part3 := None if (_subject := _maybe(0)) is None else _subject.ncols) is None else str(_part3)}")
_console.log(f"{_boolean_to_string(_maybe(1) is not None)} {_boolean_to_string(_maybe(0) is not None)} {_boolean_to_string(Grid.parse('') is None)}")
_console.log(Casing.first.to_string())
_console.log(_list_join(_mapped(_grid), ',', lambda x: _float64_to_string(x)))
_console.log(_list_join(_cubes(_grid), ',', lambda x: _float64_to_string(x)))
_t_1 = _boolean_to_string(False if (_left := _casing(1)) is None else _left == Casing.first)
_t_0 = _boolean_to_string(False if (_left2 := _casing(0)) is None else _left2 == Casing.first)
_left3: Casing | None = _casing(0)
_right: Casing | None = _casing(0)
_console.log(f'{_t_1} {_t_0} {_boolean_to_string(not (_right is None if _left3 is None else False if _right is None else _left3 == _right))}')
_console.log(f"{_captured('')} {_captured('2.5')}")
_console.log(_boolean_to_string(_bidi() != 'lookout'))
_t_2 = _word(1)
_t_3 = _word(2)
_t_9 = (_t_2 > _t_3) - (_t_2 < _t_3)
_t_4 = _word(2)
_t_5 = _word(1)
_t_8 = (_t_4 > _t_5) - (_t_4 < _t_5)
_t_6 = _word(2)
_t_7 = _word(3)
_console.log(f'{_t_9} {_t_8} {(_t_6 > _t_7) - (_t_6 < _t_7)}')
_named: str = _word(2)
_console.log(_boolean_to_string(_named == _word(3) and f'{_named}!' != _named))
