from temper_core import LoggingConsole, int_add, int_mul


class _connected:
    class Counter:
        """Temper's connected Counter class, with the members Temper would generate."""

        def __init__(self) -> None:
            self._total = 0

        def add(self, i: int, /) -> None:
            self._total += i

        def total(self, /) -> int:
            return self._total

    class Doubler:
        """Temper's connected Doubler class, with the members Temper would generate."""

        def twice(self, i: int, /) -> int:
            return i * 2


class Counter:
    __slots__ = ('_count',)
    _count: int

    def __init__(self, /) -> None:
        self._count = 0

    def add(self, i: int, /) -> None:
        self._count = int_add(self._count, i)

    def total(self, /) -> int:
        return int_add(self._count, 1000)


class Doubler:
    __slots__ = ()

    def twice(self, i: int, /) -> int:
        return int_add(int_mul(i, 2), 1000)


_console: LoggingConsole = LoggingConsole(__name__)
_counter: Counter = Counter()
_counter.add(2)
_counter.add(3)
_console.log(f'counter.total(): {_counter.total()}')
_console.log(f'new Doubler().twice(5): {Doubler().twice(5)}')
