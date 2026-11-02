from math import (
    acos,
    asin,
    atan,
    atan2,
    ceil,
    cos,
    cosh,
    e,
    exp,
    expm1,
    floor,
    fmod,
    log,
    log10,
    log1p,
    pi,
    sin,
    sinh,
    sqrt,
    tan,
    tanh,
)
from temper_core import (
    LoggingConsole,
    boolean_to_string,
    float64_max,
    float64_min,
    float64_near,
    float64_sign,
    float64_to_string,
    list_join,
)


def _near(label: str, actual: float, expected: float, /) -> None:
    subject = float64_near(actual, expected)
    result = '✅' if subject == True else f'❌ wanted {float64_to_string(expected)}, was {float64_to_string(actual)}'
    _console.log(f'{label}: {result}')


_console: LoggingConsole = LoggingConsole(__name__)
_one: float = 1.0
_one = _one
_two: float = _one + _one
_half: float = _one / _two
_near('half.abs()', abs(_half), _half)
_near('(-half).abs()', abs(-_half), _half)
_three: float = _one + _two
_near('half.acos()', acos(_half), pi / _three)
_pi: float = pi
_six: float = _two * _three
_near('half.asin()', asin(_half), _pi / _six)
_four: float = _two * _two
_near('(-one).atan()', atan(-_one), -_pi / _four)
_near('one.atan2(-one)', atan2(_one, -_one), _three * _pi / _four)
_near('(four / three).ceil()', ceil(_four / _three), _two)
_near('(-four / three).ceil()', ceil(-_four / _three), -_one)
_near('(pi / three).cos()', cos(_pi / _three), _half)
_e: float = e
_near('one.cosh()', cosh(_one), (_e + _one / _e) / _two)
_near('half.exp() == e.sqrt()', exp(_half), sqrt(_e))
_near('half.expm1()', expm1(_half), sqrt(_e) - _one)
_near('(four / three).floor()', floor(_four / _three), _one)
_near('(-four / three).floor()', floor(-_four / _three), -_two)
_near('e.log()', log(e), _one)
_five: float = _one + _four
_ten: float = _two * _five
_near('ten.log10()', log10(_ten), _one)
_near('(e - one).log1p()', log1p(_e - _one), _one)
_t_0 = (float('nan'), _one)


def fn(x: float, /) -> str:
    t_0 = (float('nan'), _two)
    def fn2(y: float, /) -> str:
        pair = list_join((float64_max(x, y), float64_min(x, y)), ', ', lambda z: float64_to_string(z))
        return f'({pair})'
    return list_join(t_0, ', ', fn2)


_max_min_combos: str = list_join(_t_0, ', ', fn)
_console.log(f'maxMinCombos: {_max_min_combos}')
_quarter: float = _half / _two
_near('2.25 % 2.0', fmod(_two + _quarter, _two), 0.25)
_near('2.25 % -2.0', fmod(_two + _quarter, -_two), 0.25)
_near('-2.25 % 2.0', fmod(-(_two + _quarter), _two), -0.25)
_near('-2.25 % -2.0', fmod(-(_two + _quarter), -_two), -0.25)
_near('pi.round()', round(_pi), _three)
_near('(-pi).round()', round(-_pi), -_three)
_near('pi.sign()', float64_sign(_pi), _one)
_near('(-pi).sign()', float64_sign(-_pi), -_one)
_zero: float = _one - _one
_console.log(f'zero.sign(): {float64_to_string(float64_sign(_zero))}')
_console.log(f'(-zero).sign(): {float64_to_string(float64_sign(-_zero))}')
_near('(pi / six).sin()', sin(_pi / _six), _half)
_near('one.sinh()', sinh(_one), (_e - _one / _e) / _two)
_near('(-pi / four).tan()', tan(-_pi / _four), -_one)
_near('(three * pi / four).tan()', tan(_three * _pi / _four), -_one)
_near('one.tanh()', tanh(_one), (_e ** 2.0 - 1.0) / (_e ** 2.0 + 1.0))
_console.log(f'one.near(one + 0.1): {boolean_to_string(float64_near(_one, _one + 0.1))}')
_console.log(f'one.near(one + 0.25, absTol = 0.25): {boolean_to_string(float64_near(_one, _one + 0.25, None, 0.25))}')
_console.log(f'one.near(one + 0.1 + 1e-9, absTol = 0.1): {boolean_to_string(float64_near(_one, _one + 0.1 + 1e-9, None, 0.1))}')
_console.log(f'one.near(one - 0.1, absTol = 0.11): {boolean_to_string(float64_near(_one, _one - 0.1, None, 0.11))}')
_console.log(f'ten.near(ten + 0.1, absTol = 0.011): {boolean_to_string(float64_near(_ten, _ten + 0.1, None, 0.011))}')
_console.log(f'ten.near(ten + 0.1, relTol = 0.011): {boolean_to_string(float64_near(_ten, _ten + 0.1, 0.011))}')
