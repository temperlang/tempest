from temper_core import LoggingConsole

_console: LoggingConsole = LoggingConsole(__name__)
_empty_string: str = ''
_foo: str = 'foo'
_console.log('emptyString is empty' if not _empty_string else 'emptyString is not empty')
_console.log('foo is empty' if not _foo else 'foo is not empty')
