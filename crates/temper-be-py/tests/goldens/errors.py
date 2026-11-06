from temper_core import (
    LoggingConsole as _LoggingConsole,
    int_add as _int_add,
    int_mul as _int_mul,
)

__all__ = ('good',)


def good(x: int, /) -> int:
    return _int_add(x, 1)


def _mismatched(x: int, /) -> int:
    raise RuntimeError()


def _undeclared() -> int:
    raise RuntimeError()


def _misspelled(s: str, /) -> int:
    raise RuntimeError()


def _misspelled_method(s: str, /) -> int:
    raise RuntimeError()


def _wrong_arity() -> int:
    raise RuntimeError()


def _unknown_type(x: object, /) -> int:
    return 1


def _unclosed(x: int, /) -> int:  # pyright: ignore[reportUnusedFunction]
    return _int_mul(x, 2)


def _after_unclosed(x: int, /) -> int:
    return _int_mul(x, 3)


class _Box:
    __slots__ = ('_value',)
    _value: int

    def __init__(self, /, value: int) -> None:
        self._value = value

    @property
    def value(self, /) -> int:
        return self._value

    def get(self, /) -> int:
        return self._value

    def bad(self, /) -> str:
        raise RuntimeError()


_console: _LoggingConsole = _LoggingConsole(__name__)
_a: int
try:
    _a = _mismatched(1)
except Exception:
    _a = -1
_b: int
try:
    _b = _undeclared()
except Exception:
    _b = -1
_c: int
try:
    _c = _misspelled('a')
except Exception:
    _c = -1
_d: int
try:
    _d = _misspelled_method('a')
except Exception:
    _d = -1
_e: int
try:
    _e = _wrong_arity()
except Exception:
    _e = -1
_box: _Box = _Box(3)
_f: str
try:
    _f = _box.bad()
except Exception:
    _f = 'caught'
_console.log(f'{good(1)} {_a} {_b} {_c} {_d} {_e}')
_console.log(f'{_unknown_type(1)} {_after_unclosed(2)} {_box.get()} {_f}')
