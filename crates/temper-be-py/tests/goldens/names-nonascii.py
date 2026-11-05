from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()


def _say_hello(όνομα: str, /) -> None:
    _console.log(f'Hello, {όνομα}!')


_console: _LoggingConsole = _LoggingConsole(__name__)
_say_hello('World')
