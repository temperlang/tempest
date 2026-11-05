from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()


def _say(message: str, /) -> None:
    _console.log(message)


class _Something:
    __slots__ = ('_message',)
    _message: str

    def __init__(self, /, message: str) -> None:
        self._message = message

    def say_message(self, /) -> None:
        _say(self._message)


_console: _LoggingConsole = _LoggingConsole(__name__)
_something: _Something = _Something('Hello, World!')
_something.say_message()
