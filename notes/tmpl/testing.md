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
  reader's. Beside them, `tests/model.rs` tests what text can't
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
