# temper-rs

An experimental, from-scratch implementation of [Temper](https://temperlang.dev/)
in Rust (edition 2024), aiming for the same language, behavior and generated
APIs with a much smaller, simpler and faster compiler. See
[DESIGN.md](DESIGN.md).

Status: early. `temper-literate` splits literate `.temper.md` files into prose
and code exactly as Kotlin Temper does, for all 129 `.temper.md` files in the
Temper and Prismora repos.

## Layout

- `crates/temper-literate`: markdown code-block finding for `.temper.md`.
- `compat/`: oracles that run Kotlin Temper's own code for reference results,
  and scripts that compare them with this implementation.

## Checking against Kotlin Temper

    uv run compat/check_literate.py

It needs an installed Kotlin Temper at `../temper/cli/build/install`
and compares every `.temper.md` file it finds.
