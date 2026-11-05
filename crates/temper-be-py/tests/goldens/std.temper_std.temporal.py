from collections.abc import Sequence as _Sequence
from temper_core import (
    arith_int_mod as _arith_int_mod,
    int_sub as _int_sub,
    int_to_string as _int_to_string,
    string_count_between as _string_count_between,
    string_get as _string_get,
    string_next as _string_next,
)

__all__ = ()


def _is_leap_year(year: int, /) -> bool:  # pyright: ignore[reportUnusedFunction]
    if _arith_int_mod(year, 4) == 0:
        if _arith_int_mod(year, 100) != 0:
            return True
        else:
            return _arith_int_mod(year, 400) == 0
    else:
        return False


def _pad_to(min_width: int, num: int, sb: list[str], /) -> None:  # pyright: ignore[reportUnusedFunction]
    decimal = _int_to_string(num, 10)
    decimal_index = 0
    decimal_end = len(decimal)
    if decimal_index - decimal_end < 0 and _string_get(decimal, decimal_index) == 45:
        sb.append('-')
        decimal_index = _string_next(decimal, decimal_index)
    n_needed = _int_sub(min_width, _string_count_between(decimal, decimal_index, decimal_end))
    while n_needed > 0:
        sb.append('0')
        n_needed = _int_sub(n_needed, 1)
    sb.append(decimal[decimal_index : decimal_end])


_days_in_month: _Sequence[int] = (0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
_day_of_week_lookup_table_leapy: _Sequence[int] = (0, 0, 3, 4, 0, 2, 5, 0, 3, 6, 1, 4, 6)
_day_of_week_lookup_table_not_leapy: _Sequence[int] = (0, 0, 3, 3, 6, 1, 4, 6, 2, 5, 0, 3, 5)
