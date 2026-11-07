# 9. The language server and editors

`tempest lsp` is a language server: an editor starts it, talks to it
over stdin and stdout, and gets errors as you type, hover, go to
definition, completion, find references, rename, an outline, and
signature help. It answers from the same
[`Analysis`](../../crates/temper-analysis/src/lib.rs) as the
[playground](08-playground.md#editor-queries), so both always agree.
`editors/vscode/` holds a VS Code extension that starts it, with
TextMate grammars for highlighting.

```mermaid
flowchart LR
  vscode["VS Code: extension.ts, vscode-languageclient"] -->|"JSON-RPC over stdio"| loop["tempest lsp: serve"]
  loop --> docs["Open documents"]
  docs --> sources["Sources: memory layer, then disk, std built in"]
  sources --> unit["The library around a file"]
  unit --> settle["Settle"]
  settle --> analysis["Analysis"]
  analysis -->|"diagnostics, answers"| loop
  playground["Playground: wasm"] --> analysis
```

## One loop, no async

The server is one plain loop, [`serve`](../../crates/temper-lsp/src/lib.rs),
in the `temper-lsp` crate, which `tempest lsp` runs. A thread reads
messages, each a `Content-Length` header and JSON, and hands them over
a channel; the loop waits for the next message or a timer, whichever
comes first. Messages are `serde_json` values, and their params and
results `lsp-types`' structs; no async runtime. Only the native
`tempest` uses the crate, so the wasm build, which answers the
playground itself, doesn't carry it or those crates.

## What gets analyzed

- **Open documents** live in memory, kept current by incremental sync:
  each change replaces a range ([`apply_change`](../../crates/temper-lsp/src/lib.rs)).
  They're a memory layer of
  [`Sources`](../../crates/temper-frontend/src/sources.rs) in front of
  the disk, so a library sees unsaved edits in every file.
- **A unit** is what's settled together: the library around a file,
  found as `tempest build` finds libraries (the nearest
  `config.temper.md`, under its work root), or else the file alone.
  One [`Analysis`](../../crates/temper-analysis/src/lib.rs) answers for
  all of its files, so references and rename cross files.
- **Staying current:** a change marks its unit stale. Once typing
  pauses for 250 ms, stale units are settled again and their
  diagnostics published, for each of the unit's own files (not std's).
  A question about a stale file settles it first, so answers always
  match the text on screen.
- **Positions** are LSP's lines and UTF-16 characters only at the
  edge ([`offset_at`](../../crates/temper-lsp/src/lib.rs),
  [`position`](../../crates/temper-lsp/src/lib.rs)); everything inside
  counts bytes in a file.
- **std** is built into the binary, so it has no files to open. Go to
  definition into std writes std's sources once, read-only, to a
  directory in the system's temporary directory, and points there.

## The queries

Each is a method of `Analysis`, shared by the server and the playground:

- [`diagnostics`](../../crates/temper-analysis/src/lib.rs), [`hover`](../../crates/temper-analysis/src/lib.rs), [`definition`](../../crates/temper-analysis/src/lib.rs), and [`completions`](../../crates/temper-analysis/src/lib.rs), as on [the playground's page](08-playground.md#editor-queries).
  A generic class's members show its type arguments
  (`push(item: String)` on a `Stack<String>`); builtin types' members
  come from core's declarations
  ([`builtin_members`](../../crates/temper-frontend/src/typing/core_members.rs));
  and completion after `.` works on any expression whose type is known.
- [`references`](../../crates/temper-analysis/src/references.rs) and
  [`rename`](../../crates/temper-analysis/src/references.rs): every
  name that resolves to the same declaration, imports followed to what
  they import. Rename refuses builtins, std, and keywords, and renames
  object literals' keys for a constructor property too.
- [`document_symbols`](../../crates/temper-analysis/src/symbols.rs):
  a file's declarations, classes with their members.
- [`signature_help`](../../crates/temper-analysis/src/signatures.rs),
  and completion of an object literal's property names: both read the
  brackets open at the cursor from the text
  ([`open_brackets`](../../crates/temper-analysis/src/signatures.rs)),
  since a call being typed hasn't settled into a call yet.

## The VS Code extension

- **Highlighting:** TextMate grammars for `.temper` and literate
  `.temper.md`, where Temper is in fenced blocks (untagged or tagged
  `temper`, not `inert`) and indented blocks, as the
  [literate split](../../crates/temper-syntax/src/literate.rs) reads
  them. Their keywords match the playground's.
  `editors/vscode/check_grammar.py` scopes samples with a small
  TextMate tokenizer.
- **The client** ([`extension.ts`](../../editors/vscode/src/extension.ts))
  starts `tempest lsp` with `vscode-languageclient`. The server is
  `temper.server.path` if set, then the tempest bundled in the package,
  then `tempest` on `PATH`, only a file that's there
  ([`search`](../../editors/vscode/src/server.ts)); none, or one that
  doesn't start, is an error notification naming what was tried, with
  a button to the setting.
- **Packaging:** `editors/vscode/build.py` checks the types, bundles
  with esbuild, and runs `vsce package`: a universal `.vsix` without
  tempest, and one per platform with it in `bin/` (linux, darwin, and
  win32, each x64 and arm64), which the Binaries workflow builds from
  its release binaries.
- **Tests in VS Code:** with `--test`, `test/suite.ts` runs in a real
  VS Code (`@vscode/test-electron`) against this platform's package,
  checking diagnostics, hover, definition, completion, and rename.
  How to install and build: [editors/vscode/README.md](../../editors/vscode/README.md).

## Where in the code

- [`temper-lsp`](../../crates/temper-lsp/src/lib.rs): the server; [`serve`](../../crates/temper-lsp/src/lib.rs), [`read_message`](../../crates/temper-lsp/src/lib.rs), and [`write_message`](../../crates/temper-lsp/src/lib.rs).
- [`temper-analysis`](../../crates/temper-analysis/src/lib.rs): the queries, with [`members`](../../crates/temper-analysis/src/members.rs), [`references`](../../crates/temper-analysis/src/references.rs), [`symbols`](../../crates/temper-analysis/src/symbols.rs), and [`signatures`](../../crates/temper-analysis/src/signatures.rs).
- Tests: `crates/temper-analysis/tests/analysis.rs` (the queries) and `crates/temper-lsp/tests/lsp.rs` (a scripted session with the server).
- `editors/vscode/`: the grammars, the extension, and their tests.
