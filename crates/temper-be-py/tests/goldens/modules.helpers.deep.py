__all__ = ('greet',)


def greet(who: str, /) -> str:
    return f'hello, {who} from deep'
