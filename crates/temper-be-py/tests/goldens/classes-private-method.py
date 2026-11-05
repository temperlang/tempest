from temper_core import (
    LoggingConsole as _LoggingConsole,
    int_add as _int_add,
    int_mul as _int_mul,
)

__all__ = ()


class _C:
    __slots__ = ()

    def _helper(self, x: int, /) -> int:
        return _int_add(x, 1)

    def method1(self, x: int, /) -> int:
        return self._helper(_int_mul(x, 2))

    def method2(self, x: int, /) -> int:
        return self._helper(_int_mul(x, 3))


_console: _LoggingConsole = _LoggingConsole(__name__)
_c: _C = _C()
_console.log(f'method1(10)={_c.method1(10)}')
_console.log(f'method2(10)={_c.method2(10)}')
