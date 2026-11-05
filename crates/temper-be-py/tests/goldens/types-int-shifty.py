from temper_core import (
    LoggingConsole as _LoggingConsole,
    arith_bit_shl32 as _arith_bit_shl32,
    arith_bit_shl64 as _arith_bit_shl64,
    arith_bit_ushr32 as _arith_bit_ushr32,
    arith_bit_ushr64 as _arith_bit_ushr64,
    arith_int_mod as _arith_int_mod,
    int_add as _int_add,
    int_to_string as _int_to_string,
    string_count_between as _string_count_between,
    string_get as _string_get,
    string_next as _string_next,
    string_step as _string_step,
)

__all__ = ()


def _insert_underscores(hex_string: str, /) -> str:
    sb = ['']
    idx = 0
    end = len(hex_string)
    if idx - end < 0:
        possible_sign = _string_get(hex_string, idx)
        if possible_sign == 43 or possible_sign == 45:
            idx = _string_next(hex_string, idx)
            sb.append(hex_string[0 : idx])
    n_first_group = _arith_int_mod(_string_count_between(hex_string, idx, end), 4)
    needs_underscore = False
    if n_first_group != 0:
        group_start = idx
        i = 0
        while i < n_first_group:
            idx = _string_next(hex_string, idx)
            i = _int_add(i, 1)
        sb.append(hex_string[group_start : idx])
        needs_underscore = True
    while idx - end < 0:
        group_start = idx
        idx = _string_step(hex_string, idx, 4)
        if needs_underscore:
            sb.append('_')
        sb.append(hex_string[group_start : idx])
        needs_underscore = True
    return ''.join(sb)


def _fmt32(n: int, /) -> str:
    return _insert_underscores(_int_to_string(n, 16))


def _fmt64(n: int, /) -> str:
    return _insert_underscores(_int_to_string(n, 16))


def _and32(a: int, b: int, /) -> None:
    c = a & b
    _console.log(f'0x:{_fmt32(a)} & {_fmt32(b)} == {_fmt32(c)}')


def _and64(a: int, b: int, /) -> None:
    c = a & b
    _console.log(f'0x:{_fmt64(a)} & {_fmt64(b)} == {_fmt64(c)}')


def _or32(a: int, b: int, /) -> None:
    c = a | b
    _console.log(f'0x:{_fmt32(a)} | {_fmt32(b)} == {_fmt32(c)}')


def _or64(a: int, b: int, /) -> None:
    c = a | b
    _console.log(f'0x:{_fmt64(a)} | {_fmt64(b)} == {_fmt64(c)}')


def _b_comp32(n: int, /) -> None:
    _console.log(f'0x:~{_fmt32(n)} == {_fmt32(~n)}')


def _b_comp64(n: int, /) -> None:
    _console.log(f'0x:~{_fmt64(n)} == {_fmt64(~n)}')


def _shl32(a: int, b: int, /) -> None:
    c = _arith_bit_shl32(a, b)
    _console.log(f'0x:{_fmt32(a)} << {b} == {_fmt32(c)}')


def _shl64(a: int, b: int, /) -> None:
    c = _arith_bit_shl64(a, b)
    _console.log(f'0x:{_fmt64(a)} << {b} == {_fmt64(c)}')


def _shr32(a: int, b: int, /) -> None:
    c = a >> (b & 31)
    _console.log(f'0x:{_fmt32(a)} >> {b} == {_fmt32(c)}')


def _shr64(a: int, b: int, /) -> None:
    c = a >> (b & 63)
    _console.log(f'0x:{_fmt64(a)} >> {b} == {_fmt64(c)}')


def _u_shr32(a: int, b: int, /) -> None:
    c = _arith_bit_ushr32(a, b)
    _console.log(f'0x:{_fmt32(a)} >>> {b} == {_fmt32(c)}')


def _u_shr64(a: int, b: int, /) -> None:
    c = _arith_bit_ushr64(a, b)
    _console.log(f'0x:{_fmt64(a)} >>> {b} == {_fmt64(c)}')


_console: _LoggingConsole = _LoggingConsole(__name__)
_console.log('and32')
_and32(0, 0)
_and32(3, 1)
_and32(2, 3)
_and32(-2, 7)
_and32(-809434545, 901070595)
_console.log('and64')
_and64(0, 0)
_and64(3, 1)
_and64(2, 3)
_and64(-2, 7)
_and64(297421889964898413, -3505189831424144594)
_console.log('or32')
_or32(0, 0)
_or32(0, 1)
_or32(2, 1)
_or32(2, 2)
_or32(3, 1)
_or32(2, 3)
_or32(-2, 7)
_or32(-809434545, 901070595)
_console.log('or64')
_or64(0, 0)
_or64(0, 1)
_or64(2, 1)
_or64(2, 2)
_or64(3, 1)
_or64(2, 3)
_or64(-2, 7)
_or64(297421889964898413, -3505189831424144594)
_console.log('bComp32')
_b_comp32(0)
_b_comp32(1)
_b_comp32(-1)
_b_comp32(2)
_b_comp32(-2)
_b_comp32(-809434545)
_b_comp32(901070595)
_console.log('bComp64')
_b_comp64(0)
_b_comp64(1)
_b_comp64(-1)
_b_comp64(2)
_b_comp64(-2)
_b_comp64(297421889964898413)
_b_comp64(-3505189831424144594)
_console.log('shl32')
_shl32(5, 0)
_shl32(5, 1)
_shl32(5, 2)
_shl32(5, 3)
_shl32(5, 4)
_shl32(5, 5)
_shl32(5, 20)
_shl32(5, 28)
_shl32(5, 29)
_shl32(5, 30)
_shl32(5, 31)
_shl32(5, 32)
_shl32(5, 33)
_shl32(5, -1)
_shl32(5, -30)
_shl32(-1, 0)
_shl32(-1, 1)
_shl32(-1, 2)
_console.log('shl64')
_shl64(5, 0)
_shl64(5, 1)
_shl64(5, 2)
_shl64(5, 3)
_shl64(5, 4)
_shl64(5, 5)
_shl64(5, 33)
_shl64(5, 62)
_shl64(5, 63)
_shl64(5, 64)
_shl64(5, -1)
_shl64(5, -60)
_shl64(-1, 0)
_shl64(-1, 1)
_shl64(-1, 2)
_console.log('shr32')
_shr32(128, 0)
_shr32(128, 1)
_shr32(128, 2)
_shr32(128, 3)
_shr32(128, 4)
_shr32(128, 5)
_shr32(128, 6)
_shr32(128, 7)
_shr32(128, 8)
_shr32(128, 9)
_shr32(128, 31)
_shr32(128, 32)
_shr32(128, 33)
_shr32(128, -1)
_shr32(128, -30)
_shr32(-128, 0)
_shr32(-128, 1)
_shr32(-128, 2)
_shr32(-128, 3)
_shr32(-128, 4)
_console.log('shr64')
_shr64(128, 0)
_shr64(128, 1)
_shr64(128, 2)
_shr64(128, 3)
_shr64(128, 4)
_shr64(128, 5)
_shr64(128, 6)
_shr64(128, 7)
_shr64(128, 8)
_shr64(128, 9)
_shr64(128, 32)
_shr64(128, 63)
_shr64(128, 64)
_shr64(128, 65)
_shr64(128, -1)
_shr64(128, -62)
_shr64(-128, 0)
_shr64(-128, 1)
_shr64(-128, 2)
_shr64(-128, 3)
_shr64(-128, 4)
_console.log('uShr32')
_u_shr32(128, 0)
_u_shr32(128, 1)
_u_shr32(128, 2)
_u_shr32(128, 3)
_u_shr32(128, 4)
_u_shr32(128, 5)
_u_shr32(128, 6)
_u_shr32(128, 7)
_u_shr32(128, 8)
_u_shr32(128, 9)
_u_shr32(128, 31)
_u_shr32(128, 32)
_u_shr32(128, 33)
_u_shr32(128, -1)
_u_shr32(128, -30)
_u_shr32(-128, 0)
_u_shr32(-128, 1)
_u_shr32(-128, 2)
_u_shr32(-128, 3)
_u_shr32(-128, 4)
_console.log('uShr64')
_u_shr64(128, 0)
_u_shr64(128, 1)
_u_shr64(128, 2)
_u_shr64(128, 3)
_u_shr64(128, 4)
_u_shr64(128, 5)
_u_shr64(128, 6)
_u_shr64(128, 7)
_u_shr64(128, 8)
_u_shr64(128, 9)
_u_shr64(128, 32)
_u_shr64(128, 63)
_u_shr64(128, 64)
_u_shr64(128, 65)
_u_shr64(128, -1)
_u_shr64(128, -62)
_u_shr64(-128, 0)
_u_shr64(-128, 1)
_u_shr64(-128, 2)
_u_shr64(-128, 3)
_u_shr64(-128, 4)
