# Code style and dependencies

Part of the [rewrite plan](README.md). The goal is code that people can read
and change easily, so these rules favor clarity over cleverness.

## Ownership and lifetimes

- **Avoid explicit lifetime annotations.** If a design needs `'a` on a struct
  or a function signature, look for a simpler one first.
- **Use ids instead of references across structures.** Tree nodes, symbols,
  types and modules are `u32`-based ids into arrays, so structures hold
  `Copy` ids, not borrowed references, and need no lifetimes.
- **Borrow on the stack.** Functions take `&Tree`, `&mut Tree` or `&str`
  parameters with elided lifetimes, and return owned values or ids.
- **Copy when it's cheap.** Ids, spans and small structs derive `Copy`.
  Copying a short string or a small `Vec` is fine when it avoids a borrow
  puzzle. Example: the literate scanner keeps its own `String` copy of the
  file rather than a borrowed `&str` with a lifetime.
- **`Rc` or `Arc` when sharing is natural** and the cost is small, such as
  immutable tables shared across threads or libraries. Not for every node.

## Structure

- A handful of crates with clear jobs, not too fine-grained: syntax
  (literate, lexing, parsing), the settle loop, tmpl, backends, and the CLI.
  Refine the split as the code grows.
- Plain functions and data. Traits only where there are several real
  implementations, such as backends.
- Errors as data: diagnostics are collected values, never panics. `Result` for
  I/O and other genuinely fallible operations.
- Doc comments on public items; short comments on why, not what.

## Dependencies

Add a dependency only when it provides clearly more value than weight, and
then prefer:

1. Widely used crates with good maintenance.
2. Among those, ones that compile fast and pull in few transitive crates.

Today tempest has no dependencies. Likely future candidates: a
file-watching crate for `watch`, and possibly an LSP protocol crate. Avoid
large frameworks (async runtimes, proc-macro-heavy libraries) unless a
specific feature truly needs them.

Each dependency added is subject to your approval, like packages in the other
projects.

## Scripts

Helper and compatibility scripts are Python, run with `uv run`, with inline
metadata declaring `requires-python = ">=3.11"` (the version Temper's Python
backend targets), and no third-party packages unless needed. Example:
`compat/check_literate.py`.

If scripts ever need third-party packages, the repo gets one uv project,
shared by all its scripts, at the top level (`pyproject.toml` with
`requires-python = ">=3.11"`, a committed `uv.lock`, managed with `uv add`
and `uv sync`). Scripts then run with `uv run` from anywhere in the repo.
Each package still needs approval.

## Performance habits

- Arenas and `Vec`s of plain data rather than many small boxed objects.
- Intern strings and types; compare ids.
- Measure before optimizing, and compare variants back to back, repeated,
  since timing on this machine is noisy.
