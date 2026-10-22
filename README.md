# temper-rs

An experimental, from-scratch implementation of [Temper](https://temperlang.dev/)
in Rust (edition 2024), aiming for the same language, behavior and generated
APIs with a much smaller, simpler and faster compiler. See
[DESIGN.md](DESIGN.md).

Status: early. `temper-syntax` splits literate `.temper.md` files into prose
and code, lexes Temper source into tokens, and parses tokens into operator
trees, as Kotlin Temper does for every `.temper` and `.temper.md` file in the
Temper and Prismora repos, except where we deliberately differ (listed in
[notes/testing-and-milestones.md](notes/testing-and-milestones.md)). Next:
the grammar that turns operator trees into calls and declarations.

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
  `parse` (operator-precedence parser), `tree`, `names` (interning) and
  `print` (text forms of trees), with the `literate-blocks`, `lex-tokens` and
  `parse-tree` binaries used by the compat checks.
- `compat/`: oracles that run Kotlin Temper's own code for reference results,
  and scripts that compare them with this implementation, fuzz both, and
  time both.
- `notes/`: the design and plan.

## Testing

    cargo test

runs each crate's own tests, which don't need Kotlin Temper. For the lexer,
`crates/temper-syntax/tests/*.cases` hold inputs with their expected tokens,
and `tests/lex.rs` also lexes randomly edited copies of them and checks
properties that hold for any input. The parser's are in `tests/parse/*.cases`
(expected trees) and `tests/parse.rs`. `UPDATE_SNAPSHOTS=1 cargo test`
accepts new expected output for review.

Comparing with Kotlin Temper, which needs an installed Kotlin Temper at
`../temper/cli/build/install`:

    uv run compat/check_literate.py   # markdown code blocks
    uv run compat/check_lex.py        # tokens, for all Temper files and cases
    uv run compat/check_parse.py      # trees, for all Temper files and cases
    uv run compat/fuzz_lex.py         # tokens, for randomly edited inputs
    uv run compat/fuzz_parse.py       # trees, for randomly edited inputs
    uv run compat/bench_lex.py        # lexer speed, ours vs Kotlin's
    uv run compat/bench_parse.py      # lexer and parser speed

The checks compare every Temper file in the Temper and Prismora repos;
`--verbose` shows diffs. Inputs where we deliberately differ from Kotlin are
listed in `compat/deviations.py` and reported as expected.
