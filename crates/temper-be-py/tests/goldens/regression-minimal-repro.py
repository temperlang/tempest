from collections.abc import Sequence
from temper_core import (
    LoggingConsole,
    int_negate,
    list_filter,
    list_join,
    list_map,
    string_split,
)

_console: LoggingConsole = LoggingConsole(__name__)
_ls: Sequence[int] = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9)
_console.log(f"All  {list_join(_ls, ', ', lambda x: str(x))}")
_console.log(f"Even {list_join(list_filter(_ls, lambda x: x & 1 == 0), ', ', lambda x: str(x))}")
_console.log(f"Odd  {list_join(list_filter(_ls, lambda x: x & 1 != 0), ', ', lambda x: str(x))}")
_console.log(f"Neg  {list_join(list_map(_ls, lambda x: int_negate(x)), ', ', lambda x: str(x))}")
_strs: Sequence[str] = string_split('1 ; 2 ; 3', ' ; ')
_console.log(f"Splt {list_join(_strs, ', ', lambda x: x)}")
