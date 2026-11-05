from temper_core import LoggingConsole as _LoggingConsole

__all__ = ()


def _explicit(b: bool, /) -> str:
    if b:
        return 'true'
    return 'false'


def _half_and_half(b: bool, /) -> str:
    if b:
        return 'true'
    return 'false'


def _implicit(b: bool, /) -> str:
    if b:
        return 'true'
    else:
        return 'false'


_console: _LoggingConsole = _LoggingConsole(__name__)
_t: bool = True
_t = _t
_f: bool = not _t
_console.log(f'explicit    true  -> {_explicit(_t)}')
_console.log(f'explicit    false -> {_explicit(_f)}')
_console.log(f'halfAndHalf true  -> {_half_and_half(_t)}')
_console.log(f'halfAndHalf false -> {_half_and_half(_f)}')
_console.log(f'implicit    true  -> {_implicit(_t)}')
_console.log(f'implicit    false -> {_implicit(_f)}')
