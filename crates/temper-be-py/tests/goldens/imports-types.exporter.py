__all__ = ('ExportedClass',)


class ExportedClass:
    __slots__ = ('_foo',)
    _foo: int

    def __init__(self, /, foo: int) -> None:
        self._foo = foo

    @property
    def foo(self, /) -> int:
        return self._foo
