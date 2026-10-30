from temper_core import LoggingConsole, int_add

_console: LoggingConsole = LoggingConsole(__name__)
_i: int = 0
while _i < 2:
    _x = _i
    _console.log(str(_x))
    _i = int_add(_i, 1)
