from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()


class _C:
    __slots__ = ('_backed',)
    _backed: int

    def __init__(self, /, backed: int | None = None) -> None:
        if backed is None:
            _console.log('initializing backed')
            backed = 1
        self._backed = backed

    @property
    def backed(self, /) -> int:
        return self._backed

    @property
    def computed(self, /) -> int:
        _console.log('computing computed')
        return 1


_console: _LoggingConsole = _LoggingConsole(__name__)
_console.log('about to create instance')
_c: _C = _C()
_console.log('about to get backed')
_console.log(f'c.backed={_c.backed}')
_console.log('about to get computed')
_console.log(f'c.computed={_c.computed}')
_console.log('done')
