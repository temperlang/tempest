from defaults.helpers import Pad, wrap
from temper_core import LoggingConsole, int_add


def _next() -> int:
    global _calls
    _calls = int_add(_calls, 1)
    return _calls


def _range(start: int, end: int | None = None, step: int | None = None, /) -> str:
    if end is None:
        end = int_add(start, 10)
    if step is None:
        step = _next()
    return f'{start}..{end} by {step}'


def _tag(text: str, mark: str | None = None, /) -> str:
    if mark is None:
        return text
    else:
        return mark + text


def _named(name: str | None = None, /) -> str:
    if name is None:
        name = 'nobody'
    if name is None:  # pyright: ignore[reportUnnecessaryComparison]
        return 'null'
    else:
        return name


class _Greeter:
    __slots__ = ('_greeting', '_punct')
    _greeting: str
    _punct: str

    def __init__(self, /, greeting: str | None = None, punct: str | None = None) -> None:
        if greeting is None:
            greeting = 'Hello'
        if punct is None:
            punct = '!'
        self._greeting = greeting
        self._punct = punct

    @property
    def greeting(self, /) -> str:
        return self._greeting

    @property
    def punct(self, /) -> str:
        return self._punct

    def greet(self, name: str | None = None, times: int | None = None, /) -> str:
        if name is None:
            name = 'world'
        if times is None:
            times = 1
        out = ''
        i = 0
        while i < times:
            out = f'{out}{self._greeting}, {name}{self._punct}'
            i = int_add(i, 1)
        return out

    @staticmethod
    def shout(text: str, mark: str | None = None, /) -> str:
        if mark is None:
            mark = '!'
        return text + mark + mark


class _Counter:
    __slots__ = ('_count',)
    _count: int

    def __init__(self, /, start: int | None = None) -> None:
        if start is None:
            start = 10
        self._count = start

    @property
    def count(self, /) -> int:
        return self._count

    @count.setter
    def count(self, value: int, /) -> None:
        self._count = value

    def add(self, by: int | None = None, /) -> int:
        if by is None:
            by = 1
        self._count = int_add(self._count, by)
        return self._count


_console: LoggingConsole = LoggingConsole(__name__)
_calls: int = 0
_console.log(_range(1))
_console.log(_range(1, 5))
_console.log(_range(1, 5, 3))
_console.log(_range(2, None, 7))
_console.log(f'calls {_calls}')
_console.log(_tag('plain'))
_console.log(_tag('marked', '#'))
_console.log(_named())
_console.log(_named(None))
_console.log(_named('Ada'))
_console.log(_Greeter().greet())
_console.log(_Greeter('Hi').greet('you', 2))
_console.log(_Greeter(None, '?').greet(None, 2))
_console.log(_Greeter.shout('hey'))
_counter: _Counter = _Counter()
_counter.add()
_console.log(f'count {_counter.add(5)} {_Counter(1).add()}')
_console.log(wrap('a'))
_console.log(wrap('b', '<'))
_console.log(wrap('c', '<', '>'))
_pad: Pad = Pad()
_console.log(_pad.apply('x'))
_console.log(Pad(0, 3).apply('y', '-'))
