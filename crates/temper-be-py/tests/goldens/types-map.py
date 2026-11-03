from collections.abc import Mapping, Sequence
from temper_core import (
    LoggingConsole,
    Pair,
    boolean_to_string,
    int_div,
    list_get,
    list_join,
    list_map,
    map_builder_set,
    map_constructor,
    mapped_for_each,
    mapped_has,
    mapped_to_list,
    mapped_to_list_with,
    mapped_to_map,
)
from types import MappingProxyType
from typing import TypeVar

K = TypeVar('K')
V = TypeVar('V')


def _list_keys(mapped: Mapping[K, V], /) -> Sequence[K]:
    return list_map(mapped_to_list(mapped), lambda entry: entry.key)


def _message_defined(code: int, /) -> str:
    return boolean_to_string(mapped_has(_messages, code))


def _color_to_hex(color: Sequence[bool], /) -> str:
    def fn(b: bool, /) -> str:
        if b:
            return 'FF'
        else:
            return '00'
    c = list_join(color, '', fn)
    return f'#{c}'


def _print_map(name: str, map_: MappingProxyType[str, str], /) -> None:
    _console.log(f'--- {name} ---')
    def fn(key: str, value: str, /) -> None:
        _console.log(f'{key} = {value}')
    mapped_for_each(map_, fn)


def _print_map_builder(name: str, map_: dict[str, str], /) -> None:
    _console.log(f'--- {name} ---')
    def fn(key: str, value: str, /) -> None:
        _console.log(f'{key} = {value}')
    mapped_for_each(map_, fn)


_console: LoggingConsole = LoggingConsole(__name__)
_messages: MappingProxyType[int, str] = map_constructor((Pair(200, 'OK'), Pair(404, 'Not Found'), Pair(500, 'Internal Server Error')))
_message: str = _messages[200]
_console.log(f'Message: {_message}')
_console.log(f'Has 200: {_message_defined(200)}, 302: {_message_defined(302)}')
_t_0: str
try:
    _t_0 = _messages[302]
except Exception:
    _t_0 = 'Found'
_console.log(f'Fallback: {_t_0}')
_console.log(f"Codes: {list_join(_list_keys(_messages), ' ', lambda it: str(it))}")
_console.log(f"Again: {list_join(list(_messages.keys()), ' ', lambda it: str(it))}")
_console.log(f"Messages: {list_join(list(_messages.values()), ', ', lambda it: it)}")
_groups: Sequence[int] = mapped_to_list_with(_messages, lambda key, value: int_div(key, 100))
_console.log(f"Groups: {list_join(_groups, ' ', lambda it: str(it))}")
_builder2: dict[str, Sequence[str]] = {}
map_builder_set(_builder2, 'Honduras', ('El Salvador', 'Guatemala', 'Nicaragua'))
_console.log(f"From builder: {list_join(_builder2['Honduras'], ', ', lambda it: it)}")
_console.log(f'Key so far: {list_get(_list_keys(_builder2), 0)}')
map_builder_set(_builder2, 'El Salvador', ('Guatemala', 'Honduras'))
map_builder_set(_builder2, 'Nicaragua', ('Costa Rica', 'Honduras'))
map_builder_set(_builder2, 'Panama', ('Colombia', 'Costa Rica'))
try:
    _builder2.pop('Honduras')
except Exception:
    pass
_was = _builder2.pop('El Salvador')
_console.log(f"Was there: {list_join(_was, ', ', lambda it: it)}")
_t_1: str
try:
    _t_1 = list_get(_builder2.pop('El Salvador'), 0)
except Exception:
    _t_1 = 'Gone'
_console.log(_t_1)
_neighbors: MappingProxyType[str, Sequence[str]] = mapped_to_map(_builder2)
_console.log(f"From built: {list_join(_neighbors['Nicaragua'], ', ', lambda it: it)}")
_map_of_none: MappingProxyType[int, int] = map_constructor(())
_map_of_one: MappingProxyType[str, str] = map_constructor((Pair('jack', 'all trades'),))
_map_of_two: MappingProxyType[int, str] = map_constructor((Pair(1, 'one'), Pair(2, 'electric boogaloo')))
_console.log(f'mapOfNone: {len(_map_of_none)}')
_console.log(f'mapOfOne: {len(_map_of_one)}')
_console.log(f'mapOfTwo: {len(_map_of_two)}')
_eight_colors: dict[str, Sequence[bool]] = {}
map_builder_set(_eight_colors, 'black', (False, False, False))
map_builder_set(_eight_colors, 'red', (True, False, False))
map_builder_set(_eight_colors, 'green', (False, True, False))
map_builder_set(_eight_colors, 'blue', (False, False, True))
map_builder_set(_eight_colors, 'yellow', (True, True, False))
map_builder_set(_eight_colors, 'cyan', (False, True, True))
map_builder_set(_eight_colors, 'purple', (True, False, True))
map_builder_set(_eight_colors, 'white', (True, True, True))
_console.log(f'with 3 bits you can draw {len(_eight_colors)} colors.')


def fn(color: str, bits: Sequence[bool], /) -> None:
    found_by_get_or = _eight_colors.get(color, ())
    if len(found_by_get_or) != 3:
        _console.log(f'getOr("{color}") failed to find key')
    want = _color_to_hex(bits)
    got = _color_to_hex(found_by_get_or)
    if want != got:
        _console.log(f'getOr("{color}") found {got} but wanted {want}')


mapped_for_each(_eight_colors, fn)
_map: MappingProxyType[str, str] = map_constructor((Pair('key', 'value'), Pair('other key', 'other value')))
_print_map('map', _map)
_builder: dict[str, str] = dict(_map)
map_builder_set(_builder, 'added key', 'added value')
map_builder_set(_builder, 'key', 'replaced value')
_print_map_builder('builder', _builder)
_print_map('map (same as above)', _map)
_new_map: MappingProxyType[str, str] = mapped_to_map(_builder)
_print_map('builder to map', _new_map)
_builder_as_mapped: Mapping[str, str] = _builder
_new_map_from_mapped: MappingProxyType[str, str] = mapped_to_map(_builder_as_mapped)
_builder.pop('other key')
map_builder_set(_builder, 'key', 'double replaced value')
_print_map('builder to map (same as above)', _new_map)
_print_map('builder to map via mapped (same as above)', _new_map_from_mapped)
_builder.clear()
map_builder_set(_builder, 'something', 'alone')
_print_map_builder('recently cleared', _builder)
_may_or_may_not_copy_map: MappingProxyType[str, str] = mapped_to_map(_map)
_mapped: Mapping[str, str] = _map
_may_or_may_not_copy_mapped: MappingProxyType[str, str] = mapped_to_map(_mapped)
_made_by: dict[str, str] = {}
map_builder_set(_made_by, 'temper', 'temper systems')
_print_map_builder('made by', _made_by)
_made_by_and_jokes: dict[str, str] = dict(_made_by)
map_builder_set(_made_by_and_jokes, 'milk', 'milking a cow')
_print_map_builder('made by (and a joke about milk)', _made_by_and_jokes)
_print_map_builder('made by (not changed from a change to a copy)', _made_by)
