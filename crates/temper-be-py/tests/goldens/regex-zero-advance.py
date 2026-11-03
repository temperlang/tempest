from temper_core import LoggingConsole, string_next
from temper_std.regex import (
    CodePoints,
    Or,
    Regex,
    Repeat,
    Sequence,
    begin,
    space,
)


def bulletize_list(text: str, /) -> str:
    replaced = _comma_item_start_regex.replace(text, lambda match_: '\n- ')
    return replaced[string_next(replaced, 0) : len(replaced)]


_console: LoggingConsole = LoggingConsole(__name__)
_comma_item_start_regex: Regex = Sequence((Or((begin, CodePoints(','))), Repeat(space, 0, None, False))).compiled()
_console.log(bulletize_list('apples, bananas, cherries'))
