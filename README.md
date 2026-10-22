# temper-rs

An experimental, from-scratch implementation of [Temper](https://temperlang.dev/)
in Rust (edition 2024), aiming for the same language, behavior and generated
APIs with a much smaller, simpler and faster compiler. See
[DESIGN.md](DESIGN.md).

Status: early. `temper-syntax` splits literate `.temper.md` files into prose
and code, and lexes Temper source into tokens, exactly as Kotlin Temper does
for every `.temper` and `.temper.md` file in the Temper and Prismora repos.

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

- `crates/temper-syntax`: source to tokens, and later to trees. Modules
  `literate` (code blocks in `.temper.md`) and `lex` (tokens), with the
  `literate-blocks` and `lex-tokens` binaries used by the compat checks.
- `compat/`: oracles that run Kotlin Temper's own code for reference results,
  and scripts that compare them with this implementation.
- `notes/`: the design and plan.

## Checking against Kotlin Temper

    uv run compat/check_literate.py   # markdown code blocks
    uv run compat/check_lex.py        # tokens

They need an installed Kotlin Temper at `../temper/cli/build/install`
and compare every Temper file they find; `--verbose` shows diffs.
