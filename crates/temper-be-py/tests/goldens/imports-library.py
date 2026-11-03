from modules import sum_
from modules.helpers import base, twice
from renamed import Checked, lambda_, native
from temper_core import LoggingConsole, int_add


def both(n: int, /) -> int:
    return int_add(int_add(sum_(n), twice(n)), base)


_console: LoggingConsole = LoggingConsole(__name__)
_console.log(f'both {both(2)}')
_checked: Checked = Checked(lambda_(native))
_console.log(f'renamed {_checked.delete()} {_checked.local} {_checked.end}')
