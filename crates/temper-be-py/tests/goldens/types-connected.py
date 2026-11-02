from temper_core import LoggingConsole


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


Counter = _connected.Counter
Doubler = _connected.Doubler
_console: LoggingConsole = LoggingConsole(__name__)
_counter: Counter = Counter()
_counter.add(2)
_counter.add(3)
_console.log(f'counter.total(): {_counter.total()}')
_console.log(f'new Doubler().twice(5): {Doubler().twice(5)}')
