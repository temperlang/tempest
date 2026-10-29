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
| 4 | Slice 7 continues, driven by which functional tests still have unknown types | `to_tmpl` for subset A's tests |
| 5 | Slice 7 continues | Naming pass and Python backend for subset A |
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
  file's `line_starts` and `wide_lines` wait for source maps.
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
