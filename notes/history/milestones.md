# Milestones 1 to 6

Part of the [rewrite plan](../README.md). Milestones 1 to 6, from the
literate split to all seven backends, moved here from
[testing-and-milestones.md](../testing-and-milestones.md), section 15,
once done; the deviations and Kotlin bugs "above" are now in
[kotlin-differences.md](../kotlin-differences.md#lexer-and-parser).

1. **Literate split.** Done.
2. **Lexer and parser** for every source file in both repos, checked against
   Kotlin's tokens and tree shape.
   - 2a, lexer: done. `compat/check_lex.py` matches Kotlin's raw lexer
     (before the parser's token adapters) on all 815 files, about 215k
     tokens, including kind, byte span, synthetic and bracket flags, and
     error kind, apart from the deviations above. Also checked on inputs
     from Kotlin's lexer tests and on randomly edited inputs, including
     words not in NFKC.
     Open differences from fuzzing (`compat/fuzz_lex.py --seed 2`):
     - Blank or whitespace-only lines right after an indented code block:
       Kotlin lexes some as code whitespace and some as prose, by a rule we
       haven't pinned down (simply extending code over them breaks 111 corpus
       files). Only token kinds differ (`Space` vs `Comment`). Low priority.
     - The two "won't copy" differences above.
     - A paragraph's doc comment starts 2 bytes early when it's in a list
       item whose marker is alone on the line before, as in `-\n  x`. Our
       paragraph ranges include the paragraph's own indentation, which
       commonmark's spans don't, and the lexer's adjustment for that also
       catches the item's indentation here. Rare; to fix in `literate`.
     - An unclosed fence ended by its list item, as in `- \`\`\`\n\n1`:
       Kotlin keeps the following lines as code, a variant of the unclosed
       fence difference above.
     With the precise deviation classifier, 10,000 edited inputs give 20
     differences, all of these kinds. An ordered list's later items (`2.`
     after `1. a`) were merged into the first item's paragraph; fixed, found
     by a new Prismora (now Chromarite) file.
   - 2b, parser: done up to operator trees, with the token adapters.
     `compat/check_parse.py` matches Kotlin's `parse` on all 815 files and
     the 156 inputs from Kotlin's parser tests (`ParseTest`,
     `TokenSourceAdapterTest`, `PostProcessCstTest`), about 266k lines of
     normalized tree, apart from the deviations above; `fuzz_parse.py`
     found no unexplained differences in 10,000 edited inputs
     (`--seed 7`), with the precise deviation classifier.
   - 2c, grammar step: done. `compat/check_ast.py` matches Kotlin's
     `buildTree` on all 817 files and the parser's and grammar's case inputs
     (1217 inputs, ~137k lines of normalized tree), apart from the
     deviations above and 6 inputs that crash Kotlin; the 194 inputs from
     Kotlin's `BuildTreeTest` are in `tests/ast/kotlin-build-tree-tests.cases`.
     `fuzz_ast.py` found no unexplained differences in 30,000 edited inputs
     (`--seed 3`, `4` and `5`), and found the Kotlin bugs above.
   - 2d, the rest of `ParseStage` (`module`): done. With `--module` and
     `--docs`, `check_ast.py` matches Kotlin on the same 1239 inputs
     (with the 20 in `tests/module/*.cases`, among them Kotlin's
     `call-join-rewrite`, `call-join-rewrite-for-docs` and `appendix` stage
     tests), apart from the deviations above; `fuzz_ast.py --module` found
     no unexplained differences in 10,000 edited inputs (`--seed 11`).
     Left: interning string literals.
3. **Settle loop** with names, core macros (`let`, `fn`, `if`, `while`,
   `class`, `import`), the interpreter and error nodes, running the simplest
   functional tests in-process. This decides whether the design holds up.
   - Done (2026-10-27), in `crates/temper-frontend` (status and picks in
     [milestone-3.md](milestone-3.md); what's left there that needs types
     moved to milestone 4). `compat/check_functional.py` runs
     69 of the 69 functional tests with matching output (2026-10-26, on
     Kotlin Temper's `connected-fallback` branch; 68 on its `main`). Our tests:
     `tests/settle/*.cases` (cases of name resolution, macro expansion
     and running, with expected trees, diagnostics and output), the
     scheduler's unit tests, and `mutated_inputs_settle`, which settles
     and runs edited inputs, checking for panics, the debug read check,
     the step budget and determinism; with `SETTLE_FUZZ_DIR` it also took
     the functional tests, Kotlin's `std` and Chromarite (seeds 4 and 5, 20
     edits per file) without a failure. Left: the rest listed there.
4. **Types in the loop**, enough for most functional tests in-process.
   Kotlin Temper's frontend stage tests show how types should work,
   especially which type errors are reported and how (`TypeStageTest.kt`
   and `frontend/typestage/*Test.kt` under `frontend/src/commonTest`), and
   its backend translators show what backends need from types. Target typing for
   object, list and function literals comes with this milestone, with new
   tests in `tests/settle`, since Kotlin Temper doesn't do it yet.
   - In progress ([milestone-4.md](milestone-4.md), with its slices). The
     first, done 2026-10-27: declared and inferred types of literals,
     locals, parameters, calls of known functions and results, as facts
     in the settle loop, with the errors Kotlin reports for them, matching
     its messages and positions (`tests/settle/types.cases`). The
     second, done 2026-10-27: flow facts, null checks narrowing locals
     that aren't `var`, so a nullable value where a non-null one is
     wanted is reported as Kotlin does (`tests/settle/flow.cases`). The
     third's first part, done 2026-10-27: builtin operators' and
     `console.log`'s signatures, and what may bubble in a function that
     doesn't declare it (`tests/settle/builtins.cases`); the final
     builtin types' methods, read from core, done 2026-10-28. The
     fourth's first part, done 2026-10-27: classes' types (`this`,
     `new`, members read, called and assigned, subtyping through
     `extends`, and inherited ties), with Kotlin's errors but
     classes named as written (`tests/settle/class-types.cases`);
     `@operator` methods for `==` and `<=>`, typed and run, and class
     values as builtin operators' operands, abstract members'
     parameters kept and typed, overrides' signatures checked, `is`
     narrowing to types, Kotlin's errors for `is` and `as` checks,
     nullable targets (`x is T?` passes null), and "Void expressions
     cannot be used as values", done 2026-10-28. Generics (the fifth
     slice) begun 2026-10-28: generic types written out and type
     parameters as types, run-time checks of generic types, calls of
     generic functions solved from their arguments and type arguments,
     and generic classes' constructors and members, generic methods, and
     solving from the type wanted, and builtin generic types' members
     read from core (`ls.length`, `m[k]`); users' generic supertypes
     (`class Sub<T> extends Sup<T>`), and overrides' signatures in
     generic classes and against generic supertypes, with "Overridable
     methods don't allow generics", function types compared part by
     part, and builtin generic methods (`ls.map(f)`) solved per call,
     done 2026-10-29. List literals' types, and target typing (the
     sixth slice) begun with list literals typed from the type their
     place wants (`let xs: List<Int> = []`), and function literals'
     parameters and results typed from it (`ls.map { x => … }`), with
     trailing blocks placed while typing, and the interpreter passing
     them where typing places them, methods' too, and `return` and
     bubbling in function literals checked against the type wanted,
     and `@fun` interfaces as the function types they stand for, with
     function literals and trailing blocks typed from them, and object
     literals' class decided from the type wanted, in typing's round,
     the first macro that waits for types, and named `@fun` interfaces
     strictly nominal, with Kotlin's documented rules for them, done
     2026-10-29.
     Next: `new Box()`,
     `[]`, `null`, `bubble()`, and `panic()` taking the type wanted, a
     literal's result binding a generic callee's type parameter, and
     the rest of the fifth, as `Never<T>` and builtin interfaces'
     inherited members ([milestone-4.md](milestone-4.md), "Order and
     decisions").
     Milestone
     3's other leftovers that need types (`case` patterns, string tags
     evaluated to a class) wait for a design and for macros that wait
     for types.
5. **tmpl and one backend** (Python or JS) passing the functional tests, with
   an API diff for that backend. Maybe Python first because it looks different
   than Temper, so it feels more interesting. Fixing Kotlin's bugs here
   ([kotlin-differences.md](../kotlin-differences.md)): tmpl keeps
   module-level declarations, statics, and values in source order (after
   top-level sorting), so no static initializes before a value it reads;
   cases for `==` and `!=` between Booleans on each backend; and `build`
   and `test` delete only stale generated files.
   - Begun (2026-10-29): the `temper-tmpl` crate for subset A, with its
     schema, tables, builder, canonical text (read and written, round
     tripping exactly), and validator, tested on hand-written tmpl
     (`tests/text/*.cases`) and random programs; nothing uses it yet
     ([tmpl-state.md](tmpl-state.md#state)).
   - 2026-10-30: the frontend's `to_tmpl` for subset A (7 of 69
     functional tests translate), then the first Python backend
     (`crates/temper-be-py`, variant `py`): the naming pass, module
     state, all 65 operators, a connection table for the 6 core
     declarations those tests use, and a printer. All 7 generate Python
     that runs and logs the expected output against the vendored
     `temper_core`, and passes pyright and mypy in strict mode, with
     Kotlin's layout and public names (`tempest py FILE...` shows it).
     Next: subset B, so Python grows toward all the functional tests,
     and its API diff.
6. **The other backends**, one at a time (first Rust?), each with its API diff.
   Rust: an index that can bubble uses a checked get returning `Err`, not
   a panic. C++: function templates are forward-declared before type
   definitions. Every backend: module-level `var`s are safe to read and
   write from several threads at once (C++: atomic `shared_ptr` access,
   not a plain `static`; Kotlin Temper races there, found 2026-10-29),
   since libraries set them lazily. Tempest's backends should be at least
   as robust as Kotlin's wherever we can, not just as robust.
   - Rust begun (2026-10-30), for subset A, ahead of the Python
     backend's subset B: `crates/temper-be-rust`, variant `rust`, with
     Kotlin's crate layout, `temper-core` runtime (vendored), string
     representation, and public names, and prettier bodies laid out as
     rustfmt lays them out. All 7 functional tests that translate to
     tmpl generate crates that build, run, log the expected output, and
     pass clippy with warnings as errors (but for three lints the tests'
     own code trips), as do three programs of its own. Module state is
     safe across threads as Kotlin's is (`OnceLock` and `RwLock`
     statics), and a `var` updated from itself alone (`n = n + 1`)
     isn't lost under concurrent updates, as Kotlin's can be; a test
     runs it from 8 threads ([tmpl/profiles.md](../tmpl/profiles.md#as-built-rust)).
     `tempest rust FILE...` shows it. Next: subset B and C, and the API
     diff.
   - JS begun (2026-10-30), for subset A: `crates/temper-be-js`,
     variant `js`, with Kotlin's package and module layout (ES modules,
     `index.js`, a public module re-exporting from its `.internal.js`),
     `@temperlang/core` runtime (vendored), and public names, and
     prettier bodies: no numbered names, template literals, `const`
     where nothing reassigns, and labels only where a jump needs one.
     All 7 functional tests that translate to tmpl, and the Rust
     backend's three programs, generate packages that run under Node,
     log the expected output, and pass `tsc --checkJs --strict` against
     declarations generated from the runtime. `Int32` and `Int64`
     arithmetic wraps exactly as Kotlin's does (`| 0`, `Math.imul`, and
     `bigint` with `clampInt64`). `tempest js FILE...` shows it. Next:
     subset B, and the API diff.
   - Java begun (2026-10-30), for subset A: `crates/temper-be-java`,
     variant `java`, with Kotlin's Maven layout (`pom.xml`, a package
     per library, its module a final class of statics, `NAMEGlobal`,
     beside `NAMEMain`), `temper-core` runtime (vendored), Java 17, and
     public names, and prettier bodies: no numbered names, no class name
     before the class's own members, `+` converting numbers in
     concatenations, and `?:` for `if` values. All 7 functional tests
     that translate to tmpl, and the Rust backend's three programs,
     compile with `javac -Xlint:all -Werror` against the runtime, with
     no Maven in the loop, and run, logging the expected output; a
     `var` updated from itself alone isn't lost from 8 threads.
     `tempest java FILE...` shows it. Next: subset B, source maps, and
     the API diff.
   - C# begun (2026-10-30), for subset A: `crates/temper-be-csharp`,
     variant `csharp`, with Kotlin's project layout (`src/ROOT.csproj`,
     the module a static class, `ROOTGlobal`, in the root namespace,
     Kotlin's `Logging.cs`, and a `program` project), `TemperLang.Core`
     runtime (vendored), .NET 6, and public names (Pascal case for
     functions and exported values), and prettier bodies: no numbered
     names, parameters exactly the source's names (they're API in C#),
     `using`s instead of one-letter aliases, a file-scoped namespace,
     `goto` only for jumps out of inner loops, and numbers formatted in
     the invariant culture, where Kotlin's follow the current one. All 7
     functional tests and the Rust backend's four programs compile with
     `csc`, nullable reference types on, and every warning and .NET
     analyzer an error but CA2211 (Kotlin's public static fields), with
     no MSBuild in the loop, and run under `sv-SE`, logging the expected
     output; a `var` updated from itself alone isn't lost from 8
     threads. `tempest csharp FILE...` shows it. Next: subset B, the
     parameter-name analyzer, source maps, and the API diff.
   - C++ begun (2026-10-30), for subset A: `crates/temper-be-cpp`,
     variant `cpp`, with Kotlin's layout (`LIB/init.hpp`, `init.cpp`,
     and `main.cpp`, the library's namespace, and `global_init_init()`),
     header-only `temper-core` runtime (vendored), C++14, and public
     names, and prettier bodies: no numbered names, a `core` alias,
     `std::to_string` and `+` for concatenations, natural control flow
     without Kotlin's extra blocks and labels, and only exports in the
     header. Safer than Kotlin's: init runs once through
     `std::call_once`, module `var`s that functions write are read and
     written under a mutex (Kotlin's race: ThreadSanitizer reports it,
     and 8 threads × 1,000 ticks give about 6,500), operands keep
     Temper's left-to-right order where it shows (Kotlin's, at the
     vendored commit, logs `hi 1 of 3 10` for `module-state` under g++),
     and shifts are defined for every count. All 7 functional tests,
     the Rust backend's four programs, and one of its own compile with
     `g++ -std=c++14`, every warning in a long list an error, through a
     precompiled header of the runtime, with no CMake in the loop, and
     run, logging the expected output, also under AddressSanitizer and
     UndefinedBehaviorSanitizer, and the 8 threads under
     ThreadSanitizer. `tempest cpp FILE...` shows it. Next: subset B
     (lists among it, never iterated through a temporary), source maps,
     and the API diff. Since 2026-10-31, init on first use, as Kotlin's
     since #636 (../api-compat.md, proposal 2): `init_once` where it had
     `std::call_once`, `GlobalSlot` storage, accessors, and entry points
     that run the init, which the runner calls with no init first, five
     ways (an exported function, one module then another that imports
     it, an import's value, and a static), under both sanitizer builds,
     and the 8 threads under ThreadSanitizer now enter `module-state`
     with no init. Rust's harnesses do the same: the threads with no
     `init`, and a crate calling two libraries with none.
   - Lua begun (2026-10-30), for subset A, the last of Kotlin's seven:
     `crates/temper-be-lua`, variant `lua`, with Kotlin's layout
     (`LIB/init.lua`, which returns the library's exports, and its
     rockspec), `temper-core` runtime (vendored), its `lua >= 5.1`, and
     public names, and prettier bodies: no numbered names, local
     functions and locals declared where they get their values, `..` and
     `print`, `c and a or b` for `if` values, and `goto` only for
     `continue` and leaving an outer loop, where Kotlin's has one for
     every early return. Righter than Kotlin's: exported `var`s that
     functions write stay current for users (Kotlin's copies them once),
     `goto` is reserved, and `Boolean`'s `<` works (Kotlin's raises).
     Past Lua's limit on locals, module-level declarations go into a
     table, as Kotlin's do. All 7 functional tests, the Rust backend's
     four programs, and three of its own run with Lua 5.4.6, all in one
     process, logging the expected output, and parse with `luac -p`;
     LuaCATS annotations type them for LuaLS, whose check runs in CI.
     `tempest lua FILE...` shows it. Next: subset B, source maps, and
     the API diff.
   - Modules, imports, and tests (2026-10-30), subset B's first step:
     tmpl's modules, other libraries' modules in `uses` sections, imports
     derived from references, and `Test` nodes; `to_tmpl` for every
     module of a library, imports from its own modules, from std, and
     from other libraries under the work root, and its tests; and
     all seven backends writing several modules, imports, other
     libraries' dependencies, and tests in Kotlin's layouts and test
     forms (Python `unittest`, Rust `#[test]`, Mocha, JUnit 5,
     MSTest, C++ functions under `TEMPER_TESTS`, and LuaUnit), each
     running the shared `modules` and `imports-library` programs. `tempest
     test -b BACKEND` builds and says what
     each library needs; running tests waits for std's `testing`.
     count_tmpl stays at 7 of 69: the 9 tests that first needed modules
     now stop at classes, generics, or methods.
   - Classes and interfaces (2026-10-30), subset B's second step: the
     frontend generates `@auto("==")`'s `eq` as a real member
     (milestone-4.md); tmpl gains classes, interfaces, their members,
     `this`, fields, statics, `new`, conversions, `is`, `as`, and
     identity, with validator rules C1 to C3 and text that reads back;
     and all seven backends emit them in Kotlin's surface, running the 14
     class functional tests and a shared `classes` program under their
     checkers and sanitizers, with Kotlin's bugs fixed
     (../kotlin-differences.md, "Classes in the backends"). count_tmpl goes
     from 7 to 21 of 69 (tmpl-state.md).
   - Nullable types, `orelse`, `bubble()`, and `panic()` (2026-10-30),
     subset B's third step: tmpl gains `Nullable` types and their nodes,
     with rule N1; `to_tmpl` translates null checks with narrowing, `??`,
     `==` with a nullable side, `orelse`, `bubble()`, and `panic()`; and
     all seven backends run the 4 functional tests that now translate and
     a shared `nullables` program under their checkers and sanitizers,
     with Kotlin's bugs fixed (../kotlin-differences.md, "Nullables in the
     backends"). count_tmpl goes from 21 to 25 of 69.
   - Generics (2026-10-31), subset B's step 3b: tmpl gains type
     arguments, type parameters, calls' type arguments (rule T4), and
     list literals, with core's generic types in its fixture; `to_tmpl`
     translates generic functions, classes, interfaces, and methods; and
     all seven backends run the 3 functional tests that now translate
     and a shared `generics` program under their checkers and
     sanitizers, with Kotlin's bugs fixed (../kotlin-differences.md,
     "Generics in the backends"). count_tmpl goes from 25 to 28 of 69.
   - Vendor sync to Kotlin main (2026-10-31, 0db80cc8, from c666f9f):
     72 functional tests, 71 passing in-process; `types/connected`
     (connected classes, #633) is a known gap in check_functional.py's
     `KNOWN_GAPS`, as Kotlin's own interpreter skips it (its issue 456).
     `control-flow/bubble` gained `orelse` folding cases with a
     `continue` in a `for` loop, which `to_tmpl` now translates (tmpl
     roadmap, "State"), so it stays on every backend. count_tmpl: 28
     of 72.
   - Vendor sync to Kotlin's `connected-type-fallback` branch
     (2026-11-02, 1f407c3a plus its uncommitted follow-up to
     `types/connected`): a user's `@connected` class may have a complete
     Temper implementation, which the interpreter runs, so
     `types/connected` passes, and check_functional runs 72 of 72 with
     `KNOWN_GAPS` empty. The frontend reports Kotlin's errors for a
     partial implementation, for a backend list without one, and for a
     connected method's default arguments (`tests/settle/classes.cases`).
     count_tmpl stays 44 of 72: backends don't read a library's host
     files yet, so `types/connected` is still "not yet" in tmpl.
   - Function values (2026-10-31), subset B's next step: tmpl gains
     function literals with their captures (rule D5), references to
     functions, calls of function values, `ToFun` and `FromFun`, `@fun`
     interfaces as `FUN` interfaces, and function types as values' types
     (rule V1), with core's members that take functions in its fixture;
     `to_tmpl` translates them, local functions that don't call
     themselves, calls of fields' function values, and core's `forEach`
     and `for … of`, inlined as Kotlin does; and all seven backends run
     the 6 functional tests that now translate and a shared `functions`
     program under their checkers and sanitizers, with Kotlin's bugs
     fixed (../kotlin-differences.md, "Function values in the backends").
     count_tmpl goes from 28 to 34 of 72.
   - Parameters' defaults (2026-11-01), subset B's next step: an
     optional parameter is typed `T?`, its default the body's first
     `let`s, as `coalesce(x, d)` (rule N2, `Coalesce` in N1); `to_tmpl`
     translates defaults of functions, methods, and constructors, calls
     that leave arguments out or pass null, and object literals that
     leave properties out; and all seven backends run the functional
     test that now translates (`classes/property-order`) and a shared
     two-module `defaults` program under their checkers and
     sanitizers, in Kotlin's surface with plainer bodies
     (../kotlin-differences.md, "Parameters' defaults in the backends").
     count_tmpl goes from 34 to 35 of 72; the other 5 tests that waited
     for defaults now wait for core's members and types, and a class
     declared in a function.
   - Core's other types (2026-11-02), subset B's next step: the core
     fixture gains string indices, `StringBuilder`, `Empty`, `Deque`,
     `DenseBitVector`, and more of `String`'s and `Float64`'s members;
     `to_tmpl` writes core's other types as `Nominal`, its statics,
     operators by its `@operator` methods, char tags, and `for … of` a
     string; and all seven backends run the 9 functional tests that now
     translate (`CORE_TYPES`) under their checkers and sanitizers, in
     Kotlin's surface with its bugs fixed (../kotlin-differences.md,
     "Core's other types in the backends"). count_tmpl goes from 35 to
     44 of 72.
   - String order by code point on every backend (2026-10-30): one
     shared program, `string-order`, checked against the interpreter,
     runs on Python, Rust, JS, Java, C#, C++, and Lua. Java now calls a small
     helper the module's class gets, JS takes the sign of
     `cmpString`'s and `cmpFloat64`'s differences, and C# uses the
     runtime's code-point comparison, as Kotlin's C# does.
