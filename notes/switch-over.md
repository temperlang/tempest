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
| 1 | The rest of the functional tests on all backends | 7–10 | 56 of 72 through tmpl and all backends (core types; connected host files and fallbacks, with `--no-connected`, 2026-11-02; divisions that can't fail and maps, and labeled loops and local classes, 2026-11-03); 58 of 72 through tmpl (other libraries' class members and constructors, and their names from the library's own naming, 2026-11-03), the 4 more waiting on std translated as a library: its `testing` module now builds as its own library, named by its own naming, with each backend connecting std's members from its own table (2026-11-03); backends run tests against std, comparing failures with `expectedFailures`: `testing/asserts` passes on all seven (2026-11-03), so 54 of 72 through every backend; std's `regex` translates (`AnyValue`, `doPure`), and `regex/match` and `regex/zero-advance` pass on Python and JS (2026-11-03), then on Rust, Java, C#, and C++, with Kotlin's support code vendored (2026-11-03), then on Lua, with jumps out of an `orelse` body (2026-11-03; `regex/zero-advance` as Kotlin's expected failure on Lua, as C++'s `regex/match`), so 56 of 72 through every backend; std's `json` translates, and `types/json-syntax-tree` passes on Python, JS, and Rust (2026-11-04), then on Java (boxed overrides), C# (`Optional<T>`), C++, and Lua, every backend building all of std (2026-11-04), so 57 of 72 through every backend; std's `temporal` translates, with `Date` a connected type on Python, JS, Java, and C# and a class elsewhere, and `types/date` passes on all seven (2026-11-04), so 58 of 72 through every backend, all that tmpl translates; locals declared without a type or value typed from their initializers, and `semantics/constness` passes on all seven (2026-11-04), so 59 of 72 through every backend; builtin values as `AnyValue` and `List.of`, and `casts/as-expr` and `types/list/operations` pass on all seven (2026-11-04), so 61 of 72 through every backend; generators and `async` through tmpl, with each backend's coroutine strategy, the five generator tests passing on Python and Rust (2026-11-05), so 66 of 72 through tmpl, 61 through every backend; then on JS, Lua, C#, C++, and Java (2026-11-05), so 66 of 72 through every backend; closures in a generator's loop each get that pass's `let`, a failed step stays failed, and C++20's `co_await` has a test (2026-11-05); local functions that use each other or themselves, as groups (a struct with methods in Rust and C++, a local class in Java, forward `local`s in Lua), and `functions/locals` passes on all seven (2026-11-05), so 67 of 72 through every backend; host code on every backend uses generated promises and generators the language's way, in each backend's `tests/host/`, and C# gets a pluggable runner and a blocking check (2026-11-05); std's `net` translates, against one test server in the testkit, with no request blocking a turn, and both `netresponse` tests pass on Python, JS, Rust, Java, and C#, and give Kotlin's expected failures on C++ and Lua, which have no `net` (a broken promise; 2026-11-05), so 69 of 72 through tmpl and those five, and 67 plus those 2 on C++ and Lua; std's `net` on C++ and Lua through a pluggable client, the prototype of pluggable I/O (concurrency.md, "What std's `net` on C++ and Lua taught"): Lua's default is LuaSocket's, and both tests pass there, so 69 of 72 on Lua where LuaSocket loads (Lua 5.4; LuaJIT and CI's Lua without LuaRocks keep the expected failures); C++'s is libcurl's, optional, so 67 plus those 2 on C++ until the container has libcurl's headers, with both passing from host code with a client of its own (2026-11-05); a library with errors through tmpl's `Error`, which fails as `bubble()` does, and `semantics/broken` (a failed run, as expected) and `semantics/type-checked-locals` pass on all seven, with every backend's checks clean (2026-11-05), so 71 of 72 through tmpl and every backend (on C++, and on Lua without LuaSocket, the 2 `netresponse` tests as Kotlin's expected failures); CI's Lua 5.4 gets LuaRocks and LuaSocket (2026-11-05); types as values (tmpl's `Type` and `TypeValue`, each language's own value), C++'s generic methods, and Rust's phantom type parameters, and `classes/angle-call` passes on all seven (2026-11-05), so 72 of 72 through tmpl and every backend, and on C++ with libcurl's headers, as the container now has, both `netresponse` tests pass through libcurl; 72 of 72 in the interpreter; generators that yield values or take parameters through tmpl on all seven, each language's own loop seeing the values from host code (2026-11-06) |
| 2 | Build tests and test-runner tests | 1–2 | not started |
| 3 | Real libraries on all backends | 4–8 | started (2026-11-06): finding libraries skips what `.gitignore` files list, so Hrefine's checkout, with a built copy under `bench/build/`, passes 41 of 41 on the interpreter; its Python build gets past divisors that fold to constants, to a template's part of another type (`src/search-params.temper.md:315`); Kelpto passes 100 of 100 on the interpreter, with calls nested up to 2000 deep (500 in wasm); `tempest test -b` runs libraries' tests with each language's own tools on six backends, and Hrefine (41), Chromarite (56), and Ordable (36) pass all theirs in C#, Java, JS, Lua, Python, and Rust, 18 of the 28 library-and-backend cells (2026-11-06); C++ through the shared `spill` pass and CMake, and Kelpto (120) through joins, sealed `when`s, and `?.` on getters, so 28 of 28 (2026-11-06; below, "Status"); Wherewith (27) the fifth, on all seven, after multi-declarator `for` scoping, `@overload` resolution, `continue` in `for … of`, and an untyped `orelse` fallback (2026-11-06) |
| 4 | Scorecard and migration plan | 1 | not started |
| 5 | Web playground | 1–2 | done (2026-11-05): the wasm compiler in a worker, every backend's output, errors, and tests run on the interpreter and on the generated JavaScript, as you type; published to Pages ([site/playground](../site/playground/README.md)) |
| | **Preview** | **15–25** | |
| 6 | CLI parity | 2–3 | not started |
| 7 | Language server | 3–5 | not started |
| 8 | Clarity refactors and architecture docs | 2–4 | started (2026-11-06): state machines dispatch by a tmpl `Switch` (Rust `match`, C++ and Java `switch`; `if` chains elsewhere, by profile), with literal branches folded and unreached or jump-only states dropped |
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

1. **The functional tests:** see [tmpl/roadmap.md](tmpl/roadmap.md),
   "Coverage", for what each remaining test needs.
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
     (`crates/tempest/src/test_run.rs`, C++ through CMake); `tempest
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
     `??` took its branches' type. Real libraries run nightly
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
   diagnostics show in the editor; IDE features come next.
6. **CLI parity.** `init`, `build`, `test`, `watch`, `repl`, and doc
   generation, with Kotlin Temper's flags and output layout.
7. **A language server** ([design.md](design.md), CLI/watch/LSP).
   Kotlin Temper's is flaky, slow, and uses a lot of memory. Tempest's
   should be fast, steady, and small: diagnostics as you type, hover
   types, go to definition, and completion, on the same incremental
   core as `watch`.
8. **Architecture docs** that a newcomer can take in quickly:
   - a few diagrams (source to tree, the settle loop, typing, tmpl,
     backends, and how `watch` and the LSP reuse it);
   - a short paragraph per stage;
   - links to the code (crate, module, and key types and functions)
     at each step.

   They live in the repo and stay current with the code. Where the code
   is hard to explain simply, refactor it first, so the docs can stay
   short. Each refactor is its own change, apart from features.
9. **One tier-2 backend** ([tmpl/targets.md](tmpl/targets.md)), such as
   Go or Swift, to show that new backends are cheap. It comes after the
   parity case is made, so it adds to the case for switching rather
   than to the team's maintenance worries.
