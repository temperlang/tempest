from temper_core import LoggingConsole, boolean_to_string, int_add, int_sub


class _Ping:
    __slots__ = ('_peer', '_n', '_most')
    _peer: '_Pong'
    _n: int
    _most: int

    def __init__(self, /, peer: '_Pong') -> None:
        self._peer = peer
        self._n = 0
        self._most = 0

    @property
    def peer(self, /) -> '_Pong':
        return self._peer

    @property
    def n(self, /) -> int:
        return self._n

    @n.setter
    def n(self, value: int, /) -> None:
        self._n = value

    @property
    def most(self, /) -> int:
        return self._most

    @most.setter
    def most(self, value: int, /) -> None:
        self._most = value

    def ping(self, /) -> int:
        self._peer.pong(self)
        return self._n

    def pass_(self, /) -> int:
        return self._peer.bump(self, self._n)

    def record(self, /) -> bool:
        self._n = int_add(self._n, 1)
        if self._n > self._most:
            self._most = self._n
        return self._n > int_sub(self._most, 1)

    def around(self, /) -> int:
        return int_add(int_add(self._n, self._peer.bump(self, 10)), self._n)


class _Pong:
    __slots__ = ()

    def pong(self, a: _Ping, /) -> None:
        o = a
        o.n = int_add(o.n, 1)

    def bump(self, a: _Ping, by: int, /) -> int:
        o = a
        o.n = int_add(o.n, by)
        return a.n


_console: LoggingConsole = LoggingConsole(__name__)
_p: _Ping = _Ping(_Pong())
_console.log(f'ping={_p.ping()}')
_console.log(f'pass={_p.pass_()}')
_console.log(f'record={boolean_to_string(_p.record())} most={_p.most}')
_console.log(f'around={_p.around()} n={_p.n}')
