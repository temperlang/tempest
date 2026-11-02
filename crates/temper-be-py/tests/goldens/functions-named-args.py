from collections.abc import Callable
from temper_core import LoggingConsole, int_mul, int_sub
from typing import TypeVar

T = TypeVar('T')


class Greet:
    __slots__ = ()

    def __init__(self, /, message: str | None = None, repeat: bool | None = None) -> None:
        if message is None:
            message = 'Hi!'
        if repeat is None:
            repeat = False
        _console.log(message)
        if repeat:
            _console.log(message)


def _change_repeat() -> str:
    global _repeat
    _repeat = True
    return _message


def _change_message() -> bool:
    global _message
    _message = 'Namaste!'
    return _repeat


class _Person:
    __slots__ = ('_name', '_age')
    _name: str
    _age: int

    def __init__(self, /, name: str | None = None, age: int | None = None) -> None:
        if name is None:
            name = 'Alice'
        if age is None:
            age = 40
        self._name = name
        self._age = age

    @property
    def name(self, /) -> str:
        return self._name

    @property
    def age(self, /) -> int:
        return self._age

    def to_string(self, /) -> str:
        return f'{{ class: Person, name: {self._name}, age: {self._age} }}'

    def __str__(self, /) -> str:
        return self.to_string()

    def greet(self, greeting: str | None = None, /) -> None:
        if greeting is None:
            greeting = 'Hi'
        _console.log(f'{greeting}, {self._name}')


class _CalcOptions:
    __slots__ = ('_scale', '_origin')
    _scale: int
    _origin: int

    def __init__(self, /, scale: int | None = None, origin: int | None = None) -> None:
        if scale is None:
            scale = 1
        if origin is None:
            origin = 0
        self._scale = scale
        self._origin = origin

    @property
    def scale(self, /) -> int:
        return self._scale

    @property
    def origin(self, /) -> int:
        return self._origin


def _calc(n: int, transform: Callable[[int], T], options: _CalcOptions | None = None, /) -> T:
    if options is None:
        options = _CalcOptions()
    origin = options.origin
    scale = options.scale
    return transform(int_mul(int_sub(n, origin), scale))


_console: LoggingConsole = LoggingConsole(__name__)
Greet()
Greet('Bye!')
Greet('Hola!', True)
Greet('Salut!')
Greet(None, True)
Greet(None, True)
_arg = True
Greet('Ni hao!', _arg)
_message: str = 'Hi!'
_repeat: bool = False
_arg2 = _change_message()
_arg3 = _change_repeat()
Greet(_arg3, _arg2)
_console.log(_Person().to_string())
_console.log(_Person('Bob').to_string())
_console.log(_Person('Carrie', 50).to_string())
_Person('Dan').greet('Hallo')
_console.log(_calc(2, lambda n: str(n)))
_console.log(_calc(2, lambda n: str(n), _CalcOptions(3, 4)))
_console.log(_calc(2, lambda n: str(n), _CalcOptions(3)))
_console.log(_calc(2, lambda n: str(n), _CalcOptions(None, 4)))


def fn(n: int, /) -> tuple[()]:
    _console.log(str(n))
    return ()


_calc(2, fn, _CalcOptions(-2))
