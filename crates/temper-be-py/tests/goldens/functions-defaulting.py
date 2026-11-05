from temper_core import (
    LoggingConsole as _LoggingConsole,
    boolean_to_string as _boolean_to_string,
    int_add as _int_add,
    string_count_between as _string_count_between,
)

__all__ = ()


def _f() -> int:
    global _f_calls
    _f_calls = _int_add(_f_calls, 1)
    return 42


def _g(x: int | None = None, /) -> int:
    if x is None:
        x = _f()
    return x


def _no_inline() -> int | None:
    _console.log('noInline')
    return None


class _Thing:
    __slots__ = ()

    def say(self, a: str | None = None, b: str | None = None, /) -> None:
        if a is None:
            a = ''
        if b is None:
            b = ''
        _console.log(f'saying {a}{b}')

    @staticmethod
    def say_static(a: str, b: str | None = None, /) -> None:
        if b is None:
            b = ''
        _console.log(f'static {a}{b}')


class _StringHolder:
    __slots__ = ('_string',)
    _string: str

    def __init__(self, /, string: str) -> None:
        self._string = string

    @property
    def string(self, /) -> str:
        return self._string


def _maybe_empty(a: _StringHolder | None, /) -> str:
    global _inked
    _inked = _int_add(_inked, 1)
    subject: str | None = None if (subject2 := None if (subject3 := None if a is None else a.string) is None else not subject3) is None else _boolean_to_string(subject2)
    if subject is None:
        return 'null'
    else:
        return subject


def _maybe_length(a: str | None, /) -> int | None:
    global _inked
    _inked = _int_add(_inked, 1)
    if a is None:
        return None
    else:
        return _string_count_between(a, 0, len(a))


def _maybe_inc() -> int | None:
    global _inked
    if _inked == 10:
        return None
    else:
        _inked = _int_add(_inked, 1)
        return _inked


def _chain_more(a: str | None, /) -> str:
    return str(-1 if (subject := None if (subject2 := _maybe_length(a)) is None else max(subject2, 0)) is None else subject)


_console: _LoggingConsole = _LoggingConsole(__name__)
_f_calls: int = 0
_console.log(f'fCalls={_f_calls}')
_console.log(f'g(   )={_g()}')
_console.log(f'fCalls={_f_calls}')
_console.log(f'g(123)={_g(123)}')
_console.log(f'fCalls={_f_calls}')
_console.log(f'g(null)={_g(_no_inline())}')
_console.log(f'fCalls={_f_calls}')
_Thing().say('hi')
_Thing().say(None, 'there')
_Thing().say('by', 'e')
_Thing.say_static('yall')
_inked: int = 0
_console.log(f'maybeInc() -> {-1 if (_subject := _maybe_inc()) is None else _subject}')
_console.log(f'inked: {_inked}')
_console.log(f"lengthish \"ab\": {_chain_more('ab')}")
_console.log(f'lengthish null: {_chain_more(None)}')
_console.log(f'inked: {_inked}')
_console.log(f"maybeEmpty \"\": {_maybe_empty(_StringHolder(''))}")
_console.log(f'maybeEmpty null: {_maybe_empty(None)}')
_console.log(f'inked: {_inked}')
