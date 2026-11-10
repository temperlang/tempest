# The command line

Temper's command line tool is `temper`. Its main commands build a
project's libraries and run their tests; the rest help start, run, and
explore Temper. This repository's compiler, tempest, is the `tempest`
command. It takes `temper`'s commands and flags, by the same names and
short forms, so scripts can switch by changing the name; what it lacks
says so ([the plan](../../notes/switch-over.md)).

| `temper` | What it does | `tempest` |
|---|---|---|
| [`build`](#build) | Brings the libraries under the work root up to date, in every target language, under `temper.out/` | `build`, and `-o DIR` to write elsewhere |
| [`test`](#test) | Runs the libraries' tests on each backend given, or on the interpreter | `test`, on the interpreter by default |
| [`run`](#run) | Builds the work root, then runs a library's top level with a backend's language | `run` |
| [`init`](#init) | Creates a new library: a `config.temper.md` and a first source file | `init`, file for file |
| [`serve`](#serve-or-lsp) | Starts the language server for an editor | `serve`, or `lsp`, over standard input and output |
| [`version`](#version-license-and-licenses), `license`, `licenses` | About the tool itself | the same |
| [`help`](#help) | The commands, or one command's options | the same |
| [`watch`](#not-in-tempest-yet) | Like `build`, but keeps running, rebuilding (and testing, with `-t`) as files change | not yet |
| [`repl`](#not-in-tempest-yet) | An interactive shell for Temper, or for a backend's language with the libraries loaded | not yet; see the [playground](playground.md) |
| [`doc`](#not-in-tempest-yet) | Generates documentation for the libraries | not yet |

> **Note:** the guide uses `temper` throughout; where `tempest` lacks a
> command, Temper's releases have it
> ([inconsistencies](../inconsistencies.md#i9-the-command-line)).

## Options

Options are written as `temper`'s: `--workroot DIR` or `-w DIR`, and
`--workroot=DIR` too. Every command takes `--help` (`-h`). These mean
the same wherever they're taken:

| Option | Taken by | Meaning |
|---|---|---|
| `-w DIR`, `--workroot DIR` | `build`, `test`, `run` | The [work root](#the-work-root) |
| `-ign FILE`, `--ignorefile FILE` | `build`, `test`, `run` | An [ignore file](#ignore-files) with the last word |
| `-b BACKEND`, `--backend BACKEND` | `build`, `test`, `run` | A backend: `cpp`, `csharp`, `java`, `js`, `lua`, `py`, or `rust` ([target languages](../backends/README.md)), or `interp`, the interpreter, for `test` and `run`. `build` and `test` take it more than once |
| `--no-connected` | `build`, `test`, `run` | Use the Temper bodies of `@connected` functions, where they have them, instead of each language's own code |
| `-v`, `--verbose` | `build`, `test`, `run` | Say more: what was written, every test's result, what the code logs, and [hints](#diagnostics) |
| `--logformat text\|json` | `build`, `test`, `run` | [How it reports](#logging) |
| `-o DIR`, `--output-directory DIR` | `init`; `build` and `run` in tempest only | Where to write |
| `--library NAME` | `test`, `run` | A library, by its dashed name (`foo-bar`) |
| `--std DIR` | tempest only | Read Temper's std library from DIR instead of the copy built in |
| `--timeout S\|none` | `test`, `run`; tempest only | Seconds each command a backend's tools start may take ([below](#what-tools-start)) |

Backends that `temper` has and tempest doesn't yet (`cppv`, `data`,
`java8`, and `mypyc`) [say so](#not-in-tempest-yet).

## The work root

Commands work on every library under a *work root*. Each
`config.temper.md` under it starts a library, and each directory in a
library is a module. Without `-w DIR`, the work root is found by
searching up from the current directory for:

1. the nearest directory that contains a `temper.out` directory, or
2. the farthest directory that contains a `config.temper.md`, or
3. the current directory itself.

That's what `temper` does. (Its `--help` says "farthest" and
"closest" the other way around, but its code searches as above.)

### Ignore files

Files that ignore files list aren't sources. `tempest` reads every
`.gitignore` and `.temperignore` as git reads `.gitignore`: the work
root's, those in directories below it, and those above it up to its git
repository's root, a deeper file's patterns after a shallower one's, and
a directory's `.temperignore` after its `.gitignore`, so it has the last
word there. `--ignorefile FILE` adds one more, with the last word over
all of them; its patterns are relative to its own directory, as in
`temper`, and match nothing under a work root that isn't at or below
that directory. `--ignorefile /dev/null` adds nothing.

`temper` reads only one ignore file: `--ignorefile`'s, or else the
nearest `.temperignore` or `.gitignore` at or above the current
directory. On most trees the two agree; tempest's way also skips what a
nested `.gitignore` lists, such as a build directory holding a copy of
a library.

## `build`

    temper build [-w DIR] [-ign FILE] [-b BACKEND]... [--no-connected] [-v] [--logformat text|json]
    tempest build [same] [-o DIR] [--std DIR]

Writes `temper.out/BACKEND/LIBRARY/` for each library and backend,
with each backend's runtime beside them as `temper-core`. Without `-b`,
it builds C#, Java, JavaScript, Lua, Python, and Rust (`temper` also
builds its `data` backend). With `-o DIR` (tempest's), it writes
`DIR/BACKEND/LIBRARY/` instead, leaving `temper.out` alone: to look at
one language's code, `tempest build -b py -o /tmp/py`.

Errors don't stop it: each is reported, every backend's code is still
written, failing where it runs, and the exit status is 1.

## `test`

    temper test -b BACKEND... [-w DIR] [-ign FILE] [--library NAME] [--no-connected] [-v] [--logformat text|json]
    tempest test [-b BACKEND|interp]... [-w DIR | DIR] [same] [--fuel N|none] [--timeout S|none] [--std DIR]

Builds the libraries and runs their tests with each backend's own tools
(Mocha, JUnit, `dotnet test`, `cargo test`, and so on), or on the
interpreter with `-b interp`. It prints the tests that failed, or with
`-v`, every test's result, then a tally, such as `Tests passed: 3 of
3`, and exits with 1 if any test fails or doesn't run.

- **Backends.** `temper` needs `-b`; tempest runs on the interpreter
  without one, which takes milliseconds and writes no files. Given
  several, tempest runs each in turn, printing each one's tally
  (`py: Tests passed: 3 of 3`) and then the total, which is `temper`'s
  one line; it fails if any fails.
- **`--library NAME`** tests only that library, after building them
  all, as `temper` does.
- **`DIR`**, tempest's, is where the work root's search starts, in
  place of the current directory; `-w DIR` names the work root itself.
  One or the other.
- **`--fuel N`**, tempest's, is how many steps each test may take on
  the interpreter, and the library's top-level code before them: 10
  billion by default, 3 to 4 minutes of a tight loop. A test that
  runs out fails alone, "ran out of steps (each test may take N)", and
  the others still run, so a test that loops forever doesn't hang the
  run. `--fuel none` lifts the limit; digits may be grouped with `_` or
  `,` (`--fuel 5_000_000_000`). The slowest real library test takes
  about 626 million steps (Kelpto's), a sixteenth of that; `-v` shows
  each test's steps.
  What the tests log is kept up to 64 KiB, then cut with a note, and
  printed only for a library with a failure. `temper` has neither
  limit.
- **`--timeout S`**, tempest's, is how long each command a backend's
  tools start may take: 600 seconds by default
  ([below](#what-tools-start)). A tests file on Lua, or a library's
  tests elsewhere, that runs over fails, "timed out after 600 s", and
  the others' results still count. `temper` has no limit.
- **Errors in an imported library** are reported once, under it, not
  again for each library importing it.

## `run`

    temper run --library NAME -b BACKEND [-w DIR] [--no-connected] [-v] [--logformat text|json]
    tempest run [same] [-ign FILE] [-o DIR] [--timeout S|none] [--std DIR]

Builds the work root for the backend, then runs the library's top
level with that language: Python imports its package, Node imports its
module, Lua runs its `init.lua`, and Rust, C++, Java, and C# build its
program quietly and run it. What it prints goes to standard output as
it runs. With `-b interp`, it runs on the interpreter, writing no
files. Errors fail the run, as in `build`, but don't stop it.

`temper` prints what the program printed after it finishes, on
standard error, and on the interpreter, twice; tempest streams it to
standard output, once, as it runs, on the interpreter too. On the
interpreter, `run` has no step limit, as a program run any other way
has none. On a backend, the program has no time limit unless
`--timeout` or `TEMPEST_TIMEOUT` gives one, as it may be a server; the
steps that build it have the usual 600 seconds.

### What tools start

What `test -b` and `run -b` start (`lua`, `python3`, Node and Mocha,
`dotnet`, `java`, `cargo`, CMake, and the programs they build) is
tempest's to stop, which `temper` leaves to the tools:

- **A time limit.** Each command may take `--timeout S` seconds, or
  `TEMPEST_TIMEOUT`'s, 600 by default; `none` lifts it. One that runs
  over is killed and fails, "timed out after S s", with the end of what
  it printed.
- **Nothing left behind.** Each command runs in a process group of its
  own, which is killed when it ends or times out, and if tempest gets
  SIGINT (Ctrl-C), SIGTERM, or SIGHUP; so what it started, such as
  `cargo test`'s test binary, goes too. On Linux, each command also
  dies when tempest does, even by SIGKILL (`PR_SET_PDEATHSIG`); what it
  started survives that one case.
- **A memory limit,** on Linux: Lua, Python, and C++'s test and main
  programs may write at most `TEMPEST_MEMORY_LIMIT` MiB, 4096 by
  default (`none` lifts it), so a runaway fails ("not enough memory",
  `MemoryError`) rather than taking the machine. It's `RLIMIT_DATA`,
  which counts memory written, not address space reserved, which the
  JVM, .NET, and Node reserve far more of than they use, so they, and
  the compilers, have none.

On Windows, a command that runs over is killed, but not what it
started, and nothing dies with tempest yet. A program run in its own
group isn't in the terminal's foreground, so one that read the
terminal would stop; Temper has no way to read it.

## `init`

    temper init [-o DIR]

Creates a library in `DIR`, by default the current directory, with a
`src/config.temper.md` and `src/NAME.temper.md` to start from
([getting started](../guide/getting-started.md)), where `NAME` is the
directory's name, dashed (`MyProj` gives `my-proj`). tempest writes
`temper`'s files byte for byte. It fails, writing nothing, if `DIR`
already has a `src` directory or a `config.temper.md`.

## `serve`, or `lsp`

    temper serve --port N [--tokens add|full|none]
    tempest serve [--std DIR]
    tempest lsp [--std DIR]

A language server for editors ([editors](editors.md)). `temper` serves
on a TCP port; tempest serves over standard input and output, the way
editors usually start servers, so given `--port`, it says so and exits
with 3. It takes `--tokens` but sends no semantic tokens yet.

## `version`, `license`, and `licenses`

    tempest version
    tempest license
    tempest licenses

`version` prints tempest's version and the commit it was built from;
`tempest --version` too (`temper` has only `version`). `license` prints
the copyright, the notices, and the license texts, as `temper license`
does. `licenses` lists the third-party Rust crates built into the
binary, each with its license, made from Cargo's metadata when tempest
is built.

## `help`

    temper help [COMMAND]
    temper COMMAND --help

The commands, each with a line, or one command's options.

## Not in tempest yet

`watch`, `repl`, and `doc`, and the backends `cppv`, `data`, `java8`,
and `mypyc`, say they aren't in tempest yet, and exit with 3, so a
script can tell them from a failure:

- **`watch`** waits for a file watcher of its own, which will reuse
  `build` and `test` ([the plan](../internals/detail/08-playground.md#watch-planned)).
  Meanwhile, run `tempest build` or `tempest test` after a change.
- **`repl`**: Temper as you type is the [playground](playground.md)'s.
- **`doc`** waits for a documentation generator.
- **The backends:** `data` writes data files a library declares, and
  `cppv`, `java8`, and `mypyc` are variants of C++, Java, and Python
  that tempest's backends don't cover yet.

## Diagnostics

Each problem is reported as `path:line:column: level: message`, at one
of three levels:

- **error**: the code is wrong. The library doesn't build, its tests
  count as failed, and the command exits with 1.
- **warn**: worth a look but not wrong, such as an `Int32` literal too
  big to fit, which wraps (as in `temper`, for now). The command still
  succeeds.
- **hint**: advice about correct code, such as an unnecessary type
  check. Editors and the playground show hints; the command line says
  them only with `-v`. `temper` calls these `info` and always prints
  them; tempest's JSON records still say `info`.

## Logging

With `--logformat json`, `build`, `test`, and `run` write one JSON
record per line, for tools, all on standard error, in place of their
text (what `run`'s program prints still goes to standard output):

```text
{ "level": "error", "template": "UndeclaredName", "pos": { "loc": "lib/a.temper", "left": 51, "right": 57 }, "values": [], "message": "tripel has not been declared" }
{ "level": "error", "template": "TestFailed", "values": [ "py", "t2", "expected f(2) == (7) not (6)" ], "message": "Test failed (py): t2 - expected f(2) == (7) not (6)" }
{ "level": "summary", "template": "TestSummaryAllRun", "values": [ 1, 2 ], "message": "Tests passed: 1 of 2" }
{ "task": "Test", "ok": false }
```

The records have `temper`'s shape: `level`, `template` (the kind of
message), `pos` where there is one (the file, and the start and end in
UTF-16 code units), and `values`, ending with the task's result. Two
differences: tempest adds `message`, the text it would have printed,
and its diagnostics' `values` are empty, as it keeps only their text;
their `template` names are tempest's (`UndeclaredName` where `temper`
says `BecauseNameUndeclared`). The tests' records are `temper`'s.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Failure: errors in the code, a test that failed or didn't run, a library that didn't build, or a backend this build of tempest was built without |
| 2 | A usage error: an unknown command or option, a missing value, or a backend that doesn't exist |
| 3 | Not in tempest yet: `watch`, `repl`, `doc`, a backend `temper` has, or `serve --port` |
| 101 | A bug in tempest: it panicked, saying where on standard error. Please report it with what you ran |

`temper` exits with 1 on failure and 127 on a usage error, which
shells otherwise use for a command that wasn't found.

## Gone: one backend's files

tempest used to have a command per backend, `tempest py FILE...` and
so on, printing one language's code for loose files. `temper` has no
such commands, so they're gone: make a library (`tempest init`, or a
`config.temper.md` beside the files) and run `tempest build -b py -o
DIR`, which writes the same files, laid out as `temper.out/py`.
