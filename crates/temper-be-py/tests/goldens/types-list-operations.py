from collections.abc import Callable, MutableSequence, Sequence
from temper_core import (
    LoggingConsole,
    boolean_to_string,
    float_cmp,
    int_add,
    int_negate,
    int_sub,
    list_builder_add,
    list_builder_add_all,
    list_builder_set,
    list_builder_sort,
    list_builder_splice,
    list_filter,
    list_get,
    list_get_or,
    list_join,
    list_map,
    list_slice,
    listed_sorted,
    listed_to_list,
    string_split,
)
from typing import Any, TypeVar, cast

T = TypeVar('T')


def _report_length(list_: Sequence[T], /) -> None:
    _console.log(f'Length: {len(list_)}')


def _report_split(string: str, sep: str, /) -> None:
    strings = string_split(string, sep)
    len_ = str(len(strings))
    _console.log(f"Split {list_join(strings, ', ', lambda x: x)} - {len_}")


def _something(callback: Callable[[Sequence[str]], None], /) -> None:
    callback(('Called', 'with', 'listed.'))


def _run_something() -> None:
    def fn(words: Sequence[str], /) -> None:
        _console.log(list_join(words, ' ', lambda word: word))
    _something(fn)


_console: LoggingConsole = LoggingConsole(__name__)
_report_length((4, 5, 6))
_ls: Sequence[int] = listed_sorted((5, 6, 7, 8, 9, 0, 1, 2, 3, 4), lambda i, j: int_sub(i, j))
_console.log(f"All  {list_join(_ls, ', ', lambda x: str(x))}")
_console.log(f"Even {list_join(list_filter(_ls, lambda x: x & 1 == 0), ', ', lambda x: str(x))}")
_console.log(f"Odd  {list_join(list_filter(_ls, lambda x: x & 1 != 0), ', ', lambda x: str(x))}")
_console.log(f"Neg  {list_join(list_map(_ls, lambda x: int_negate(x)), ', ', lambda x: str(x))}")
_listed: Sequence[int] = _ls
_console.log(f"Neg  {list_join(list_map(_listed, lambda x: int_negate(x)), ', ', lambda x: str(x))}")
_listed_list: Sequence[int] = listed_to_list(_listed)
_console.log(f'Listed list length is {len(_listed_list)}')
_console.log('Looping')
_n: int = len(_ls)
_i = 0
while _i < _n:
    _console.log(f'  {list_get(_ls, _i)}')
    _i = int_add(_i, 1)
_report_split('a ; b ; c', ' ; ')
_report_split('def', '')
_report_split(' g  h i', ' ')
_report_split(' ', ' ')
_report_split('', ' ')
_report_split('', '')
_report_split('🌍', '')
_t_0 = ((), ('bar',), ('bar', 'baz'), ('bar', 'baz', 'boo'), ('bar', 'baz', 'boo', 'far'), ('bar', 'baz', 'boo', 'far', 'faz'), ('far', 'faz', 'foo', 'bar', 'baz', 'boo'))


def fn11(list_: Sequence[str], /) -> MutableSequence[str]:
    builder = list(list_)
    list_builder_sort(cast('list[Any]', builder), lambda a, b: (a > b) - (a < b))
    return builder


_triangle_of_foo_bar: MutableSequence[MutableSequence[str]] = list(list_map(_t_0, fn11))
_j = 0
while _j < len(_triangle_of_foo_bar):
    _row = list_get(_triangle_of_foo_bar, _j)
    _console.log(f"Fwd  [{list_join(_row, ' ', lambda x: x)}]")
    _row.reverse()
    _j = int_add(_j, 1)
_triangle_of_foo_bar.reverse()
_j2 = 0
while _j2 < len(_triangle_of_foo_bar):
    _row2 = list_get(_triangle_of_foo_bar, _j2)
    _console.log(f"Back [{list_join(_row2, ' ', lambda x: x)}]")
    _j2 = int_add(_j2, 1)
_message_builder: MutableSequence[str] = []
_message_builder.append('hello')
list_builder_set(_message_builder, 0, 'hi')
list_builder_add_all(_message_builder, ('there', "y'all"))
list_builder_add(_message_builder, 'say', 0)
_copy_that: MutableSequence[str] = list(_message_builder)
list_builder_add_all(_message_builder, ('more', 'than'), 1)
list_builder_add_all(_copy_that, _message_builder)
_console.log(list_join(_message_builder, ' ', lambda x: x))
_console.log(list_join(listed_sorted(_copy_that, lambda a, b: (b > a) - (b < a)), ' ', lambda x: x))
_console.log(list_join(_copy_that, ' ', lambda x: x))
_typed: MutableSequence[str] = _message_builder
_old_last: str = _copy_that.pop()
_console.log(f"Removed {_old_last} so {list_join(_copy_that, ' ', lambda x: x)}")
_next_last: Sequence[str] = list_builder_splice(_copy_that, int_sub(len(_copy_that), 1), 1)
_old_words: Sequence[str] = list_builder_splice(_copy_that, 4, 3, _next_last)
list_builder_splice(_copy_that, None, 0, _old_words)
list_builder_splice(_copy_that, len(_copy_that), None, ('and', 'the', 'end'))
_console.log(list_join(_copy_that, ' ', lambda x: x))
list_builder_splice(_copy_that, None, None, ('gone',))
_console.log(list_join(_copy_that, ' ', lambda x: x))
_message_builder.clear()
_message_builder.append('Bye')
_console.log(list_join(_message_builder, ' ', lambda x: x))
_i2 = -2
while _i2 <= 10:
    _console.log(f"Slice({_i2}, {int_add(_i2, 2)}): [{list_join(list_slice(_ls, _i2, int_add(_i2, 2)), ', ', lambda x: str(x))}]")
    _i2 = int_add(_i2, 1)
_console.log(f'Outside fallback: {list_get_or(_ls, len(_ls), 123)}')


def _boolean_to_string(b: bool, /) -> str:
    return boolean_to_string(b)


_ls2 = (True, False)
_joined = list_join(_ls2, ', ', _boolean_to_string)
_inverted = list_map(_ls2, lambda b: not b)


def fn25(b: bool, /) -> str:
    if b:
        return 'not false'
    else:
        return 'not true'


_joined_and_inverted = list_join(_inverted, ', ', fn25)
_console.log(f'list = [{_joined}]')
_console.log(f'list (each inverted) = [{_joined_and_inverted}]')
_only_true = list_filter(_ls2, lambda b: b)
_only_false = list_filter(_ls2, lambda b: not b)
_console.log(f"list (== true only) = [{list_join(_only_true, ', ', _boolean_to_string)}]")
_console.log(f"list (== false only) = [{list_join(_only_false, ', ', _boolean_to_string)}]")
_sizes = (0.25, 3.0)


def fn28(f: float, /) -> str:
    if float_cmp(f, 1.0) < 0:
        return 'small'
    else:
        return 'big'


_labels = list_map(_sizes, fn28)
_console.log(f"sizes = [{list_join(_labels, ', ', lambda s: s)}]")
_run_something()
_a_list_builder: MutableSequence[str] = []
_a_list_builder.append('first')
_built_first = tuple(_a_list_builder)
_a_list_builder.append('second')
_built_second = listed_to_list(_a_list_builder)
_a_list_builder.append('third')
_console.log(f"builtFirst has {list_join(_built_first, ', ', lambda s: s)}")
_console.log(f"builtSecond has {list_join(_built_second, ', ', lambda s: s)}")
