from temper_core import LoggingConsole, int_add

_console: LoggingConsole = LoggingConsole(__name__)
_nci = 1
_console.log(f'nci = {_nci}')
_nci = 2
_console.log(f'nci = {_nci}')
_nc: int
_nc = 1
_console.log(f'nc = {_nc}')
_nc = 2
_console.log(f'nc = {_nc}')
_i = 0
while _i < 3:
    _ncl: int
    _ncl = _i
    _console.log(f'ncl = {_ncl}')
    _i = int_add(_i, 1)
_ci = 1
_console.log(f'ci = {_ci}')
_console.log('failed to set ci')
_console.log(f'ci = {_ci}')
_c: int
_c = 1
_console.log(f'c = {_c}')
_console.log('failed to set c')
_console.log(f'c = {_c}')
_i2 = 0
while _i2 < 3:
    _cl: int
    _cl = _i2
    _console.log(f'cl = {_cl}')
    _i2 = int_add(_i2, 1)
