from temper_core import LoggingConsole as _LoggingConsole
from types_.exporter import ExportedClass as _ExportedClass

__all__ = ()

_console: _LoggingConsole = _LoggingConsole(__name__)
_x: _ExportedClass = _ExportedClass(42)
_console.log(f'foo = {_x.foo}')
