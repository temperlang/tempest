# temper-rs

A from-scratch Rust implementation of Temper. Read `notes/README.md` first.

- Match Kotlin Temper's behavior, error messages and generated public APIs,
  not its implementation. Check against it with oracles in `compat/` and by
  running Kotlin Temper (including `temper repl`).
- Keep it simple: one tree, a settle loop instead of stages, errors as values
  that still reach code generation.
- Code style (`notes/code-style.md`): no explicit lifetime annotations where
  avoidable; ids into arenas instead of references; borrow on the stack, copy
  when cheap, `Rc`/`Arc` when sharing is natural.
- Dependencies only when worth their weight, widely used, and fast to
  compile. Ask before adding one.
- Generated code: no numbered names except on real conflicts; source maps for
  every backend.
- Scripts in Python, run with `uv run`, declaring `requires-python = ">=3.11"`
  (the version Temper's Python backend targets).
- Timing on this machine is noisy: compare variants back to back, repeated.
