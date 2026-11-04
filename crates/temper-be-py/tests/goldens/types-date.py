from datetime import date
from temper_core import (
    LoggingConsole,
    date_from_iso_string,
    date_to_string,
    date_today,
    int_add,
    list_get,
    years_between,
)

_console: LoggingConsole = LoggingConsole(__name__)
_d: date = date(2023, 6, 29)
_console.log(f'year    = {_d.year}\nmonth   = {_d.month}\nday     = {_d.day}\nweekday = {_d.isoweekday()}')
_console.log(f'ISO 8601: {date_to_string(_d)}')
_yonks_ago: date = date(12, 11, 10)
_console.log(f'ISO 8601 yonks: {date_to_string(_yonks_ago)}')
_console.log(f'yonksAgo.year = {_yonks_ago.year}')
_list = (1700, 1800, 1900, 2000, 2100, 2400)
_length = len(_list)
_i = 0
while _i < _length:
    _y = list_get(_list, _i)
    _i = int_add(_i, 1)
    _console.log(f'{_y}: Jan 1 weekday {date(_y, 1, 1).isoweekday()}, Mar 1 weekday {date(_y, 3, 1).isoweekday()}')
date_today()
_start: date = date(2010, 3, 21)
_first_years: int = years_between(_start, date(2012, 3, 20))
_console.log(f'First years between: {_first_years}')
_second_years: int = years_between(_start, date(2012, 3, 21))
_console.log(f'Second years between: {_second_years}')
_rdc: date = date_from_iso_string('1596-03-31')
_console.log(f'Happy birthday, René: day={_rdc.day}, month={_rdc.month}, year={_rdc.year}')
