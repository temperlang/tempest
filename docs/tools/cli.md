# The command line

Temper's command line tool is `temper`. Its main commands build a
project's libraries and run their tests; the rest help write and
explore Temper. This repository's compiler, tempest, is the `tempest`
command, which has `build` and `test` today, plus `lsp`, and is growing
toward the rest ([the plan](../../notes/switch-over.md)).

| `temper` | What it does | `tempest` |
|---|---|---|
| `build` | Brings the libraries under the work root up to date, in every target language, under `temper.out/` | `tempest build` |
| `test` | Runs the libraries' tests on a backend, or on the interpreter | `tempest test` |
| `watch` | Like `build`, but keeps running, rebuilding (and testing, with `-t`) as files change | not yet |
| `init` | Creates a new library: a `config.temper.md` and a first source file | not yet |
| `run` | Loads a library built for a backend, running its top level | not yet |
| `repl` | An interactive shell for Temper, or for a backend's language with the libraries loaded | not yet; see the [playground](playground.md) |
| `doc` | Generates documentation for the libraries | not yet |
| `serve` | Starts the language server for an editor | `tempest lsp`, on standard input and output |
| `version`, `license`, `licenses`, `help` | About the tool itself | `help` only |

> **Note:** the guide uses `temper` throughout; where `tempest` lacks a
> command, Temper's releases have it
> ([inconsistencies](../inconsistencies.md#i9-the-command-line)).

## The work root

Commands work on every library under a *work root*. Each
`config.temper.md` under it starts a library, and each directory in a
library is a module. Without `-w DIR` (`--workroot`), the work root is
found by searching up from the current directory for:

1. the farthest directory that contains a `temper.out` directory, or
2. the closest directory that contains a `config.temper.md`.

Files that a `.temperignore` or `.gitignore` lists aren't sources.
`tempest` reads `.gitignore` files; `temper` also takes
`--ignorefile FILE`.

## `build`

    temper build [-w DIR] [-b BACKEND]... [--no-connected] [-v]

Writes `temper.out/BACKEND/LIBRARY/` for each library and backend.
Without `-b`, it builds C#, Java, JavaScript, Lua, Python, and Rust
(`temper` also builds its `data` backend). Backend IDs are in
[target languages](../backends/README.md).

- `--no-connected`: use the Temper bodies of `@connected` functions,
  where they have them, instead of each language's own code.
- `-v` (`--verbose`): log more.

## `test`

    temper test -b BACKEND [-w DIR] [--library NAME] [--no-connected] [-v]
    tempest test [-b interp|BACKEND] [-v] [DIR]

Builds the libraries and runs their tests with the backend's own tools
(Mocha, JUnit, `dotnet test`, `cargo test`, and so on), or on the
interpreter with `-b interp`. It ends with a tally, such as
`Tests passed: 3 of 3`, and exits with an error if any test fails.
`temper` needs `-b`; `tempest` defaults to the interpreter, and with
`-v` shows what the code logs.

## `watch`

    temper watch [-w DIR] [-b BACKEND]... [-t BACKEND]... [--limit N]

Builds, then rebuilds whenever a source changes. `-t` also runs the
tests on that backend after each build. Ctrl+C stops it.

## `init`

    temper init [-o DIR]

Creates a library in `DIR`, by default the current directory, with a
`src/config.temper.md` and `src/NAME.temper.md` to start from
([getting started](../guide/getting-started.md)).

## `run`

    temper run --library NAME -b BACKEND [-w DIR]

Builds the work root, then loads the library `NAME` (dashed, as
`foo-bar`) with the backend's language, running its top level.

## `repl`

    temper repl [-b BACKEND] [-w DIR]

An interactive Temper shell; `help()` lists its topics, and
`describe(n, "stage")` shows how Temper saw an earlier entry at a
stage of compiling. With `-b js`, say, it builds the work root and
starts that language's shell (`node`) with the libraries loaded
instead.

## `doc`

    temper doc [-w DIR] [-o DIR] [-b BACKEND]...

Generates documentation for the libraries, by default under
`temper.out/-docs`.

## `serve`, or `tempest lsp`

    temper serve --port N
    tempest lsp

A language server for editors ([editors](editors.md)).

## Logging

`temper`'s `--logformat json` writes one JSON record per line, for
tools, instead of text with source snippets.
