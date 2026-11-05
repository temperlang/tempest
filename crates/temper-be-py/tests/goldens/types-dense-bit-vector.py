from temper_core import (
    DenseBitVector as _DenseBitVector,
    LoggingConsole as _LoggingConsole,
    boolean_to_string as _boolean_to_string,
    dense_bit_vector_set as _dense_bit_vector_set,
)

__all__ = ()

_console: _LoggingConsole = _LoggingConsole(__name__)
_v: _DenseBitVector = _DenseBitVector(16)
_console.log(f'v[2]={_boolean_to_string(_v.get(2))}')
_console.log(f'v[3]={_boolean_to_string(_v.get(3))}')
_console.log('v[3] <- true')
_dense_bit_vector_set(_v, 3, True)
_console.log(f'v[2]={_boolean_to_string(_v.get(2))}')
_console.log(f'v[3]={_boolean_to_string(_v.get(3))}')
_console.log('v[3] <- false')
_dense_bit_vector_set(_v, 3, False)
_console.log(f'v[2]={_boolean_to_string(_v.get(2))}')
_console.log(f'v[3]={_boolean_to_string(_v.get(3))}')
