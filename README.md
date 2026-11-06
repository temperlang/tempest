# Tempest

A from-scratch implementation of [Temper](https://temperlang.dev/) in Rust
(edition 2024): the same language, behavior, and generated APIs as Temper
today, from a much smaller, simpler, and faster compiler. See
[DESIGN.md](DESIGN.md) for the design and [notes/](notes/README.md) for
the plan.

## Status

- **The whole pipeline works:** parsing, the settle loop (names, macros,
  types), an interpreter, tmpl (the backend IR), and seven backends: C++,
  C#, Java, JavaScript, Lua, Python, and Rust.
- **Temper's functional tests:** all 72 run on the interpreter and pass
  on every backend, matching Temper's expected output. (std's `net` needs
  libcurl for C++ and LuaSocket for Lua; without them, its two tests fail
  as Temper's do.)
  Generated code passes each language's strict checkers (clippy, `tsc
  --strict`, pyright and mypy, LuaLS, and compilers with warnings as
  errors); C++ also runs under AddressSanitizer, UBSan, and TSan.
- **Async and generators** on every backend, one turn at a time, with
  each language's own `await` and loops working on generated promises and
  generators.
- **Real libraries:** Hrefine, Chromarite, and Ordable pass all their
  tests on the interpreter and, built by `tempest test -b`, in C#, Java,
  JavaScript, Lua, Python, and Rust, run by each language's own tools;
  C++ and Kelpto come next ([notes/switch-over.md](notes/switch-over.md)).
  Kelpto passes on the interpreter.
- **Coming:** the language server, CLI parity, and architecture docs ([notes/switch-over.md](notes/switch-over.md)).

## Try it

- **Playground (online):**
  <https://temperlang.github.io/tempest/c9cb414b4800a065953f72df4ab69943/playground/>,
  the whole compiler in the browser, as wasm
  ([site/playground](site/playground/README.md)).

    cargo run --release -p tempest -- test [-b interp|BACKEND] [DIR]
    cargo run --release -p tempest -- build [-w DIR] [-b BACKEND]...
    cargo run --release -p tempest -- py|rust|js|java|csharp|cpp|lua [--out DIR] FILE...

- `test` finds the libraries under DIR (each directory with a
  `config.temper.md`) and runs their tests, as `temper test` does: on
  the interpreter, or with `-b BACKEND`, built into `temper.out` and run
  by the language's own tools (`temper_core.testing` for Python, Mocha,
  LuaUnit, JUnit through `javac`, `dotnet test`, or `cargo test`; C++
  comes next).
- `build` writes `temper.out/BACKEND/LIBRARY/…`, laid out as Temper's.
- A backend's name translates files and prints, or writes, the result.
  It's an internal development command, not part of Temper's CLI, and
  will go once tests use `build` instead.

Temper's std library is built in, so the binary needs nothing beside it.
Each backend is a cargo feature, all on by default.

## Principles

- Match Temper's behavior, error messages, and generated public APIs, not
  its implementation; check against Temper itself (`compat/`).
- Keep it simple: one tree, a settle loop instead of stages, and errors as
  values that still reach code generation.
- No explicit lifetimes where avoidable: ids into arenas, borrows on the
  stack, copies when cheap ([notes/code-style.md](notes/code-style.md)).
- Few dependencies, each widely used and quick to compile.
- Readable generated code: no numbered names except on real conflicts,
  and source maps for every backend.

## Layout

| Crate | What it does |
|---|---|
| `temper-syntax` | Source to trees: literate files, lexing, parsing, the grammar step |
| `temper-frontend` | Libraries to settled trees: the settle loop, typing, the interpreter, and tmpl translation |
| `temper-tmpl` | tmpl, the backend IR ([notes/tmpl/](notes/tmpl/README.md)), with shared lowerings such as coroutines |
| `temper-be-*` | One crate per backend, tmpl in and source out, each depending only on `temper-tmpl` |
| `temper-be-testkit` | What the backends' tests share: programs, goldens, and tool discovery |
| `temper-funtests` | Temper's functional tests, from `vendor/temper/` |
| `tempest` | The command-line tool |
| `temper-wasm` | The compiler for the browser, the base of the playground |

Also: `compat/` (scripts comparing with Temper, fuzzing, and timing),
`vendor/temper/` (Temper's runtimes, std, and functional tests, synced
with patches in `vendor/temper-patches/`), and `notes/`.

## Testing

    cargo test

runs every crate's tests, needing no Temper checkout. Backends' tests
compare goldens of generated code (`UPDATE_SNAPSHOTS=1` accepts changes)
and run the generated code with each language's own tools, skipping with
a message when a tool is missing; [notes/test-speed.md](notes/test-speed.md)
has the details.

Comparing with Temper, using the copy in `vendor/temper/` (or `--temper
DIR` for a checkout):

    uv run compat/check_functional.py   # the functional tests, in-process
    uv run compat/count_tmpl.py         # which tests translate, and why not
    uv run compat/sync_temper.py        # refresh vendor/temper/ (--check in CI)

and `compat/check_*.py`, `fuzz_*.py`, and `bench_*.py` for each stage.
Scripts run with [uv](https://docs.astral.sh/uv/), for Python 3.11+.

CI runs formatting, clippy, every crate's tests with each language's
toolchain, the wasm build, the functional tests, and Chromarite's and
Ordable's tests on the interpreter and on five backends, all on Rust
1.85.0, the workspace's `rust-version`. A
nightly job tests C++ on more compilers and under sanitizers.

Written with Claude Code and Opus 5.5.
