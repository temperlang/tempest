from abc import ABCMeta, abstractmethod
from temper_core import LoggingConsole


class _Foo:
    __slots__ = ('_backing',)
    _backing: str

    def __init__(self, /, backing: str) -> None:
        self._backing = backing

    @property
    def bar(self, /) -> str:
        return self._backing

    @bar.setter
    def bar(self, value: str, /) -> None:
        self._backing = value


class _Bar:
    __slots__ = ('_backing',)
    _backing: str

    def __init__(self, /, backing: str) -> None:
        self._backing = backing

    @property
    def bar(self, /) -> str:
        return self._backing

    def _set_bar(self, value: str, /) -> None:
        self._backing = value

    def using_set(self, value: str, /) -> None:
        self._set_bar(value)


class Whatever(metaclass=ABCMeta):
    __slots__ = ()

    @property
    @abstractmethod
    def something(self, /) -> int:
        ...

    @something.setter
    @abstractmethod
    def something(self, blah: int, /) -> None:
        ...


class ConcreteWhatever(Whatever):
    __slots__ = ('_something',)
    _something: int

    def __init__(self, /, something: int) -> None:
        self._something = something

    @property
    def something(self, /) -> int:
        return self._something

    @something.setter
    def something(self, value: int, /) -> None:
        self._something = value


_console: LoggingConsole = LoggingConsole(__name__)
_console.log(_Foo('initial').bar)
_foo: _Foo = _Foo('initial')
_foo.bar = 'next'
_console.log(_foo.bar)
_bar: _Bar = _Bar('initial')
_bar.using_set('next')
_console.log(_bar.bar)
_what: Whatever = ConcreteWhatever(4)
_what.something = 5
_console.log(f'something set to {_what.something}')
