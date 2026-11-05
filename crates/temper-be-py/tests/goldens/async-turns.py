from collections.abc import Callable as _Callable
from concurrent.futures import Future as _Future
from temper_core import (
    LoggingConsole as _LoggingConsole,
    adapt_generator_factory as _adapt_generator_factory,
    async_launch as _async_launch,
    complete_promise as _complete_promise,
    int_add as _int_add,
    int_mul as _int_mul,
    new_unbound_promise as _new_unbound_promise,
)
from typing import Any as _Any, Generator as _Generator

__all__ = ('later', 'ticks')


def later(n: int, /) -> _Future[int]:
    pb: _Future[int] = _new_unbound_promise()
    @_adapt_generator_factory
    def fn(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
        if False:
            yield
        _complete_promise(pb, _int_mul(n, 2))
    _async_launch(fn)
    return pb


def _generated(body: _Callable[[], _Generator[_Any, _Any, None]], /) -> _Generator[_Any, _Any, None]:
    return body()


def ticks(n: int, each: _Callable[[int], None], /) -> _Generator[_Any, _Any, None]:
    @_adapt_generator_factory
    def fn(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
        i = 0
        while i < n:
            each(i)
            yield
            i = _int_add(i, 1)
    return _generated(fn)


class _Counter:
    __slots__ = ('_n',)
    _n: int

    def __init__(self, /) -> None:
        self._n = 0

    @property
    def n(self, /) -> int:
        return self._n

    @n.setter
    def n(self, value: int, /) -> None:
        self._n = value

    def bump(self, tag: str, times: int, /) -> None:
        _console.log(f'{tag} start')
        i = 0
        while i < times:
            before = self._n
            self._n = _int_add(before, 1)
            i = _int_add(i, 1)
        _console.log(f'{tag} done n={self._n}')


_console: _LoggingConsole = _LoggingConsole(__name__)


def _fn(i: int, /) -> None:
    _console.log(f'tick {i}')


_t: _Generator[_Any, _Any, None] = ticks(2, _fn)
next(_t, None)
next(_t, None)
next(_t, None)
_c: _Counter = _Counter()
_gate_a: _Future[int] = _new_unbound_promise()
_gate_b: _Future[int] = _new_unbound_promise()


@_adapt_generator_factory
def _fn2(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
    _c.bump('A', 10000)
    try:
        k = yield do_await(_gate_a)
    except Exception:
        k = -1
    _c.bump(f'A{k}', 10000)


_async_launch(_fn2)


@_adapt_generator_factory
def _fn3(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
    _c.bump('B', 10000)
    try:
        k = yield do_await(_gate_b)
    except Exception:
        k = -1
    _c.bump(f'B{k}', 10000)


_async_launch(_fn3)


@_adapt_generator_factory
def _fn4(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
    if False:
        yield
    _console.log('opener: calling complete')
    _complete_promise(_gate_a, 1)
    _complete_promise(_gate_b, 2)
    _console.log('opener: complete returned')


_async_launch(_fn4)
