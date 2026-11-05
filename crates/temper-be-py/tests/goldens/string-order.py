from temper_core import (
    LoggingConsole as _LoggingConsole,
    boolean_to_string as _boolean_to_string,
    float_cmp as _float_cmp,
)

__all__ = ()


def _show(a: str, b: str, /) -> str:
    return f'{(a > b) - (a < b)} {_boolean_to_string(a < b)} {_boolean_to_string(a <= b)} {_boolean_to_string(a > b)} {_boolean_to_string(a >= b)}'


def _cmp(a: float, b: float, /) -> int:
    return _float_cmp(a, b)


_console: _LoggingConsole = _LoggingConsole(__name__)
_console.log(_show('｡', '😀'))
_console.log(_show('😀', '｡'))
_console.log(_show('', '𐀀'))
_console.log(_show('￿', '􏿿'))
_console.log(_show('a｡b', 'a😀'))
_console.log(_show('😀', '😁'))
_console.log(_show('😀', '😀'))
_console.log(_show('😀', '😀a'))
_console.log(_show('', '😀'))
_console.log(_show('', ''))
_console.log(_show('z', 'a'))
_console.log(_show('abc', 'abd'))
_console.log(f'{_cmp(2.5, 1.0)} {_cmp(1.0, 2.5)} {_cmp(1.5, 1.5)}')
