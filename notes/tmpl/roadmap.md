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
| 5 | Slice 7 continues | Naming pass and Python backend for subset A (done, 2026-10-30: 7 of 7 run); then the Rust backend for subset A (done, 2026-10-30: 7 of 7 run); then the JS backend for subset A (done, 2026-10-30: 7 of 7 run) |
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
