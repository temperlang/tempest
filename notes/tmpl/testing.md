# tmpl: testing

Part of the [tmpl design](README.md).

## Level 1: Testing

Proposed, in order of when they're worth building:

- **Round trips,** on every tmpl the tests produce: text to model to
  text is identical; model to binary to model is equal; text with spans
  stripped reads back equal apart from spans.
- **The validator everywhere:** after the frontend's translation and
  after each pass, in tests and debug builds.
- **Golden tests** in `tests/tmpl/*.cases`, as `tests/settle` does:
  Temper source to starting tmpl, and starting tmpl to lowered tmpl per
  profile, compared as canonical text without spans.
- **Shared backend tests** (kotlin-differences.md): one set of inputs
  with expected output per backend, including the known Kotlin bugs
  (Boolean `==`, initialization order, Rust's checked index, C++ forward
  declarations, module `var`s under threads).
- **Fuzzing:** translate mutated inputs as `mutated_inputs_settle` does,
  checking that translation never panics, output validates, and passes
  are deterministic; and random valid tmpl from the schema, lowered
  under random profiles.
- **A tmpl interpreter, later** (milestone-3.md): run a program before
  and after lowering and compare output, and compare with the
  frontend's interpreter on the functional tests.

## Level 3: Golden tests

Wanted by the user (2026-10-29): golden tests of both the canonical
text and the view, which leaves out derivable detail. The prettier and
more stable the names, the more stable the diffs. Proposed layout,
with the `=== name` cases and `UPDATE_SNAPSHOTS=1` that the lexer's
and settle tests use:

- **`crates/temper-tmpl/tests/text/*.cases`:** hand-written tmpl, then
  `--- canonical` (what the writer gives back), `--- view`, and
  `--- problems` (validator output, `RULE @node: message`). Every case
  also round trips: read then write gives the canonical text, and
  write then read gives an equal program. As built (2026-10-29,
  Proposed): a section shows only what isn't the default, so
  `--- canonical` only where the input isn't canonical already, and
  `--- problems` only where there are some; `--- error` shows why
  text doesn't read. `--- view` comes with the view. `basics.cases`
  holds valid programs (formats.md's example among them),
  `problems.cases` the validator's rules, and `errors.cases` the
  reader's. With core's declarations (2026-10-29), `tests/qname.rs`
  checks qnames against Kotlin's examples and their derivation, and
  `tests/core_fixture.rs` checks `tests/fixtures/core.tmpl`, a `uses`
  section for core's members, against the vendored `core.temper`. Beside them, `tests/model.rs` tests what text can't
  express (broken structure, spans), and `tests/round_trip.rs`
  generates random valid programs, which must validate without
  problems and round trip (`TMPL_FUZZ_SEED`, `TMPL_FUZZ_ROUNDS`).
- **`crates/temper-tmpl/tests/lower/*.cases`:** starting tmpl, then per
  variant `--- py` (lowered canonical text), `--- py view`, and
  `--- py names`: each declaration whose output name isn't its source
  name, or that's compiler-made, as `~count#9 int32`.
- **`crates/temper-frontend/tests/tmpl/*.cases`:** Temper source, then
  `--- tmpl` (starting canonical text) and `--- view`, adding lowered
  sections as backends arrive.
- **The suffix count:** a golden file listing every suffixed name per
  variant across the functional tests and std, with the total, so a
  change that adds one shows in review. Chromarite and Ordable are
  counted the same way by a compat check against their checkouts.

Spans are stripped from goldens; a separate test checks P1 and that
lowering keeps every source node's span.

### Backends: goldens, runs, and type checkers

As built with the Python backend (2026-10-30):

- **Goldens:** each backend's output for the functional tests that tmpl
  covers, one file per test, as Kotlin's `…BackendTest` goldens
  (`crates/temper-be-py/tests/goldens/algos-fibonacci.py`), accepted
  with `UPDATE_SNAPSHOTS=1`; layout files are checked by the test.
- **Runs:** each generated program runs against its runtime library and
  must log what the functional test expects of that backend
  (`temper-funtests` honors the log blocks' backend selectors).
- **Type checkers** (the user's direction, 2026-10-30): every backend's
  generated code is checked by that language's checker in its tests,
  automatically, never as a separate manual step. Compiled languages get
  this from their compilers; JS uses TypeScript's (below), and Lua's is
  to be chosen. For Python: pyright in strict mode, locally (where on
  `PATH`) and in CI, and mypy in strict mode too, in CI, where both are
  installed at their latest versions each run, since the two sometimes
  disagree; locally mypy is optional (`MYPY` names one). A test skips a
  missing checker with a message, except in CI, where
  `TEMPEST_REQUIRE_CHECKERS` makes a missing one fail.
- **Rust** (2026-10-30): the compiler, by building every generated
  crate, and clippy, with `-D warnings` but for three lints that fire
  on what the functional tests do on purpose (`self_assignment` for
  `zero = zero` and `unnecessary_min_or_max` for `1.min(3)`, which
  Kotlin's output trips too, and `disallowed_names` for a local named
  `foo`, which Kotlin's output avoids only by numbering every name).
  The crates build offline in one cargo workspace with one target
  directory, kept under `CARGO_TARGET_TMPDIR` so builds are
  incremental, against the vendored `temper-core` and its `Cargo.lock`;
  `temper-core`'s one dependency, `indexmap`, must be in cargo's cache
  (CI fetches it first). The goldens are also checked to be as rustfmt
  lays them out. A missing cargo, clippy, rustfmt, or `indexmap` skips
  with a message, or fails with `TEMPEST_REQUIRE_CHECKERS`. Programs of
  the backend's own (`tests/programs/`) cover what the functional
  tests don't yet; one is hand-written tmpl, for constructs the
  frontend doesn't translate yet.
- **JS** (2026-10-30): each generated package runs under Node (`node`,
  or `NODE`) against the vendored `@temperlang/core`
  (`vendor/temper/be-js/temper-core`), copied into a `node_modules`
  beside the packages, and TypeScript's checker, `tsc --checkJs --allowJs
  --noEmit --strict --module nodenext`, checks all of them in one run
  (`tsc`, or `TSC`), with JSDoc as their only annotations. It checks
  them against declarations (`.d.ts`) that `tsc` generates from the
  runtime's own JSDoc first, with `--skipLibCheck`, since those
  declarations don't all check themselves (`Pair` is declared twice, and
  `Union` is undeclared). CI installs the latest TypeScript with npm,
  on Node 24. A missing Node or `tsc` skips with a message, or fails
  with `TEMPEST_REQUIRE_CHECKERS`. Besides the 7 functional tests, the
  JS backend runs the Rust backend's programs
  (`crates/temper-be-rust/tests/programs/`), read from there, with
  goldens of its own.

### Binary goldens (planned with the binary format)

Proposed (2026-10-29), from the user's question. Besides round trips,
check in a few small binary goldens, so an accidental change to the
format's bytes shows in review, and its version must be raised:

- **Kept small:** a handful of cases covering every section, not every
  text golden.
- **Readable failures:** a mismatch shows a diff of a structured dump
  (`tmpl dump`: each section with its offset and length, then its
  decoded records) of expected against actual, plus the first byte that
  differs, never raw bytes alone. The dump is also checked in beside
  each binary golden, so reviewers see what changed.
- **Compatibility:** binaries from earlier format versions stay as
  fixtures that newer readers must still read, or reject with a clear
  version error, as versioning decides.
