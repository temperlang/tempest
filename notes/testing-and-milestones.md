# Testing, milestones and risks

Part of the [rewrite plan](README.md).

## 14. Testing and checking compatibility

- **Oracles:** small Java programs that call Kotlin Temper's jars give
  reference results per file: literate blocks (done), then tokens (done),
  then rough tree shape. Each layer is diffed on every Temper source file in the Temper
  and Prismora repos.
- **Functional tests:** the 69 markdown tests, run in-process (interpreter)
  and on every backend, comparing `log` output, with the same per-backend
  skips as Kotlin's `FunctionalTestStatus.kt` to start with.
- **API diffs:** for each backend, extract the public surface from Kotlin's
  and the new output for the same libraries (functional tests, std,
  Prismora), and diff them. Every intended change is listed in an allowlist
  with its reason.
- **Semantics and error messages,** compared against Kotlin Temper from
  several sources:
  - **Frontend stage tests** (`frontend/src/commonTest/.../stage-tests/` and
    the frontend unit tests): inputs with expected diagnostics and results.
    Use them for what the language does and what errors say, not for their
    intermediate representations.
  - **Backend tests** (`JsBackendTest`, `RustBackendTest` and the rest):
    expected generated code, which pins down public API and behavior.
  - **REPL tests** in the CLI module: expected REPL output and error text.
  - **Running Kotlin Temper itself,** including `temper repl` scripted
    through stdin, on the same inputs as the new implementation. That makes a
    differential oracle for values, types and messages on any snippet, not
    just existing tests. Small differences in wording are allowed where
    deliberate and listed.
- **Unit tests** in Rust for each crate, written from behavior, borrowing
  cases from Kotlin's tests where they pin down semantics.

## 15. Milestones

1. **Literate split.** Done.
2. **Lexer and parser** for every source file in both repos, checked against
   Kotlin's tokens and tree shape.
   - 2a, lexer: done. `compat/check_lex.py` matches Kotlin's raw lexer
     (before the parser's token adapters) on all 814 files, about 212k
     tokens, including kind, byte span, synthetic and bracket flags, and
     error kind. Also checked on inputs from Kotlin's lexer tests. Gap: Kotlin
     marks identifiers not in NFKC as errors; we don't, lacking Unicode
     normalization tables (no such identifiers in the corpus).
   - 2b, parser: next, including the token adapters (comment grouping,
     string fixing, semicolon insertion, call joins, modifier words).
3. **Settle loop** with names, core macros (`let`, `fn`, `if`, `while`,
   `class`, `import`), the interpreter and error nodes, running the simplest
   functional tests in-process. This decides whether the design holds up.
4. **Types in the loop**, enough for most functional tests in-process.
5. **tmpl and one backend** (Python or JS) passing the functional tests, with
   an API diff for that backend.
6. **The other backends**, one at a time, each with its API diff.
7. **Watch, incremental rebuilds and the language server.**

## 16. Risks and open questions

- **Settle loop convergence** with macros that rewrite code: which facts must
  be monotonic, how to detect oscillation, and how an error can unblock
  waiting items without hiding the real cause. Settle with small experiments
  in milestone 3.
- **Type inference as incremental steps:** per-expression versus per-function
  solving. Start per call site, and fall back to per function where needed.
- **Semantic fidelity** beyond the functional tests: generated-code details,
  diagnostics, typing edge cases. Mitigated by API diffs and behavior tests
  ported from Kotlin's unit tests.
- **Backend simplicity:** whether shared lowerings absorb enough to keep each
  printer small, especially for C++ and Rust.
- **Two implementations at once** while Kotlin Temper remains the product.
