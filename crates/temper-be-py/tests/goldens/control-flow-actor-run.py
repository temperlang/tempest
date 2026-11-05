from collections.abc import Callable as _Callable
from concurrent.futures import Future as _Future
from temper_core import (
    LoggingConsole as _LoggingConsole,
    adapt_generator_factory as _adapt_generator_factory,
    int_add as _int_add,
)
from typing import Any as _Any, Generator as _Generator

__all__ = ()


def _f(body: _Callable[[], _Generator[_Any, _Any, None]], /) -> None:
    generator = body()
    _console.log('f0')
    next(generator, None)
    _console.log('f1')
    next(generator, None)
    _console.log('f2')
    next(generator, None)
    _console.log('f3')


_console: _LoggingConsole = _LoggingConsole(__name__)


@_adapt_generator_factory
def _fn(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
    i = 0
    while True:
        _console.log(f'generated {i}')
        i = _int_add(i, 1)
        yield


_f(_fn)
