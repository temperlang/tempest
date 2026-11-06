# 8. Wasm, the playground, watch, and a language server

The whole compiler also builds to wasm, about 4.3 MB (1.4 MB
gzipped), and runs in a browser with no server. The
[playground](../../site/playground/README.md) uses it: type Temper on the
left, and see errors, the program's output, its tests' results, and all
seven backends' code on the right, updated as you type. The editor also
underlines errors, shows types on hover, goes to definitions, and
completes names.

```mermaid
flowchart LR
  editor["Editor page: main.ts"] -->|"source, after 200 ms"| worker["Compiler worker: compiler-worker.ts"]
  worker --> analyze["temper-wasm: analyze"]
  analyze -->|"diagnostics, first"| editor
  analyze --> finish["finish: run and backends"]
  finish -->|"the rest"| editor
  editor -->|"hover, definition, completions"| worker
  editor -->|"Run"| jsworker["JS worker: runs the generated JavaScript"]
```

## One compile, everything at once

The page settles the source once, then reuses that one settled library
for everything. [`compile`](../../crates/temper-wasm/src/lib.rs) does it
all in one call; the page calls it in two halves,
[`analyze`](../../crates/temper-wasm/src/lib.rs) and then
[`finish`](../../crates/temper-wasm/src/lib.rs), so errors show before
the slower run and backends:

1. its diagnostics, with positions counted as JavaScript and editors
   count them;
2. if asked, a run on the [interpreter](04-interpreter.md), with its
   tests;
3. tmpl, translated by each backend the page asks for, through the
   same [compile path](07-cli.md#one-compile-path) as `tempest build`.

std is settled the first time it's needed and kept, since it never
changes and settling it takes most of a translation's time.

The answer is one JSON object, shaped like this:

```json
{"diagnostics": [],
 "run": {"output": ["triple(14) = 42"], "failure": null},
 "backends": {"py": {"files": [{"path": "main/…", "text": "…"}]},
              "rust": {"files": ["…"]}}}
```

`analyze` gives just `{"diagnostics": [...]}`, and `finish` the rest.

Run shows the interpreter's results beside the generated JavaScript's,
run in a worker of its own. So the playground is also a quick check that
two very different ways of running a program agree.

## The page

- [`main.ts`](../../site/playground/src/main.ts): CodeMirror 6, a tab per
  backend, Run, errors in the editor, and the source in the URL for
  sharing.
- [`compiler-worker.ts`](../../site/playground/src/compiler-worker.ts):
  holds the wasm, so typing never waits on it. Only the newest request
  waits; a run that takes too long gets its worker replaced.
- [`js-worker.ts`](../../site/playground/src/js-worker.ts): runs the
  generated JavaScript, with the runtime from `vendor/temper/be-js/`.
- [`build.py`](../../site/playground/build.py): builds the wasm and bundles
  the page. GitHub Pages publishes it from `main`.

## Editor queries

`analyze` keeps the settled library in the wasm, inside an
[`Analysis`](../../crates/temper-analysis/src/lib.rs), until the
next source. The editor asks it questions at a position, and the
answers come from what settling already worked out, with nothing
settled again:

- [`diagnostics`](../../crates/temper-analysis/src/lib.rs): every
  error and warning, at its span in its file. The editor underlines all
  of them, and settling keeps going past errors, so later code still
  gets types.
- [`hover`](../../crates/temper-analysis/src/lib.rs): for a name,
  its declaration's kind (`let`, `var`, `function`, `parameter`,
  `method`, `field`, `class`, and so on), its type, or a function's
  signature with its parameters' names, and its doc comment; for other
  code, the smallest typed expression's type.
- [`definition`](../../crates/temper-analysis/src/lib.rs): where a
  name, or a member after `.`, is declared. In the playground, F12, or
  Ctrl or Cmd and click.
- [`completions`](../../crates/temper-analysis/src/lib.rs): the
  names in scope, innermost first, or after `x.`, the members of `x`'s
  class.

A query names the node at the position from each name's symbol
([`Scopes`](../../crates/temper-frontend/src/scope.rs)), each member
read's resolution, and each declaration's and expression's type
([`Typing`](../../crates/temper-frontend/src/typing.rs)).

`Analysis` counts bytes in a file. Editors count UTF-16 code units, so
the wasm converts once, at its boundary, with
[`Position`](../../crates/temper-analysis/src/lib.rs) and
[`byte_offset`](../../crates/temper-analysis/src/lib.rs). A query
waits until the worker has settled the text on screen, so positions
always match.

## Coming: `watch` and a language server

Neither exists yet. Both are meant to reuse what's here rather than add
a second compiler:

```mermaid
flowchart LR
  disk["Disk"] --> sources["Sources: layers"]
  buffers["Unsaved editor buffers"] --> sources
  sources --> settle["Settle the library"]
  settle --> analysis["Analysis: diagnostics, hover, definition, completions"]
  analysis --> lsp["Language server"]
  analysis --> playground["Playground"]
  settle --> compile["Compile path and test runs"]
  compile --> watch["watch: rebuild and retest"]
```

- **Watch** may not need to keep anything in memory. If building a
  library takes tens of milliseconds, rebuilding on each change is
  simplest: `watch` is then a loop around `build` or `test`, printing
  `Analysis::diagnostics`.
- **Unsaved buffers** become a memory layer of
  [`Sources`](../../crates/temper-frontend/src/sources.rs) in front of
  the disk, as the playground's source already is, so a build sees them
  without writing files.
- **A language server** wraps the same `Analysis` queries, with LSP's
  line and UTF-16 column positions converted at its boundary as the
  wasm's are.

## Where in the code

- [`compile`](../../crates/temper-wasm/src/lib.rs) and [`compile_into`](../../crates/temper-wasm/src/lib.rs): everything in one call; [`analyze`](../../crates/temper-wasm/src/lib.rs) and [`finish`](../../crates/temper-wasm/src/lib.rs): the same in the playground's two halves.
- [`hover`](../../crates/temper-wasm/src/lib.rs), [`definition`](../../crates/temper-wasm/src/lib.rs), and [`completions`](../../crates/temper-wasm/src/lib.rs): the editor's queries as wasm exports, in UTF-16.
- [`temper-analysis`](../../crates/temper-analysis/src/lib.rs): the queries themselves, in their own crate over the frontend only, for the playground now and a language server later. Tests: `crates/temper-analysis/tests/analysis.rs`.
- [`std_program`](../../crates/temper-wasm/src/lib.rs): std, settled once and kept.
- [`build.rs`](../../crates/temper-wasm/build.rs): a 16 MiB stack for the wasm build.
- Older single-purpose calls, `parse`, `run`, and `generate`, are in the same file.

More: [site/playground/README.md](../../site/playground/README.md), and
[notes/design.md](../../notes/design.md), section 11, on watch and the
language server.
