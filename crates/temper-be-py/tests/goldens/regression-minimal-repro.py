from collections.abc import Sequence as _Sequence
from temper_core import (
    LoggingConsole as _LoggingConsole,
    int_negate as _int_negate,
    list_filter as _list_filter,
    list_join as _list_join,
    list_map as _list_map,
    string_split as _string_split,
)

__all__ = ()

_console: _LoggingConsole = _LoggingConsole(__name__)
_ls: _Sequence[int] = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9)
_console.log(f"All  {_list_join(_ls, ', ', lambda x: str(x))}")
_console.log(f"Even {_list_join(_list_filter(_ls, lambda x: x & 1 == 0), ', ', lambda x: str(x))}")
_console.log(f"Odd  {_list_join(_list_filter(_ls, lambda x: x & 1 != 0), ', ', lambda x: str(x))}")
_console.log(f"Neg  {_list_join(_list_map(_ls, lambda x: _int_negate(x)), ', ', lambda x: str(x))}")
_strs: _Sequence[str] = _string_split('1 ; 2 ; 3', ' ; ')
_console.log(f"Splt {_list_join(_strs, ', ', lambda x: x)}")
