from temper_core import LoggingConsole as _LoggingConsole, int_add as _int_add

__all__ = ()

_console: _LoggingConsole = _LoggingConsole(__name__)
_i: int = 0
while _i < 2:
    _x = _i
    _console.log(str(_x))
    _i = _int_add(_i, 1)
