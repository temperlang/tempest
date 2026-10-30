from temper_core import LoggingConsole
from types_.exporter import ExportedClass

_console: LoggingConsole = LoggingConsole(__name__)
_x: ExportedClass = ExportedClass(42)
_console.log(f'foo = {_x.foo}')
