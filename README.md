# Tempest

An experimental, from-scratch implementation of [Temper](https://temperlang.dev/)
in Rust (edition 2024), aiming for the same language, behavior and generated
APIs with a much smaller, simpler and faster compiler. See
[DESIGN.md](DESIGN.md).

Status: early. `temper-syntax` splits literate `.temper.md` files into prose
and code, lexes Temper source into tokens, parses tokens into operator
trees, and turns those into calls, blocks and declarations (the grammar
step), as Kotlin Temper does for every `.temper` and `.temper.md` file in
the Temper, Chromarite and Ordable repos, except where we deliberately
differ (listed in
[notes/testing-and-milestones.md](notes/testing-and-milestones.md)).
`temper-frontend` has the first slice of the settle loop: names, the core
macros (`let`, `var`, `fn`, `if`, `while`, `for`, `do`, `orelse`, labels,
`break`, `continue`, `return`, `for … of`, `when`, `??`, `?.`,
destructuring `let`), simple classes and interfaces, type parameters
(kept in the tree; the interpreter ignores them), lists and list
builders, `Float64` methods and formatting, strings, string indices,
`StringBuilder` and `char'a'`, top-level declarations in dependency
order, imports and std, classes with inheritance, generators and
`async`, and a flat-frame interpreter, running all 69 of Kotlin Temper's
functional tests with matching output. Next: static types.

## Principles

Details are in [notes/](notes/README.md); in short:

- Match Kotlin Temper's behavior, error messages and generated public APIs,
  not its implementation. Check against it with the oracles in `compat/` and
  by running Kotlin Temper itself, including `temper repl`.
- Keep it simple: one tree, a settle loop instead of stages, and errors as
  values that still reach code generation.
- Code without explicit lifetime annotations where avoidable: ids into arenas
  instead of references, borrows on the stack, copies when cheap
  ([notes/code-style.md](notes/code-style.md)).
- Dependencies only when worth their weight, widely used, and fast to compile.
- Generated code: no numbered names except on real conflicts, and source maps
  for every backend.
- Scripts in Python, run with `uv run`, for Python 3.11+ (what Temper's Python
  backend targets).
- Timing on the dev machine is noisy, so compare variants back to back,
  repeated.

## Layout

- `crates/temper-syntax`: source to trees. Modules `literate` (code blocks in
  `.temper.md`), `lex` (tokens), `adapt` (token adapters: semicolon
  insertion, string wrapping and the like), `operator` (the operator table),
  `parse` (operator-precedence parser), `ast` and `grammar` (the grammar
  step: calls, blocks, declarations), `module` (the rest of Kotlin's
  `ParseStage`: call joins rewritten, `;;;` metadata as JSON), `unpack` (numbers, strings and doc
  comments from tokens), `tree`, `names` (interning), `nfkc` (checks that
  identifiers are in Unicode NFKC) and `print` (text forms of trees), with the `literate-blocks`, `lex-tokens`, `parse-tree` and
  `ast-tree` binaries used by the compat checks.
- `crates/temper-frontend`: libraries to settled trees. Modules `library`
  (one arena per library build), `engine` (the settle loop's scheduler),
  `settle` (its steps), `scope` (scopes and symbols), `macros`, `classes`, `core` (the
  core constructs' layouts), `builtins`, `interp` (the interpreter) and
  `print`, `project` (libraries on disk, found as Kotlin's `temper test`
  finds them) and `library_tests` (running their tests), with the
  `run-temper` binary that settles and runs files.
- `crates/temper-tmpl`: tmpl, the IR between the frontend and the backends
  ([notes/tmpl/](notes/tmpl/README.md)), with no dependencies. So far,
  for subset A (functions, locals, `if`, loops, calls, builtins, and
  module init): the model and its tables, the `schema` of node kinds, a
  `Builder`, the canonical `text` writer and reader, and the validator.
  Nothing uses it yet.
- `crates/tempest`: the `tempest` command, a thin binary over the other
  crates; so far `tempest test`.
- `crates/temper-wasm`: a small wasm entry point, the seed of a browser
  playground, which also keeps the wasm size measurement honest.
- `compat/`: oracles that run Kotlin Temper's own code for reference results,
  and scripts that compare them with this implementation, fuzz both, and
  time both.
- `notes/`: the design and plan.
- `NOTICE`: the Unicode license notice for the Unicode data in
  `temper-syntax`.

## Testing

    cargo test

runs each crate's own tests, which don't need Kotlin Temper. For the lexer,
`crates/temper-syntax/tests/*.cases` hold inputs with their expected tokens,
and `tests/lex.rs` also lexes randomly edited copies of them and checks
properties that hold for any input. The parser's are in `tests/parse/*.cases`
(expected trees) and `tests/parse.rs`, the grammar step's in
`tests/ast/*.cases` and `tests/ast.rs`, and the module step's in
`tests/module/*.cases` and `tests/module.rs`. The settle loop's are in
`crates/temper-frontend/tests/settle/*.cases` (settled trees,
diagnostics and what running logs) and `tests/settle.rs`. tmpl's are in
`crates/temper-tmpl/tests/text/*.cases` (hand-written tmpl, with its
canonical text, validator problems, or read error) and `tests/text.rs`,
with a round trip over random programs in `tests/round_trip.rs`, qnames
in `tests/qname.rs`, and a fixture of core's members checked against
`core.temper` in `tests/core_fixture.rs`.
`UPDATE_SNAPSHOTS=1 cargo test` accepts new expected output for review.

A Temper library's tests run on the interpreter as `temper test` runs
them, finding the libraries (each directory with a `config.temper.md`)
under DIR's work root, printing each test's result and Kotlin's summary,
`Tests passed: 56 of 56`, and failing if any test fails or doesn't run:

    cargo run --release -p tempest -- test [--std DIR] [DIR]

DIR defaults to the current directory. Temper's std library is built into
the binaries and the wasm build, from `vendor/temper/std`, so nothing
ships beside them; `--std DIR` loads it from a directory instead, to test
against another copy. The compiler reads files only through
temper-frontend's `Sources`, layers of the disk, the built-in std, and
in-memory files, so the wasm build and tests need no file system. Chromarite and Ordable pass all their tests this
way.

Comparing with Kotlin Temper, which needs an installed Kotlin Temper at
`../temper/cli/build/install`:

    uv run compat/check_literate.py   # markdown code blocks
    uv run compat/check_lex.py        # tokens, for all Temper files and cases
    uv run compat/check_parse.py      # trees, for all Temper files and cases
    uv run compat/check_ast.py        # grammar step trees and diagnostics
    uv run compat/check_ast.py --module   # also call joins and metadata JSON
    uv run compat/fuzz_lex.py         # tokens, for randomly edited inputs
    uv run compat/fuzz_parse.py       # trees, for randomly edited inputs
    uv run compat/fuzz_ast.py         # grammar step, for randomly edited inputs
                                      #   (--module: also call joins, metadata)
    uv run compat/bench_lex.py        # lexer speed, ours vs Kotlin's
    uv run compat/bench_parse.py      # lexer and parser speed
    uv run compat/bench_ast.py        # lexer, parser and grammar step speed
    uv run compat/bench_settle.py     # settle loop speed (ours only)
    uv run compat/bench_interp.py     # interpreter speed and memory, ours vs
                                      #   Kotlin's (in a warmed-up JVM)
    uv run compat/measure_size.py     # binary and wasm size, peak memory
                                      #   (--record: append to
                                      #   notes/size-and-speed.csv)
    uv run compat/size_speed_table.py # latest measurements, as Markdown
    uv run compat/gen_nfkc.py         # regenerate NFKC tables from the JDK

Running Kotlin Temper's functional tests in-process, from the copy of
them (with std and core) in `vendor/temper/`, so no Temper checkout is
needed; `--temper DIR` uses a checkout's instead:

    uv run compat/check_functional.py [--verbose] [--temper DIR] [TEST ...]
    uv run compat/sync_temper.py [TEMPER_DIR]   # refresh vendor/temper/

Counting the expressions typing leaves of unknown type, with the calls
it leaves unresolved, over the functional tests, std, and the libraries
under each LIBRARY directory given (`--verbose` lists each one):

    uv run compat/count_untyped.py [--verbose] [--temper DIR] [LIBRARY ...]

Translating a library into tmpl, the backend IR, and printing its
canonical text, or what it uses that the translation doesn't cover yet;
and counting the functional tests that translate cleanly, with why the
rest don't:

    target/release/run-temper --tmpl FILE...
    uv run compat/count_tmpl.py [--verbose] [--temper DIR]

CI (`.github/workflows/ci.yml`) runs formatting, clippy with warnings as
errors, the tests with and without the `net` web client, the wasm build,
the functional tests, and `tempest test` on fresh checkouts of Chromarite
and Ordable. `.github/workflows/binaries.yml` builds `tempest`
for Linux, macOS, and Windows, and the small `temper-wasm`, by hand or on
a `v*` tag, which also attaches them to a release.
Both pin Rust 1.85.0, the workspace's `rust-version` and the first with
edition 2024, so every crate also works as a library on that toolchain.
Raising it is a deliberate change of its own.

The checks compare every Temper file in the Temper, Chromarite and
Ordable repos; `--verbose` shows diffs. Inputs where we deliberately differ from Kotlin are
listed in `compat/deviations.py` and reported as expected.

Written with Claude Code and Opus 5.5.
