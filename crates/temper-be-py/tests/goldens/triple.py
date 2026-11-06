from temper_core import LoggingConsole as _LoggingConsole, int_mul as _int_mul

__all__ = ()


def _triple(n: int, /) -> int:
    return _int_mul(n, 3)


_console: _LoggingConsole = _LoggingConsole(__name__)
_console.log(f'triple(14) = {_triple(14)}')
