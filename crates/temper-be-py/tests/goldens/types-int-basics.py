from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()

_console: _LoggingConsole = _LoggingConsole(__name__)
_console.log('1.min(3)')
_console.log(str(min(1, 3)))
_console.log('3.min(1)')
_console.log(str(min(3, 1)))
_console.log('1.max(3)')
_console.log(str(max(1, 3)))
_console.log('3.max(1)')
_console.log(str(max(3, 1)))
