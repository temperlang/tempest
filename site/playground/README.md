# The Temper playground

The whole compiler, as wasm, in the browser: type Temper on the left,
and see every backend's output, the errors, and the tests' results on
the right, updated as you type, with no server.

    uv run site/playground/build.py [--deps DIR] [--smoke [--chromium PATH]]
    python3 -m http.server -d site/_out/playground

`build.py` builds `crates/temper-wasm` (`small-wasm` profile, wasm32; set
`RUSTUP_TOOLCHAIN=1.85.0` to match CI), installs the npm packages that
`package-lock.json` pins (into `node_modules/`, or into `--deps DIR`,
linked from there), type-checks the TypeScript with `tsc`, bundles and
minifies it with esbuild, and prints each file's size. CI runs it with
`--skip-wasm` after its wasm build; the Pages workflow publishes it,
with the docs site around it ([site/docs](../docs/build.py)), at
`playground/` under it, so the top bar's Temper logo links to the site's
home (`../`); served alone, that link just leads back to the playground.

## How it fits together

- **The page** (`src/main.ts`): a CodeMirror 6 editor, tabs for each
  backend the wasm build reports (generated files read-only and
  highlighted, with a file picker), Run, and Errors. Typing compiles
  after 200 ms; the source also goes into the URL's hash, compressed,
  for sharing.
- **The compiler's worker** (`src/compiler-worker.ts`) holds the wasm
  instance, so typing never waits on it. One request runs at a time,
  and only the newest one waits; a run that takes too long while a
  newer one waits gets its worker replaced.
- **The wasm API** (`crates/temper-wasm`): `compile` settles the source
  once (the page calls it in two halves, `analyze` and `finish`, so
  errors show first) and returns JSON with the diagnostics (UTF-16 offsets, as
  editors count, plus line and column), what the interpreter logged and
  its tests' results, as `tempest test` reports them, and each backend's
  files. `std_library` gives std's own JavaScript, once. Their types are
  in `src/api.ts`, narrowed from the JSON where it comes in.
- **Run** shows the interpreter's results beside the generated
  JavaScript's, run in a worker of its own (`src/js-worker.ts`): each
  generated module becomes a blob URL, with imports rewritten to the
  library's, std's, and the bundled `@temperlang/core` runtime from
  `vendor/temper/be-js/`. Its tests register with `it`, as under Mocha.
- **Highlighting:** Temper's is a small stream language
  (`src/temper-language.ts`); the backends' are CodeMirror's legacy
  modes.
- **Examples** are `examples/*.temper`, bundled as text; each starts
  with a `// Title` comment. `7-triple.temper` is the running example of
  [the architecture docs](../../docs/internals/README.md), which the footer's "How
  it works" links to on the docs site, beside the playground.

## IDE features

- **Errors** are underlined as you type, all at once, through
  `@codemirror/lint`, and listed in the Errors tab.
- **Hover** over a name for its kind, type (a function's signature),
  and doc comment, or over other code for its type.
- **Go to definition:** F12, or Ctrl or Cmd and click. A declaration
  in std opens std's file, read-only, on the right; pick a tab to go
  back.
- **Completion:** names in scope as you type, or Ctrl-Space; after
  `x.`, the members of whatever `x` is (builtin types' too, and a
  generic class's with its type arguments); in an object literal, the
  property names its class takes.
- **References:** Shift+F12 marks every use of a name; Escape clears.
- **Rename:** F2 asks for the new name and renames every use.
- **Signature help:** typing `(` or `,` in a call shows its signature,
  with the current argument in bold.

The worker keeps the source it last settled, and the wasm's `hover`,
`definition`, `completions`, `references`, `rename`, and
`signature_help` answer from it, at UTF-16 offsets
([docs/internals/08-playground.md](../../docs/internals/08-playground.md#editor-queries)).

## Tests

- **Every example works:** `crates/temper-wasm/tests/examples.rs`
  compiles each file in `examples/` as the page does, with no
  diagnostics, every backend's files, and the interpreter's output and
  tests' results pinned. A new example needs its line there.
- **A smoke test in a browser:** `smoke.mjs` serves the build with a
  tiny static server and drives headless Chromium through it, with
  `site/docs/mermaid`'s playwright-core: each example fills every
  backend's tab, has no errors, and runs, with both runs agreeing; then
  a hover, a permalink across a reload, and the Temper link. Console
  errors and failed requests fail it. It takes 8 to 13 s.

      node site/playground/smoke.mjs [DIR] [--chromium PATH]

  or `build.py --smoke`. Chromium is `--chromium`, else
  `$TEMPEST_CHROMIUM`, else Playwright's (`npx playwright-core install
  --only-shell chromium` in `site/docs/mermaid`). CI runs both.
