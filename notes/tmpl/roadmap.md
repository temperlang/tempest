# tmpl: design roadmap

Part of the [tmpl design](README.md).

## Design roadmap

Each level unlocks code, so design and building interleave.

| Level | State | Unlocks |
|---|---|---|
| 1: purpose and decisions | Done, reviewed (2026-10-29) | The crate boundary and its rules; nothing to build alone |
| 2: structure | Done, reviewed (2026-10-29) | The `temper-tmpl` crate skeleton: schema macro, node families, type and declaration tables, builder, the text writer and reader with a round trip, and the validator's structural rules, all tested on hand-written tmpl |
| 3: details, in subsets | Subset A drafted (2026-10-29), and its model, text, and validator built, then qnames, connected declarations, and core as declarations (2026-10-29, "State" below); B, C, and D sketched | Per subset: exact fields, validator rules, and passes, then the frontend's translation (`to_tmpl`) for the functional tests that subset covers, then backends |

Level 3 goes subset by subset, each just ahead of the code that needs
it (Proposed):

- **Subset A:** functions, locals, `if`, `while`, returns, calls, the
  builtin types' operators and members, `console.log`, and module init.
  Then `to_tmpl` for the functional tests that use only these, then the
  naming pass, the shared output tree, and the Python backend running
  them, as its Kotlin-compatible variant first ([question 22](open-questions.md)).
- **Subset B:** classes, properties, equality, object literals,
  bubbling and `orelse`, `for … of`, closures, and imports. Python grows
  to most functional tests, with its API diff (milestone 5).
- **Subset C, Rust:** the results lowering, cells for assigned
  captures, properties as methods, first-use init, and synchronized
  module `var`s. Then the Rust backend (milestone 6).
- **Subset D:** generators and `async`, with the state-machine pass
  for Rust; then the other backends' profiles.

**Interleaving with milestone 4** (Proposed). Milestone 4's remaining
critical path ([milestone-4.md](../milestone-4.md), "Order and
decisions") runs in alternating chunks with tmpl's, since `to_tmpl`
needs every expression of a test typed:

| Chunk | Milestone 4 | tmpl |
|---|---|---|
| 1 | `new Box()`, `[]`, `null`, `bubble()`, and `panic()` taking the type wanted; `Never<T>` | Crate skeleton, schema macro, type and declaration tables (done for subset A, with chunk 2's tmpl work) |
| 2 | Builtin interfaces' members inherited by users' classes; a literal's result binding a generic callee's type parameter | Text writer, reader, and round trip; structural validator (done for subset A) |
| 3 | Slice 7 begins: count expressions of unknown type; resolve every `==` ([nodes.md, "Equality"](nodes.md#equality)) | Level 3 for subset A |
| 4 | Slice 7 continues, driven by which functional tests still have unknown types | `to_tmpl` for subset A's tests (done, 2026-10-30: 7 of 69) |
| 5 | Slice 7 continues | Naming pass and Python backend for subset A (done, 2026-10-30: 7 of 7 run); then the Rust backend for subset A (done, 2026-10-30: 7 of 7 run); then the JS backend for subset A (done, 2026-10-30: 7 of 7 run); then the Java backend for subset A (done, 2026-10-30: 7 of 7 run); then the C# backend for subset A (done, 2026-10-30: 7 of 7 run); then the C++ backend for subset A (done, 2026-10-30: 7 of 7 run); then the Lua backend for subset A (done, 2026-10-30: 7 of 7 run), the last of Kotlin's seven |
| then | Kotlin's diagnostics, where cheap | Subsets B, C, and D |

A test whose expressions aren't all typed is reported by `to_tmpl` and
skipped, so the count of translated functional tests tracks slice 7.
No reference cycles (slice 8) follows tmpl, as decided.

## State

**2026-10-29: the frontend's side of chunk 3 (milestone 4),** for
`to_tmpl`: typing gives every expression's type, with
`compat/count_untyped.py` counting what's left of unknown type (86 of
36,711 expressions, all in 9 functional tests, from 5,617; 60 of the 69
tests fully typed, from 1), and records what each call, operator, and
member read resolves to (`Typing::resolved`): a declaration, the `eq`
of an `@auto("==")` class, a builtin by this crate's key names, a null
check, or a call of a function value, with the `==` uses that have
only identity to call noted (4, all in std's regex). The frontend
doesn't depend on this crate; the names are shared by convention
(typing.md, "Resolutions"; milestone-4.md, "Every expression typed,
first part"). The `@auto("==")` `eq` isn't a declaration in the tree
yet; the frontend will generate it, as Kotlin's `AddAutoMembers` does,
so `to_tmpl` never synthesizes members.

**2026-10-29: subset A's model, text, and validator,** in
`crates/temper-tmpl` (no dependencies; nothing uses it yet):

- `schema.rs`: one `schema!` table of subset A's 30 node kinds (33
  after the qname chunk, below), each
  with its family, type rule, needed profile features, and fields
  (payload slots, the may-bubble flag, and children with their
  places), generating `Kind`, its metadata, and `Node` with
  `Nodes::get`.
- `nodes.rs`, `types.rs`, `decls.rs`, `ids.rs`, `ops.rs`, and
  `profile.rs`: node columns, the type table with fixed scalars and
  interned signatures, declarations and strings, ids and spans, the
  operator table (113 entries), and profiles with one feature so far.
- `builder.rs`: `Builder`, giving each node its type by the local
  rules and marking calls that may bubble.
- `text/`: the canonical writer and reader, with spans; read then
  write gives the same bytes, and write then read an equal program
  (`Program`'s `==` compares meaning, as ids of nodes, types, and
  strings may differ).
- `validate.rs`: rules S1 to S4, T1 to T3 and T5, R1 to R6, F1 to F5,
  D1, D2, and P1, each problem with its code.
- Tests: golden cases (`tests/text/*.cases`: canonical text, problems,
  and read errors), unit tests (`tests/model.rs`), and a round trip of
  random valid programs (`tests/round_trip.rs`).

Left for later chunks: the view (`text::view`), passes and `lower`,
analyses beyond the validator's own (liveness, module state,
`naming::hints`), the naming pass, the binary format, `to_tmpl` in the
frontend, and backends; T4 (type arguments) and D3 to D5 wait for
generics, exports' types, captures, and `@imu` (subset B).

**Choices made while building** (Proposed, for review):

- **The schema's form:** each kind is a line such as `If(Stmt,
  Untyped) { cond: NodeId = one Expr, then: NodeId = one Block, els:
  Option<NodeId> = opt Block }`: family, type rule, an optional profile
  feature, and fields with their Rust type and slot (`a`, `b`, `ab`,
  `flag`, or `one`, `opt`, or `many` children with their place). At
  most one `opt` or `many` field per kind, which takes the children
  the `one` fields leave. The builder is hand-written per kind, since
  each computes its type; a test checks it against the decoder.
- **`var` is a declaration fact only,** not also a node flag on `Let`
  and `ModuleLet` as nodes.md's table had it, so it's stored once. A
  node's flags are `MAY_BUBBLE` and `DERIVED` (the span bit).
- **May-bubble is read from a function's signature,** not stored
  again as a declaration fact; so D5 has nothing to check in subset A.
- **No type arguments yet:** subset A has no generic callees, so `b`
  is unused and T4 waits for subset B.
- **`Decl` is an array of records,** not columns; it holds `doc` (a
  `StrId`) in place of a separate `DocTable`, and leaves out
  `visibility`, `key`, and `link` until subset B needs them. A source
  file's `line_starts` and `wide_lines` waited for source maps (added
  2026-10-30, below).
- **`Error` goes only in expression places;** as a statement, it's an
  `ExprStatement`.
- **The builder never fails;** it computes types by the local rules
  (`NONE` where it can't), and everything else is the validator's. The
  reader checks only what text decides (formats.md, Level 3, "As
  built").
- **The operator table's names** (`ops.rs`, called the key table
  before the rename to `Op`): per type, `add`, `sub`, `mul`, `div`,
  `mod`, `neg`, `eq`, `lt`, `le`, `gt`, `ge`, `cmp`, `min`, `max`, the
  bit operators as `bitAnd`, `bitOr`, `bitXor`, `bitNot`, `shl`, `shr`,
  and `ushr`, `Float64.pow`, `Boolean.not`, `String.cat` (two
  operands; the frontend chains it for templates), the members as
  core names them (`Int32.toString`, `String.toInt32`, …), and
  `console.log`. Parameters with defaults (`toString`'s radix) are left
  out until `Omitted` (subset B). `div` and `mod` bubble for every
  numeric type, as typing's signatures say.
- **Problems' order:** by writing order, a declaration's at the node
  that defines it; a problem shows `@n` (a node's number in writing
  order) or `#n` (a declaration).
- **Definite assignment** (R5 and R6) runs over the structured tree,
  with a loop's assignments counted as possibly done before each
  iteration; a `let` in a loop is a fresh binding each time. A read of
  a module `let` or `var` before its initializer in the init block is
  F5 (not R5) either way; reads inside functions aren't checked yet
  (question 19's warning needs the module-state analysis).

**2026-10-29: qnames, connected declarations, and core as
declarations** ([question 43](open-questions.md)'s chunk), in
`crates/temper-tmpl`, in two steps:

- **A pure rename first,** with no change in behavior: `Builtin` to
  `Op` (the node kind and the builder's `op`), `builtins.rs` to
  `ops.rs`, `Key` to `Op`, `KeyInfo` to `OpInfo`, `KEYS` to `OPS`, and
  the reader's `Tok::Key` to `Tok::Op` ([question 39](open-questions.md)).
- **Then the migration:**
  - `qname.rs`: `QName`, parsed and printed exactly as Kotlin's
    `QName.kt` does, escapes included; `QNames::derive`, every
    declaration's qname from the library, the module's path, owners,
    names, and kinds, with `#n` only where ambiguous and stored
    overrides; and the one helper that says how a connected declaration
    resolves, `resolves_by_qname` (core and std by qname, the rest by
    name), with `library_of` and `is_external`.
  - `ops.rs`: only the 65 operators on builtin types; core's other 48
    members are external declarations
    (`tests/fixtures/core.tmpl`).
  - Declarations: kinds `Library` (a `uses` section's), `Class`,
    `Interface`, `Method`, and `Getter`; facts `OPTIONAL`, `KEEP`, and
    `CONNECTED` (with a backend list); a stored qname override; and
    parameter names for external functions. `Program::uses` lists the
    used libraries, and `Module::path` each module's directory.
  - Types: non-generic `Nominal { decl }`, and which parameters are
    optional in signatures (`Signature`).
  - Nodes: `CallMethod` (receiver, then arguments; virtual exactly when
    the method's owner is an interface, `Program::is_virtual`, not
    stored), `Omitted`, and `BodilessFunction` (a connected function
    without a body), 33 kinds in all.
  - Text: `uses` sections, `in "path"`, the marks `qname "…"`, `keep`,
    `connected(…)`, and `export`, `?` on optional parameters, bodiless
    functions, method calls, and `omitted`; `%` is only for operators.
    The goldens log through `~console`, from `getConsole()`, as Kotlin
    does.
  - Validator: Q1 to Q4; R2 for `CallMethod`; T3 for receivers and for
    `Omitted`, only for optional parameters; S3 for `Omitted` outside
    a call's arguments; and D2 letting external declarations be owned
    by their `uses` library.
  - Tests: goldens for all of it, `tests/qname.rs` (Kotlin's examples,
    derivation, and resolution), `tests/core_fixture.rs` (the fixture
    against the vendored `core.temper`, read as text as the frontend's
    `builtin_supertypes_are_cores` does, so neither crate depends on
    the other), and the round trip's generator extended to every new
    kind.

What's left for `to_tmpl`, from this chunk: the frontend's own `uses`
section for what each library uses of core and std (from typing's
resolutions, which still name builtins by the old key names); qnames
computed by Kotlin's rules where they differ from derivation, stored as
overrides; the compiler-made `~console` per module, initialized by
`getConsole()` first in its init block; `Omitted` for left-out
arguments; and std's module paths in `uses` (a known gap, [names.md](names.md#as-built-2026-10-29)).

**2026-10-30: `to_tmpl` for subset A,** the frontend's translation
into tmpl (`crates/temper-frontend/src/to_tmpl.rs`; the frontend now
depends on temper-tmpl, which stays dependency-free):

- **What it takes:** a settled library with one module, no errors, and
  no tests. Types come from `Typing::expr` and `Typing::decls`, and what
  each call calls from `Typing::resolved`; nothing is resolved again.
  Anything else gives `NotYet`, saying what ("a class", "a nullable
  type", …) and where, never a panic.
- **What it covers:** top-level functions, module `let`s and `var`s with
  `InitLet`s, and top-level statements, in the frontend's order (as
  Kotlin's `sortTopLevels` arranges them); parameters, `var` ones
  included; locals; `if` as a statement and a value; `&&` and `||`;
  `while`, `do … while`, and `for` without `continue`; `break` and
  `continue`; returns, with a function's last value returned, pushed
  into a trailing `if`'s branches; calls of the library's functions;
  the 65 operators, `!=` as `Boolean.not` of the equality; core's
  members; string templates; and `console.log`.
- **Core's members** are copied as used from temper-tmpl's checked core
  fixture (`tests/fixtures/core.tmpl`, read once), by typing's key
  names (`Int32.toString`, `String.isEmpty`, `Console.log`,
  `getConsole`), so signatures, optional parameters, connected marks,
  and parameter names have one source. A member the fixture lacks is
  "not yet" (`Float64.near`, `List`'s).
- **`~console`:** declared the first time a module logs, as Kotlin
  does: the module's first item, initialized first in its init block
  from `getConsole(omitted)`, and `console.log(s)` a call of
  `Console.log` on it.
- **Not yet:** with the counts of functional tests that first need it,
  from `uv run compat/count_tmpl.py`: classes (23), several modules
  (9, imports), generic types (7, and builtin generic ones 4), errors
  (4, tests that expect them), `orelse` and `bubble()` (3), function
  types (2), `async` (2), parameters' defaults (2), generic functions
  (2), and one each of unknown types, nullable types, `continue` in a
  `for` loop, and core's `Float64.near`.
- **Tests:** `crates/temper-frontend/tests/to_tmpl.rs`: each of the 7
  tests translates with no validator problems, round trips through text
  with its spans, and matches its golden (`tests/to_tmpl/*.tmpl`,
  without spans); and every functional test translates validly or says
  why not.
- **To look at it:** `run-temper --tmpl FILE...` prints a library's
  canonical text, without spans, and a status line on stderr.

**Coverage, 2026-10-30:** 7 of 69 functional tests translate cleanly
(`algos/fibonacci`, `algos/hello-world`, `control-flow/if-return`,
`control-flow/loop-reenterable`, `names/nonascii`, `types/int/basics`,
and `types/string/is-empty`), 62 are not yet, and none is invalid. The
stretch targets need subset B: `functions/simple-locals` a function
type and a trailing block, `types/int/limits` and `types/float/basics`
`orelse`, `types/int/shifty` `StringBuilder` and `List`, and
`types/float/ops` static `let`s and `when`.

**2026-10-30: the Python backend's first variant, `py`,** for subset A
(Kotlin-compatible API and names, pretty bodies), with what it needed
in temper-tmpl:

- **`crates/temper-funtests`** (no dependencies): the vendored
  functional tests as a small API: list them, each test's sources (main
  first) and the std modules it imports, and its expected log per
  backend, honoring log blocks' selectors (```` ```log ~interp ````).
  The frontend's `to_tmpl` test and the Python backend's tests use it.
  The compat scripts already used the vendored copy, with `--temper DIR`
  for a checkout.
- **In temper-tmpl:** `naming` (the naming pass, `NamingRules`, `NameMap`,
  and `snake_case`, Kotlin's camel-to-snake conversion), and
  `analysis` (a walk, `module_writes` for Python's `global`, and
  `module_refs`). The naming pass's as-built rules are in
  [names.md](names.md#as-built-2026-10-30).
- **`crates/temper-be-py`** (depends only on temper-tmpl; the frontend,
  syntax, and funtests crates are dev-dependencies): `names` (Python's
  `NamingRules`: keywords, builtins, and the runtime names it imports
  reserved; snake_case; `_` before private module-level names; `_`
  after reserved ones), `ops` (all 65 operators, with `temper_core`'s
  wrapping helpers for `Int32` and `Int64` `add`, `sub`, `mul`, `div`,
  `neg`, and `shl`, as Kotlin's; Python's own operators elsewhere),
  `connected` (the table by qname for `getConsole`, `Console.log`,
  `Int32.toString`, `Int32.min`, `Int32.max`, and `String.isEmpty`),
  `translate` (tmpl to Python's tree), and `tree` (the output tree and
  its printer). No printing crate exists yet, so the printer stays in
  the Python crate until a second backend wants the same machinery.
- **Layout, as Kotlin's:** `fibonacci/pyproject.toml`,
  `fibonacci/fibonacci/__init__.py` (`from .fibonacci import *`), and
  `fibonacci/fibonacci/fibonacci.py`, depending on `temper-core==0.6.0`.
- **Type annotations:** parameters, results, module `let`s and `var`s,
  and locals declared without a value; other locals are inferred.
  `Console` is `temper_core`'s `LoggingConsole`.
- **Tests:** goldens of each of the 7 tests' module
  (`tests/goldens/*.py`, `UPDATE_SNAPSHOTS=1`), and a run test: each
  program runs with Python 3.11 or later against the vendored runtime
  (`vendor/temper/be-py/temper-core`, from the same Kotlin commit as
  the rest of `vendor/temper/`, now synced by `compat/sync_temper.py`),
  logs what the test expects of `py`, and passes pyright and mypy in
  strict mode ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)).
  Plus tables' tests (every operator spelled, every connection a
  connected declaration of core) and the naming pass's.
- **To look at it:** `tempest py [--out DIR] FILE...` prints the
  generated files, or writes them under DIR.

**What's left for Python,** first things first:

- **Type annotations beyond subset A:** nullable types as `X | None`,
  function types as `Callable`, classes and core's other types
  (`temper_core`'s names, as Kotlin annotates them), and generics.
- **Subset B** (classes, imports and several modules, `orelse`, closures,
  and the rest), toward all the functional tests, with the API diff.
- **The hoisting pass** (statements out of expressions), for
  `BlockValue`, and for `String.cmp` on operands that aren't names or
  literals; and `lower`, to run passes by profile.
- **Labeled jumps** out of a loop that isn't the innermost, and
  `continue` in `do … while`: not yet.
- **Value ranges,** so `Int32` arithmetic that can't overflow uses
  Python's own operators (`i - 1` for `i > 0`).
- **Docstrings** from doc comments, as Kotlin emits them, once `to_tmpl`
  carries docs.

**2026-10-30: the Rust backend's first variant, `rust`,** for subset A,
ahead of subset C, since subset A needs none of C's lowerings (results
are spelled directly, below), with what it needed in temper-tmpl:

- **In temper-tmpl:** `NamingRules` gained `raw_keywords` and
  `raw_prefix`, so Rust's keywords become raw identifiers (`r#type`)
  while the rest of what's reserved takes the suffix
  ([names.md](names.md#as-built-2026-10-30)).
- **`crates/temper-be-rust`** (depends only on temper-tmpl; the
  frontend, syntax, and funtests crates are dev-dependencies), with
  Python's modules: `names` (Rust's `NamingRules`: snake_case, no
  private prefix, raw keywords, and `self`, `Self`, `super`, `crate`,
  and the names the translator prints taking `_`), `ops` (all 65
  operators, as Kotlin's: `wrapping_*` for `Int32` and `Int64`,
  `temper_core::int_div` and the rest with `?` for what bubbles,
  `temper_core::float64::cmp` for `Float64` comparisons, and Rust's own
  operators elsewhere), `connected` (by qname: `getConsole`,
  `Console.log` as `println!`, `Int32.toString`, `Boolean.toString`,
  `Int32.min`, `Int32.max`, and `String.isEmpty`; and a table of
  connected types, where `Console` is erased, as Kotlin's), `translate`,
  and `tree` (the output tree and a printer that lays code out as
  rustfmt does). No printing crate yet: the two printers share only
  indentation.
- **Layout, as Kotlin's:** `fibonacci/Cargo.toml` (depending on
  `../temper-core` at `=0.6.0`, `rust-version` 1.71.1),
  `src/lib.rs` (`init(config)`, and the root module's exports),
  `src/main.rs`, `src/mod.rs` (the module), and `src/support/mod.rs`
  (the crate's `Config`). The module's `init` runs its init block once
  (`INIT_ONCE`), and its module `let`s and `var`s are locals there,
  except those functions use, or that are exported, which are statics
  with getters, as Kotlin's: a `OnceLock` for a `let` and an `RwLock`
  of an `Option` for a `var`. One improvement: a `var` assigned from
  itself alone, through operators (`ticks = ticks + 1`), is read and
  written under one write lock, so updates from several threads aren't
  lost ([profiles.md](profiles.md#as-built-rust)).
- **Prettier bodies:** names numbered only on conflicts, a function's
  last `return` as its value, `Ok(e?)` as `e`, `!(a < b)` as `a >= b`,
  literals inside format strings and locals named in them
  (`println!("Hello, {όνομα}!")`), types written only where Rust can't
  infer them, `mut` only where assigned, and only the `use`s needed.
- **Failure without lowering:** a function that may bubble returns
  `temper_core::Result`, a call that may bubble gets `?`, and `Error`
  is `return Err(…)`; the init block is a closure returning
  `temper_core::Result<()>`.
- **Tests:** goldens of each translated test's `src/mod.rs`
  (`tests/goldens/*.rs`), checked to be as rustfmt lays them out; and a
  run test: every generated crate builds offline, in one workspace with
  one target directory, against the vendored `temper-core`
  (`vendor/temper/be-rust/temper-core`, from the same Kotlin commit),
  logs what the test expects of `rust`, and passes clippy with
  `-D warnings` but for three lints the tests' own code trips
  ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)).
  Besides the 7 functional tests, three programs of its own
  (`tests/programs/`): `module-state` (statics, run from 8 threads by a
  harness), `operators` (wrapping, shifts, bubbling, string
  comparisons, and `do … while`), and `jumps.tmpl`, hand-written tmpl
  for a labeled `break` and a `continue` in `do … while`, which the
  frontend doesn't translate yet.
- **To look at it:** `tempest rust [--out DIR] FILE...`.

**What's left for Rust,** first things first:

- **Subset B and C:** classes (with Kotlin's struct, wrapper, and trait
  shapes), imports and several modules, `orelse` (over results, the
  results pass), closures with cells for assigned captures, nullable
  types as `Option`, and generics; then the API diff.
- **Copying `temper-core` beside the crate,** as Kotlin's `temper.out`
  does, so `tempest rust --out` builds alone.
- **Liveness for clones:** a `String` local is cloned at every read
  that needs a value of its own, but for a `return`; the last use
  needn't be.
- **The library's version:** tmpl says `0.0.0` where Kotlin's
  `Cargo.toml` (and `pyproject.toml`) say `0.0.1`, its default.
- **First-use init,** as profiles.md proposes, if exported functions
  should work before `init` is called; Kotlin's need `init` first, as
  these do.

**2026-10-30: the JS backend's first variant, `js`,** for subset A,
with what it needed in temper-tmpl, kept apart from the backend itself:

- **In temper-tmpl:** `NamingRules::block_scoped`, for targets whose
  locals are block-scoped and may not shadow (JS's `let` and `const`):
  two locals share a name only when their blocks don't nest
  ([names.md](names.md#as-built-2026-10-30)); Python and Rust leave it
  off. And `analysis::module_state`, which says which module `let`s and
  `var`s functions use and which the init block initializes at its top
  level; the Rust backend's translator now uses it in place of its own
  copy, with the same output, and JS's the second half.
- **`vendor/temper/be-js/temper-core`:** Kotlin's `@temperlang/core`,
  from the same commit as the rest of `vendor/temper/`, by `git archive`,
  now synced by `compat/sync_temper.py`.
- **`crates/temper-be-js`** (depends only on temper-tmpl; the frontend,
  syntax, and funtests crates are dev-dependencies), with Python's and
  Rust's modules: `names` (JS's `NamingRules`: names keep their case;
  Kotlin's reserved words and names, the globals the translator prints,
  and the runtime's helpers take `_`; block-scoped), `ops` (all 65
  operators, as Kotlin's: `Int32` as a `number` with `| 0` after `+`,
  `-`, and negation, `Math.imul`, and `divIntInt` and `modIntInt`;
  `Int64` as a `bigint` with `clampInt64` after `+`, `-`, `*`, `/`, and
  negation; the runtime's shifts where JS's are wrong; and `cmpFloat64`,
  `cmpString`, and `eqFloat64` for `Float64` and `String` comparisons),
  `connected` (by qname: `getConsole` erased, `Console.log` as the
  global `console.log`, `Int32.toString` and `Boolean.toString` as
  `.toString()`, `Int32.min` and `Int32.max` as `Math`'s, and
  `String.isEmpty` as `!s`; and a table of connected types, where
  `Console` is erased, as Rust's), `translate`, and `tree` (the output
  tree, and a printer in Prettier's default style).
- **Layout, as Kotlin's:** `fibonacci/package.json` (ES modules,
  depending on `@temperlang/core` 0.6.0), `fibonacci/index.js`,
  `fibonacci/fibonacci.js` (the public module, re-exporting the
  library's exports), and `fibonacci/fibonacci.internal.js` (the code).
  Module state is plain and eager, as Kotlin's
  ([profiles.md](profiles.md#as-built-js)).
- **Prettier bodies:** no numbered names, template literals for string
  concatenation (an `Int32` or `Boolean` in one without `.toString()`),
  `const` for what's never reassigned, `a !== b` and `a >= b` for
  negated comparisons, negative literals folded, labels only where a
  jump from an inner loop needs one, and no trailing `return;`.
- **Types:** JSDoc `@param`, `@returns`, and `@type` (on declarations
  without a value), enough for `tsc --checkJs --strict`.
- **Tests:** goldens of each program's `.internal.js`
  (`tests/goldens/*.js`), and a run test: every generated package runs
  under Node against the vendored runtime, logs what the test expects
  of `js`, and passes `tsc --checkJs --strict` against declarations
  generated from the runtime
  ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)).
  Besides the 7 functional tests, the Rust backend's three programs,
  read from its `tests/programs/`. Plus tables' tests: every operator
  spelled, every connection a connected declaration of core, and every
  runtime helper exported by the vendored runtime and reserved.
- **To look at it:** `tempest js [--out DIR] FILE...`.

**What's left for JS,** first things first:

- **Subset B** (classes, imports and several modules, `orelse`,
  closures, nullable types, and generics), toward all the functional
  tests, with the API diff; with several modules, `.internal.js` must
  export what the library's other modules import.
- **The hoisting pass,** for `BlockValue` (shared with Python).
- **Exported names, exactly as Kotlin's:** Kotlin escapes only JS's
  keywords in an export's name, and adds `_` to one that already ends
  in `_`; this reserves the runtime helpers and globals it prints too,
  so an export named `console` or `cmpString` would differ. Importing
  helpers under other names where a module-level name takes theirs
  would fix it.
- **`Float64` division and remainder by zero:** Kotlin's JS gives
  Infinity or NaN, where tmpl's operators bubble (as Python's and
  Rust's do). This follows Kotlin for now; which is right is a
  question for the team.
- **Copying `@temperlang/core` beside the package,** as Kotlin's
  `temper.out/js` has it.
- **Value ranges,** so `Int32` arithmetic that can't overflow drops its
  `| 0`.

**2026-10-30: source maps for JS, backends as features, and in
wasm,** before the Java backend:

- **In temper-tmpl:** a source file's line table, `line_starts` and
  `wide_lines` (nodes.md, "Source positions"), which `to_tmpl` fills
  from the source text (`Builder::set_file_text`); and `source_map`, a
  writer that records where output from each span starts
  (`MappedText`), a Source Map v3 encoder with Base64 VLQs, and a
  decoder for tests and tools. Columns count UTF-16 units on both
  sides. Maps leave out `sourcesContent`, as tmpl carries no source
  text, and name each source by its path after a prefix that leads
  from the map to the library's root.
- **In the JS backend:** a statement in the output tree may carry its
  span (`Stmt::Spanned`), and a function its name's; the translator
  gives each statement the span of the tmpl node it comes from, and
  the printer marks each statement, each function's JSDoc and
  `function`, and its name, with its source name in `names`.
  `translate_with_source_maps` adds `NAME.internal.js.map` after the
  module, which ends with a `//# sourceMappingURL=` comment, when the
  program has spans. Expressions aren't mapped yet.
- **`tempest js --out DIR`** writes maps, as Kotlin always does, naming
  the sources relative to the package; printing to stdout leaves them
  out. Kotlin writes no `sourceMappingURL` comment, so Node and
  browsers don't find its maps, and its `sourceRoot` doesn't lead to
  the sources ([kotlin-differences.md](../kotlin-differences.md)).
- **Tests:** VLQ and mappings against known vectors, line tables with
  wide characters, and JSON; goldens of each subset-A program's decoded
  map (`tests/goldens/*.map.txt`, a line per mapping with the text at
  both ends), each mapping checked to land on matching Temper (a
  `while` on `while`, a name on its name); Node's own source map
  support (`findSourceMap` after loading the module with
  `--enable-source-maps`) reading every mapping to the same position,
  in a source file that exists; and a stack trace from an error thrown
  in generated code pointing at the Temper line and column.
- **Backends as cargo features** of `tempest` and `temper-wasm` (`py`,
  `rust`, and `js`, all on by default; `tempest` also forwards `net`),
  so a small build can leave some out; each one's share is in
  size-and-speed.md, "Backends' share".
- **`temper-wasm`'s `generate`:** a library `main` from one source to
  any built-in backend's files, as JSON, with JS's source map; a test
  checks that it matches each backend's native goldens for subset A.

**2026-10-30: the Java backend's first variant, `java`,** for subset A,
with what it needed in temper-tmpl, kept apart from the backend itself:

- **In temper-tmpl:** `analysis::reads_only_itself`, whether a module
  `var`'s new value reads only it, locals, and literals, through
  operators, so the update may run under one lock; the Rust backend's
  translator now uses it in place of its own copy, with the same output.
  Nothing else was needed: no profile fields yet (failure, module init,
  and module `var`s are still the translator's choices, as for Rust and
  JS, since no pass reads them), and no live ranges (Java's names come
  from the block-scoped rules, as JS's).
- **`vendor/temper/be-java/temper-core`:** Kotlin's `temper-core` for
  Java, from the same commit as the rest of `vendor/temper/`, by `git
  archive`, now synced by `compat/sync_temper.py`. Its main sources need
  only the JDK.
- **`crates/temper-be-java`** (depends only on temper-tmpl; the frontend,
  syntax, and funtests crates are dev-dependencies), with JS's modules:
  `names` (Java's `NamingRules`: names keep their case; Kotlin's
  keywords, contextual keywords, and literals, and the types the
  translator names, take `_`; block-scoped; and the package and class
  names Kotlin derives from the library's), `ops` (all 65 operators:
  Java's own for `int` and `long`, which wrap and mask shift counts as
  Temper does, but `Core.divIntInt` and `Core.modIntInt`;
  `Integer.compare` and its kin, `Double.compare` for every `Float64`
  comparison, `Boolean.compare` for `Boolean`'s, and `compareTo` and
  `equals` for strings), `connected` (by qname: `getConsole` as
  `Core.getConsole(Logger.getLogger(PACKAGE))`, `Console.log`,
  `Int32.toString` and `Boolean.toString` as `Integer.toString` and
  `Boolean.toString`, `Int32.min` and `Int32.max` as `Math`'s, and
  `String.isEmpty` as `isEmpty()`; and connected types, `Console` as
  `temper.core.Core.Console`), `translate`, and `tree` (the output tree,
  and a printer with four-space indents, as Kotlin's output has).
- **Layout, as Kotlin's:** `fibonacci/pom.xml` (Kotlin's template, with
  `temper-core` 0.6.0 and Java 17),
  `fibonacci/src/main/java/fibonacci/FibonacciGlobal.java` (the module,
  a final class of statics whose static initializer is the init block),
  and `FibonacciMain.java` beside it. Module state is class init, with
  `volatile` `var`s and locked self-updates
  ([profiles.md](profiles.md#as-built-java)).
- **Prettier bodies:** no numbered names, no class name before the
  class's own calls and fields, `+` converting an `Int32` or `Boolean`
  in a concatenation once it's concatenating (`"fib(1)=" + fib(zero +
  1)`), `?:` for `if` values (Kotlin hoists them into statics), init-only
  module `let`s and `var`s as the initializer's locals, sorted imports,
  `a >= b` for negated comparisons, negative literals folded, labels
  only where a jump from an inner loop needs one, statements Java would
  find unreachable dropped, and no trailing `return;`.
- **Tests:** goldens of each program's `NAMEGlobal.java`
  (`tests/goldens/*.java`), and a run test: `javac` compiles the
  vendored runtime once, cached under `CARGO_TARGET_TMPDIR`, then every
  generated package and a harness running `module-state` from 8
  threads, with `-Xlint:all -Xdoclint:all,-missing -Werror`, no lint
  allowed; and `java` runs each, which logs what the test expects of
  `java` ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)).
  No Maven in the loop. Besides the 7 functional tests, the Rust
  backend's three programs, read from its `tests/programs/`. Plus
  tables' tests: every operator spelled, every connection a connected
  declaration of core, and every `Core` method called one that the
  vendored runtime has.
- **In `tempest` and `temper-wasm`:** `tempest java [--out DIR] FILE...`,
  behind a `java` feature, on by default in both, and `generate`'s
  backend 3, checked against the native goldens (with `main` for the
  library's names).

**What's left for Java,** first things first:

- **Subset B** (classes, imports and several modules, `orelse`,
  closures, nullable types as boxed types, generics, and function types
  as named functional interfaces, as Kotlin's), toward all the
  functional tests, with the API diff.
- **The hoisting pass,** for `BlockValue` (shared with Python and JS).
- **Source maps,** as Kotlin writes them beside each `.java` file, and
  maybe a stack-trace check like JS's.
- **String ordering by code point:** done (2026-10-30, below): a
  helper in the module's class, since `temper-core` has none.
- **Javadoc** from doc comments, as Kotlin emits it, once `to_tmpl`
  carries docs.
- **Copying `temper-core` beside the package,** as Kotlin's
  `temper.out/java` has it, and the Maven path checked against the fast
  path (test-speed.md).
- **`Float64` division and remainder by zero:** Infinity or NaN, as
  Kotlin's and JS's, where tmpl's operators bubble.

**2026-10-30: the C# backend's first variant, `csharp`,** for subset A,
and string order by code point on every backend:

- **In temper-tmpl:** `Case::Pascal` and `naming::pascal_case`, Kotlin's
  camel-to-Pascal conversion (`DisplayP3_ToSrgbLinear`, as Chromarite's
  C# has it), sharing `snake_case`'s word breaks, now factored out as
  `words`. Nothing else: C#'s labels, lock, and helper names are the
  translator's.
- **`vendor/temper/be-csharp/`:** Kotlin's `TemperLang.Core` and its
  library template (`Logging.cs`, `Program.cs`, and the rest), from the
  same commit as the rest of `vendor/temper/`, by `git archive`, now
  synced by `compat/sync_temper.py`.
- **`crates/temper-be-csharp`** (depends only on temper-tmpl), with
  Java's modules: `names` (Pascal case for functions and exported
  values, keywords as verbatim `@name`s, block-scoped, and the root
  namespace from the library's name), `ops` (all 65 operators: C#'s own
  for `int` and `long`, but the runtime's `Core.Div`, `Core.Mod`, and
  `Core.UShr`; `CompareTo` for integers' and booleans' three-way
  comparisons, `Float64.Compare` for floats', and
  `StringUtil.CompareStringsByCodePoint` for strings', as Kotlin's),
  `connected` (`getConsole` as the library's
  `Support.Logging.LoggingConsoleFactory.CreateConsole(ROOT)`,
  `Console.log` as `Log`, `Int32.toString` in the invariant culture,
  `Boolean.toString` as `? "true" : "false"`, `Int32.min` and
  `Int32.max` as `Math`'s, and `String.isEmpty` as `Length == 0`; and
  `Console` as `ILoggingConsole`), `translate`, and `tree` (Allman
  braces and a file-scoped namespace).
- **Layout, as Kotlin's:** `fibonacci/src/Fibonacci.csproj`,
  `fibonacci/src/FibonacciGlobal.cs` (the module, a static class whose
  static constructor is the init block), Kotlin's `Logging.cs`, and
  `fibonacci/program/` with its project and `Program.cs`
  ([profiles.md](profiles.md#as-built-c)).
- **Prettier bodies:** no numbered names, parameters and locals named
  as the source names them, `using`s instead of Kotlin's one-letter
  aliases (`C::Core`), qualified from `global::` only where a name of
  the program's would hide a class, a file-scoped namespace, blank lines
  between members, `?:` for `if` values, init-only module values as the
  constructor's locals, no self-assignments (C# warns of `zero = zero`),
  `goto` only for jumps out of inner loops (Kotlin wraps a function in
  labeled blocks to return early), and an interpolated string in the
  invariant culture for a concatenation of several numbers.
- **Tests:** goldens of each program's `ROOTGlobal.cs`, Kotlin's
  templates compared with the vendored ones, and a run test without
  MSBuild: `csc` compiles the runtime once, cached, then every library
  in one run with nullable reference types and every warning and .NET
  analyzer an error but CA2211; then `Program.cs` and a runner, run
  under `sv-SE` ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)).
  Besides the 7 functional tests, the Rust backend's four programs.
  Plus tables' tests, as Java's.
- **In `tempest` and `temper-wasm`:** `tempest csharp`, behind a
  `csharp` feature, on by default in both, and `generate`'s backend 4,
  checked against the native goldens.
- **String order:** the Rust backend's `string-order` program, checked
  against the interpreter, runs on every backend. Java's `compareTo`
  (Kotlin's, which orders UTF-16 units and gives a difference) is now a
  call of `compareCodePoints`, a private helper the module's class gets
  when it compares strings: UTF-16 units until the first difference,
  where a surrogate sorts above the rest. It lives in the class rather
  than a support file beside the package, since `temper-core` has no
  such helper, it's small, and a support file would add a class to the
  library's package. C# uses the runtime's
  `StringUtil.CompareStringsByCodePoint`, as Kotlin's C# does. JS's
  `<=>` takes `Math.sign` of `cmpString` and `cmpFloat64`, which give a
  difference (Kotlin's JS too). Python gained `Boolean.toString` (the
  runtime's `boolean_to_string`) for the program, and a Rust
  concatenation of literals alone is now one literal, not a `format!`
  clippy flags. Python and Rust strings order by code point natively.

**What's left for C#,** first things first:

- **Subset B,** as for Java, with properties for getters and setters,
  as Kotlin's, and nullable reference types.
- **The parameter-name analyzer** in the NuGet package
  ([surface.md](surface.md#parameter-names-where-languages-have-named-arguments-proposed-2026-10-30)),
  with packing.
- **The hoisting pass,** for `BlockValue`.
- **Source maps,** as Kotlin writes them beside each `.cs` file.
- **Copying `TemperLang.Core` beside the library,** as Kotlin's
  `temper.out/csharp` has it, and the package path (`dotnet build` of
  the generated projects, tried by hand) checked in a test against the
  fast path.
- **XML doc comments** from Temper's docs, once `to_tmpl` carries them.
- **Dependencies' root namespaces** among the names that would hide a
  class (a dependency `core` is namespace `Core`), with imports.

**2026-10-30: the C++ backend's first variant, `cpp`,** for subset A:

- **In temper-tmpl:** two analyses, `analysis::calls_nothing` (an
  expression of literals, locals, module `let`s, and operators alone,
  which may run under a lock) and `analysis::Effects` (what evaluating
  an expression may do that its order would show: write module state or
  call something that may, read a `var` that functions write, or
  bubble, through calls, with core's pure connected declarations listed
  in `PURE_CORE`), the "order-sensitive operands" analysis of
  [analyses.md](analyses.md); and `naming::hints`, the naming hints for
  temporaries (a callee's name with `get` or `to` dropped, the type's
  name, then `value`). Tests in `tests/analysis.rs`.
- **`vendor/temper/be-cpp/`:** Kotlin's header-only `temper-core` for
  C++, from the same commit as the rest of `vendor/temper/`, by `git
  archive`, now synced by `compat/sync_temper.py`.
- **`crates/temper-be-cpp`** (depends only on temper-tmpl), with C#'s
  modules: `names` (source names, Kotlin's keyword list, names the
  translator prints and the runtime's macros escaped with `_`,
  block-scoped, and the namespace from the library's name), `ops` (all
  65 operators: the runtime's `Int` and `Int64` helpers for wrapping
  arithmetic, division, remainder, and shifts left, but C++'s own `/`,
  `%`, and `>>` where a literal makes them safe; C++'s own comparisons,
  bitwise operators, and `Float64` arithmetic; `Float64::lt` and the
  like, `Compare::cmp`, and `Float64::cmp`, as Kotlin's), `connected`
  (`getConsole` and `Console.log` as the runtime's, `Int32.toString` as
  `std::to_string`, `Boolean.toString` as the runtime's, `Int32.min` and
  `max` as `std::`'s, `isEmpty` as `.empty()`, and `Console` as
  `std::shared_ptr<core::Console::Type>`), `translate`, and `tree`
  (Google's layout).
- **Layout, as Kotlin's:** `fibonacci/init.hpp`, `fibonacci/init.cpp`
  (namespace `fibonacci`, `global_init_init()`), and
  `fibonacci/main.cpp` ([profiles.md](profiles.md#as-built-c-1)). No
  build files, as Kotlin writes none.
- **Prettier bodies:** no numbered names, a `core` alias for
  `temper::core`, `+` and `std::to_string` for concatenations (Kotlin's
  `cat` goes through a `std::stringstream`), no extra blocks inside
  loops and branches, no labeled blocks for early returns, init-only
  module values as the init's locals, no self-assignments, and only
  exports in the header.
- **Safer than Kotlin's:** init once through `std::call_once`; module
  `var`s that functions write only under a mutex; Temper's order of
  evaluation kept with temporaries where it shows; shifts defined for
  every count; string literals compared as `std::string`s, never as
  pointers; no `-Wall` warnings (Kotlin's unused labels fail
  `-Werror`).
- **Tests:** goldens of each program's header and module, and a run test
  without CMake: `g++ -std=c++14` with 37 warnings, all errors, through
  a cached precompiled header of the runtime; one `main.cpp` and a
  runner, also under AddressSanitizer and UndefinedBehaviorSanitizer,
  and `module-state` from 8 threads under ThreadSanitizer
  ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)).
  Besides the 7 functional tests, the Rust backend's four programs and
  `hiding`, C++'s own. Plus tables' tests, as C#'s.
- **In `tempest` and `temper-wasm`:** `tempest cpp`, behind a `cpp`
  feature, on by default in both, and `generate`'s backend 5, checked
  against the native goldens.
- **String order:** C++'s `std::string` compares bytes as `unsigned
  char`, so UTF-8 orders by code point; the shared program confirms it.

**What's left for C++,** first things first:

- **Subset B,** with lists never iterated or referred to through a
  temporary ([profiles.md](profiles.md#as-built-c-1), "Temporaries that
  die in use"), classes as Kotlin's `struct`s with `make`, and getters.
- **The hoisting pass,** for `BlockValue`, and with it the
  evaluation-order temporaries as a shared pass, which can also go where
  the translator can't yet (`&&`'s right, a loop's condition).
- **Locals that shadow namespace-scope variables** a function doesn't
  use, which `-Wshadow` reports; the naming pass avoids only those a
  function refers to. No program here has one yet.
- **Several modules and libraries:** headers that include their
  dependencies', and init calling theirs, once.
- **Source maps,** as Kotlin writes them (`init.cpp.map`).
- **Copying `temper-core` beside the library,** as Kotlin's
  `temper.out/cpp` has it, and the package path checked in a test.
- **Init on first use,** if Kotlin takes [api-compat.md](../api-compat.md)'s
  proposal (it did, in #636; done 2026-10-31, below).

**2026-10-30: the Lua backend's first variant, `lua`,** for subset A,
the last of Kotlin Temper's seven backends:

- **In temper-tmpl:** nothing new. Lua uses `analysis::module_state`
  (which module values are declared before the functions),
  `analysis::module_refs` (functions referred to before they're
  defined, declared first), `analysis::Effects` (whether `and` and `or`
  may skip or repeat an operand, below), and `NamingRules::block_scoped`.
  It needs no temporaries, so no `naming::hints`, and no locks, so no
  `calls_nothing`.
- **`vendor/temper/be-lua/`:** Kotlin's `temper-core` for Lua, from the
  same commit as the rest of `vendor/temper/`, by `git archive`, now
  synced by `compat/sync_temper.py`.
- **In the interpreter:** `Boolean`'s `<` and the rest, which it
  couldn't compare ("Can't compare these"); Lua's own test program,
  checked against it, found it.
- **`crates/temper-be-lua`** (depends only on temper-tmpl), with C++'s
  modules: `names` (source names, ASCII only, Lua 5.4's keywords with
  `goto`, which Kotlin's list lacks, `_ENV`, and the names the
  translator prints, block-scoped), `ops` (all 65 operators: the
  runtime's `int32_add` and the rest for `Int32`, which wrap and divide
  as Temper does, its `band`, `shl32`, and the rest for bitwise
  operators and shifts, `float_eq` and `float_cmp` for `Float64`
  comparisons, and `str_cmp`, `int_cmp`, and `bool_cmp` for `<=>`, as
  Kotlin's; Lua's own `+`, `-`, and `*` for `Int64`, arithmetic for
  `Float64`, and comparisons for integers and strings; and `Boolean`'s
  order as logic, `not a and b`, where Kotlin's `a < b` fails),
  `connected` (`getConsole` erased, `Console.log` as `print`,
  `Int32.toString` as `tostring` or, with a radix, the runtime's,
  `Boolean.toString` as `tostring`, `Int32.min` and `max` as `math`'s,
  and `isEmpty` as `s == ""`), `translate`, and `tree` (StyLua's layout
  with two-space indents).
- **Layout, as Kotlin's:** `fibonacci/init.lua` (the root module, which
  returns its exports) and `fibonacci/fibonacci-dev-1.rockspec`
  ([profiles.md](profiles.md#as-built-lua)).
- **Prettier bodies:** no numbered names, `local function`s and locals
  declared where they get their values, `..` for concatenation (an
  `Int32` without `tostring`), `print` for logging, `c and a or b` for
  an `if` value, natural control flow, with `goto` only for `continue`
  and leaving an outer loop, `do return end` only where a `return` isn't
  last, no self-assignments, and strings as UTF-8.
- **Righter than Kotlin's:** exported `var`s that functions write stay
  current in the exports table; `goto` is reserved; and `Boolean`'s
  order works. Where its own spellings use `and` and `or`, which skip
  their right side, it checks that no operand Temper evaluates is
  skipped or repeated where that could show.
- **Lua's limits:** past 128 module-level locals, those after the first
  100 are fields of a `module` table, as Kotlin's; a function with more
  than 200 locals alive at once is not yet.
- **Tests:** goldens of each program's `init.lua`, Kotlin's rockspec, and
  a run test without LuaRocks: from the directory holding the libraries
  and the vendored runtime, with Kotlin's `LUA_PATH`, one `lua` process
  requires every library in turn, each logging what the test expects of
  `lua`; one also runs alone, as Kotlin runs it; `module-state`'s
  exports are read back after its functions write them; `luac -p`
  parses every file; and LuaLS checks them against their LuaCATS
  annotations and a typed view of the runtime
  ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)).
  Besides the 7 functional tests, the Rust backend's four programs and
  three of Lua's own: `hiding` (names Lua reserves or the translator
  prints, `Boolean`'s order, `if` values, strings, and shifts, checked
  against the interpreter), `repeats` (tmpl: a `continue` in a `do …
  while` whose body declares a local, and a `continue` of an outer
  loop), and `locals` (221 module-level declarations, past Lua's 200).
- **In `tempest` and `temper-wasm`:** `tempest lua`, behind a `lua`
  feature, on by default in both, and `generate`'s backend 6, checked
  against the native goldens.

**What's left for Lua,** first things first:

- **Subset B,** with classes as Kotlin's (`temper.type`, metatables, and
  `:` methods), `null` as Kotlin's `temper.null` sentinel, string
  lengths and indices in bytes as its runtime counts them, and closures
  as Lua's own.
- **The hoisting pass,** for `BlockValue`, and for an `if` value of type
  `Boolean` whose condition has effects (Lua's `and` and `or` can't
  choose a `false`), which is not yet.
- **Source maps,** as Kotlin writes them (`init.lua.map`).
- **Copying `temper-core` beside the library,** as Kotlin's
  `temper.out/lua` has it, and the package path (`luarocks make`)
  checked against the fast path, once Kotlin's rockspecs install
  ([kotlin-differences.md](../kotlin-differences.md), "Lua rockspecs
  that don't install").
- **LuaLS locally:** the check runs only where `lua-language-server` is
  installed, CI for now.
- **Lua 5.1 and LuaJIT,** which Kotlin's rockspec accepts but nothing
  tests.
- **Exported names with non-ASCII letters,** which Kotlin spells after
  NFKD normalization (`x3bfx301…`) and tempest in hex of the letters as
  written (`_3cc…`): the same only for ASCII names.

**2026-10-30: the backends' shared test kit, and `tempest build`:**

- **Test kit** (a restructuring of its own): `crates/temper-be-testkit`,
  a dev-dependency of every backend, holds what their harnesses shared
  ([testing.md](testing.md#backends-goldens-runs-and-type-checkers)),
  and the shared programs moved there from the Rust backend.
- **`tempest build [-w DIR] [-b BACKEND]...`** writes
  `temper.out/BACKEND/LIBRARY/…` as Kotlin's `temper build` lays it
  out, for every library under the work root, with each backend's
  runtime copied beside them (`runtime::FILES` in each backend crate)
  and a `CMakeLists.txt` per C++ library (`temper_be_cpp::cmake_lists`,
  which Kotlin doesn't write). Without `-b`, Kotlin's default backends
  but `data`. Its tree matches Kotlin's for the functional tests tmpl
  translates, but for intended differences
  (`crates/tempest/tests/kotlin_layout.rs`). Package files now give a
  library without a version Kotlin's `0.0.1` (Python, Rust, Java;
  `Program::version_or_default`).
- **Libraries import each other by name** in the frontend
  (`Library::libraries`, `project::Config`), and package files name the
  libraries a program `uses` as Kotlin's do (`Program::dependencies`);
  `to_tmpl` doesn't translate imports yet, so that waits for subset B's
  modules, as do Chromarite, Ordable, and Hrefine, whose tests import
  `std/testing`.

**2026-10-30: subset B's first step, modules, imports, and tests,**
for the libraries (Chromarite, Hrefine, and Ordable), which all failed
first at "several modules", since their tests import std's `testing`:

- **In temper-tmpl** ([modules.md](modules.md#level-3-modules-imports-and-tests-as-built-2026-10-30)):
  `Module::tests` and the `Test` node (a compiler-made function of std's
  `Test`, with its name for reports); other libraries' modules as
  `Module` declarations owned by their `uses` library, owning external
  functions, module `let`s and `var`s, classes, and interfaces; imports
  derived, not stored (`analysis::{home, imports, module_imports,
  test_imports}`); rules M1 (imports: exported, earlier modules, no
  writes) and M2 (a test's signature); text for both
  ([formats.md](formats.md)); the naming pass naming other libraries'
  declarations and keeping imported names from a module's own
  ([names.md](names.md)); and the round trip's generator making tests,
  std's `Test`, and imports of exported functions only.
- **In `to_tmpl`:** every module of the library, in the order they run,
  named by its path; imports followed to what they name, but an
  imported `var`, copied when the import runs, as the interpreters read
  it; what's used of std and of other libraries under the work root as
  `uses` sections by module; and tests (`SettledModule::test_cases`),
  whose names must be constant strings. `tempest build` gives the used
  libraries their configs' versions.
- **Shared programs:** `modules`, `imports-library`, and `testing`
  (`crates/temper-be-testkit/programs/`), with tmpl goldens.
- **`tempest test -b BACKEND`** builds for that backend and says what
  each library needs; running tests waits for std's `testing` in tmpl
  ([test-speed.md](../test-speed.md)).
- **Python:** a module per file, as Kotlin's (`pkg/pkg.py` the root,
  `pkg/helpers/__init__.py` for a module with modules under it,
  `pkg/helpers/deep.py`), `__init__.py` importing each module in order
  and then the root's names; `from pkg.helpers import twice` for other
  modules' and libraries' functions and `let`s, and another module's
  `var` through the module (`import pkg.helpers as _m_pkg_helpers`), so
  reads are current and writes land; std as `temper_std` and
  `temper-std` in `pyproject.toml`. Tests are a `unittest.TestCase` per
  module in `tests/test_MODULE.py`, a method per test (`test_twice_works`,
  its name as the docstring), making std's `Test` and calling
  `soft_fail_to_hard()` in a `finally`, as Kotlin's.
- **Rust:** a Rust module per module, as Kotlin's (`src/mod.rs`,
  `src/helpers/mod.rs`, `src/helpers/deep/mod.rs`), `init` initializing
  the libraries used and then each module in order; other modules' and
  libraries' declarations by path (`crate::helpers::twice`,
  `modules::sum`), `let`s and `var`s through their getters;
  `Cargo.toml` depending on other libraries by path, and on std as a
  dev-dependency where only tests use it. Tests are Kotlin's `#[test]`
  functions in a `#[cfg(test)] mod tests`, named from the test's words.
- **JS:** `PATH.js` and `PATH.internal.js` per module, `index.js`
  importing each module, `package.json` exporting each; imports from
  other modules' public files and other libraries' packages (std's as
  `@temperlang/std/testing`), live bindings keeping `var`s current.
  Tests are Kotlin's Mocha `it`s in `test/PATH.js`, importing from the
  module's `.internal.js`, which also exports what they use, with a
  `NAME$set` for each `var` they assign. Each `.internal.js` has its
  own source map.
- **Java:** a package per module with its `Global` and `Main` classes,
  as Kotlin's; references through those classes, imported or written
  in full; each Main loads the modules it imports first, in Temper's
  order (Kotlin's leaves them to the JVM's lazy initialization); std as
  `temper.std`, with other libraries and std in the pom, in test scope
  where only tests use them. Tests are a JUnit 5 class per module, a
  method per test between `new Test()` and `softFailToHard()`.
- **C#:** a namespace per module (`Mods.Helpers.Deep`), as Kotlin's;
  `using`s, or `global::` where a name would hide another; `Program.cs`
  running the imported modules' static constructors first; project
  references to other libraries and std. Tests are Kotlin's MSTest
  project, a class per module and a method per test.
- **C++:** `LIB/P.hpp` and `P.cpp` per module, as Kotlin's, in the
  library's namespace; headers include what they import, and each init
  calls the imported modules' inits first, once each; other libraries'
  names qualified (`modules::sum`). What C++ can't tell apart (two
  modules' inits or exports of one name) is not yet. Tests are Kotlin's
  functions of a `Test`, but only under `TEMPER_TESTS`, so a library
  builds without std, run by a separate `test.cpp` through the
  runtime's `run_tests`.
- **Lua:** `init.lua` and `PATH.lua`, required as `lib/PATH`; imports
  as locals from `require`, `var`s read through the other module's
  exports; the root module requires every module, so requiring the
  library runs them all (Kotlin's doesn't). Tests are Kotlin's LuaUnit
  file (`tests/PATH-test.lua`), reaching what the module doesn't export
  through a `$test` table, and, since Lua's `Test` is its runtime's,
  they run without std (under a LuaUnit stand-in in the harness).
- **Harnesses:** every backend runs `modules` and `imports-library`
  with the imported library beside it, and checks `testing`'s test form
  by goldens; Rust and C++ also build and run the tests against a
  stand-in for std's `testing` (`init`, `Test`'s constructor, and
  `softFailToHard`), Lua under a LuaUnit stand-in, and Python and JS
  parse them. Left: `analysis::Effects` sees one module's functions, so
  a call into another module counts as having no effect (C++ passes it
  every module's items for now); module paths whose parts aren't names
  in a target; and running tests for real, which waits for std.

**Coverage, 2026-10-30, after modules:** still 7 of 69: of the 9 tests
that first needed several modules, each now stops at something else
(classes 3, generic types 3, a call of a method 2, `for … of` 1), so
"several modules" is gone from the count, and classes (26) lead. The
libraries' next blockers are classes too: Chromarite's own class types
(`gamut.temper.md`), Ordable's classes (`smart.temper.md`), and
Hrefine's (`host.temper.md`); past them, each test's `assert` is a call
of std's `Test`'s method, and std's `testing` must translate for tests
to run.

**2026-10-30: classes (subset B, step 2).** The frontend generates
`@auto("==")`'s `eq` as a real member first (milestone-4.md,
"`@auto("==")` members generated"); then, in this crate and `to_tmpl`
([nodes.md](nodes.md), "Subset B's first step: classes"):

- **Model:** `Class` and `Interface` items with `Field`, `StaticField`,
  `Method` (methods, getters, setters, and constructors, by their
  declarations' kinds), `AbstractMethod`, and static `Function`
  members; `SetField` and `InitStatic`; `This`, `GetField`, `GetStatic`,
  `New`, `Upcast`, `Downcast`, `As`, `Is`, and `Identical`; declaration
  kinds `Field`, `StaticField`, `Constructor`, and `Setter`, facts
  `PRIVATE`, `PROTECTED`, `OPERATOR_EQ`, `OPERATOR_CMP`, and `AUTO`, and
  supertypes on declarations; validator rules C1 to C3; analyses
  `is_subtype`, `ancestors`, `members`, and `is_plain_to_string` (the
  one place that recognizes a class's display string, as Kotlin's
  `isPlainToString`); the naming pass naming members in their class's
  own namespace; imports following `new`, statics, `is`, conversions,
  and supertypes; and text for all of it.
- **In `to_tmpl`:** every module is declared first, then every class
  and interface, then their members, so code may name any class; a
  class's own constructor or the one its header implies; members read,
  called, and assigned by their resolutions, or for an assignment, by
  the object's class and the member's name, since typing doesn't
  resolve assignments yet; `==` and comparisons by `@operator` methods,
  identity where typing noted it; and `Upcast` wherever a value goes
  where a supertype is wanted, `Downcast` where an `is` check narrowed
  it. Not yet: generic classes, connected members, parameters'
  defaults, methods as values, supertypes from core (as `Stringable`),
  and static fields assigned again.
- **Backends,** all seven, each running the 14 class functional tests
  (`temper_be_testkit::CLASSES`) and the shared `classes` program,
  with goldens, under their checkers and sanitizers, in Kotlin's
  surface and layout (`kotlin_layout.rs` now covers the class tests
  too), Kotlin's bugs fixed (kotlin-differences.md, "Classes in the
  backends"):
  - **Python:** classes with `__slots__`, `__init__`, `@property` for
    public fields, getters, and setters, `_x` storage, `ClassVar` statics
    set in the init block, `@staticmethod`s; interfaces with `ABCMeta`
    and `@abstractmethod`s; `__eq__` and `__str__`; diamond picks as
    delegating methods.
  - **JS:** `class X extends type(…)` mixins, `#` storage and private
    members, accessors, `static new(props)` factories, statics set from
    init, forwarders for diamonds, `instanceof` and `requireInstanceOf`,
    source maps for members.
  - **Rust:** a class is a cloneable `Arc` handle with `new`, getters,
    `set_x`, and `builders`; an interface a trait plus a handle, as
    Kotlin's; `Display` and `PartialEq`; `var` fields each behind an
    `RwLock`; statics in `OnceLock`s.
  - **Java:** a file per type, `getX`/`isX`/`setX`, Kotlin's nested
    `Builder`, `default` methods with `X.super.m()` forwarders as
    Kotlin's, `instanceof` and casts.
  - **C#:** a file per type, `I`-prefixed interfaces, properties, default
    members through Kotlin's static helpers, statics as properties over
    the module class's fields, `override ToString`.
  - **C++:** a `struct` per type in the header, `make` factories,
    `get_x`/`set_x`, interfaces as virtual bases with pure members,
    Kotlin's const rule, statics set by the init, `checked_cast`, and an
    ASan-checked `lifetimes` program (field aliasing, chained calls).
  - **Lua:** `temper.type` tables named by module, `T.get`, `T.set`,
    `T.methods`, statics on the type, `__tostring`, LuaCATS classes for
    LuaLS.
  - **Shared needs found:** `analysis::Effects`, `module_state`,
    `module_writes`, and `module_refs` see only top-level functions, so
    C++, Lua, and C# gather class members themselves, and each backend
    computes the diamond winner; both belong here next.

**Coverage, 2026-10-30, after classes:** 21 of 69, from 7. The rest
first need: a generic type 12, a generic class 5, a parameter's default
5, a library with errors 4, a builtin generic type 4, `orelse` and
`bubble()` 3, a function type 2, builtin `async` 2, a nullable type 2,
a generic function 2, and one each of `AnyValue`, `continue` in a `for`
loop, a call of a function value, another library's class member and
constructor, core's `Float64.near`, and `for … of`. The libraries'
next blockers: nullable types for Chromarite (`convert.temper.md`) and
Hrefine (`url.temper.md`), and a builtin generic type for Ordable
(`smart.temper.md`); std's `testing` needs connected members,
generics, and nullable types before tests can run on a backend.

**2026-10-30: nullable types, `orelse`, `bubble()`, and `panic()`
(subset B, step 3a)** ([nodes.md](nodes.md), "Subset B's next step";
[types.md](types.md)):

- **Model:** `TypeData::Nullable`; nodes `Null`, `IsNull`, `NonNull`,
  `AsNullable`, `EqNullable`, `OrElse`, `Bubble`, and `Panic`, and `As`
  from `T?`; rule N1; text that reads back, and the round trip's
  generator making them. The type enums are reshaped ahead of generics
  (`Nominal { decl, args }`, `TypeParam`), with backends' placeholder
  `not_yet` arms.
- **In `to_tmpl`:** null checks as `IsNull`, narrowed reads as
  `NonNull`, `??` as a block value, `==` with a side that may be null
  as `EqNullable`, `orelse` (as a value, a statement, or a function's
  last value, typed from its parts' values where typing has no type),
  `bubble()`, `panic()`, and `is` and `as` on nullable values.
- **Backends,** all seven, running the 4 functional tests that now
  translate (`temper_be_testkit::NULLABLES`) and the shared `nullables`
  program, with goldens, under their checkers and sanitizers, in
  Kotlin's surface, Kotlin's bugs fixed (kotlin-differences.md,
  "Nullables in the backends"): `panic()` no `orelse` catches, but in
  JS and Lua yet; Lua's `Int64.toInt32`; C#'s culture; Java's boxes.
  Java and Rust have their own programs (`java-nullables`,
  `nullable-edges`), and the Kotlin layout comparison covers the 4
  tests too.
- **Not yet:** `??` on a `var` (a false error, as in Kotlin); `panic()`
  apart from `bubble()` in JS and Lua.

**Coverage, 2026-10-30, after nullable types:** 25 of 69, from 21
(`control-flow/bubble`, `semantics/mutually-referencing-types`,
`types/float/basics`, and `types/int/limits`). The rest first need: a
generic type 12, a generic class 5, a parameter's default 5, a library
with errors 4, a builtin generic type 4, a function type 2, builtin
`async` 2, a generic function 2, and one each of `AnyValue`, `continue`
in a `for` loop, a call of a function value, another library's class
member and constructor, an assignment, core's `Float64.near`, and `for
… of`.

**2026-10-31: generics (subset B, step 3b)** ([nodes.md](nodes.md),
"Subset B's next step: generics"; [types.md](types.md)):

- **Model:** `Nominal { decl, args }` with type arguments, `TypeParam`
  declarations owned by their generic declarations, calls' `TypeArgs`
  (rule T4), `GetField` typed through its object's type arguments,
  subtyping through declared supertypes (invariant arguments), and
  `ListLit`; core's `Listed`, `List`, `ListBuilder`, `Pair`, `MapKey`,
  `Mapped`, `Map`, and `MapBuilder` in the core fixture.
- **In `to_tmpl`:** generic functions, classes, interfaces, and methods,
  calls with their type arguments, list literals, and core's generic
  types and members that take no function types.
- **Backends,** all seven, running the 3 functional tests that now
  translate (`temper_be_testkit::GENERICS`) and the shared `generics`
  program, with goldens, under their checkers and sanitizers (Java also
  `java-generics`, and C++'s ASan `lifetimes` with lists), in Kotlin's
  surface, Kotlin's bugs fixed (kotlin-differences.md, "Generics in the
  backends"): JS's `getOr` of null values, Rust's class bounds and
  string literals for type parameters, C++'s aliasing of list fields,
  and Java's boxed `==`.
- **Not yet:** generic methods in C#'s with constraints, C++, and
  Rust's interfaces; nullable type parameters in C# and C++; members
  that take function values (`map`, `filter`, `join`, …); the Kotlin
  layout comparison for the generics tests.

**Coverage, 2026-10-31, after generics:** 28 of 69, from 25
(`casts/specific`, `imports/values`, and `types/list/empty`). The rest
first need: a parameter's default 6, a function type 6, a library with
errors 4, a builtin type core's fixture lacks 4, another library's class
member 3, an expression of unknown type 2, builtin `async` 2, `for … of`
2, core's `Listed.join` 2, and one each of `AnyValue`, `continue` in a
`for` loop, a call of a function value, type arguments, an assignment,
another library's constructor, a builtin generic type, and core's
`Float64.near`, `Listed.sorted`, and `Listed.map`.

**2026-10-31: vendor sync to Kotlin main (0db80cc8).** Three new
functional tests (`control-flow/async-broken-early`, `await-in-loop`,
and `types/connected`), none of which translate yet, and
`control-flow/bubble` gained `orelse` folding cases with a `continue`
in a `for` loop. `to_tmpl` now translates that: tmpl's `while` has no
step, so the step runs before each `continue` of its loop, as `{ step;
continue loop; }`, where the step declares nothing and has no loops or
functions (`i++` declares a temporary, so `control-flow/loops` still
waits). A `do … while` with a `continue` stays not yet. A connected
class (#633), whose methods have no bodies, is "not yet" rather than
invalid tmpl.

**Coverage, 2026-10-31, after the sync:** 28 of 72 (from 28 of 69). The
rest first need: a parameter's default 6, a function type 6, a library
with errors 4, a builtin type core's fixture lacks 4, a builtin generic
type 3, another library's class member 3, an expression of unknown type
2, builtin `async` 2, `for … of` 2, core's `Listed.join` 2, and one each
of `AnyValue`, `continue` in a `for` loop whose step declares, a call of
a function value, type arguments, an assignment, a connected class,
another library's constructor, and core's `Float64.near`,
`Listed.sorted`, and `Listed.map`.

**2026-10-31: C++ and Rust initialize on first use,** matching Kotlin
main's #636 (api-compat.md, proposal 2). C++: `global_init_<module>()`
declared first in each header, through `temper::core::init_once`;
module values in `temper_globals` and statics in a class's
`temper_statics`, as `GlobalSlot`s, which the init constructs first,
then runs its imports' inits, then the top level; accessors `x()` and
`Type::x()`; and entry points that run the init first. Rust: getters
read through `temper_core::initialized` and `init_on_first_use()`, and
`init` wraps its module inits in `temper_core::initializing`. Where
Kotlin's is unsafe, tempest's differs: a C++ accessor of a `var` that
functions write gives a copy under the module's lock, and a Rust read
on another thread waits for the whole init (kotlin-differences.md). New
tests call entry points with no init first, in C++ under both
sanitizer builds, and the 8 threads enter `module-state` with no init,
under ThreadSanitizer in C++. Kotlin comparisons that waited for a
CLI built from main: `kotlin-out.txt` re-recorded (one new file,
`module_init.hpp`), and `check_lex.py`, `check_parse.py`, and
`check_ast.py` agree on block-quoted code since #632, so those three
cases are no longer listed as deviations.

**2026-10-31: function values (subset B's next step)** ([nodes.md](nodes.md),
"Subset B's next step: function values"; [types.md](types.md),
"Function values"):

- **Model:** `FunctionLiteral` (its declaration a `Lambda`, which lists
  its captures), `FunctionRef`, `CallValue` (typed by `TypeRule::ValueCall`),
  `ToFun`, and `FromFun`; the fact `FUN` on `@fun` interfaces; function
  types as values' types, read back in text (`fn (A): R`); analyses
  `fun_apply`, `fun_signature`, `captures`, `assigned_captures`, and
  `owned_by`, and `Builder::fun_signature`, with `finish` interning each
  `@fun` interface type's signature; rules V1 and D5, with R3, R5, and
  R6 for captures; the naming pass naming literals, their parameters
  and locals (the init block's too), and keeping captures live to the
  end; the round trip's generator making called literals and
  references; and core's `map`, `filter`, `join`, `sorted`, `reduce`,
  `reduceFrom`, `ListBuilder.sort`, and `Mapped.forEach` in the core
  fixture, whose test now reads function types in core.temper.
- **In `to_tmpl`** (`to_tmpl_functions.rs`): literals and trailing
  blocks typed as typing gives them, references to declared functions
  and static methods, calls of values (a local's, a parameter's, a
  field's, a call's result), function types, `ToFun` and `FromFun` where
  a value's `@fun` interface or function type isn't the one wanted,
  local functions that don't call themselves (a local holding a
  literal), and core's `forEach` and `for … of`, inlined as Kotlin does
  (a `while` over the list's indices, so `return` returns from the
  function around it, and `break` and `continue` are the loop's). A
  generic external method's type parameters are copied before its
  signature, which names them. A constant `/` or `%`, which typing lets a
  function that can't bubble use, as Kotlin does, is "not yet" there,
  rather than invalid tmpl, since tmpl's operators may bubble.
- **Backends,** all seven, running the 6 functional tests that now
  translate (`temper_be_testkit::FUNCTIONS`) and the shared `functions`
  program, with goldens, under their checkers and sanitizers, in
  Kotlin's surface, Kotlin's bugs fixed ([profiles.md](profiles.md),
  "As built, function values"; kotlin-differences.md, "Function values
  in the backends"): Python lambdas or nested `def`s with `nonlocal`,
  and a factory for a loop's `let`s, which Python's closures would share;
  Rust `Arc<dyn Fn … + Send + Sync>` and `move` closures with clones,
  cells as `Arc<RwLock<T>>`, and plain closures where core takes `&dyn
  Fn`; JS arrows or local `function`s; Java's `java.util.function`
  interfaces by signature, or one synthesized in `PKG.function`, lambdas,
  method references, adapters, and local-class cells; C# `Func` and
  `Action`, typed lambdas, local functions, and method groups; C++
  `std::function`, `[=]` lambdas, `shared_ptr` cells, and `self` for
  `this`, under ASan, UBSan, and TSan; Lua one-line functions or `local
  function`s. The C++ effects analysis stops at a literal's body and
  takes a call of a value to read and write anything.
- **Not yet:** a local function that calls itself; a captured `var`
  parameter that's assigned (Rust, Java); bound method references
  (`k.isBig`), which Kotlin rejects too; generic functions as values;
  parameters' defaults in function types; a Python closure that assigns
  a loop's local; synthesized Java interfaces of other than scalars; the
  Kotlin layout test for these tests.

**Coverage, 2026-10-31, after function values:** 34 of 72, from 28
(`control-flow/for-of-folded`, `functions/as-values`,
`functions/constructor-callbacks`, `functions/simple-locals`,
`imports/functions`, and `types/list/sorting`). The rest first need: a
parameter's default 6, a builtin generic type 4, another library's
class member 4, a library with errors 4, a builtin type core's fixture
lacks 4, an expression of unknown type 2, builtin `async` 2, and one
each of `AnyValue`, `continue` in a `for` loop whose step declares, a
local function that calls itself, type arguments, core's
`String.split`, an assignment, a connected class, another library's
constructor, core's `Float64.near`, a generic call's type arguments, a
division that can't fail where nothing may bubble
(`types/list/reduce`), and core's `Mapped.toListWith`.

**2026-11-01: parameters' defaults (subset B's next step)** ([nodes.md](nodes.md),
"Subset B's next step: parameters' defaults"; [types.md](types.md)):

- **Model:** a parameter with a default is `OPTIONAL` and typed `T?`
  (null means its default, as in Kotlin), and its function's body starts
  with its default, `let x' = coalesce(x, d)`, in parameter order (rule
  N2, `analysis::defaults`); `Coalesce`, Temper's `??` (rule N1), only
  for defaults so far; text `(coalesce(a, b)): T`; the round trip's
  generator making defaulted functions and calls that leave arguments
  out or pass null.
- **In `to_tmpl`** (`to_tmpl_defaults.rs`): defaults of functions,
  methods, static methods, and constructors (their own and implied
  ones, from a header's fields); calls that leave
  trailing optional arguments out (`Omitted`) or pass `T` (as `T?`) or
  null, an object literal's left-out properties included; a trailing
  block that typing places before later optional parameters' arguments;
  and, on the way, a class's or interface's value in a string template,
  as a call of its `toString()`. Function types keep optional
  parameters (`fn (Int32, _?: Int32?): String`), so a reference to a
  function with defaults translates.
- **Backends,** all seven, running the functional test that now
  translates (`temper_be_testkit::DEFAULTS`) and the shared `defaults`
  program (two modules: defaults left out, given, and null, defaults
  that read earlier parameters or run only when needed, nullable
  parameters, methods, static methods, constructors, an object literal,
  and another module's function and class), with goldens, under their
  checkers and sanitizers, in Kotlin's surface ([profiles.md](profiles.md),
  "As built, parameters' defaults"; kotlin-differences.md, "Parameters'
  defaults in the backends"): Python `x: int | None = None`, Rust
  `Option<T>` with `None` passed and an options builder for exported
  classes, JS `[x]`, Java and C++ overloads, C# `int? x = null`, and Lua
  `x?`; bodies set the default in place where the language allows, or
  in one expression.
- **Not yet:** Kotlin's "Optional arguments must follow required
  arguments" (backends then give only trailing optional parameters
  defaults); a C++ interface's abstract method with optional
  parameters; function literals with defaults, and calls of function
  values that leave arguments out, which no test makes (typing types no
  literal without a declared type, which can't say there are defaults);
  a `var` parameter with a default; and
  `Coalesce` for `??` in general, which stays a block value.

**Coverage, 2026-11-01, after parameters' defaults:** 35 of 72, from 34
(`classes/property-order`). The 5 other tests that waited for defaults
now first need something else: core's `String.countBetween`
(`functions/defaulting`, `functions/connected`), a builtin type core's
fixture lacks (core's `Empty` in `functions/named-args`, and in
`types/string/read`), and a class declared in a function
(`classes/object-literals`). All 37 not yet first need: a builtin type
core's fixture lacks 6, a builtin generic type 4, another library's
class member 4, a library with errors 4, an expression of unknown type
2, builtin `async` 2, core's `String.countBetween` 2, and one each of
`AnyValue`, a class declared in a function, `continue` in a `for` loop
whose step declares, a local function that calls itself, type
arguments, core's `String.split`, an assignment, a connected class,
another library's constructor, core's `Float64.near`, a generic call's
type arguments, a division that can't fail where nothing may bubble,
and core's `Mapped.toListWith`.

**2026-11-02: fields first** ([typing.md](../typing.md), "Fields
first"). The Temper team agreed that a constructor doesn't use `this`
until every field is set, and tempest now checks it after typing, a
deliberate difference from Kotlin, which accepts such constructors
(and whose Rust backend then fails to compile them). Also "Property b
not initialized in constructor" for an explicit constructor that
leaves a field unset on a path that returns normally. No library or
functional test the scan covers breaks the rule. The Rust backend's
constructor still wants fields set first by top-level statements;
branches and loops there are a follow-up. `run-temper --untyped` now
prints diagnostics with their files and lines. Coverage stays 35 of 72.

**2026-11-02: core's other types (subset B's next step)** ([nodes.md](nodes.md),
"Subset B's next step: core's other types"; [types.md](types.md)):

- **Core fixture:** string indices (`StringIndex`, `NoStringIndex`, and
  the sealed `StringIndexOption`, with its `@operator` `compareTo` and
  `eq`), `String`'s `begin`, `end`, `get`, `hasIndex`, `next`, `prev`,
  `step`, `hasAtLeast`, `countBetween`, `slice`, `split`, `indexOf`,
  and `fromCodePoints`; `StringBuilder`; `Empty` and `empty()`; `Deque`;
  `DenseBitVector`; `Float64`'s `near`, `e`, and `pi`; and
  `Mapped.toListWith`. Its test now reads core's declarations whose
  parameters span lines.
- **In `to_tmpl`:** a builtin type that isn't a scalar is `Nominal` of
  core's declaration, so `is` and `as` on it work as for a class; core's
  statics are `GetStatic`; operators on string indices call
  `StringIndexOption`'s `@operator` methods; `char'a'` is an `Int32`
  constant; `for … of` a string inlines core's `String.forEach`; core's
  optional parameters that named arguments skip are `Omitted`; and a
  library's bodiless `@connected` function is "not yet". `run-temper
  --tmpl` prints invalid tmpl too, to show where its problems are.
- **Backends,** all seven, running the 9 functional tests that now
  translate (`temper_be_testkit::CORE_TYPES`), with goldens, under their
  checkers and sanitizers, in Kotlin's surface ([profiles.md](profiles.md),
  "As built, core's other types"; kotlin-differences.md, "Core's other
  types in the backends"). Python's naming now shares a name only
  between locals of one type (with or without null), and not into a
  `let`'s initializer (`NamingRules::same_type_shares`). Lua runs
  `types/float/ops` and `types/string/read` only on Lua 5.3 or later,
  since Kotlin's runtime gets them wrong on LuaJIT.
- **Not yet:** `functions/connected` (a library's own connected
  functions, with their backend code); `types/int/shifty` and
  `types/map`, which divide by a constant where nothing may bubble;
  and the builtin generic types of `async` (`SafeGenerator`,
  `PromiseBuilder`).

**Coverage, 2026-11-02, after core's other types:** 44 of 72, from 35
(`functions/defaulting`, `functions/named-args`,
`regression/minimal-repro`, `types/dense-bit-vector`, `types/deque`,
`types/float/ops`, `types/string/build`, `types/string/indices`, and
`types/string/read`). All 28 not yet first need: another library's
class member 4, a library with errors 4, a builtin generic type
(`async`'s) 3, a division that can't fail where nothing may bubble 3,
an expression of unknown type 2, builtin `async` 2, and one each of
`AnyValue`, a class declared in a function, `continue` in a `for` loop
whose step declares, a library's own connected function, a local
function that calls itself, type arguments, an assignment, a connected
class, another library's constructor, and a generic call's type
arguments.

**2026-11-02: connected-type fallbacks** (Kotlin's
`connected-type-fallback` branch; [kotlin-differences.md](../kotlin-differences.md),
"A user's `@connected` type"). A user's `@connected` class may have a
complete Temper implementation, which the interpreter runs, so
check_functional passes all 72. The frontend marks such a class
(`CLASS_CONNECTED`, and `CLASS_CONNECTED_LIST` with a backend list) and
reports Kotlin's errors for it. In tmpl, a user's connected class is
"not yet", now by that mark, since its Temper implementation would
otherwise translate as an ordinary class on every backend, where
listed ones must use host code. **Not yet, together:** backends reading
a library's host files (`__connected__.py`, `_connected.rs`,
`_connected.js`, `_connected.lua`, `_connected.hpp`, and Java's and
C#'s classes), and choosing per backend between host code and the
Temper implementation, as Kotlin's `ConnectedChoice.kt` does (a
listed backend, without `--no-connected`, uses host code; the others
drop the `connected` mark and translate the implementation), with
Java and C# leaving out a host file named for a class they generate.
The fallback doesn't make this small: the choice itself is easy, but
every backend still needs host files copied, imported, and named for
`types/connected`'s `Counter` and `functions/connected`'s functions,
so both wait for that chunk. Coverage stays 44 of 72.

**2026-11-02: backends read host files** (Kotlin's
`connected-type-fallback` branch at a4cd9a9f; [names.md](names.md),
"Connected declarations"; [kotlin-differences.md](../kotlin-differences.md),
"A user's `@connected` type"):

- **Frontend:** the library records each `@connected` function and
  class with its backend list, a bodiless function's parameters'
  names, and `@keep`. `to_tmpl` makes a connected function with a body
  a `Function` with `CONNECTED` (and its backends), a bodiless one a
  `BodilessFunction` called directly, and a connected class with its
  Temper implementation an ordinary `Class` with `CONNECTED`;
  `@connected([])` connects to no backend; `@keep` is `KEEP`.
- **In this crate:** `host` gives a program its host files and
  `--no-connected` (`Program.host`, set by the build, not in text), and
  `uses_host` makes Kotlin's `ConnectedChoice` per backend: bodiless,
  host; `--no-connected`, Temper; no list or listed, host; else Temper.
  `replaced_by_fallbacks` names Java's and C#'s class files to leave
  out. The naming pass names bodiless functions' parameters.
- **Backends,** all seven, in Kotlin's layout: each copies its host
  files (Python and Lua inline `__connected__.py` and `_connected.lua`
  into the module; Rust puts `_connected.rs` in `src/_connected/mod.rs`),
  wraps a host function after its defaults (`_connected.sum(i, j,
  bonus)`, `WorkConnected.Sum(…)`, `_connected::sum(…)`), and makes a
  host class a name for the host's (`Counter = _connected.Counter`), or
  in Java and C# its own file. Generated code exposes what host code
  uses, as Kotlin's (JS's `.internal.js` exports everything in a module
  with host code). A missing host file is "not yet", naming it.
- **Tests:** `temper_be_testkit::CONNECTED` runs `functions/connected`
  and `types/connected` on every backend, as is and under
  `--no-connected` (`Case::no_connected`, log blocks named
  `no-connected`), in a library named `work`, as host code expects.
  Python skips its strict checkers for libraries with host code, which
  the tests' `__connected__.py` isn't written for.
- **CLI:** `tempest build --no-connected`; a library's other files go
  with its program as host files.

**Coverage, 2026-11-02, after host files:** 46 of 72 through tmpl and
every backend, from 44 (`functions/connected` and `types/connected`).

**2026-11-03: divisions that can't fail, and maps** ([nodes.md](nodes.md),
"Ops"; [kotlin-differences.md](../kotlin-differences.md), "Divisions
that can't fail, and maps, in the backends"):

- **In this crate:** the operator table has 69 operators, with
  `divSafe` and `modSafe` for `Int32` and `Int64`, which never bubble.
  `same` compares calls' type arguments by their types, not by where
  each program interned them.
- **In `to_tmpl`:** `/` by an integer constant other than 0, and `%` by
  a positive one, is `divSafe` or `modSafe` wherever it is, as Kotlin's
  `DivIntIntSafe` and the like, so a function that can't bubble may
  divide by one; a divisor typing works out as constant only by folding
  is still "not yet". **Typing:** a builtin generic type's constructor
  given its type arguments (`new Map<Int, Int>(…)`) wants its
  parameters' types for its arguments, so `[]` there is typed.
- **Naming:** `NamingRules::init_literals_avoid_module` (Java and C++):
  a function literal in the init block doesn't take the name of a
  module `let`, or of an init local whose block holds it.
- **Backends,** all seven, running `temper_be_testkit::DIVISIONS`
  (`types/int/shifty`, `types/list/reduce`, and `types/map`), with
  goldens, under their checkers and sanitizers: each spells the new
  operators as Kotlin's, connects `Mapped.toListWith` (C# `Mapped.forEach`
  too), and the fixes in kotlin-differences.md. Rust's printer breaks a
  long `static` after `=`, long type parameters one per line, and counts
  a `?` on the line of the call it follows.

**Coverage, 2026-11-03, after divisions that can't fail:** 49 of 72
through tmpl and every backend, from 46 (`types/int/shifty`,
`types/list/reduce`, and `types/map`). The 23 not yet first need:
another library's class member 4 (std's `regex`, `testing`, and
`json`), a library with errors 4, an expression of unknown type 3, a
builtin generic type (`async`'s) 3, builtin `async` 2, and one each of
`AnyValue`, a class declared in a function, `continue` in a `for` loop
whose step declares, a local function that calls itself, type
arguments, an assignment, another library's constructor, and a generic
call's type arguments.

**2026-11-03: labeled loops' jumps and classes declared in functions**
([kotlin-differences.md](../kotlin-differences.md), "Labeled loops and
classes declared in functions, in the backends"). Chosen over std's
`testing`, which needs another library's class members declared in
tmpl, std translated as a library of its own with connected members
chosen per backend, and each backend running tests and reading their
failures: two or three chunks, for one functional test now.

- **In `to_tmpl`:** a jump to a labeled block that ends in a loop
  (`outer: for (…)`) is a jump of that loop; a `for` loop's step may
  declare locals, declared again in each copy before a `continue`;
  `i++` alone is `i = i + 1`; what follows a `while (true)` that
  nothing breaks is left out, as Java and Rust reject it; a class
  declared in a function that names none of the function's locals is
  declared at its module's level, as Kotlin's `ClosureConvertClasses`
  does (one that does is still "not yet"); a type alias (`let
  ChangeType = String`) is no item; a call's explicit type arguments
  (`leastX<C>(a, b)`) are typing's; `x as I` for an `x` already an `I`
  is an `Upcast`; an assignment as a value (`(x = 1) orelse …`) is a
  block giving `void`; and an `orelse` whose body can't bubble is its
  body.
- **Frontend:** the `eq` a field-less class below an `@auto("==")`
  interface gets is `other is C`, not `if (other is C) { true } else {
  false }`.
- **Backends,** all seven, running `temper_be_testkit::LOOPS_AND_CLASSES`
  (`algos/myers-diff`, `classes/object-literals`, `control-flow/loops`,
  and `interfaces/property-members`) under their checkers: Python's
  jumps out of an inner loop use the runtime's `Label` and `LabelPair`,
  as Kotlin's; Rust and Java build generic classes' builders; C# writes
  a function's `where` constraints and hoists a block value past reads
  of `var`s no block value assigns; JS declares a generic class's
  factory generic and an interface's inherited abstract properties; Lua
  bounds `---@generic` and types a local of a type parameter `any`; and
  the fixes in kotlin-differences.md.

**Coverage, 2026-11-03, after labeled loops and local classes:** 53 of
72 through tmpl and every backend, from 49 (`algos/myers-diff`,
`classes/object-literals`, `control-flow/loops`, and
`interfaces/property-members`). The 19 not yet first need: another
library's class member 4 (std's `regex`, `testing`, and `json`), a
library with errors 4, a builtin generic type (`async`'s) 3, an
expression of unknown type 2 (`classes/angle-call`'s generic static
method, and `semantics/constness`'s `var` without a type or value),
builtin `async` 2, and one each of `AnyValue`, a local function that
calls itself, another library's constructor, and a generic call's type
arguments (`List.of<AnyValue>`).

**2026-11-03: other libraries' class members and constructors, and
their names** ([modules.md](modules.md), "Other libraries' modules";
[names.md](names.md), "With modules"; open questions 46 and 47, decided
with the user). The first half of std's `testing`:

- **In `to_tmpl`:** another library's class or interface comes with its
  type parameters and supertypes, and each member code uses (field,
  method, getter, setter, static), and its constructor (its own, or its
  header's), is declared the first time it's used, `@connected` ones
  marked connected, with their parameters' names; a template's part of
  another library's class calls its `toString()`. `x is C` for a
  nullable local is a null check and then `is` on its non-null value,
  as tmpl's `is` checks only what isn't null. A test's name may be a template of constants, as
  `"a test ${name}"` for a module `let name = "case"`, folded as
  Kotlin folds it.
- **Naming (open question 46):** `naming::Interface` holds what a
  library's own naming gave its module-level declarations and their
  members, by qname; a build gives each program its dependencies'
  (`Program::interfaces`, per backend, not in text), and the naming pass
  spells those names, never deriving them again. A library with no
  interface is an error (`NameMap::missing`), but std's, derived until
  std builds as a library. `tempest build` translates libraries in
  dependency order, each backend naming each with the interfaces of
  those before it; the testkit's `program_for` does the same for its
  shared libraries.
- **Backends,** all seven: another library's members and constructors
  are used as the program's own are, by its naming's names; only what
  `qname::by_connection` says (core's, and std's connected members) goes
  through the connection tables. Each backend's naming now reports a
  missing interface as "not yet".
- **One library per program (open question 47):** as already built:
  `to_tmpl` translates one library, its dependencies only as `uses`
  sections; the interfaces are now the rest of what comes in.
- **Tests:** a naming test where the dependency's naming changes an
  export (a reserved word) and a connected function, and an interface's
  name that derivation wouldn't give is kept; the shared library
  `renamed` (exports named for target keywords: `lambda`, `native`, and
  a class `Checked` with `end`, `delete()`, and `local`) used by
  `imports-library`, on every backend; and tmpl goldens of the five
  functional tests that now translate.

**Coverage, 2026-11-03, after other libraries' members:** 58 of 72
through tmpl, from 53 (`regex/match`, `regex/zero-advance`,
`testing/asserts`, `types/date`, and `types/json-syntax-tree`); still 53
through every backend, as those five need std translated as a library of
its own, its connected members chosen per backend by Kotlin's
support-network keys: the next chunk. The 14 not yet first need: a
library with errors 4, a builtin generic type (`async`'s) 3, an
expression of unknown type 2, builtin `async` 2, and one each of
`AnyValue`, a local function that calls itself, and a generic call's
type arguments.

**2026-11-03: std as a library of its own, its first module**
([modules.md](modules.md), "std as a library"; open questions 46 and
48, the latter decided by the user):

- **std builds as its own program:** `temper_frontend::stdlib::program`
  settles std's modules that translate (`stdlib::MODULES`: `testing`
  so far) and translates them as the library `std` (`to_tmpl` treats
  std's modules as its own when the library is `std`); a golden
  (`tests/to_tmpl/std.tmpl`) checks it, and Python's goldens check its
  `temper-std` package (`std.temper_std.testing.py`).
- **In `to_tmpl`:** std's `@connected` class members translate, marked
  connected and keeping their Temper bodies; a function's `Never<Void>`
  result is its fallback, `Void`; a block whose one part is an
  assignment is a value (`orelse do { x = true }`); and a `let` whose
  `if` or `when` initializer jumps in a branch (`char'\n' -> continue`)
  is the local declared, then the `if` as a statement that assigns it in
  each branch that doesn't jump.
- **Backends are pluggable (48):** each backend's `connected::STD` lists
  the std keys its Kotlin support network connects (Lua's connects
  every std key, as Kotlin's does), and `qname::by_connection` asks the
  backend (`connects_std`) by the member's key (`qname::std_key`); what
  it doesn't connect uses the body. No backend list or count upstream.
- **Naming (46):** std's names come from its own naming, as any
  library's: the testkit, `tempest build`, and the wasm build give a
  program that uses std std's interface; the fallback that derived std's
  names again is gone, so a std declaration std's program doesn't name
  (regex's, json's, temporal's, until they translate) is a "not yet".
- **Not yet:** backends running tests (each language's runner, and
  `expectedFailures`), std's output in `tempest build`, and `regex`,
  `json`, and `temporal`.

**Coverage, 2026-11-03, after std's `testing`:** unchanged: 58 of 72
through tmpl, 53 through every backend. The five that wait on std need,
besides std's interface: `testing/asserts`, each backend running tests
and comparing failures with `expectedFailures`; `regex/match` and
`regex/zero-advance`, `AnyValue` in tmpl for std's `regex`; and
`types/date` and `types/json-syntax-tree`, the builtin `doPure` for
std's `json`, then `Date` as a connected type per backend.

**2026-11-03: backends running tests** ([kotlin-differences.md](../kotlin-differences.md),
"Running tests with std's `testing`, in the backends"). `testing/asserts`
runs as tests on four backends, against std translated as a library:

- **Testkit:** `TESTS` lists the functional tests that run as tests
  (`runAsTest = true`); a `Case`'s `tests` holds their
  `expectedFailures` (`temper_funtests::expected_failures`, from the
  metadata), and `check_tests` compares a run's failures with them as
  Kotlin's `assertTestingTestFromJunit` does: the failing tests' names,
  lowercased, through a backend's undoing of its test naming
  (`test_name_words`), and each expected cause in some failure's.
  `junit_failures` reads JUnit XML as Kotlin's `parseJunitResults`.
- **Naming:** two rules: `members_prefixed` (Python), where fields'
  storage and private members take a `_` that public members don't, so
  std's private `_messages` (styled `messages`) no longer clashes with
  `messages()` and is `_messages`, not `_messages2`; and
  `fields_own_names` (Rust), where a field shares a name only with the
  property of its own source name, as its private fields get accessor
  functions (`_passing` is `passing2` beside the getter `passing`, and
  `_messages` `messages2` beside `messages()`).
  tmpl's `qname::connected_private`: a private std member a backend
  connects (`Test.bail()`) isn't declared.
- **Python:** `Test.bail()` is `assert False,
  str(test.messages_combined())`, as Kotlin's `BailCall`; tests run by
  `python -m temper_core.testing result.xml` with `temper-std` beside
  them, under mypy and pyright.
- **Rust:** `Test.bail()` returns `Err(Error::with_optional_message(…))`;
  std builds as the crate `temper-std` without a root module; tests run
  by `cargo test`, read as Kotlin's `cargoTestToJunitXml`; the
  workspace's stand-in std is gone. The printer collapses `else { if … }`
  and drops a unit tail after a block's statements.
- **JS:** `Test.bail()` is `strict.fail(…)` from Node's `assert`; tests
  run with Mocha's `xunit` reporter (`MOCHA`); `@temperlang/std` is in
  `node_modules`, checked by `tsc`; a tested module's `.internal.js`
  exports the classes its tests construct.
- **Lua:** std's members are the runtime's functions named from their
  keys, as Kotlin's (`temper.test_assert`); tests run with LuaUnit
  (vendored from Kotlin's resources), `-o junit`.
- **Java:** `Test.bail()` is `Core.throwAssertionError(…)` and its
  calls go through the connection table; running tests through JUnit's
  launcher (5.9.2's jars in Maven's cache) is drafted but waits on std's
  translation ("a `Param` expression").
- **Not yet:** Java's std, and C#'s and C++'s test runs (C#'s
  `AssertFailedException` bail and an MSTest runner over csc's output;
  C++'s std in place of its `testing` stand-in).

**Coverage, 2026-11-03, after backends running tests:** 58 of 72 through
tmpl; still 53 through every backend: `testing/asserts` passes on
Python, Rust, JS, and Lua, and waits on Java, C#, and C++.

**2026-11-03: tests on all seven backends**
([kotlin-differences.md](../kotlin-differences.md), "Running tests with
std's `testing`, in the backends"). `testing/asserts` runs as tests on
Java, C#, and C++ too:

- **Java:** std translates (a function literal's parameters and body
  aren't operands to spill first); std is Kotlin's `temper.std` package
  and `dev.temperlang:temper-std`, with JUnit as a compile dependency
  (`coordinates`); tests run through JUnit 5.9.2's launcher.
- **C#:** `Test.bail()` throws MSTest's `AssertFailedException`, as
  Kotlin's (`Expr::Throw`); std is `TemperLang.Std` with MSTest's
  packages; a conditional whose branch needs statements is an `if`
  (`sink`); a class a sibling namespace hides is written with
  `global::`; std, then the library with its tests, compile strictly
  against MSTest.TestFramework 3.1.1 from NuGet's cache (not vendored),
  and a reflection runner runs the `[TestMethod]`s.
- **C++:** `Test.bail()` is the runtime's `testBail(test)`; std's real
  `testing` replaces the test's stand-in, and every library's tests
  compare their JUnit XML with `expectedFailures`.
- **Shared:** `kit::runner_failures` reads the Java and C# runners'
  output; a private connected std member isn't declared in C# and C++
  either.
- **Naming** ([names.md](names.md), "Public names before private
  ones"): a class's public members are named before its private ones;
  `fields_apart` (Rust) replaces `fields_own_names`, so Rust's fields
  keep Kotlin's names (`passing`, `messages`), and only the private
  accessor functions tempest adds take a number (`passing2()`).

**Coverage, 2026-11-03, after tests on all seven backends:** 58 of 72
through tmpl; 54 through every backend. The four left that wait on std:
`regex/match` and `regex/zero-advance` (`AnyValue` in tmpl for std's
`regex`), and `types/date` and `types/json-syntax-tree` (the builtin
`doPure` for std's `json`, then `Date` as a connected type per backend).

**2026-11-03: std's regex** ([modules.md](modules.md), "std as a
library"; [kotlin-differences.md](../kotlin-differences.md), "std's
`regex`, in Python and JS"). std's `regex` translates as part of std,
and `regex/match` and `regex/zero-advance` run on Python and JS:

- **In `to_tmpl`:** `AnyValue` is tmpl's scalar; a builtin value used
  as one (`casts/as-expr`'s upcasts) is a "not yet", as targets box it.
  `doPure { … }` is its block's body in place, as Kotlin's `DoPureFn`
  (local classes in it declared at the module's level, and a block left
  with no statements is its value); `raw"…"` without interpolations is
  its text as written; a `Void` block ending in a statement is its
  statements; a header's constructor's default reads earlier
  parameters, not their fields not yet set (`RegexRefs`'s `match = {
  full: group }`); a bodiless connected static member is a
  `BodilessFunction`.
- **tmpl:** rule C1 allows a class's connected method without a body,
  and a connected `BodilessFunction` as a static member;
  `qname::connected_member` (was `connected_private`) and
  `connected_member_node`: a std member a backend connects isn't
  declared, public or private, as Kotlin's.
- **Python:** std's regex keys connect to `temper_core.regex`'s
  functions, the receiver first; an import of Python's or the runtime's
  takes an alias where the module binds the name (std's regex class
  `Sequence`), the module's names from `analysis::imports`.
- **JS:** the regex keys are `@temperlang/core` helpers, reserved; four
  cast to the types std's calls need, for `tsc`.
- **Lua, translation only:** std's members connected (static ones too)
  aren't declared, std's classes other than `testing`'s are imported and
  typed by their LuaCATS class, and a function literal's hoisting stays
  in it. std's `RegexFormatter.pushCode` returns out of an `orelse`,
  which Lua's `pcall` body can't yet (Kotlin returns a flow signal).
- **Testkit:** `WITH_STD` (`regex/match`, `regex/zero-advance`), and
  `Case::needs_std` puts std's output beside them; Rust, Java, C#, C++,
  and Lua translate std's `testing` alone (`std_testing_program`,
  `stdlib::program_of`) until their regex connections are in place.
- **Not yet:** regex on Rust (Kotlin's `std/regex/support.rs` and the
  `regex` crate), Java (`temper-std`'s `regex/Core.java`), C#
  (`std/Regex/RegexSupport.cs`, `IntRangeSet.cs`), C++ (`regex.hpp` is
  in the vendored core), and Lua (jumps out of an `orelse`); none of
  the first three's support files are vendored yet.

**Coverage, 2026-11-03, after std's regex:** 58 of 72 through tmpl
(`casts/as-expr` now waits on a builtin value as `AnyValue`, not the
type); still 54 through every backend, with `regex/match` and
`regex/zero-advance` passing on Python and JS.

**2026-11-03: std's regex on four more backends**
([kotlin-differences.md](../kotlin-differences.md), "std's `regex`, in
Rust, Java, C#, and C++"). `regex/match` and `regex/zero-advance` pass
on Rust, Java, C#, and C++, besides Python and JS:

- **Vendored:** Kotlin's regex support for Rust (`std/regex/support.rs`),
  Java (`temper/std/regex/Core.java`), and C# (`Regex/RegexSupport.cs`,
  `IntRangeSet.cs`), at the commit the rest is from;
  `compat/sync_temper.py --commit` copies a commit's files, not the
  checkout's.
- **Each backend's own table:** std's regex keys connect as Kotlin's do
  (Rust's `Connection::Support`, functions of the module's support
  code; Java's `StdStatic`, `temper.std.regex.Core`'s; C#'s
  `RegexSupport`'s; C++'s runtime `Regex::`), and each backend writes
  its support files into std's output (`STD_SUPPORT`). A std static
  connected (`regexCompileFormatted`) goes through the table too, and a
  connected member of any kind isn't declared
  (`qname::connected_member_node`, now in every backend's class loop).
- **Rust:** `regex = "=1.12.2"`, optional, in std's crate, and a crate
  importing `std/regex` asks for its feature; `AnyValue` is
  `temper_core::AnyValue`; a statement `orelse` whose body jumps runs in
  place in labeled blocks (`or_else_in_place`, `tree::retarget_tries`);
  a constructor may run `this`-free statements among its fields'
  assignments; std's crate allows `dead_code` and `unused_variables`.
- **C++:** `AnyValue` is `std::shared_ptr<core::AnyValueBase>`; the ASan
  build doesn't fail on GCC 13's false `maybe-uninitialized` in
  libstdc++'s `<regex>`.
- **Naming:** `NamingRules::types_apart` (Java, C#): a module's types
  and its functions and `let`s don't clash, so std's local class
  `Digit` keeps its name beside the `let` `Digit`, as Kotlin's.
- **C#:** a class hidden by a namespace this code is in is named from
  `global::`; std compiles strictly, but for the analyzers' rules on
  Kotlin's support files; CA1716 is allowed (`Or`).
- **Tests:** Rust, Java, C#, and C++ build all of std's modules
  (`std_testing_program` is gone); Rust's and C#'s runs name the
  `match` library by its key (cargo refuses a keyword; one C# assembly
  would let its namespace hide `Match` from every library); Rust's
  layout check skips the regex tests' root modules (`LAYOUT_GAPS`:
  rustfmt stops overflowing nested calls' last arguments sooner).
- **Not yet:** Lua (jumps out of an `orelse` body, Kotlin's flow
  signal; then its regex runtime, which lacks `regexformatter_*`
  functions and is slow on huge code ranges); the Rust printer's
  layout of deeply nested calls.

**Coverage, 2026-11-03, after std's regex on four more backends:** 58
of 72 through tmpl; still 54 through every backend, `regex/match` and
`regex/zero-advance` passing on all but Lua.

**2026-11-03: std's regex on Lua** ([kotlin-differences.md](../kotlin-differences.md),
"std's `regex`, in Lua"). `regex/match` and `regex/zero-advance` run on
Lua, so on all seven backends:

- **Jumps out of an `orelse` body,** as Kotlin's `TryStatement`: the
  body is still a function `temper.pcall` runs, and a `return`,
  `break`, or `continue` out of it returns a signal (`return v,
  "return"`, `return "break", "flow"`, or the label it goes to); `local
  ok, value, kind = temper.pcall(…)` and an `if` after it run the
  fallback or redo the jump (`TryBody`, `Translator::jump`,
  `return_stmt`, `or_else`). A jump out of an `orelse` nested in
  another passes its signal on. std's `RegexFormatter.pushCode`
  `return`s out of one; `HOISTING` covers all three jumps, an outer
  loop's, and nesting.
- **Tests:** Lua runs `kit::WITH_STD`, each in a directory of its own,
  with std's output in one on its `LUA_PATH`, under `timeout 300`.
  LuaLS reads std's output as a library, not checking it: as Kotlin's,
  it calls runtime functions the runtime lacks (`RegexFormatter`'s
  `pushCodeTo`, `pushCaptureName`, `adjustCodeSet`, only where Lua's
  `regexFormat` never goes) or with arguments they don't take.
- **What passes:** `regex/match` passes on Lua 5.4.6, its log as
  Kotlin's. `regex/zero-advance` is Kotlin's expected failure on Lua
  (its issue 166): the runtime's engine replaces nothing for
  `(^|,)\s*`, and the log expected is what it gives, as C++'s
  `regex/match` (198). Neither runs on LuaJIT, whose missing `utf8` the
  engine needs (`NEEDS_LUA53`). The engine's other known problems
  (`[\d]` and `[\w]` erring, `[^\w]`, a negated range up to U+10FFFF
  hanging) don't come up in these tests.

**Coverage, 2026-11-03, after std's regex on Lua:** 58 of 72 through
tmpl; 56 through every backend (`regex/match` with C++'s expected
failure, `regex/zero-advance` with Lua's). The two left that wait on
std: `types/date` and `types/json-syntax-tree` (std's `json`, then
`Date` as a connected type per backend).

**2026-11-04: std's json** ([modules.md](modules.md), "std as a
library"; [kotlin-differences.md](../kotlin-differences.md), "std's
`json`"). std's `json` translates as part of std (`stdlib::MODULES`),
and `types/json-syntax-tree` runs on Python, JS, and Rust:

- **In `to_tmpl`:** a function's last value that never completes (an
  `orelse` whose parts all return, jump, or fail, as `parseJsonObject`'s
  `do { … } orelse panic()`, or `bubble()` and `panic()` themselves) is a
  statement, with nothing returned after it; a `let` whose initializer
  `return`s in a branch is declared, then assigned branch by branch, as
  for `break` and `continue`; `x as C` for an `x` already a `C` is `x`;
  and a core method's receiver is fit to its owner's type when that
  isn't generic (`StringIndex`'s `compareTo`, `StringIndexOption`'s).
- **Validator (F1):** nothing after `bubble()` or `panic()` runs.
  `run-temper --tmpl` gives each problem's source position.
- **Python:** a cast to a core type is to its Python class, without type
  arguments (`cast_by_type(x, dict)` for a `MapBuilder`), and a local
  initialized by a cast is annotated, as `cast_by_type` gives its
  argument's type to pyright.
- **JS:** a list helper's receiver is kept in a temporary where its
  other arguments need statements first (`ls[i++]`); `stringToInt64` is
  cast to give a `bigint` (`MISTYPED_HELPERS`, which now says which go
  through `unknown`).
- **Rust:** a value `orelse` whose fallback bubbles or jumps is `if let
  Ok(v) = attempt { v } else { fallback }`; a body that ends in a
  `return` on every path needs no `break 'ok`, and one that can't fail
  (its failure caught by an `orelse` of its own) is just its statements;
  a constructor may run statements that read fields already set
  (`this.stack.add(…)`, through the field's local); `bubble();` as a
  statement is a `return`, so it may end a function as its value; a
  string field passed to a runtime function is borrowed (`&self.0.x`);
  a literal in pieces compares as `&str`; a narrowed `if let` local is
  already a reference. std's crate allows three clippy style lints its
  json source trips.
- **Not yet:** Java (an override of a generic method returning `T`,
  bound to a primitive, must return the box: `Boolean decodeFromJson`),
  and C#, C++, and Lua, untried; they build std's `testing` and `regex`
  alone (`kit::STD_REGEX`, `kit::WITH_STD_REGEX`). `Date` as a
  connected type per backend (std's `temporal`, `types/date`).

**Coverage, 2026-11-04, after std's json:** 58 of 72 through tmpl; still
56 through every backend, `types/json-syntax-tree` passing on Python,
JS, and Rust.

**2026-11-04: std's json on four more backends**
([kotlin-differences.md](../kotlin-differences.md), "std's `json`, in
Java, C#, C++, and Lua"). Every backend builds all of std, and
`types/json-syntax-tree` passes on all seven; the testkit's
`STD_REGEX`, `WITH_STD_REGEX`, and `std_program_of` are gone:

- **Java:** an override that binds a supertype's type parameter to a
  primitive boxes it, as Kotlin's `BoxedTypeAdjustments`
  (`translate/classes.rs`, `box_adjustments`): a result is the box
  (`Boolean decodeFromJson(…)`), which Java's return boxes; a parameter
  is the box in the public method, which unboxes into a private helper
  holding the body (`encodeToJson(Boolean x, …)` calls
  `encodeToJsonUnboxed(x.booleanValue(), …)`).
- **C#:** `T?` is the runtime's `Optional<T>`, as Kotlin's: its null is
  `Optional<T>.None`, its checks `HasValue`, its value `.Value`; a value
  passed for a declared `T?` is wrapped (`Optional.Of<int>(x)`, or
  `Optional<A>.None` for null) and a declared `T?` result unwrapped
  (`Optional.ToNullable<int>(r)`, `Optional.OrNull<A>(r)`), as Kotlin's
  `WrapAsOptional` and `UnwrapOptional`, on calls of functions and
  methods. The analyzers' CA1707 (`Int32_Value`, Kotlin's Pascal case)
  and CA1725 (an override's own parameter names) are allowed.
- **C++:** `T?` is `NullableParam<T>`, as Kotlin's; `x as C` for a core
  type kept in a `shared_ptr` (`multis as MapBuilder<…>`) is a
  `checked_cast` to its pointee; a scalar local nothing reads is
  `static_cast<void>(x);`, for `-Wunused-variable`.
- **Lua:** `as` to core's lists and maps calls the runtime's checks
  (`cast_to_listbuilder`, `cast_to_mapbuilder`, …), as Kotlin's; a cast
  of a cast is the outer one, as LuaLS reads only the first `@as`.
  `types/json-syntax-tree` runs on Lua 5.4, not LuaJIT, whose `Int64`s
  are the runtime's tables (`NEEDS_INTEGERS`).
- **Not yet:** a `T?` field or getter read in C# (only calls wrap and
  unwrap); Java's boxing for a supertype from another library (it
  looks for overridden members in the program's own types). `Date` as a
  connected type per backend (std's `temporal`, `types/date`).

**Coverage, 2026-11-04, after std's json on four more backends:** 58 of
72 through tmpl; 57 through every backend. The one left that waits on
std: `types/date`.

**2026-11-04: std's temporal** ([kotlin-differences.md](../kotlin-differences.md),
"std's `temporal`"). std's `temporal` translates as part of std
(`stdlib::MODULES`), and `types/date` passes on all seven backends:

- **In `to_tmpl`:** std's `@connected` class is connected (in std's own
  program and in a user's `uses` section), and so is a connected
  constructor; an explicit constructor that `throws Bubble` bubbles
  (`new Date(…)^`).
- **tmpl:** `qname::connected_type`: a std class a backend connects to a
  type of its own, by its key (`std/temporal.type Date`); the backend
  doesn't declare it, its type is the backend's, and its connected
  members go through the table. Naming names a class's bodiless static
  (`Date.today()`), which dependents call.
- **Python, JS, Java, C#:** `Date` is `datetime.date`, `globalThis.Date`,
  `LocalDate`, and `DateTime`; fields, the getter, `toString()`,
  statics, and the constructor connect as Kotlin's. C# vendors
  Kotlin's `TemporalSupport.cs` (`sync_temper.py`). Python's own
  modules of std import as `temper_std.…` (was `std.…`).
- **Rust:** `Date` is std's class; a constructor that sets its fields in
  branches declares a local per field and makes the instance after
  (`constructor_in_branches`); `today()` is tempest's own support code
  (`std-support/temporal/support.rs`, std only), behind an empty
  `temporal` feature, re-exported publicly as Kotlin's
  (`StdSupport::public`).
- **C++ and Lua:** `Date` is std's class with its connected members the
  runtime's (`core::Date::makeDate<Date>`, `temper.date_year(d)`); a
  private function only connected members call isn't written (C++).
- **Not yet:** `@json`'s adapters (`Date.jsonAdapter()`), and on the
  connected backends `Date`'s other members, `encodeToJson` and
  `decodeFromJson`, which Kotlin keeps as private module functions.

**Coverage, 2026-11-04, after std's temporal:** 58 of 72 through tmpl;
58 through every backend, every test tmpl translates. The 14 left don't
translate yet (see "Coverage, 2026-11-03, after other libraries'
members").

**2026-11-04: locals typed from their initializers, and generic
methods' types** ([milestone-4.md](../milestone-4.md)):

- **Typing:** a local declared without a type or a value (`var nc;`)
  takes the join of its initializers' types, the assignments to it
  that no other assignment to it comes before along any path, as
  Kotlin's `TyperPlan` (`typing/initializers.rs`); one that reads the
  local (`c = c + 1`) reads it unknown, as Kotlin's reads it `Invalid`.
  `semantics/constness` translates and passes on all seven backends.
- **Rust:** a local declared without a value and assigned right after,
  by a value that doesn't read it, is declared with that value (`let
  c: i32 = 1;`), as clippy's `needless_late_init` wants.
- **Typing:** a generic method's own type keeps its type parameters, as
  a generic function's does, so tmpl can declare it (it was erased, and
  every generic method of a class was "an expression of unknown type").
- **JS:** a temporary declared before it's assigned (`let t$0; { …; t$0
  = v; }`) has the JSDoc type of the value it holds, as other locals
  declared without a value do, since TypeScript 7 types it implicitly
  `any` where a `panic()` comes before the assignment (TS7034).
- **Not yet:** `classes/angle-call` now first needs `Type` as a value
  (`whatever(): Type { …; Void }`): a type and a node in tmpl, and each
  backend's spelling, which Kotlin's are inconsistent about
  (kotlin-differences.md, "Types as values"); then C++'s generic methods
  (templates in the header) and every backend's run of the generic
  methods, which no functional test reached before.

**Coverage, 2026-11-04, after locals typed from their initializers:** 59
of 72 through tmpl and every backend. The 13 left: `async` 5 (and the
two `netresponse` tests, which need it and `std/net`), a library with
errors 2 (Kotlin's garbage nodes), and one each of a type as a value
(`classes/angle-call`), a builtin value as `AnyValue` (`casts/as-expr`),
a local function that calls itself (`functions/locals`), and a generic
call's type arguments (`types/list/operations`).

**2026-11-04: builtin values as `AnyValue`, and `List.of`**
([kotlin-differences.md](../kotlin-differences.md), "Builtin values as
`AnyValue`, and `List.of`, in the backends"):

- **tmpl:** a builtin value other than `Void` is a subtype of `AnyValue`
  (`is_subtype`), so `to_tmpl` upcasts it as any other (`(upcast(1)):
  AnyValue`); the "not yet" for it is gone. Core's `ListBuilder.splice`
  is in `core.tmpl`, connected on every backend. Text reads `omitted`
  with its parameter's type as the call binds it (`Listed<String>`, not
  `Listed<T>`), as `to_tmpl` makes it.
- **Typing:** `List.of(…)` and `List.of<T>(…)` are list literals, as
  Kotlin's frontend unifies them, `List<T>` given `T`; where nothing
  says what's wanted, a `[]` among lists (`[[], ["a"]]`) is a list of
  the others' element type.
- **In `to_tmpl`:** `List.of` is a `ListLit`; `ignore(x)` is `x` as a
  statement.
- **Backends:** Rust's upcast to `AnyValue` is `v.as_any_value()` (an
  option's, mapped), and a nullable value's `as` is `and_then(cast)`;
  C++'s boxes, `core::any_box(…)`, a string literal as a `std::string`;
  JS's takes anything as `unknown` without a cast; Python's `x as T` for
  `x: T?` is `cast_none(x)`, and a cast to an interface with abstract
  members ignores mypy's `type-abstract` on that line. Python's and
  C++'s runtime helpers take `None` and `nullptr` for an argument left
  out before one given (`splice(b, None, 0)`); Rust's a left-out
  `Listed` as `None::<temper_core::Listed<_>>`. Rust's printer breaks a
  `let`'s type too long for its line between its outermost type
  arguments, puts a value on the next line where the same line can't
  hold its first, and lets a lone `&*f(…)` argument overflow, as
  rustfmt; a module `let` nothing reads is named with a `_`, as a
  local is. Lua's `return panic()` is `error("panic")` alone, as LuaLS
  takes `return error(…)` as returning `nil` (the shared `nullables`
  program's `stops`).
- **Naming:** where locals are block-scoped without shadowing (Java,
  C#, JS), a function literal in the module's init avoids the names of
  the literals before it whose blocks nest with its own, since Java and
  C# declare a literal as a local there (`fn`, then `fn11`).
- **Testkit:** `ANY_VALUES` (`casts/as-expr`, `types/list/operations`);
  `TEMPEST_CASES=a,b` runs only the functional tests whose keys contain
  one of these (and those other runs read), to work on a few.
- **Not yet:** `functions/locals`, a local function that calls itself
  or one declared after it: Kotlin's Rust makes a closure group (a
  struct of the captures, the functions its methods), its C++ a
  `std::function` captured by reference, and Lua needs the local
  declared first; a chunk of its own.

**Coverage, 2026-11-04, after builtin values as `AnyValue`:** 61 of 72
through tmpl and every backend. The 11 left: `async` 5 (and the two
`netresponse` tests), a library with errors 2, a type as a value
(`classes/angle-call`), and a local function that calls itself
(`functions/locals`).

**Generators and `async`** (2026-11-05): `Await`, `Yield`, and
generator literals in tmpl, with each backend's coroutine strategy, and
the state machines' lowering in `temper_tmpl::coroutines` ([nodes.md](nodes.md),
"Coroutines"); typing gives `await p` the promise's type. The five
generator tests (testkit's `ASYNC`) translate, and pass on Python
(generators) and Rust (state machines). **Coverage:** 66 of 72 through
tmpl, 61 through every backend. Next: JS, Lua, and C# as generators;
C++ (`async_run`, C++14) and Java as state machines, with C++20's
`co_await` on a promise as opt-in interop; then `netresponse`, which
needs std's `net` and a test server per backend.

**Coverage, 2026-11-05, after `async` on every backend:** 66 of 72
through tmpl and every backend, from 61. JS (`function*` and
`adaptAwaiter`), Lua (coroutines, with promises and `async` added to
its runtime, which Kotlin's lacks), and C# (iterators, an `orelse`
around a `yield` split by `goto`s, as Kotlin's) write generators; Java
(`CompletableFuture`) and C++ (`async_run`, C++14) write state
machines, and C++20 code can `co_await` a promise (vendored patch 0005).
A closure made in a generator's loop now gets that pass's `let` on the
state-machine backends too, by a copy where it's made, and a step sets
its state to `-1` first, so one that fails is done (2026-11-05;
testkit's `functions` program, on every backend). The 6 left: `netresponse` 2, a library with errors 2,
`classes/angle-call`, and `functions/locals`.

**2026-11-05: local functions that use each other or themselves**
([kotlin-differences.md](../kotlin-differences.md), "Local functions
that use each other or themselves"):

- **tmpl:** a run of `let`s of function literals (not `var`s) may use
  each other's locals in their literals, before their `let`s, as a
  `letrec`; R3 puts those a block's groups use early in scope from its
  start, and text reads such a `let` ahead. `analysis::function_groups`
  gives each group (the run's `let`s that such uses connect, where one
  uses itself or a later one), its early members, and its captures
  besides its own functions. `cells::copy_captured_params` copies a
  captured `var` parameter that's assigned to a `var` local, for
  backends with cells (Rust, Java, C++).
- **In `to_tmpl`:** a block declares such functions' locals ahead.
- **Backends:** Python, JS, and C# need nothing (nested `def`s,
  `function`s, and local functions); Lua declares a function an earlier
  one calls first (`local putter`, then `function putter`); Java writes
  a group as a local class of methods, C++ as a local struct of its
  captures with `const` methods, and Rust as a `Clone` struct of its
  captures with an `impl` of methods, each with an instance and the
  functions the code after uses as values. Java synthesizes functional
  interfaces over non-scalar types, with imports.
- **Tests:** `functions/locals` (testkit's `FUNCTIONS`), on every
  backend; to_tmpl.rs's `failed_steps_are_done` (each step sets its
  state to `-1` first; no Temper program can show a failed step, as a
  generator can't bubble, F4, and `panic()` ends the program) and
  `reassigned_loop_captures_fail_clearly` (the lowering's error for a
  captured loop `let` assigned again, which still needs a per-pass cell
  to work).

**Coverage, 2026-11-05, after local function groups:** 67 of 72
through tmpl and every backend, from 66. The 5 left: `netresponse` 2
(std's `net`), a library with errors 2 (Kotlin's garbage nodes), and
`classes/angle-call` (types as values).

**2026-11-05: one turn at a time on every backend**
([kotlin-differences.md](../kotlin-differences.md), "One turn at a
time"): every backend runs one `async` step at a time, `complete` only
queues its waiter, and generated code calls only the runtime's entry
points (vendored patches 0007 to 0012, Python and Java after Temper PR
#532). Host code awaits generated promises and loops over generated
generators the language's way ([api-compat.md](../api-compat.md)).
Tested by testkit's `async-turns` program on every backend and Python's
host script. Next: host scripts for the other six backends, and
value-yielding generators in tmpl (`yield(x)` is "not yet").

**2026-11-05: host code on every backend.** Each backend's `tests/host/`
uses `async-turns` the language's way from hand-written code, run by its
functional tests: C# `await` and `foreach`, Java `join`, `thenApply`, and
for-each, JS `await` and `for…of`, Lua `for … in` and a block of its
own, Python asyncio's bare `await` (now type-checked, with promises
annotated `TemperFuture`), Rust `.await` on a std-only executor and
`for`, and C++20 `co_await` and range-for against generated code. Each
steps a generator after a failed step, and blocks in a turn where its
backend can block. C# gets a pluggable runner and that check, and settles
a promise when its turn ends, since its host `await` resumed inside the
turn (vendored patch 0014). The vendored Rust runtime's tests run in CI.
**Coverage:** 67 of 72 through tmpl and every backend, unchanged. Next:
the `netresponse` tests (std's `net`, and a test server per backend).

**2026-11-05: std's `net`**
([kotlin-differences.md](../kotlin-differences.md), "std `net`"):

- **Frontend:** std's bodiless `@connected` functions (`sendRequest`)
  translate as connected ones, which backends connect by key, and std's
  `@connected` interface (`NetResponse`) is a connected type; `net` is
  in std's library.
- **Backends:** Python, JS, Java, and C# connect `sendRequest` and
  `NetResponse` to their runtimes' (C#'s `NetSupport.cs` vendored);
  Rust declares `NetResponse` and connects `send_request` to tempest's
  own `support.rs`, sending on a thread of its own. No request blocks a
  turn: Java's runs on the pool (patch 0016), Rust's holds the runner
  (patch 0017), and Python's already runs on `temper-io`. C++ and Lua
  have no `net`, as Kotlin's: a request gives a broken promise (Lua's
  runtime, patch 0018; C++'s std).
- **Also:** C#'s `orelse` splits its `try`s across an `if` that awaits;
  Rust's printer follows rustfmt on a pair whose left operand breaks, a
  `let` condition that doesn't fit, a chain's last `?`, and an `if`
  value whose condition breaks, and a state machine's cell isn't
  borrowed twice nor narrowed with `if let`, and a step that can't
  fail isn't a closure.
- **Testkit:** `NET`; `test_server_port`, one local server per process
  answering as Kotlin's `TestWebServer`, on `TEST_SERVER_PORT` where
  it's free (goldens read another port as it); `without_net`, the logs
  a broken promise gives.

**Coverage, 2026-11-05, after std's `net`:** 69 of 72 through tmpl,
Python, JS, Rust, Java, and C#, from 67; on C++ and Lua, 67 and the
two `netresponse` tests as Kotlin's expected failures. The 3 left: a
library with errors 2 (Kotlin's garbage nodes), and
`classes/angle-call` (types as values).

**2026-11-05: std's `net` on C++ and Lua, the prototype of pluggable
I/O** ([concurrency.md](../concurrency.md), "What std's `net` on C++
and Lua taught"):

- **One client shape:** `send(request, respond)`, responding once
  with a response or nothing, set once at the edge
  (`temper::core::net::set_client`, `temper.set_net_client`), with a
  default behind it; the other five keep their own as they are.
- **C++** (patch 0019): std's bodiless `sendRequest` calls the
  runtime's `net::sendRequest<NetResponse>` (`connected::BODILESS`),
  whose default is libcurl's multi interface on a `temper-io` thread
  where the build defines `TEMPER_NET_CURL`; the generated CMake does
  where `find_package(CURL)` finds it. Responses post back to the
  Temper thread (`async_post`), and the drain waits while a request is
  out (`async_hold`).
- **Lua** (patch 0018): LuaSocket's `socket.http` over non-blocking
  sockets in coroutines of their own, which an I/O source steps with
  `socket.select` when no step is ready.
- **Testkit:** the test server takes any free port, as Kotlin's, and
  only the net tests' goldens read it as 28461.

**Coverage, 2026-11-05, after std's `net` on C++ and Lua:** 69 of 72
on Lua where a Lua loads LuaSocket (Lua 5.4 with LuaRocks; LuaJIT, and
CI's Lua without LuaRocks, keep the 2 expected failures); C++ unchanged,
67 and the 2 as expected failures, until libcurl's headers are in the
container; both `netresponse` tests pass on C++ and Lua from host code
with a client of its own.

**2026-11-05: a library with errors** ([nodes.md](nodes.md), "Level 1:
Errors stay values"; [kotlin-differences.md](../kotlin-differences.md),
"Code with errors"):

- **Frontend:** `to_tmpl` no longer refuses a library with errors: an
  error node, a value where an error is reported (one that doesn't fit
  its place), a call of an undeclared name or with such an argument
  after only names and values, and the end of a body typing reports
  as reachable without a value, are each tmpl's `Error`, with the
  message. Nothing after one in a block is translated. Typing now
  reports that end, as Kotlin does.
- **tmpl:** `Error` ends flow in the validator, and counts as failing
  for an `orelse`.
- **Backends:** each fails there as `bubble()` does; Python raises,
  Rust breaks to its `orelse` or ends `init` with `Err(…)`, Java's
  static initializer calls `Core.throwBubble()`, and C# drops it as a
  statement. The testkit checks a run expected to fail
  (`Case::fails`, `check_case`).
- **Testkit:** C++'s libraries of one name are named among every case,
  so `TEMPEST_CASES` no longer changes the `functions` golden (it did
  since 2026-10-31, before `netresponse`); the net host tests read
  their cases unfiltered.

**Coverage, 2026-11-05, after a library with errors:** 71 of 72 through
tmpl and every backend (on C++ the 2 `netresponse` tests as Kotlin's
expected failures until libcurl's headers are here, and on Lua where no
Lua loads LuaSocket). Left: `classes/angle-call`, which needs types as
values.

**Target versions** (to do, 2026-11-04): C#'s generated projects should
target `net48;net6.0`, as Kotlin's library template does, not `net6.0`
alone; checking the `net48` side builds and tests needs .NET Framework
reference assemblies, a package to approve first; and the modern target
should move to .NET 8 (LTS) together with Kotlin. Java keeps 17 as its
modern floor, and may add a Java 21 target later (pattern-matching
`switch`, record patterns), as Java 8 is one. Also still open: C#'s
`Optional<T>` wrapping for a `T?` field or getter read (only calls wrap
and unwrap).

## Level 3: after subset A

Subset A's details are in the topic files' Level 3 sections. What the
later subsets will need, briefly (Proposed):

- **Subset B** (classes and the rest of Python's functional tests):
  - Nodes: `Class`, `Interface`, `Method`, `Constructor`, `Getter`,
    `Setter`, `Field`, `Test`; `This`, `GetField`, `SetField`,
    `GetStatic`, `SetStatic`, `CallMethod` (virtual or not), `New`,
    `FunctionRef`, `CallValue`, and `FunctionLiteral` with captures;
    the conversions, `Is`, and `As`; `NullLit`, `IsNull`,
    `EqNullable`, and `Identical`; `Labeled`, `OrElse`, `Bubble`,
    `Panic`, `ForOf`, `When`, and `Omitted`, with parameter defaults.
  - Types: `Nominal`, `TypeParam`, `Nullable`, `Never`, and subtyping
    through explicit conversions.
  - Modules: imports, tests, and `uses` sections with link keys for
    external declarations.
  - Surface: constructor and property forms, and the `Eq` and `Show`
    protocols.
  - Python's passes: labeled jumps (flags or restructuring) and
    `Switch` as `if` chains; `ForOf`, `orelse` (as `try`), and
    `EqNullable` stay native or map directly.
  - Validator: conversions in T3, captures in D5, and `@imu` in D4.
  - Text: class bodies, `uses` sections, annotations
    (`@operator("==")`), and conversions as keyword forms, such as
    `upcast(e)`.
- **Subset C** (Rust): `Result` and the results pass (`Ok`, `Fail`,
  `Propagate`, `ResultOr`); cells for assigned captures;
  properties as methods; first-use init (`EnsureInit`) and synchronized
  module `var`s; liveness and last use for clones; Rust's
  `NamingRules` (snake_case, `r#`, rebinding allowed).
- **Subset D:** `Yield` and `Await`, generator and `async` marks, and
  the state-machine pass (`Dispatch`, `Switch`, `Goto`, `Label`), with
  Rust's native `async` ([question 20](open-questions.md)); then the other backends'
  profiles and `NamingRules`: C#'s ban on shadowing, Lua's `goto` for
  `continue`, Java's named function types, C++'s evaluation-order
  temporaries, and JS.
