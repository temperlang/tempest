from temper_core import LoggingConsole


class _C:
    __slots__ = ()

    @property
    def place(self, /) -> str:
        return 'Hilo, HI'


_console: LoggingConsole = LoggingConsole(__name__)
_c: _C = _C()
_console.log(f'Hello, {_c.place}!')
