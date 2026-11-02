from temper_core import (
    DenseBitVector,
    LoggingConsole,
    boolean_to_string,
    dense_bit_vector_set,
)

_console: LoggingConsole = LoggingConsole(__name__)
_v: DenseBitVector = DenseBitVector(16)
_console.log(f'v[2]={boolean_to_string(_v.get(2))}')
_console.log(f'v[3]={boolean_to_string(_v.get(3))}')
_console.log('v[3] <- true')
dense_bit_vector_set(_v, 3, True)
_console.log(f'v[2]={boolean_to_string(_v.get(2))}')
_console.log(f'v[3]={boolean_to_string(_v.get(3))}')
_console.log('v[3] <- false')
dense_bit_vector_set(_v, 3, False)
_console.log(f'v[2]={boolean_to_string(_v.get(2))}')
_console.log(f'v[3]={boolean_to_string(_v.get(3))}')
