from temper_core import (
    NetResponse as _NetResponse,
    TemperFuture as _TemperFuture,
    std_net_send as _std_net_send,
)

__all__ = ('NetRequest',)


class NetRequest:
    __slots__ = ('_url', '_method', '_body_content', '_body_mime_type')
    _url: str
    _method: str
    _body_content: str | None
    _body_mime_type: str | None

    def __init__(self, /, url: str) -> None:
        self._url = url
        self._method = 'GET'
        self._body_content = None
        self._body_mime_type = None

    def post(self, content: str, mime_type: str, /) -> None:
        self._method = 'POST'
        self._body_content = content
        self._body_mime_type = mime_type

    def send(self, /) -> _TemperFuture[_NetResponse]:
        return _std_net_send(self._url, self._method, self._body_content, self._body_mime_type)
