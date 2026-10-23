# temper-rs

An experimental, from-scratch implementation of [Temper](https://temperlang.dev/)
in Rust (edition 2024), aiming for the same language, behavior and generated
APIs with a much smaller, simpler and faster compiler. See
[DESIGN.md](DESIGN.md).

Status: early. `temper-syntax` splits literate `.temper.md` files into prose
and code, lexes Temper source into tokens, parses tokens into operator
trees, and turns those into calls, blocks and declarations (the grammar
step), as Kotlin Temper does for every `.temper` and `.temper.md` file in
the Temper and Prismora repos, except where we deliberately differ (listed
in [notes/testing-and-milestones.md](notes/testing-and-milestones.md)).
`temper-frontend` has the first slice of the settle loop: names, the core
macros (`let`, `var`, `fn`, `if`, `while`, `for`, `do`, `orelse`, labels,
`break`, `continue`, `return`, `for … of`, `when`, `??`, `?.`), simple
classes and interfaces, type parameters (kept in the tree; the
interpreter ignores them), lists and list builders, `Float64` methods
and formatting, and a tree interpreter, running 39 of Kotlin Temper's 69
functional tests with matching output. Next: the rest of classes, and imports.

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
  core constructs' layouts), `builtins`, `interp` (a tree interpreter) and
  `print`, with the `run-temper` binary that settles and runs files.
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
diagnostics and what running logs) and `tests/settle.rs`. `UPDATE_SNAPSHOTS=1 cargo test`
accepts new expected output for review.

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
    uv run compat/gen_nfkc.py         # regenerate NFKC tables from the JDK

Running Kotlin Temper's functional tests in-process, which needs only
its sources at `../temper`:

    uv run compat/check_functional.py [--verbose] [TEST ...]

The checks compare every Temper file in the Temper and Prismora repos;
`--verbose` shows diffs. Inputs where we deliberately differ from Kotlin are
listed in `compat/deviations.py` and reported as expected.
