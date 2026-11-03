from collections.abc import Sequence as Sequence_
from temper_core import (
    LoggingConsole,
    boolean_to_string,
    int_add,
    list_get,
    string_count_between,
)
from temper_std.regex import (
    Capture,
    CodePoints,
    CodeRange,
    CodeSet,
    Group,
    Match,
    Or,
    Regex,
    RegexNode,
    Repeat,
    Sequence,
    begin,
    dot,
    entire,
    one_or_more,
    word_boundary,
)


def _check_group(group: Group, /) -> None:
    name = group.name
    value = group.value
    begin = group.begin
    end = group.end
    size = str(string_count_between(_text, begin, end))
    begin2 = str(string_count_between(_text, 0, begin))
    _console.log(f"match group '{name}': '{value}' at {begin2} size {size}")


_console: LoggingConsole = LoggingConsole(__name__)
_regex: Regex = Sequence((one_or_more(CodeSet((CodePoints('ab'),))), CodePoints('c'))).compiled()
_full: Regex = entire(_regex.data).compiled()
_strings: Sequence_[str] = ('abc', 'ac', 'bbabac', 'ab', 'dc', 'acc', 'cbc')
_length = len(_strings)
_i = 0
while _i < _length:
    _string = list_get(_strings, _i)
    _i = int_add(_i, 1)
    _console.log(f'found {_string}: {boolean_to_string(_regex.found(_string))}')
    _console.log(f'matches {_string}: {boolean_to_string(_full.found(_string))}')
_catcher: Regex = Sequence((Repeat(CodePoints('a'), 1, None, False), Capture('intro', Repeat(CodePoints('b'), 1, None, False)), CodePoints('c'), Repeat(Or((Capture('option1', CodePoints('d')), Capture('alphaOption2', CodePoints('e')))), 1, None, False))).compiled()
_text: str = '🌍aaabbceef'
_match: Match = _catcher.find(_text)
_check_group(_match.full)
_check_group(_match.groups['intro'])
_check_group(_match.groups['alphaOption2'])
_thing: Regex = Repeat(CodeSet((CodeRange(97, 122),), False), 1, None, False).compiled()
_multi_text: str = '---abc---xyz---'
_first: Match = _thing.find(_multi_text)
_second: Match = _thing.find(_multi_text, _first.full.end)
_console.log(f'Found both {_first.full.value} and {_second.full.value}.')
_more_text: str = '🌍aaabbcee🌎abcd🌏'


def fn(match_: Match, /) -> str:
    groups = match_.groups
    try:
        option1 = groups['option1'].value
    except Exception:
        option1 = ''
    try:
        alpha_option2 = groups['alphaOption2'].value
    except Exception:
        alpha_option2 = ''
    begin = string_count_between(_more_text, 0, match_.full.begin)
    return f'-{option1}{alpha_option2}{begin}-'


_replaced: str = _catcher.replace(_more_text, fn)
_console.log(f'replaced: {_replaced}')
_not_replaced: str = _catcher.replace('nope', lambda match_: 'hi')
_console.log(f'not replaced: {_not_replaced}')
_no_match: str
try:
    _no_match = _catcher.find('not here').full.value
except Exception:
    _no_match = 'sorry'
_console.log(f'No match says {_no_match}')
_or: RegexNode = Or((CodePoints('ab'), CodePoints('bc'))).compiled().data
_option1: str = 'ab'
_option2: str = 'bc'
_option_none: str = 'cb'
_console.log(f'or option 1: {boolean_to_string(_or.found(_option1))}')
_console.log(f'or option 2: {boolean_to_string(_or.found(_option2))}')
_console.log(f'or option none: {boolean_to_string(_or.found(_option_none))}')
_begin: Regex = Sequence((begin, CodePoints('a'))).compiled()
_begins_with_a_true: str = 'ab'
_begins_with_a_false: str = 'bab'
_console.log(f'begin true: {boolean_to_string(_begin.found(_begins_with_a_true))}')
_console.log(f'begin false: {boolean_to_string(_begin.found(_begins_with_a_false))}')
_negated_basic: Regex = entire(Sequence((CodePoints('a'), CodeSet((CodeRange(97, 99),), True))).compiled().data).compiled()
_nb_true: str = 'a🌍'
_nb_false: str = 'ac'
_console.log(f'negated basic true: {boolean_to_string(_negated_basic.found(_nb_true))}')
_console.log(f'negated basic false: {boolean_to_string(_negated_basic.found(_nb_false))}')
_simpler_lines: str = 'It was the best of times.\nIt was the worst of times.'
_console.log(Repeat(CodeSet((), True), 1, None, False).compiled().find(_simpler_lines).full.value)
_t_0: str
try:
    _t_0 = Repeat(CodeSet((), False), 1, None, False).compiled().find(_simpler_lines).full.value
except Exception:
    _t_0 = '-'
_console.log(_t_0)
_id_start: RegexNode = CodeSet((CodeRange(97, 122), CodeRange(65, 90), CodePoints('_')), False).compiled().data
_id_continue: Or = Or((_id_start, CodeSet((CodeRange(48, 57),), False).compiled().data))
_word: Regex = Sequence((word_boundary, _id_start, Repeat(_id_continue, 0, None), word_boundary)).compiled()
_expected: str = 'def_456'
_found: str = _word.find(f'123_abc {_expected} ghi_789').full.value
_console.log(f'Found id: {_found}')
_string_to_escape: str = '$(*^'
_regex_with_interpolated_string: Regex = Sequence((dot, Repeat(CodePoints(_string_to_escape), 1, None, False), dot)).compiled()
_string_to_escape_and_more: str = f'a{_string_to_escape}{_string_to_escape}b'
_escaped_match: Match = _regex_with_interpolated_string.find(_string_to_escape_and_more)
_console.log(_escaped_match.full.value)
_own_full_match: Match = Sequence((dot, Capture('full', dot))).compiled().find('ab')
_own_full: str = _own_full_match.groups['full'].value
_console.log(f'Own full: {_own_full} vs {_own_full_match.full.value}')
_combo_lines: str = 'It was the best\nof times.\r\n\rIt was the worst of times.'
_list = Or((CodePoints('\n'), Sequence((CodePoints('\r'), Repeat(CodePoints('\n'), 0, 1, False))))).compiled().split(_combo_lines)
_length2 = len(_list)
_i2 = 0
while _i2 < _length2:
    _line = list_get(_list, _i2)
    _i2 = int_add(_i2, 1)
    _console.log(f'{_line};')
