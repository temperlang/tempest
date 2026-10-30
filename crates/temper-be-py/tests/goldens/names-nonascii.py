from temper_core import LoggingConsole


def _say_hello(όνομα: str, /) -> None:
    _console.log(f'Hello, {όνομα}!')


_console: LoggingConsole = LoggingConsole(__name__)
_say_hello('World')
