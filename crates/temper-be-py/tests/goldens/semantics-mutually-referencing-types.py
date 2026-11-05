from temper_core import (
    LoggingConsole as _LoggingConsole,
    boolean_to_string as _boolean_to_string,
)

__all__ = ()


class _Bar:
    __slots__ = ('_x',)
    _x: '_Foo | None'

    def __init__(self, /, x: '_Foo | None') -> None:
        self._x = x

    @property
    def x(self, /) -> '_Foo | None':
        return self._x


class _Foo:
    __slots__ = ('_x',)
    _x: _Bar | None

    def __init__(self, /, x: _Bar | None) -> None:
        self._x = x

    @property
    def x(self, /) -> _Bar | None:
        return self._x


_console: _LoggingConsole = _LoggingConsole(__name__)
_empty_foo: _Foo = _Foo(None)
_empty_bar: _Bar = _Bar(None)
_non_empty_foo: _Foo = _Foo(_empty_bar)
_non_empty_bar: _Bar = _Bar(_empty_foo)
_stuffed_bar: _Bar = _Bar(_non_empty_foo)
_console.log(f'emptyFoo.x has Foo -> {_boolean_to_string(_empty_foo.x is not None)}')
_console.log(f'nonEmptyFoo.x has Foo -> {_boolean_to_string(_non_empty_foo.x is not None)}')
