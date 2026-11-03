from abc import ABCMeta, abstractmethod


class Stringable(metaclass=ABCMeta):
    __slots__ = ()

    @abstractmethod
    def to_string(self, /) -> str:
        ...


class Circle(Stringable):
    __slots__ = ('_radius',)
    _radius: int

    def __init__(self, /, radius: int) -> None:
        self._radius = radius

    @property
    def radius(self, /) -> int:
        return self._radius

    def to_string(self, /) -> str:
        return f'Circle of radius {self._radius}'

    def __str__(self, /) -> str:
        return self.to_string()


class Rectangle:
    __slots__ = ('_width', '_height')
    _width: int
    _height: int

    def __init__(self, /, width: int, height: int) -> None:
        self._width = width
        self._height = height

    @property
    def width(self, /) -> int:
        return self._width

    @property
    def height(self, /) -> int:
        return self._height
