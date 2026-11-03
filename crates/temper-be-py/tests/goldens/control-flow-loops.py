from temper_core import (
    Label,
    LabelPair,
    LoggingConsole,
    arith_int_mod,
    int_add,
)

_console: LoggingConsole = LoggingConsole(__name__)
_i = 0
while _i < 5:
    if _i == 1 or _i == 3:
        _i = int_add(_i, 1)
        continue
    _console.log(str(_i))
    _i = int_add(_i, 1)
_i2 = 0
while _i2 < 5:
    if _i2 == 3:
        break
    _console.log(str(_i2))
    _i2 = int_add(_i2, 1)
_n: int = 0
_i3 = 0
outer = Label()
while _i3 < 4:
    with outer:
        _str = f'row {_i3} ='
        _j = 0
        while True:
            _t = _n
            _n = int_add(_t, 1)
            _str = f'{_str} {_t}'
            if _i3 <= _j:
                _console.log(_str)
                _i3 = int_add(_i3, 1)
                outer.continue_()
            _j = int_add(_j, 1)
_prime: int = 1
_i4 = 0
outer_1 = Label()
while _i4 < 25:
    with outer_1:
        _j2 = 0
        outer_2 = Label()
        while _j2 < 25:
            with outer_2:
                _k = 0
                while _k < 25:
                    _prime = int_add(_prime, 1)
                    if _prime != 2 and arith_int_mod(_prime, 2) == 0:
                        _i4 = int_add(_i4, 1)
                        outer_1.continue_()
                    if _prime != 3 and arith_int_mod(_prime, 3) == 0:
                        _j2 = int_add(_j2, 1)
                        outer_2.continue_()
                    if _prime != 5 and arith_int_mod(_prime, 5) == 0:
                        _k = int_add(_k, 1)
                        continue
                    if _prime != 7 and arith_int_mod(_prime, 7) == 0:
                        _k = int_add(_k, 1)
                        continue
                    _console.log(str(_prime))
                    _k = int_add(_k, 1)
                _j2 = int_add(_j2, 1)
        _i4 = int_add(_i4, 1)
_val: int = 0
_i5 = 0
with LabelPair() as outer_3:
    while _i5 < 10:
        with outer_3.continuing:
            _str2 = f'row {_i5} ='
            _j3 = 0
            while True:
                _t2 = _val
                _val = int_add(_t2, 1)
                _str2 = f'{_str2} {_t2}'
                if _i5 == 4:
                    _console.log('break')
                    outer_3.break_()
                if _i5 <= _j3:
                    _console.log(_str2)
                    _i5 = int_add(_i5, 1)
                    outer_3.continue_()
                _j3 = int_add(_j3, 1)
