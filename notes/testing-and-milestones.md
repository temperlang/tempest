# Testing, milestones and risks

Part of the [rewrite plan](README.md).

## 14. Testing and checking compatibility

- **What must match:** behavior, error messages and generated public APIs.
  Internal representations, like tokens and trees, needn't. Comparing them
  with Kotlin is a cheap way to find behavior differences early, so we match
  them while that's easy, but a later stage may change them where nothing
  observable changes, noting why.
- **Where Kotlin is flaky, do better.** Tempest has one behavior, ours.
  Each deliberate difference from Kotlin is listed below with its reason,
  and in `compat/deviations.py`, which the comparison scripts use to report
  those inputs as expected rather than failing. That list goes away when the
  comparisons do. Some Kotlin bugs are copied for now, where they only affect
  broken input, to keep comparisons quiet; they're listed too.
- **Our own tests,** run by `cargo test` without Kotlin Temper, so they last
  past the rewrite: per stage, a few `.cases` files of inputs with expected
  output (for the lexer, inputs from Kotlin's lexer tests plus our own edge
  cases), and properties checked on randomly edited copies of them. Fewer,
  denser files rather than one per case.
- **Fuzzing against Kotlin:** `compat/fuzz_lex.py`, `fuzz_parse.py` and
  `fuzz_ast.py` edit inputs randomly and compare both sides, shrinking any
  difference to a small input to add as a case, and saving the original.
  Real source rarely exercises error paths, so this finds what the corpus
  doesn't. A difference counts as a listed deviation only where it starts
  (`compat/deviations.py`): the first output lines that disagree must be
  about a block quote, a multi-quoted string (to the end of the input if it
  has scriptlet lines, whose blocks may stay open), or what follows a `$`
  error before `{`, or the outputs must match apart from positions. So a
  bug elsewhere in an input with those features is still found. The parse
  and grammar fuzzers also set apart inputs that crash Kotlin and those
  that already differ a step earlier.
- **Benchmarks:** each stage gets a `--bench` mode on both sides and a
  script that alternates them; results go as rows in
  [size-and-speed.csv](size-and-speed.csv) (`compat/measure_size.py
  --record` for sizes and memory), and
  [size-and-speed.md](size-and-speed.md) explains only what rows can't.
- **Oracles:** small Java programs that call Kotlin Temper's jars give
  reference results per file: literate blocks, tokens, and parse trees
  (`compat/oracle/*.java`). Each layer is diffed on every Temper source file
  in the Temper, Chromarite and Ordable repos, and on our test cases' inputs.
- **Functional tests:** the 69 markdown tests, run in-process (interpreter)
  and on every backend, comparing `log` output, with the same per-backend
  skips as Kotlin's `FunctionalTestStatus.kt` to start with.
- **API diffs:** for each backend, extract the public surface from Kotlin's
  and the new output for the same libraries (functional tests, std,
  Chromarite), and diff them. Every intended change is listed in an allowlist
  with its reason.
- **Library tests:** `tempest test` (crate `tempest`, logic in
  temper-frontend's `project` and `library_tests`) runs a library's
  `test` blocks on the interpreter, finding libraries as Kotlin's `temper
  test` does and printing, as it does, only failures and its summary
  line (`--verbose` lists every test). Chromarite passes 56 of 56
  and Ordable 29 of 29, as with Kotlin's `temper test -b cpp`
  (2026-10-27), and CI runs both from fresh checkouts. That needed two
  fixes: a function type's parameter names, `fn (row: T): Void`, were
  read as uses, and `m[i, j]` on an object didn't call its `get`.
  Differences from Kotlin's discovery: `.gitignore` isn't read (dot
  directories, `temper.out` and `temper.keep` are skipped), files
  outside any library aren't built, libraries can't import each other
  by name yet, and nothing is written to `temper.out`. Ordable's tests
  take 14–22 s in a release build, almost all in its property tests
  (tens of thousands of natural comparisons on the interpreter), worth
  profiling with the interpreter's other costs. Std is built in
  (temper-frontend's `stdlib`, from `vendor/temper/std`), for the wasm
  build too; `--std DIR` mounts its directory at std's root instead
  (`Sources::with_std_dir`). `run-temper` stays in
  temper-frontend for the compat scripts, sharing import loading
  (`project::load_imports`); moving it into `tempest` (maybe as `tempest
  run`) is a separate change.
- **Library tours:** build Chromarite and Ordable with tempest and run their
  tours (`demos/tour/` in each, with a hand-written program per language and
  one expected output) unchanged, matching their expected output. That proves
  the generated APIs work for real callers, in every language, so it's the
  test that tempest is a Temper. A backend change that would break them goes
  behind a compatibility mode that keeps Kotlin's API
  ([kotlin-differences.md](kotlin-differences.md), "How far to differ"), so
  tempest never requires changing the tours. If the team adopts an improved
  translation, Chromarite, Ordable, and others update their tours to match,
  after that decision, not as a condition of using tempest.
- **Backend tests, shared from the start:** one set of Temper inputs, each
  with an expected output per backend (or "skip, because …"), run by every
  backend, rather than a separate set per backend as Kotlin Temper has
  (its `*BackendTest.kt` files range from 12 cases for Lua to 64 for Java,
  so most inputs are checked on only some backends). Backend-only cases
  only where a backend truly needs one, which should be rare. Also golden
  files of every backend's generated code for all the functional tests,
  so a codegen change shows up as a diff. The connected-fallback
  functional test (one input, a connected file per backend) is the model.
  Worth recommending to Kotlin Temper too
  ([kotlin-differences.md](kotlin-differences.md)).
- **Other platforms (planned):** CI tests only on Linux, while the
  binaries also ship for macOS and Windows. What differs there: CRLF
  line endings (Git's default checkout on Windows) in lexing, multi-line
  strings, and positions; paths (backslashes, drive letters, mapping
  `-std-/` back to disk, diagnostics); and case-insensitive file systems
  for module names. First, cover these on Linux: a `tests/settle` case
  with CRLF source giving the same results and positions as LF (checking
  how Kotlin handles CRLF first), and `Sources` unit tests with
  Windows-style paths. Then a small job on `windows-latest` and
  `macos-latest`: `cargo test --workspace`, and the built binary running
  `tempest test` on a small library, leaving fmt, clippy, wasm, and the
  functional check to Linux. No `.gitattributes` forcing LF, since
  users' files will have CRLF anyway.
- **One-file downloads, `tempest license`, and `tempest licenses`
  (planned):** each download should hold just the binary, with no
  `COPYRIGHT`, `LICENSE-*`, or `NOTICE` beside it. Instead, as in Kotlin
  Temper, `tempest license` prints tempest's own copyright, licenses,
  and notices (including the Unicode notice), and `tempest licenses`
  prints those of every crate built into that binary. This also fills
  a gap: the downloads now carry only tempest's own license, though the
  binary includes about 25 third-party crates (`ureq`, `rustls`, `ring`,
  and so on), mostly MIT or Apache-2.0 but also ISC, BSD-3-Clause, and
  CDLA-Permissive-2.0 (`webpki-roots`), whose licenses ask for their
  notices to go with binaries. Plan: a uv script that reads
  `cargo metadata` for the binary's own dependencies and features, takes
  each crate's license files from Cargo's local source cache, and writes
  one text file that the binary embeds. Commit that file, and have CI
  check it's current, so builds need no network and a new dependency
  can't slip in without its license. For now, the wasm download keeps
  both texts as files beside it, since it has no command line; revisit
  later (exported functions returning the texts, say) as wasm use
  takes shape. Record the size the texts add to the binary.
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

### What the parse comparison compares

`compat/check_parse.py` compares what decides behavior: how tokens group
into operators, which tokens the adapters insert or drop, string content
after trimming and joining, the parser's errors, and comments. Both sides
print the same normalized form (`compat/oracle/ParseTree.java`): one line
per node in pre-order, with the operator's name, or a token's kind, byte
span, synthetic flag and text; then errors as message template, operator,
bound, count and byte span; then comments as kind and span. Normalized
away:

- Kotlin's internal objects (stack elements, CST classes) and node spans,
  which are internal; error spans are compared.
- UTF-16 positions, converted to UTF-8 byte offsets.
- The root: Kotlin returns the root's only child when there is one, so we
  print that too.

Tokens are kept in the tree, so the comparison also covers the adapters.

### What the grammar comparison compares

`compat/check_ast.py` compares the grammar step's output
(`compat/oracle/AstTree.java`), on the same inputs plus
`tests/ast/*.cases`: one line per node in pre-order with its kind and byte
span, names with their kind (declared, used, builtin) and text, values with
their type and content, and error nodes; then the grammar step's
diagnostics (level, template, values, span), not the parser's, which
`check_parse.py` covers. As in Kotlin's `ParseStage`, the parse tree is
first split at `;;;` into code and JSON metadata. Normalized away:

- Kotlin's classes (`CallTree`, `ValueLeaf`, ...) and value objects: a value
  is its type name and content, a float its bits in hex, a function its
  name.
- What's inside error trees: Kotlin makes them calls to `error` with the
  unparsable parts as strings; we compare their spans.
- Empty trees' ends: Kotlin may end an empty tree before its start; both
  sides print the end as at least the start, and our spans never invert.
- Positions, where the trees match apart from them: they come from parse
  node spans, which differ by the "spans cover all parts" deviation below.
  Exact positions are compared for everything else.

With `--module` (or `--docs`), both sides also do the rest of Kotlin's
`ParseStage` for a library (or documentation): the code wrapped in a block,
call joins rewritten (`rewriteCallJoins`), and the metadata converted to
JSON (`toJsonValue`, called by reflection), printed as one more line, with
the inputs in `tests/module/*.cases` too. Temporary names print as their
hint and their index in order of first appearance, `f#0`.

### Deliberate deviations from Kotlin

Each has cases in `tests/*.cases` or `tests/parse/*.cases`.

- **Code in markdown block quotes is prose,** like an `inert` block.
  Kotlin counts it as code only because commonmark visits code blocks at any
  depth, and it can't work past one line, since the `>` markers on later
  lines lex as operators. Nothing uses it. Code in list items stays code.
- **Multi-quoted strings** (`tests/parse/multi-quoted.cases`):
  - Blocks a scriptlet line leaves open are kept per string, and come back
    in order at the next scriptlet line. Kotlin reverses them, and its stash
    outlives the string, so a later string's scriptlet can reopen another
    string's `{`.
  - A string that ends right after a scriptlet line closes what that line
    left open. Kotlin closes only the innermost, so the string goes on and
    swallows the code after it.
  - A `${...}` on a scriptlet line is code, and the line stays a scriptlet
    line after it. Kotlin turns the rest of the line into content, and its
    adapter treats the `${...}` as a string hole.
  - A single-line string left open at a scriptlet line's end closes there,
    and the next line is read for its margin. Kotlin reads it as code.
  - `"""` on a scriptlet line is an error and opens nothing. Kotlin opens a
    nested string anyway, which takes the following lines.
  - A scriptlet line's last token is kept. Kotlin drops it, meaning to drop
    the line break, so a scriptlet at the end of input loses its last token
    (Kotlin's own test `stringsOnStringTemplateLine` expects the `}` gone).
  - A content part after a scriptlet statement starts a new statement (a
    `;` goes between, unless one is there or after `{` or `}`). Kotlin
    juxtaposes them.
  - The `}` around a string with scriptlets doesn't start a call join, as
    the `)` around other strings doesn't. Kotlin joins the next line's
    word into a call.
  - A close quote closes the innermost string past any operators without
    open brackets. Kotlin looks only one level down from a leaf, so a
    string ending in a scriptlet expression like `: w` takes the quote into
    `w`.
- **`${` nests inside a hole,** as anywhere else in code. Kotlin makes the
  `$` an error and the `{` a block.
- **Spans cover all of a node's parts.** Kotlin leaves operands adopted by a
  separator out of its span, which can shift error positions, and gives hex
  digits joined across `${}` in `\u{...}` the first part's position. The
  grammar step's positions come from these spans, so its trees differ in
  positions too (12 corpus and case inputs), and once in structure: a
  comment inside an adopted operand, as in `n s //c` then `public e,`,
  is found before the statement in Kotlin, which then gets a comment call
  and a block, but not in ours.
- **Spans of rewritten call joins** cover what they wrap: the delegating
  function, its call and the temporary name used in it span the children
  after the join's symbol. Kotlin's `Tree.replace` passes an inclusive end
  to `spanningPosition`, which takes an exclusive one, so its function
  spans all but the last of them (just the start of a lone block) and the
  call and name get odd spans from that. `check_ast.py --module` masks
  those three spans; the declaration's are compared.
- **Operands nested over 1000 deep are an error** (`NestedTooDeeply`),
  where Kotlin overflows its stack, already at about 300 nested calls or
  a 1000-term `a+a+...`.
- **An illegal assignment isn't performed** (a `let` or non-`var` field
  assigned again): it's reported, and its value stands in for it, where
  Kotlin fails at run time.
- **NFKC, as Kotlin:** words and quoted names not in Unicode NFKC are
  errors (`temper_syntax::nfkc`, with tables generated from the JDK by
  `compat/gen_nfkc.py`, Unicode 15.0.0 for Java 21). Kotlin's lexer only
  checks words with a code point past Latin-1, so `ªb` (NFKC `ab`) passes
  and is a different name from `ab`; we match that, though it looks
  accidental. Quoted names are always checked.

### Kotlin bugs found

Copied for now, to keep comparisons quiet, since they only affect broken
input; to fix when the comparisons end:

- A close bracket only closes the innermost open bracket. Kotlin means to
  look past others for closers other than `>` (`isAmbiguousCloser` in
  `BracketPartners.kt`), but its check matches every closer.
- Semicolon insertion counts a `{` twice when it inserts a `;` before it
  (`AutomaticSemicolonInserter` reprocesses the pushed-back token), so
  later `}`s may be taken for the end of a `${` hole.

Crashes, which we don't copy (inputs in the fuzzers' `found/` directories):

- `Parse.kt` throws `IndexOutOfBoundsException` on input starting with `,`
  or `|`: the follower check reads `child(-1)` of the empty root.
- `StringFixer.fixupCollectedMq` throws on a multi-quoted string whose
  close it never sees, as in markdown `    """\na\n\n    ""` (index -1), and
  on some unbalanced scriptlets: `check(fragments.isEmpty())` on
  `"""\n:{//\n\``, and a null `stack.last()` on `"""\n:{\n\`\n~`.
- `Lexer.kt:782` threw on markdown `">     ,\n    }"`; fixed on a classic
  Temper branch.
- `MarkdownSupport.buildTaggedRange` throws `NoSuchElementException` on
  the markdown `[i]:h\n"`: a link reference definition, then a line.
- `Lift.kt` throws `IllegalStateException` ("too few preceding ShiftLeft")
  on `():o`: inside `(...)`, the `Expression` garbage alternative matches
  nothing without failing, so the infix `:` has one operand to swap. Ours
  gives `(Call : o)`.
- `Tree.replace` gives what it plants the span of all but the last
  replaced child (an inclusive end passed as exclusive), as seen in
  rewritten call joins; we don't copy it (deviations above).
- Deep nesting overflows Kotlin's stack (`StackOverflowError`) at about 300
  nested calls `f(f(...))` or a 1000-term `a+a+...`.

Slow, which we don't copy: Kotlin's grammar takes exponential time on some
nesting, about doubling per level: 14 s for 22 nested `(` and similar for
`a[a[...]]`, `f<f<...>>`, `@a (@a (...))` and calls whose callee nests, as
`fn (x: fn (x: ...) {}) {}`. An alternative parses a whole operand, fails
on what follows, and the next parses it again: `Prefix` takes `(` as a
prefix operator and fails at `)`, the generic `Infix` does the same for
`[` and `<`, `TaggedString` parses the tag before checking for a string,
and the two forms of a decoration both parse its first operand. Ours checks
the failing condition first, with the same results.

Other differences we won't copy:

- An unclosed fenced code block followed by blank lines at the end of the
  file: Kotlin takes its last code line as the closing fence, so that code
  is lost to prose.
- `"\` before a character outside the BMP, like an emoji: Kotlin's escape
  takes one UTF-16 unit and splits the surrogate pair.

## 15. Milestones

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
     read from core (`ls.length`, `m[k]`). Next: users' supertypes with
     arguments, and the rest of that slice ([milestone-4.md](milestone-4.md)). Milestone
     3's other leftovers that need types (`case` patterns, string tags
     evaluated to a class) wait for a design and for macros that wait
     for types.
5. **tmpl and one backend** (Python or JS) passing the functional tests, with
   an API diff for that backend. Maybe Python first because it looks different
   than Temper, so it feels more interesting. Fixing Kotlin's bugs here
   ([kotlin-differences.md](kotlin-differences.md)): tmpl keeps
   module-level declarations, statics, and values in source order (after
   top-level sorting), so no static initializes before a value it reads;
   cases for `==` and `!=` between Booleans on each backend; and `build`
   and `test` delete only stale generated files.
6. **The other backends**, one at a time (first Rust?), each with its API diff.
   Rust: an index that can bubble uses a checked get returning `Err`, not
   a panic. C++: function templates are forward-declared before type
   definitions.
7. **Watch, incremental rebuilds and the language server.**

Later, maybe:

- **A library cache** of frontend results ([library-cache.md](library-cache.md)).
- **Tempest in Temper.** Once tempest and Temper are stable, translating
  tempest into Temper would show where Temper falls short for fast,
  allocation-careful code, and each gap would improve Temper itself, not
  just tempest. Likely gaps: unsigned and narrow integers, byte-level
  strings, slices or views, arenas with integer ids, and control over
  copying. Value types as such have hard semantics in many targets; struct
  types stored in byte buffers, as discussed for Temper, might be much
  easier, with struct instances still separately allocated objects, or
  facades over a buffer. Not a priority.
- **A standard Temper C ABI**, which the C++ and Rust backends (and maybe
  a future C99 backend) implement, so native code can back faster
  Python and Lua backends ("fast Python", "fast Lua"), alongside the
  pure ones, which stay useful. Publishing should make the expected
  package variants easy: for Python, platform wheels with native code
  plus a pure wheel as the fallback; for Lua, rocks with and without C
  modules. Publishing and consuming Temper-built libraries should be as
  easy as possible everywhere.
- **`Rc` instead of `Arc` in Rust** for much generated code, once the
  actor and isolation model says what may cross threads, which would
  make Rust output somewhat faster. A hand-converted copy of Ordable's
  generated Rust (Arc to Rc, RwLock to RefCell, Send and Sync bounds
  dropped) sorted 20,000 names
  about twice as fast (26 to 14 ms), with natural comparisons 20%
  faster and key building up to twice as fast (2026-10-26).
- **Process isolation**, for an actor model, maybe even a BEAM (Erlang)
  backend. It needs careful global semantics: module-level mutable state
  per process or not at all, and messages that are immutable or copied,
  so no mutable object is shared between processes. Backend-language code
  could still get around it, which may be fine, as a promise about Temper
  code rather than a sandbox. On BEAM, Temper's mutable objects and
  builders would need emulating, which is its own cost. Async and promises
  might map onto messages.

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
- **Ruling out reference cycles** ([acyclic.md](acyclic.md)): always
  planned for Temper, never built. Open: top types and open interfaces,
  closures, core's builders, and how much ordinary code it rejects.
