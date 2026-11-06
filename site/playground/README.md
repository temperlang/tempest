# The Temper playground

The whole compiler, as wasm, in the browser: type Temper on the left,
and see every backend's output, the errors, and the tests' results on
the right, updated as you type, with no server.

    uv run site/playground/build.py [--deps DIR]
    python3 -m http.server -d site/_out/playground

`build.py` builds `crates/temper-wasm` (`small` profile, wasm32; set
`RUSTUP_TOOLCHAIN=1.85.0` to match CI), installs the npm packages that
`package-lock.json` pins (into `node_modules/`, or into `--deps DIR`,
linked from there), type-checks the TypeScript with `tsc`, bundles and
minifies it with esbuild, and prints each file's size. CI runs it with
`--skip-wasm` after its wasm build; the Pages workflow publishes it.

## How it fits together

- **The page** (`src/main.ts`): a CodeMirror 6 editor, tabs for each
  backend the wasm build reports (generated files read-only and
  highlighted, with a file picker), Run, and Errors. Typing compiles
  after 300 ms; the source also goes into the URL's hash, compressed,
  for sharing.
- **The compiler's worker** (`src/compiler-worker.ts`) holds the wasm
  instance, so typing never waits on it. One request runs at a time,
  and only the newest one waits; a run that takes too long while a
  newer one waits gets its worker replaced.
- **The wasm API** (`crates/temper-wasm`): `compile` settles the source
  once and returns JSON with the diagnostics (UTF-16 offsets, as
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
  it works" links to on GitHub.

## Next: IDE features

Diagnostics already show in the editor through `@codemirror/lint`.
Hover types, completion, and go to definition need wasm exports that
answer for a position, called from the compiler's worker the way
`compile` is, and CodeMirror's `hoverTooltip`, `autocompletion`
sources, and a key binding on the page.
