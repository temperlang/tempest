from modules import sum_ as _sum_
from modules.helpers import base as _base, twice as _twice
from renamed import Checked as _Checked, lambda_ as _lambda_, native as _native
from temper_core import LoggingConsole as _LoggingConsole, int_add as _int_add

__all__ = ('both',)


def both(n: int, /) -> int:
    return _int_add(_int_add(_sum_(n), _twice(n)), _base)


_console: _LoggingConsole = _LoggingConsole(__name__)
_console.log(f'both {both(2)}')
_checked: _Checked = _Checked(_lambda_(_native))
_console.log(f'renamed {_checked.delete()} {_checked.local} {_checked.end}')
