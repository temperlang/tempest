from temper_core import (
    int_sub as _int_sub,
    string_count_between as _string_count_between,
)

__all__ = ()


def _undefined_plus_arg_inside() -> int:  # pyright: ignore[reportUnusedFunction]
    raise RuntimeError()


def _pad_left(string: str, min_size: int, pad: str, /) -> str:  # pyright: ignore[reportUnusedFunction]
    needed = _int_sub(min_size, _string_count_between(string, 0, len(string)))
    if needed <= 0:
        return string
    raise RuntimeError()


raise RuntimeError()
