from temper_core import (
    LoggingConsole,
    arith_int_mod,
    cast_by_type,
    int_add,
    int_div,
    string_to_int32,
)


def _more(x: int | None, /) -> int:
    raise RuntimeError()


def _something(x: str | None, /) -> str:
    if x is not None:
        return x
    else:
        raise RuntimeError()


def _report(x: str | None, /) -> None:
    try:
        message = _something(x)
    except Exception:
        message = 'null'
    try:
        t_0: int
        try:
            cast_by_type(x, str)
            t_0 = 1
        except Exception:
            t_0 = 0
        suffix = str(_more(t_0))
    except Exception:
        suffix = ''
    _console.log(message + suffix)


def _calc_rows(cols: int, total: int, /) -> int:
    _console.log(f'calcRows({cols}, {total})')
    if arith_int_mod(total, cols) != 0:
        raise RuntimeError()
    return int_div(total, cols)


def _count_below(s: str, /) -> int:
    i = 0
    while i < string_to_int32(s):
        i = int_add(i, 1)
    return i


def _branch(a: int, b: int, /) -> None:
    c = ('00' if b == 0 else '01' if b == 1 else '02' if b == 2 else '0x') if a == 0 else '1x' if a == 1 else '2x' if a == 2 else '3x' if a == 3 else 'xx'
    _console.log(f'branch({a}, {b}) -> {c}')


_console: LoggingConsole = LoggingConsole(__name__)
_report('hi')
_report(None)
_console.log(str(_calc_rows(3, 6)))
_t_0: int
try:
    _t_0 = _count_below('abc')
except Exception:
    _t_0 = -1
_console.log(str(_t_0))
_branch(0, 2)
_branch(2, 0)
