import modules.helpers as _m_modules_helpers
from modules.helpers import base, bump, twice
from modules.helpers.deep import greet
from temper_core import LoggingConsole, int_add


def sum_(n: int, /) -> int:
    return int_add(int_add(twice(n), base), _base)


_console: LoggingConsole = LoggingConsole(__name__)
_calls: int = _m_modules_helpers.calls
_base: int = 1
_console.log(greet('modules'))
_console.log(f'sum {sum_(3)}')
bump()
bump()
_console.log(f'calls {_calls}')
_console.log(f'calls now {bump()}')
