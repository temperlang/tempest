# The REPL

`tempest repl` reads Temper a line at a time and runs each complete
input on the interpreter, against what earlier inputs declared,
printing each result as `interactive#N: value`. It's `temper repl`'s
counterpart; the [tutorial](../guide/tutorial/01-classes-and-functions.md)
uses it throughout.

```console
$ let greet(name: String): String { "Hello, ${name}!" }
interactive#0: void
$ greet("REPL")
interactive#1: "Hello, REPL!"
$ [1, 2, 3].map { x => x * 10 }
interactive#2: [10,20,30]
$ class Point(public x: Int, public y: Int) {}
interactive#3: Point
$ { x: 1, y: 2 }
interactive#4: {class: Point, x: 1, y: 2}
```

Ctrl-D (the end of input) ends the session, with exit code 0 whatever
failed. Piped input works the same, without prompts:

```sh
printf '1 + 2\nconsole.log("hi")\n' | tempest repl
```

## Inputs

A line ends an input unless something is still open: a bracket (`(`,
`[`, `{`, or `${`), a multi-quoted string (`"""`, ended by a line that
doesn't go on with it, such as a blank one), or a block comment. A line
of only comments waits too, so a doc comment joins what it documents.
Meanwhile the prompt is `  ` instead of `$ `. A `"` string never spans
lines: one left open is an error at the end of its line, rather than
swallowing the lines after it.

So `else` on a line of its own starts a new input, as it would be an
error in a file too; keep `} else {` together. To type several
statements as one input, put them on one line, separated by `;`, or
use `--separator semisemi`, which ends inputs at a `;;` at the end of a
line, as OCaml's REPL does; `--separator eof` makes all of the input
one.

## Results

Each input prints what it reports, in order: its errors and warnings,
what it logs as it runs, why it failed if no error already says, and
then its result, as `temper repl` prints it:

| Input | Result |
|---|---|
| An expression | Its value: `3`, `2.0`, `"text"` (with Temper's escapes), `true`, `null`, `[1,2,3]` for a list, `[(1,"a")]` for a map, `{class: Point, x: 1, y: 2}` for an object (its fields), and `ƒ` for a function |
| A trailing `;`, or a declaration | `void` |
| A class or interface | Its name, as `Point` |
| Anything that failed | `fail` |

A value that contains itself prints `↻` where it recurs, and a very deep
or long one ends with `…`, so printing never ends the session.

## Names

Each input sees the names earlier ones declared. Declaring a name again
makes a new one for later inputs, while code that already used the old
one keeps it, so `let s = "${s}!"` reads the earlier `s`:

<!-- repl: restart -->
```console
$ let s = "first"
interactive#0: void
$ let shout(): String { "${s}!" }
interactive#1: void
$ let s = "${s} and second"
interactive#2: void
$ s
interactive#3: "first and second"
$ shout()
interactive#4: "first!"
```

A class declared again works the same way. Its old instances print
their class with the input that declared it, as `interactive#5.Point`,
when a later `Point` hides it.

A top-level `var` can be assigned from later inputs, checked against its
type. A function can't use a name a later input declares, since an
input's names are settled once; mutually recursive functions go in one
input (on one line, or with `;;`).

## Errors

Errors are reported with the input's number, line, and column, and the
line with the span marked:

<!-- repl: restart -->
```console
$ let total: Int = "many"
interactive#0:1:18: error: Cannot assign to Int32 from String
    let total: Int = "many"
                     ^^^^^^
interactive#0: void
$ total + nope
interactive#1:1:9: error: nope has not been declared
    total + nope
            ^^^^
interactive#1: fail
```

As in a library, an input with errors still runs as far as it can, and
the session goes on. A declaration whose input failed stays declared,
and a later read of it fails, saying why:

```console
$ let half = 1 / 0
interactive#2: fail
$ half + 1
Failed: half has no value: interactive#2 failed
interactive#3: fail
```

## Async

Promises and `async` blocks last across inputs, and an input may
`await` at its top level. An input that awaits a promise nothing has
resolved yet prints `1 promise is unresolved.` and `fail`, and goes on
when a later input resolves it:

<!-- repl: restart -->
```console
$ let reply = new PromiseBuilder<String>();
interactive#0: void
$ let answer = await reply.promise
1 promise is unresolved.
interactive#1: fail
$ reply.complete("yes");
interactive#2: void
$ answer
interactive#3: "yes"
```

## Imports

Inputs import std's modules, as `let { Date } = import("std/temporal");`,
and the libraries of the work root by name. The work root is `-w`'s,
or else the one around the current directory, found as `build` finds
it, if there is one.

## Stopping

Ctrl-C stops the input running, and the session goes on. At the
prompt, it drops the input begun so far, once the line is entered. If
tempest itself is ever stuck, pressing Ctrl-C again ends it.

## Options

| Option | Meaning |
|---|---|
| `-w DIR`, `--workroot DIR` | The work root whose libraries inputs import |
| `-ign FILE`, `--ignorefile FILE` | An [ignore file](cli.md#ignore-files) for the work root |
| `-e CODE`, `--exec-before CODE` | An input to run first; may be given more than once |
| `--separator newline\|semisemi\|eof` | What ends an input ([above](#inputs)) |
| `--prompt normal\|hide` | Whether to prompt; by default, only when standard input is a terminal |
| `--fuel N\|none` | The steps each input may take; none by default |
| `-v`, `--verbose` | Report hints too |
| `--std DIR` | Read std from DIR instead of the copy built in |

`temper repl -b BACKEND`, which opens a backend language's own REPL
with the work root's libraries loaded, isn't in tempest yet, and says
so (exit code 3).

## Compared with `temper repl`

The same inputs, results, and `interactive#N:` lines, with these
differences:

- **Names:** classes print as `Point`, not `Point__0`, and a top-level
  `var` carries over to later inputs.
- **Errors:** reported as tempest reports them, `interactive#N:line:column:`,
  and a call of a function whose body has an error says why it failed.
- **Robustness:** a cyclic value prints, Ctrl-C stops only the input
  running, and a crash in tempest is reported without ending the
  session.
- **Piped input** gets no prompt, and nothing echoed.
- **Not yet:** line editing, history, completion, and color; and the
  REPL's own functions, `help`, `describe`, and `translate`.
