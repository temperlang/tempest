from collections.abc import Callable as _Callable
from concurrent.futures import Future as _Future
from temper_core import (
    LoggingConsole as _LoggingConsole,
    adapt_generator_factory as _adapt_generator_factory,
    async_launch as _async_launch,
    break_promise as _break_promise,
    new_unbound_promise as _new_unbound_promise,
)
from typing import Any as _Any, Generator as _Generator

__all__ = ()

_console: _LoggingConsole = _LoggingConsole(__name__)
_b: _Future[int] = _new_unbound_promise()


@_adapt_generator_factory
def _fn(do_await: _Callable[[_Future[_Any]], _Any], /) -> _Generator[_Any, _Any, None]:
    try:
        v = yield do_await(_b)
    except Exception:
        v = -1
    _console.log(f'got {v}')


_async_launch(_fn)
_break_promise(_b)
