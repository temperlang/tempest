from temper_core import (
    LoggingConsole as _LoggingConsole,
    string_from_code_point as _string_from_code_point,
    string_step as _string_step,
)

__all__ = ()


class _StringBuilderOwner:
    __slots__ = ('_builder',)
    _builder: list[str]

    def __init__(self, /, builder: list[str]) -> None:
        self._builder = builder

    @property
    def builder(self, /) -> list[str]:
        return self._builder


_console: _LoggingConsole = _LoggingConsole(__name__)
_builder: list[str] = ['']
_builder.append('Hi ')
_builder.append(_string_from_code_point(127759))
_source: str = 'Wow!! Neato!'
_bang_index: int = _string_step(_source, 0, 3)
_space_index: int = _string_step(_source, _bang_index, 2)
_builder.append(_source[_bang_index : _space_index])
_built: str = ''.join(_builder)
_builder.append('\n')
_builder.append(_built)
_console.log(''.join(_builder))
try:
    _builder.append(_string_from_code_point(1114112))
except Exception:
    _console.log('High code blocked.')
try:
    _builder.append(_string_from_code_point(55296))
except Exception:
    _console.log('Surrogate blocked.')
_console.log(''.join(_builder))
_builder.clear()
_builder.append('Back to square zero.')
_console.log(''.join(_builder))
_owner: _StringBuilderOwner = _StringBuilderOwner([''])
_owner.builder.append('Hi from inside.')
_console.log(''.join(_owner.builder))
