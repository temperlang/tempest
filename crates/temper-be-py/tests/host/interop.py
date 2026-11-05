"""Host code using the generated `async_turns` package the standard way,
for `functional.rs`'s `runs`: it prints a line per check, which the test
compares with what it expects."""

import asyncio
import threading

import temper_core

temper_core.init_simple_logging()
with temper_core.top_level_turn():
    import async_turns  # noqa: F401  (its top-level code logs)
temper_core.await_safe_to_exit()

from async_turns import later, ticks  # noqa: E402

# Outside the runner, blocking on a promise waits for it.
print("result", later(1).result(timeout=10))


# asyncio awaits a generated promise.
async def awaited() -> int:
    return await later(2)


print("asyncio", asyncio.run(awaited()))


# With the asyncio runner, steps run on the event loop's thread.
async def on_loop() -> None:
    threads: list[str] = []
    runner = temper_core.asyncio_runner()

    def run(step):
        threads.append(threading.current_thread().name)
        runner(step)

    temper_core.set_async_runner(run)
    try:
        value = await later(3)
    finally:
        temper_core.set_async_runner(None)
    print("asyncio runner", value, sorted(set(threads)))


asyncio.run(on_loop())

# Blocking inside a turn, before the promise settles, raises.
try:
    with temper_core.top_level_turn():
        later(4).result(timeout=10)
except RuntimeError as e:
    print("in a turn:", "would deadlock" in str(e))

# Python's own loop steps a generator.
seen: list[int] = []
for _ in ticks(3, seen.append):
    pass
print("for", seen)


# A generator stepped again after a failed step is done.
def fail_at_one(i: int) -> None:
    if i == 1:
        raise ValueError("boom")


g = ticks(3, fail_at_one)
next(g)
try:
    next(g)
except ValueError:
    print("failed step")
print("then", next(g, "done"))
