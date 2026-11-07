# Switching over: the plan to the team preview

The goal is to make tempest good enough that the Temper team chooses
to switch to it. The first step is a **preview for the team**. Update
the status and estimates here as work lands.

## The preview

What the team sees:

- every functional test passing on all seven backends;
- two or three real libraries built by tempest, with their tests
  passing in every language and their APIs matching Kotlin Temper's;
- the scorecard (below);
- the web playground;
- the language server, the full CLI, and architecture docs, shown
  as what's coming, with whatever already works.

## Order and estimates

Estimates are in chunks, each about 1–1.5 hours; an active day fits
3–5 of them.

| # | Work | Chunks | Status (2026-11-06) |
|---|---|---|---|
| 1 | The rest of the functional tests on all backends | 7–10 | done (2026-11-05): 72 of 72 in the interpreter and through tmpl on every backend; std's `net` needs libcurl on C++ and LuaSocket on Lua, without which its 2 tests give Kotlin's expected failures. Then generators that yield values or take parameters, on all seven, each language's own loop seeing the values from host code (2026-11-06). Step by step: [history/tmpl-state.md](history/tmpl-state.md#state) |
| 2 | Build tests and test-runner tests | 1–2 | not started |
| 3 | Real libraries on all backends | 4–8 | started (2026-11-06): finding libraries skips what `.gitignore` files list, so Hrefine's checkout, with a built copy under `bench/build/`, passes 41 of 41 on the interpreter; its Python build gets past divisors that fold to constants, to a template's part of another type (`src/search-params.temper.md:315`); Kelpto passes 100 of 100 on the interpreter, with calls nested up to 2000 deep (500 in wasm); `tempest test -b` runs libraries' tests with each language's own tools on six backends, and Hrefine (41), Chromarite (56), and Ordable (36) pass all theirs in C#, Java, JS, Lua, Python, and Rust, 18 of the 28 library-and-backend cells (2026-11-06); C++ through the shared `spill` pass and CMake, and Kelpto (120) through joins, sealed `when`s, and `?.` on getters, so 28 of 28 (2026-11-06; below, "Status"); Wherewith (27) the fifth, on all seven, after multi-declarator `for` scoping, `@overload` resolution, `continue` in `for … of`, and an untyped `orelse` fallback (2026-11-06) |
| 4 | Scorecard and migration plan | 1 | not started |
| 5 | Web playground | 1–2 | done (2026-11-05): the wasm compiler in a worker, every backend's output, errors, and tests run on the interpreter and on the generated JavaScript, as you type; published to Pages ([site/playground](../site/playground/README.md)); one missing or extra bracket is one error at the gap, with the rest of the file still parsed and typed, not one error to the end of the file (2026-11-06) |
| | **Preview** | **15–25** | |
| 6 | CLI parity | 2–3 | started (2026-11-06): errors stay local, as in Kotlin: `tempest build` writes every backend's code past them and exits with 1, `tempest test` runs the tests on the interpreter and every backend, failing only those that reach an error, and a misspelled member, an undeclared type, or a value of the wrong type fails only where it runs ([kotlin-differences.md](kotlin-differences.md), "Keep on trucking") |
| 7 | Language server | 3–5 | done for the preview (2026-11-07): the shared layer, the `temper-analysis` crate over the frontend only (diagnostics, hover with kinds, types, signatures, and doc comments, go to definition, and completion of names in scope and class members), answering from one settled library; in the playground through wasm exports, with diagnostics shown before the run and backends ([docs/internals/08-playground.md](../docs/internals/08-playground.md#editor-queries)) (2026-11-06); `tempest lsp` over stdio, on the library around each file with unsaved edits, and find references, rename across files, outlines, signature help, object literals' property names, generic receivers' type arguments, builtin types' members, completion after `.` on any expression, and definition into std, in the server and the playground; TextMate grammars and a VS Code extension, packaged as a `.vsix` ([docs/internals/09-language-server.md](../docs/internals/09-language-server.md)) (2026-11-07); the server on `serde_json` and `lsp-types` with incremental sync, the extension on `vscode-languageclient`, platform packages with tempest bundled, built by the Binaries workflow, and tests in a real VS Code (2026-11-07) |
| 8 | Clarity refactors and architecture docs | 2–4 | started (2026-11-06): state machines dispatch by a tmpl `Switch` (Rust `match`, C++ and Java `switch`; `if` chains elsewhere, by profile), with literal branches folded and unreached or jump-only states dropped; simplifications before the docs (2026-11-06): stale module docs and design.md sections 5 and 9 refreshed, each backend described once (`temper_tmpl::Backend`, its crate's `BACKEND`) and listed once (the `tempest` crate, now a library too), and one compile path for `tempest build`, the one-backend commands, and the wasm; shared hoisting and function literals' lowering wait until after the preview; architecture docs (2026-11-06): [docs/internals/](../docs/internals/README.md), eight numbered pages from source to seven languages and the playground, with Mermaid diagrams, links to the code that CI checks (`compat/check_anchors.py`), and `triple.temper` as their running example, a playground example and a shared test program whose goldens are the pages' output; design.md's language decisions moved to [language.md](language.md); a docs audit (2026-11-07): DESIGN.md folded into the README, finished milestones and the tmpl state log moved to [history/](history/README.md), and the parser's Kotlin differences merged into [kotlin-differences.md](kotlin-differences.md#lexer-and-parser) |
| 9 | One tier-2 backend | 2–3 | not started |

Risks to the date:
- **`async`:** done on all seven backends (2026-11-05), with closures
  made in a generator's loop each getting that pass's `let` on the
  state-machine backends too (2026-11-05), and one turn at a time on
  every backend, with host code's own `await` and loops (2026-11-05),
  tested from hand-written host code on all seven (2026-11-05).
- **Library surprises:** real libraries use much more of Temper than the
  functional tests do.
- **Kotlin Temper's main moving:** each sync with it costs about a chunk.
  tempest's fixes to the vendored files are patches each sync
  re-applies, and `sync_temper.py --check` (in CI) checks
  (2026-11-04).

## Each step

1. **The functional tests:** done: all 72 pass on the interpreter and
   every backend; how they got there is in
   [history/tmpl-state.md](history/tmpl-state.md#state).
2. **Build and test tests:** `tempest build` and `tempest test` on
   small projects in every backend, checked like Kotlin Temper's.
3. **Real libraries.**
   - Chromarite, Ordable, Hrefine, Kelpto, and Wherewith build with tempest, and
     their test suites pass in every language.
   - Each library's public API matches Kotlin Temper's per backend
     ([api-compat.md](api-compat.md)); every difference is fixed or
     written down with its reason.
   - Packaging too: `pyproject.toml`, `Cargo.toml`, `package.json`,
     the pom, and the `.csproj`, so the output can be published, not
     just run. Dependencies are written as Kotlin does; after the
     preview, close the gaps for publishing: C# references sibling
     project folders rather than versioned packages, versions should
     follow each registry's rules, and C++ has no package metadata
     (vcpkg or Conan, for later consideration, since neither is
     clearly well used).
   - Status (2026-11-06): `tempest test -b BACKEND` builds into
     `temper.out` and runs the tests with the language's own tools
     (each backend crate's `src/tools.rs`, run by
     `crates/temper-driver/src/test_run.rs`, C++ through CMake); `tempest
     build -v` lists every gap in tmpl, not only the first; a library
     that doesn't build still counts its tests, as not run. Passing, all
     tests, all 28 cells: Hrefine (41), Chromarite (56), Ordable (36),
     and Kelpto (120) on C++, C#, Java, JS, Lua, Python, and Rust. C++
     needed the shared `temper_tmpl::spill` pass to make choices' parts
     and loops' conditions statements where its order of evaluation
     needs temporaries (`spill_with`, its own marks), and core's list
     and map reads read rather than write, and Kelpto needed typing's
     joins of classes, sealed `when`s without `else`, a `[]` branch
     typed by the other, `?.` on a getter's bare name, and a literal's
     unbound result as its bound, as Kotlin; then backend fixes: every
     member of another library's interface a class extends in tmpl's
     `uses`, which Rust's trait impls and C++'s classes fill; private
     static functions that static fields' values call, reachable from
     the module's init (JS, Python, C#, Java); Lua's null defaults kept
     as `temper.null`; and C++ headers spelling `core::` only as a whole
     name (`kelpto_core::`). Wherewith (27 tests) passes on all seven
     too (35 of 35 cells), after typing resolved `@overload` calls to
     the methods' own names, as Kotlin (typing.md, "Overloads"), a
     multi-declarator `for` saw its earlier declarations, `continue` in
     a `for … of` targeted the inlined loop, and an `orelse` fallback's
     `??` took its branches' type. A private library (119
     tests) passes 119 of 119 on the interpreter and on C++, C#,
     Java, JS, Python, and Rust (2026-11-06; Lua skipped), once one undeclared name in it
     is fixed, which Kotlin folds away silently and tempest reports (so
     as committed, 115 on the interpreter, and no backend builds). That
     took `@overload`s of lists and of classes told apart, a dead `if
     (DEBUG)` branch's undeclared names as warnings, generic `is`
     checks, identity of nullable generic values, `[]` and list literals
     typed by a generic call's solution, a bare `List` typed by its
     value, `new Alias(…)` of a generic class, `when`s of only `null`
     and `panic()`, `orelse continue`, `--i` as a value, inherited
     generic defaults in Python and C#, and a fix or two in each
     backend (C#'s `Core` beside a module `core`, Rust's printer no
     longer exponential in call depth, C++'s private templates in the
     header), plus three runtime fixes from Kotlin branches as vendor
     patches 0021 to 0023. Real libraries run nightly
     on GitHub, from a list ([test-speed.md](test-speed.md), "Real
     libraries, nightly").
   - Kotlin Temper bugs these libraries hit are fixed in tempest's
     output, not worked around. One example: Chromarite's per-language
     immutability check (`demos/immutable/`), where Kotlin leaks in
     C++, JS, Lua, and Python, should pass everywhere with tempest.
4. **The scorecard,** one page measured on the same libraries:
   - build and test time, cold and warm, and the watch loop (tracked
     per CI run since 2026-11-04: [size-and-speed.md](size-and-speed.md),
     "Charts"; Python's functional tests, with strict mypy and pyright,
     down from 432 to 8 seconds in the CI run by batching the checks
     and running cases in bulk, 2026-11-05, and C++'s from about 213
     to 71–104 seconds (`runs`), with the sanitizers' build in bulk
     and ThreadSanitizer's of `module-state` alone, 2026-11-05:
     [test-speed.md](test-speed.md));
   - peak memory;
   - the Kotlin Temper bugs tempest fixes ([kotlin-differences.md](kotlin-differences.md),
     each fix with the test that pins it, 2026-11-04);
   - error messages side by side on common mistakes;
   - what's still missing.

   Plus a migration plan: both compilers run in CI for a while, and
   their generated output is diffed.
5. **The web playground.** The wasm build runs the whole compiler in
   the browser: type Temper, and see every backend's output update
   as you type, with no server. Done (2026-11-05): tests run on the
   interpreter and on the generated JavaScript, side by side, and
   diagnostics show in the editor. IDE features (2026-11-06): hover
   types, go to definition, and completion in the playground, over one
   shared layer, the `temper-analysis` crate, that the language server
   reuses; references, rename, and signature help too (2026-11-07).
6. **CLI parity.** `init`, `build`, `test`, `watch`, `repl`, and doc
   generation, with Kotlin Temper's flags and output layout.
7. **A language server** ([design.md](design.md), CLI/watch/LSP).
   Kotlin Temper's is flaky, slow, and uses a lot of memory. Tempest's
   should be fast, steady, and small: diagnostics as you type, hover
   types, go to definition, and completion, on the same incremental
   core as `watch`. Status (2026-11-07): `tempest lsp` and the VS Code
   extension ([docs/internals/09-language-server.md](../docs/internals/09-language-server.md));
   it settles a library again on each pause in typing, which is fast
   enough for the libraries we have, rather than incrementally.
8. **Architecture docs** that a newcomer can take in quickly:
   - a few diagrams (source to tree, the settle loop, typing, tmpl,
     backends, and how `watch` and the LSP reuse it);
   - a short paragraph per stage;
   - links to the code (crate, module, and key types and functions)
     at each step.

   They live in the repo and stay current with the code. Where the code
   is hard to explain simply, refactor it first, so the docs can stay
   short. Each refactor is its own change, apart from features.

   Status (2026-11-06): written, in [docs/internals/](../docs/internals/README.md). Spots
   still hard to describe simply, each a refactor of its own: each
   backend's `translate` calling the shared lowerings in its own order;
   C++'s, C#'s, and Lua's statements inside expressions split between
   `spill` and each translator (shared hoisting); `test_run.rs` keeping
   a second list of languages; and the wasm settling source its own
   way, with older single-purpose calls beside `compile`.
9. **One tier-2 backend** ([tmpl/targets.md](tmpl/targets.md)), such as
   Go or Swift, to show that new backends are cheap. It comes after the
   parity case is made, so it adds to the case for switching rather
   than to the team's maintenance worries.
