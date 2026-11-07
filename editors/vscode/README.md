# Temper for VS Code

Highlighting for Temper (`.temper`) and literate Temper (`.temper.md`),
and tempest's language server, `tempest lsp`, for:

- errors and warnings as you type, for the whole library around a file
  (its `config.temper.md` and every file in it), unsaved edits included;
- hover: what a name is, its type or signature, and its doc comment;
- go to definition, into std's sources too (read-only copies);
- completion of names in scope, members after `.` (builtin types' too),
  and an object literal's property names;
- find references and rename, across the library's files;
- the outline (document symbols), and signature help in calls.

## Install

Each platform's package bundles tempest: `temper-linux-x64-VERSION.vsix`,
and likewise `linux-arm64`, `darwin-x64`, `darwin-arm64`, `win32-x64`,
and `win32-arm64`. The universal `temper-VERSION.vsix` has none, and
uses `tempest` on `PATH`. CI's Binaries workflow builds them all.

Install one with `code --install-extension temper-linux-x64-VERSION.vsix`,
or in VS Code, the Extensions view's `...` menu, "Install from VSIX...",
then reload the window ("Developer: Reload Window") so the new one runs.
Each build has its own VERSION, package.json's major and minor with the
minutes since 2026 as the patch, so a new package always replaces an
older one.

The server is **Temper: Server Path** (`temper.server.path`), a path
or a name on `PATH`, if set; else the bundled tempest, then `tempest`
on `PATH`. If there's none, or it doesn't start, an error says what was
tried and how to fix it, with a button to open the setting. **Temper: Server Args**
(`temper.server.args`) adds arguments, as `["--std", "/path/to/std"]`.
"Temper: Restart Language Server" restarts it; its log is in the Output
panel's "Temper" channel.

## Build

With Node 22 or later, and uv:

```sh
cargo build --profile small -p tempest
uv run editors/vscode/build.py --tempest target/small/tempest --test
```

This writes `editors/vscode/out/temper-VERSION.vsix` and
`out/temper-linux-x64-VERSION.vsix` (on Linux x64), and with `--test`,
runs the tests in a real VS Code. `--binaries DIR` bundles each
`DIR/tempest-RUST_TARGET/tempest[.exe]` in its platform's package.

## How it's built

- `syntaxes/`: the TextMate grammars, checked by `check_grammar.py`,
  whose small tokenizer scopes samples.
- `src/extension.ts`: starts `tempest lsp` with `vscode-languageclient`,
  which syncs documents incrementally and maps VS Code's features to
  LSP's requests. `src/server.ts` finds tempest, tested by
  `src/server.test.ts` under Node's test runner.
- `test/suite.ts`: diagnostics (and an incremental edit), hover,
  definition, completion, and rename, in a real VS Code on a small
  library, run by `test/run.ts` with `@vscode/test-electron` against
  the platform package as built, with its tempest. On Linux without a
  display, it runs under `xvfb-run`, or else Electron's headless mode.
- `build.py`: type checks, bundles with esbuild, tests, and packages
  with `vsce package`, from a staging directory with just what ships.

The architecture is in
[docs/internals/09-language-server.md](../../docs/internals/09-language-server.md).
