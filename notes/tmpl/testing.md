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
