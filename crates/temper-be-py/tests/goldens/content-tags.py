from abc import ABCMeta as _ABCMeta, abstractmethod as _abstractmethod
from collections.abc import Callable as _Callable, Sequence as _Sequence
from temper_core import (
    LoggingConsole as _LoggingConsole,
    arith_int_mod as _arith_int_mod,
    int_add as _int_add,
    int_sub as _int_sub,
    list_get as _list_get,
    string_split as _string_split,
)
from typing import ClassVar as _ClassVar

__all__ = (
    'Mark',
    'Buffer',
    'Effect',
    'AppendParseEffect',
    'EventParseEffect',
    'After',
    'Propagator',
    'propagate_over',
    'Escaper',
    'TextEscaper',
    'QuoteEscaper',
    'Both',
    'Picker',
    'ContextualAutoescapingAccumulator',
    'Quoting',
)


class Mark:
    __slots__ = ('_label',)
    _label: str

    def __init__(self, /, label: str) -> None:
        self._label = label

    @property
    def label(self, /) -> str:
        return self._label


class Buffer:
    __slots__ = ('_sb',)
    _sb: list[str]

    def __init__(self, /) -> None:
        self._sb = ['']

    def append(self, x: str, /) -> None:
        self._sb.append(x)

    def mark(self, m: Mark, /) -> None:
        self._sb.append(f'{{{m.label}}}')

    def to_string(self, /) -> str:
        return ''.join(self._sb)

    def __str__(self, /) -> str:
        return self.to_string()


class Effect(metaclass=_ABCMeta):
    __slots__ = ()

    @_abstractmethod
    def enact_on(self, c: Buffer, /) -> None:
        ...


class AppendParseEffect(Effect):
    __slots__ = ('_text',)
    _text: str

    def __init__(self, /, text: str) -> None:
        self._text = text

    @property
    def text(self, /) -> str:
        return self._text

    def enact_on(self, c: Buffer, /) -> None:
        c.append(self._text)


class EventParseEffect(Effect):
    __slots__ = ('_event',)
    _event: Mark

    def __init__(self, /, event: Mark) -> None:
        self._event = event

    @property
    def event(self, /) -> Mark:
        return self._event

    def enact_on(self, c: Buffer, /) -> None:
        c.mark(self._event)


class After:
    __slots__ = ('_effects', '_state_after')
    _effects: _Sequence[Effect]
    _state_after: bool

    def __init__(self, /, effects: _Sequence[Effect], state_after: bool) -> None:
        self._effects = effects
        self._state_after = state_after

    @property
    def effects(self, /) -> _Sequence[Effect]:
        return self._effects

    @property
    def state_after(self, /) -> bool:
        return self._state_after


class Propagator:
    __slots__ = ('_quote',)
    _quote: str

    def __init__(self, /, quote: str) -> None:
        self._quote = quote

    @property
    def quote(self, /) -> str:
        return self._quote


def _ignore(problem: str, is_error: bool, /) -> None:
    if is_error:
        _console.log(problem)


def propagate_over(p: Propagator, before: bool, known: str | None, same: _Callable[[bool, bool], bool], callout: _Callable[[str, bool], None], /) -> After:
    if known is None:
        return After((), before)
    if known == '!':
        return After((EventParseEffect(Mark('bang')),), before)
    angles = len(_string_split(known, '<'))
    if angles > 1:
        callout('angle bracket in fixed text', False)
    if not same(before, before):
        callout('states differ from themselves', True)
    quotes = _int_sub(len(_string_split(known, p.quote)), 1)
    return After((AppendParseEffect(known),), not before if _arith_int_mod(quotes, 2) == 1 else before)


class Escaper(metaclass=_ABCMeta):
    __slots__ = ()

    @_abstractmethod
    def apply_string(self, x: str, /) -> str:
        ...


class TextEscaper(Escaper):
    __slots__ = ()
    instance: _ClassVar['TextEscaper']

    def apply_string(self, x: str, /) -> str:
        return f'[{x}]'


class QuoteEscaper(Escaper):
    __slots__ = ()
    instance: _ClassVar['QuoteEscaper']

    def apply_string(self, x: str, /) -> str:
        return f'({x})'


class Both(Escaper):
    __slots__ = ('_first', '_second')
    _first: Escaper
    _second: Escaper

    def __init__(self, /, first: Escaper, second: Escaper) -> None:
        self._first = first
        self._second = second

    @property
    def first(self, /) -> Escaper:
        return self._first

    @property
    def second(self, /) -> Escaper:
        return self._second

    def apply_string(self, x: str, /) -> str:
        return self._second.apply_string(self._first.apply_string(x))


class Picker:
    __slots__ = ('_strict',)
    _strict: bool

    def __init__(self, /, strict: bool) -> None:
        self._strict = strict

    @property
    def strict(self, /) -> bool:
        return self._strict

    def escaper_for(self, in_quote: bool, callout: _Callable[[str, bool], None], /) -> Escaper:
        if self._strict and in_quote:
            callout('a value in quotes', False)
        if in_quote:
            return Both(TextEscaper.instance, QuoteEscaper.instance)
        else:
            return TextEscaper.instance


class ContextualAutoescapingAccumulator(metaclass=_ABCMeta):
    __slots__ = ()

    @property
    @_abstractmethod
    def accumulated(self, /) -> str:
        ...


class Quoting(ContextualAutoescapingAccumulator):
    __slots__ = ('_in_quote', '_collector')
    _in_quote: bool
    _collector: Buffer

    def __init__(self, /) -> None:
        self._in_quote = False
        self._collector = Buffer()

    @staticmethod
    def new_collector() -> Buffer:
        return Buffer()

    @staticmethod
    def initial_state() -> bool:
        return False

    @staticmethod
    def propagator() -> Propagator:
        return Propagator("'")

    @staticmethod
    def picker() -> Picker:
        return Picker(False)

    @staticmethod
    def same_state_predicate() -> _Callable[[bool, bool], bool]:
        return lambda a, b: a == b

    @staticmethod
    def merge_states(a: bool, b: bool, callout: _Callable[[str, bool], None], /) -> bool:
        if a != b:
            callout('branches end in different contexts', True)
        return a

    @staticmethod
    def check_end_state(in_quote: bool, callout: _Callable[[str, bool], None], /) -> None:
        if in_quote:
            callout('ends inside quotes', True)

    @staticmethod
    def from_collector(c: Buffer, /) -> str:
        return c.to_string()

    @staticmethod
    def enact(m: Mark, c: Buffer, /) -> None:
        c.mark(m)

    def append_safe(self, x: str, /) -> None:
        after = propagate_over(Quoting.propagator(), self._in_quote, x, Quoting.same_state_predicate(), _ignore)
        self._in_quote = after.state_after
        list_ = after.effects
        length = len(list_)
        i = 0
        while i < length:
            e = _list_get(list_, i)
            i = _int_add(i, 1)
            e.enact_on(self._collector)

    def append(self, x: str, /) -> None:
        escaped = Quoting.picker().escaper_for(self._in_quote, _ignore).apply_string(x)
        self._collector.append(escaped)

    @property
    def accumulated(self, /) -> str:
        return self._collector.to_string()


def _greet(name: str, who: str, /) -> str:
    collector = Quoting.new_collector()
    collector.append("say 'hi ")
    collector.append(QuoteEscaper.instance.apply_string(TextEscaper.instance.apply_string(name)))
    collector.append("' to ")
    collector.append(TextEscaper.instance.apply_string(who))
    Quoting.enact(Mark('bang'), collector)
    return Quoting.from_collector(collector)


def _branch(c: bool, x: str, /) -> str:
    collector = Quoting.new_collector()
    collector.append(TextEscaper.instance.apply_string(x))
    if c:
        collector.append("''")
    collector.append(" '")
    collector.append(QuoteEscaper.instance.apply_string(TextEscaper.instance.apply_string(x)))
    collector.append("'")
    return Quoting.from_collector(collector)


def _each(xs: _Sequence[str], /) -> str:
    accumulator = Quoting()
    length = len(xs)
    i = 0
    while i < length:
        x = _list_get(xs, i)
        i = _int_add(i, 1)
        accumulator.append_safe("'")
        accumulator.append(x)
        accumulator.append_safe("' ")
        accumulator.append(x)
        accumulator.append_safe(';')
    return accumulator.accumulated


_console: _LoggingConsole = _LoggingConsole(__name__)
TextEscaper.instance = TextEscaper()
QuoteEscaper.instance = QuoteEscaper()
_console.log(_greet('Ann', 'Bob'))
_console.log(_branch(True, 'a'))
_console.log(_branch(False, 'b'))
_console.log(_each(('c', 'd')))
