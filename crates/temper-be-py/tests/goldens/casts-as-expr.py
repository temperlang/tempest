from abc import ABCMeta, abstractmethod
from temper_core import LoggingConsole, cast_by_type, cast_none


class _Apple(metaclass=ABCMeta):
    __slots__ = ()

    @property
    @abstractmethod
    def name(self, /) -> str:
        ...


class _Fuji(_Apple):
    __slots__ = ()

    @property
    def name(self, /) -> str:
        return 'Fuji'


class _Gala(_Apple):
    __slots__ = ()

    @property
    def name(self, /) -> str:
        return 'Gala'


_console: LoggingConsole = LoggingConsole(__name__)
_x: _Apple | None
_x = _Fuji()
_console.log(cast_none(_x).name)
_console.log(cast_by_type(_x, _Fuji).name)
_x = _Gala()
_t_0: str
try:
    _t_0 = cast_by_type(_x, _Fuji).name
except Exception:
    _t_0 = 'Cast failed!'
_console.log(_t_0)
_y: object | None = _x
_t_1: str
try:
    _t_1 = cast_by_type(_y, _Apple).name  # type: ignore[type-abstract]
except Exception:
    _t_1 = 'Cast from AnyValue failed!'
_console.log(_t_1)
_t_2: str
try:
    _t_2 = cast_by_type(_y, _Gala).name
except Exception:
    _t_2 = 'Cast from AnyValue failed!'
_console.log(_t_2)
_t_3: str
try:
    _t_3 = cast_by_type(_y, _Fuji).name
except Exception:
    _t_3 = 'Cast from AnyValue failed!'
_console.log(_t_3)
_x = None
_t_4: str
try:
    cast_none(_x)
    _t_4 = 'Tasty nothing.'
except Exception:
    _t_4 = 'Was null!'
_console.log(_t_4)
thing0: object = False
thing1: object = 1
thing2: object = 'two'
thing3: object = 3.14
