from temper_core import (
    LoggingConsole,
    arith_int_mod,
    int64_add,
    int64_div,
    int64_mul,
    int64_negate,
    int64_sub,
    int64_to_int32,
    int64_to_int32_unsafe,
    int_add,
    int_div,
    int_mul,
    int_negate,
    int_sub,
    string_to_int32,
    string_to_int64,
)


def _check_parse32(string: str, /) -> None:
    try:
        converted = str(string_to_int32(string, 16))
    except Exception:
        converted = 'failed'
    _console.log(f'Parse {string} -> {converted}')


def _check_parse64(string: str, /) -> None:
    try:
        converted = str(string_to_int64(string, 16))
    except Exception:
        converted = 'failed'
    _console.log(f'Parse {string} -> {converted}')


_console: LoggingConsole = LoggingConsole(__name__)
_console.log('Int32')
_one: int = 1
_one = _one
_big: int = int_mul(2147483647, _one)
_small: int = int_mul(int_negate(-2147483648), _one)
_console.log(f'{_big} + 1 == {int_add(_big, 1)}')
_console.log(f'{_small} - 1 == {int_sub(_small, 1)}')
_console.log(f'{_big} * 4 == {int_mul(_big, 4)}')
_console.log(f'{_small} * 4 == {int_mul(_small, 4)}')
_console.log(f'{int_add(_small, 1)} * 4 == {int_mul(int_add(_small, 1), 4)}')
_console.log(f'{int_add(_small, 1)} * {_big} == {int_mul(int_add(_small, 1), _big)}')
_console.log(f'{_small} / -1 == {int_div(_small, -1)}')
_console.log(f'{_small} % -1 == {arith_int_mod(_small, -1)}')
_console.log(f'-({_small}) == {int_negate(_small)}')
_three: int = int_mul(3, _one)
_five: int = int_mul(5, _one)
_console.log(f'{_five} / {_three} == {int_div(_five, _three)}')
_console.log(f'{_five} / -{_three} == {int_div(_five, int_negate(_three))}')
_console.log(f'-{_five} / {_three} == {int_div(int_negate(_five), _three)}')
_console.log(f'-{_five} / -{_three} == {int_div(int_negate(_five), int_negate(_three))}')
_console.log(f'{_five} % {_three} == {arith_int_mod(_five, _three)}')
_console.log(f'{_five} % -{_three} == {arith_int_mod(_five, int_negate(_three))}')
_console.log(f'-{_five} % {_three} == {arith_int_mod(int_negate(_five), _three)}')
_console.log(f'-{_five} % -{_three} == {arith_int_mod(int_negate(_five), int_negate(_three))}')
_console.log('Int64')
_not_so_big: int = _big
_not_so_small: int = _small
_console.log(f'{str(_not_so_big)} + 1 == {str(int64_add(_not_so_big, 1))}')
_console.log(f'{str(_not_so_small)} - 1 == {str(int64_sub(_not_so_small, 1))}')
_big64: int = int64_mul(9223372036854775807, _one)
_small64: int = int64_mul(int64_negate(-9223372036854775808), _one)
_console.log(f'{str(_big64)} + 1 == {str(int64_add(_big64, 1))}')
_console.log(f'{str(_small64)} - 1 == {str(int64_sub(_small64, 1))}')
_console.log(f'{str(_small64)} / -1 == {str(int64_div(_small64, -1))}')
_console.log(f'{str(_small64)} % -1 == {str(arith_int_mod(_small64, -1))}')
_console.log(f'-({str(_small64)}) == {str(int64_negate(_small64))}')
_three64: int = _three
_five64: int = _five
_console.log(f'{str(_five64)} / {str(_three64)} == {str(int64_div(_five64, _three64))}')
_console.log(f'{str(_five64)} / -{str(_three64)} == {str(int64_div(_five64, int64_negate(_three64)))}')
_console.log(f'-{str(_five64)} / {str(_three64)} == {str(int64_div(int64_negate(_five64), _three64))}')
_console.log(f'-{str(_five64)} / -{str(_three64)} == {str(int64_div(int64_negate(_five64), int64_negate(_three64)))}')
_console.log(f'{str(_five64)} % {str(_three64)} == {str(arith_int_mod(_five64, _three64))}')
_console.log(f'{str(_five64)} % -{str(_three64)} == {str(arith_int_mod(_five64, int64_negate(_three64)))}')
_console.log(f'-{str(_five64)} % {str(_three64)} == {str(arith_int_mod(int64_negate(_five64), _three64))}')
_console.log(f'-{str(_five64)} % -{str(_three64)} == {str(arith_int_mod(int64_negate(_five64), int64_negate(_three64)))}')
_check_parse32('7FFFFFFF')
_check_parse32('-80000000')
_check_parse32('80000000')
_check_parse32('-80000001')
_console.log(f'Not so big: {int64_to_int32_unsafe(_not_so_big)}')
_t_0: str
try:
    _t_0 = str(int64_to_int32(_big64))
except Exception:
    _t_0 = 'no'
_console.log(f'Actually big: {_t_0}')
_check_parse64('7FFFFFFFFFFFFFFF')
_check_parse64('-8000000000000000')
_check_parse64('8000000000000000')
_check_parse64('-8000000000000001')
_console.log(f'{str(min(_small64, _big64))} < {str(max(_small64, _big64))}')
