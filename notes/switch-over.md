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

| # | Work | Chunks | Status (2026-11-03) |
|---|---|---|---|
| 1 | The rest of the functional tests on all backends | 7–10 | 53 of 72 through tmpl and all backends (core types; connected host files and fallbacks, with `--no-connected`, 2026-11-02; divisions that can't fail and maps, and labeled loops and local classes, 2026-11-03); 58 of 72 through tmpl (other libraries' class members and constructors, and their names from the library's own naming, 2026-11-03), the 5 more waiting on std translated as a library; 72 of 72 in the interpreter |
| 2 | Build tests and test-runner tests | 1–2 | not started |
| 3 | Real libraries on all backends | 4–8 | not started |
| 4 | Scorecard and migration plan | 1 | not started |
| 5 | Web playground | 1–2 | not started |
| | **Preview** | **15–25** | |
| 6 | CLI parity | 2–3 | not started |
| 7 | Language server | 3–5 | not started |
| 8 | Clarity refactors and architecture docs | 2–4 | not started |
| 9 | One tier-2 backend | 2–3 | not started |

Risks to the date:
- **`async`:** its state machines are the biggest single piece of step 1.
- **Library surprises:** real libraries use much more of Temper than the
  functional tests do.
- **Kotlin Temper's main moving:** each sync with it costs about a chunk.

## Each step

1. **The functional tests:** see [tmpl/roadmap.md](tmpl/roadmap.md),
   "Coverage", for what each remaining test needs.
2. **Build and test tests:** `tempest build` and `tempest test` on
   small projects in every backend, checked like Kotlin Temper's.
3. **Real libraries.**
   - Chromarite, Ordable, Hrefine, and Kelpto build with tempest, and
     their test suites pass in every language.
   - Each library's public API matches Kotlin Temper's per backend
     ([api-compat.md](api-compat.md)); every difference is fixed or
     written down with its reason.
   - Packaging too: `pyproject.toml`, `Cargo.toml`, `package.json`,
     the pom, and the `.csproj`, so the output can be published, not
     just run.
   - Kotlin Temper bugs these libraries hit are fixed in tempest's
     output, not worked around. One example: Chromarite's per-language
     immutability check (`demos/immutable/`), where Kotlin leaks in
     C++, JS, Lua, and Python, should pass everywhere with tempest.
4. **The scorecard,** one page measured on the same libraries:
   - build and test time, cold and warm, and the watch loop;
   - peak memory;
   - the Kotlin Temper bugs tempest fixes ([kotlin-differences.md](kotlin-differences.md));
   - error messages side by side on common mistakes;
   - what's still missing.

   Plus a migration plan: both compilers run in CI for a while, and
   their generated output is diffed.
5. **The web playground.** The wasm build runs the whole compiler in
   the browser: type Temper, and see every backend's output update
   as you type, with no server.
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
