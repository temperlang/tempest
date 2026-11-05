# Actors and isolation: a brainstorm

Nothing here is decided (2026-11-05). It gathers thinking so the team can
settle the philosophy later.

## The model on the table

Two Temper PRs sketch it:

- [#530](https://github.com/temperlang/temper/pull/530) (closed, unmerged):
  `@actor` classes and a frontend check on what crosses their boundary.
  - One turn runs at a time per actor. An `await` is a turn boundary, so
    other calls may run while an `async` block waits.
  - Calls from inside the actor's own chain run inline, and a call cycle
    panics instead of deadlocking.
  - Only sendable values cross: deeply immutable values, other actors, and
    `Promise<T>` of a sendable `T`.
- [#507](https://github.com/temperlang/temper/pull/507) (open, experimental
  be-elixir): `@imu` objects become structs, `@actor` objects become BEAM
  processes, and async runs single-threaded inside each process. Real
  parallelism exists only between actors.

## Why it matters

- **Cheaper sharing inside an actor.** One turn at a time means mutable
  objects need no atomics or locks: `Rc` instead of `Arc` in Rust, and
  plain counts instead of atomic ones in C++.
- **BEAM and Swift as natural targets.** On the BEAM, actors are processes
  and Temper code would look native there. Being really useful to Elixir
  would be a coup: that ecosystem mostly rewrites libraries or wraps C. Swift
  6 has nearly this model built in: `actor` types, a compiler-checked
  `Sendable` protocol, and reentrancy at `await`, as in #530.

## The hard part: libraries, not apps

Temper makes libraries. The host app, not the library, decides which
threads touch which objects, so isolation holds only as far as each
backend enforces it.

| Enforced by | Backends | How |
|---|---|---|
| The compiler | Rust, Swift 6 | A non-sendable type is `!Send` or non-`Sendable`, and the host's compiler refuses to share it |
| The runtime | BEAM, JS | Processes copy messages; JS runs on one thread, and workers clone data |
| Nobody | Java, C#, C++, Lua, free-threaded Python | Shared memory: any thread can reach any object |

## A possible structure

1. **Native markers wherever they exist.** In Rust and Swift, a
   non-sendable Temper type becomes non-sendable natively, so the host's
   compiler enforces isolation for free. These are also the backends where
   non-atomic sharing is safe to turn on.
2. **Make the safe path the easy one.**
   - Public APIs mostly use immutable values and actor handles.
   - Mutable objects stay internal, or are clearly confined to one actor.
   - The frontend checks public surfaces, as #530 does, and could warn when
     a library exports mutable state.
3. **Cheap runtime checks where nothing else enforces it.** In Java, C#,
   C++, and Python, an opt-in or debug mode records each non-sendable
   object's owning thread and checks it on access, as UI toolkits do. This
   catches misuse in tests and costs nothing in production.
4. **Say it in the generated docs.** A non-sendable type's doc comment says
   "confine to one thread or actor", in that language's terms.
5. **Stay thread-safe by default where nothing enforces it.** Java, C#, and
   C++ keep today's thread-safe sharing unless a library opts in. Take the
   cheap path only where a compiler or runtime guarantees isolation.

## Keeping today's work neutral

- **Async in Rust:** the runtime adds no `Send` bounds. A `Promise<T>` is
  `Send` exactly when its contents are, through Rust's auto traits.
  - Today's `Arc` output gives `Send + 'static` promises that run on any
    executor.
  - `Rc` contents would still run on a single-threaded executor.
  - Rust's rule matches #530's: a promise is sendable when `T` is
    ([api-compat.md](api-compat.md)).
- **Backends stay pluggable:** a BEAM or Swift backend would declare its own
  strategies, like any other ([design.md](design.md)).

## Open questions

- Whether Temper adopts `@actor` and sendable checks at all, and whether
  `@imu` becomes more central to library design.
- What a non-actor object means across threads in the backends that don't
  enforce isolation: undefined, checked in debug mode, or always
  thread-safe.
- Whether isolates (groups of objects sharing one turn) are needed beyond
  single actors, and how a library declares them.
- How Python's subinterpreters and free threading fit in.
