# tmpl: the backend IR

Part of the [rewrite plan](README.md). A fractal design: Level 1
(2026-10-29) is purpose, boundaries, the API's shape, core decisions,
targets, formats, versioning, and testing. Level 2 (2026-10-29, after
the user's review of Level 1) is structure: node families, types,
declarations, modules, profiles, the public surface, analyses, and the
formats' shape. Level 3 (each node's fields, each pass) comes next; its
headings are placeholders at the end. It refines design.md section 7
("tmpl, the backend IR") and section 8 ("Naming").

**Decided** marks the user's direction, with its date; everything else
is **Proposed**, for review.

## 0. Design roadmap

Each level unlocks code, so design and building interleave.

| Level | State | Unlocks |
|---|---|---|
| 1: purpose and decisions | Done, reviewed (2026-10-29) | The crate boundary and its rules; nothing to build alone |
| 2: structure | This draft (2026-10-29) | The `temper-tmpl` crate skeleton: schema macro, node families, type and declaration tables, builder, the text writer and reader with a round trip, and the validator's structural rules, all tested on hand-written tmpl |
| 3: details, in subsets | Next | Per subset: exact fields, validator rules, and passes, then the frontend's translation (`to_tmpl`) for the functional tests that subset covers, then backends |

Level 3 goes subset by subset, each just ahead of the code that needs
it (Proposed):

- **Subset A:** functions, locals, `if`, `while`, returns, calls, the
  builtin types' operators and members, `console.log`, and module init.
  Then `to_tmpl` for the functional tests that use only these, then the
  naming pass, the shared output tree, and the Python backend running
  them, as its Kotlin-compatible variant first (question 22).
- **Subset B:** classes, properties, equality, object literals,
  bubbling and `orelse`, `for … of`, closures, and imports. Python grows
  to most functional tests, with its API diff (milestone 5).
- **Subset C, Rust:** the results lowering, cells for assigned
  captures, properties as methods, first-use init, and synchronized
  module `var`s. Then the Rust backend (milestone 6).
- **Subset D:** generators and `async`, with the state-machine pass
  for Rust; then the other backends' profiles.

**Interleaving with milestone 4** (Proposed). Milestone 4's remaining
critical path ([milestone-4.md](milestone-4.md), "Order and
decisions") runs in alternating chunks with tmpl's, since `to_tmpl`
needs every expression of a test typed:

| Chunk | Milestone 4 | tmpl |
|---|---|---|
| 1 | `new Box()`, `[]`, `null`, `bubble()`, and `panic()` taking the type wanted; `Never<T>` | Crate skeleton, schema macro, type and declaration tables |
| 2 | Builtin interfaces' members inherited by users' classes; a literal's result binding a generic callee's type parameter | Text writer, reader, and round trip; structural validator |
| 3 | Slice 7 begins: count expressions of unknown type; resolve every `==` (section 11.1, "Equality") | Level 3 for subset A |
| 4 | Slice 7 continues, driven by which functional tests still have unknown types | `to_tmpl` for subset A's tests |
| 5 | Slice 7 continues | Naming pass and Python backend for subset A |
| then | Kotlin's diagnostics, where cheap | Subsets B, C, and D |

A test whose expressions aren't all typed is reported by `to_tmpl` and
skipped, so the count of translated functional tests tracks slice 7.
No reference cycles (slice 8) follows tmpl, as decided.

## 1. Purpose

tmpl is the one form between the frontend and every backend: a settled,
typed, fully explicit program that a backend can translate without
asking anything of the frontend. It is also:

- **The library cache's body** ([library-cache.md](library-cache.md)):
  the starting tmpl, before any backend's lowering, is the stable form a
  built library can ship.
- **A readable debugging and test format,** through its text form.
- **Where shared backend work lives:** lowering, naming, and the checks
  that keep each backend small (design.md section 9).
- **The source of each library's public surface,** carrying enough
  intent that every backend can present an idiomatic API (4.9).

## 2. Boundaries

**Decided:** a standalone crate, mostly or entirely: the data model, its
formats, a validator, and passes and helpers that adapt a program to
different kinds of languages. No frontend, and no translators for real
targets.

```text
temper-syntax ─► temper-frontend ──(to_tmpl)──► temper-tmpl ◄── temper-be-py, temper-be-rust, …
                                                    ▲                    ▲
                                       (later) tmpl interpreter    shared output tree + printer
```

- **In the crate (Proposed name `temper-tmpl`,** matching the others):
  the model, builder, validator, profiles, lowering passes, analyses,
  surface helpers, the naming pass and `NamingRules` (design.md section
  8), the text reader, writer and viewer, and later the binary codec.
  No dependencies, not even on `temper-syntax`.
- **In the frontend:** the translation from settled trees into tmpl, in
  `temper-frontend` (it depends on `temper-tmpl`, never the reverse).
- **In each backend crate** (one per language): the translator to the
  shared output tree, the table from builtin and connected keys to its
  runtime, and its named **variants** (Decided 2026-10-29: several
  translation options per language must be cheap, as variants inside
  the one crate, not separate crates). A variant is data: a profile,
  `NamingRules`, and surface choices, over the crate's shared
  translation code. For example, `temper-be-py` might offer a variant
  compatible with Kotlin Temper's current output, for existing users,
  and another refined toward idiomatic Python, chosen by name on the
  command line. Nothing in tmpl assumes one variant per language, and
  API compatibility ([api-compat.md](api-compat.md)) is checked per
  variant. Backends depend on `temper-tmpl` and the printing crate
  only.
- **Not in tmpl:** output syntax, precedence, layout, source map
  encoding, runtime libraries, or anything chosen per backend before
  lowering.

## 3. The crate's public API, in shape

Proposed; names are placeholders until Level 3.

```rust
/// One library's program: modules plus the tables they share.
pub struct Program {
    pub library: LibraryInfo,      // name, version, dependencies
    pub modules: Vec<Module>,      // in dependency order, then source order
    pub nodes: Nodes,              // arena: kinds, types, children, payloads
    pub types: TypeTable,          // interned: equal types, equal ids
    pub decls: DeclTable,          // every declared thing, local or external
    pub strings: Interner,
    pub spans: SpanTable,          // per node: file and byte range
    pub docs: DocTable,            // per declaration
}

pub struct Builder { … }           // the frontend's only way in; checks as it goes
pub struct Profile { … }           // section 11.5
impl Profile { pub fn full() -> Profile; } // everything high-level: the starting tmpl

pub fn validate(p: &Program, profile: &Profile) -> Vec<Problem>;
pub fn lower(p: Program, profile: &Profile) -> Program; // only the passes it needs

pub mod passes;   // each lowering, also callable alone (tests, tools)
pub mod analysis; // section 11.8
pub mod surface;  // section 11.6: builder, property, and equality forms
pub mod naming;   // NameRequest, NamingRules, assign() -> NameMap
pub mod text;     // write, read, view(ViewOptions)
pub mod binary;   // write, read (later)
```

A backend variant's whole use of the crate: `lower(program,
&VARIANT.profile)`, then `naming::assign(&lowered, &VARIANT.rules)`,
then walk the result, asking `surface` and `analysis` where it needs
to.

## 4. Core decisions

### 4.1 One model, high-level to low-level, chosen by profile

**Decided (the direction):** one data model whose constructs range from
high-level (`for … of`, generators, `orelse` and bubbling, closures,
classes with properties) to low-level (plain loops, explicit error
results, state machines, closure conversion). Each backend declares a
profile of what it handles natively; shared passes lower only what it
lacks; a validator checks a tree against a profile. State machines are
only a tmpl post-pass (milestone-3.md, flat frames).

- **Decided (2026-10-29): a profile is a set of independent features
  plus a few either-or choices.** Targets don't line up on one scale:
  Go has closures but no exceptions, Lua has coroutines but no
  `continue`, C99 has `goto` but no closures. The choices are things
  like bubbling as exceptions or results (section 11.5).
- **Passes run in one fixed order** (Proposed), each removing one
  feature and declaring what it may introduce (closure conversion may
  add classes; the state-machine pass needs labeled loops and a
  switch). `lower` picks the needed subset; the order never varies, so
  output is deterministic.
- **Lowered output is still tmpl,** checked by the validator against the
  profile after every pass in debug builds and tests.
- **Low-level nodes are defined when a target needs them.** Error codes,
  closure conversion and `goto`-like jumps serve only the stress list
  today; the model leaves room rather than carrying dead code.

### 4.2 Fully explicit

**Decided:** more explicit than the frontend, since tmpl must be enough
for every backend. Proposed meaning, item by item:

- **Types on every expression,** as interned `TypeId`s (a `u32` each).
  A backend never infers a type.
- **Resolved names:** every reference points at a `DeclId`, which
  carries exactly a `NameRequest` (design.md section 8; section 11.3).
  tmpl holds no output names; the naming pass returns them in a side
  table.
- **Conversions are nodes:** upcasts to an interface, a function
  becoming a `@fun` interface's value and back, a non-null value used as
  nullable, a checked not-null, and `Never<T>` used as a `T`. Kotlin's
  Rust translator works these out again from types; here they're in the
  tree.
- **Captures are listed** on each function literal, each marked if
  assigned after capture (Python's `nonlocal`, shared cells in Rust and
  Java, PHP's `use (&$x)`), and checked by the validator against a
  recomputation.
- **Evaluation order** is left to right, as Temper's, and a fact rather
  than a shape: tmpl keeps nested expressions, and a pass hoists into
  temporaries only where a profile says the target doesn't guarantee
  the order (C++ arguments), and only for impure operands.
- **Bubbling paths:** every call that may bubble is marked, and its
  handler is the nearest enclosing `orelse` or the function's exit, so
  every lowering of failure reads the same facts.
- **Operators are typed builtins, and there is no generic `==`**
  (Decided 2026-10-29). Each builtin type's operators are its own
  builtins with stable keys (`Boolean.eq`, `Int32.add`), never a
  generic operator a backend may get wrong (Kotlin's Boolean `==` bug in
  Lua, JS, and Python). A class's `==` is a call of its operator method,
  a null comparison is a null check, and identity is its own explicit
  operation (11.1, "Equality").
- **Source positions** on every node (file and byte range), and **doc
  comments** on every declaration, for source maps and generated docs.

### 4.3 Independent of the frontend, and one starting form

**Decided:** nothing from the frontend in the model. Proposed in detail:

- **Its own types, names, values and metadata,** all ids into its own
  tables. Literals are a small enum (Boolean, the numeric types, String,
  null, Void), never interpreter values.
- **One starting tmpl for all backends.** Kotlin consults the backend's
  `SupportNetwork` while translating from the frontend, so its tmpl is
  per backend and can't be cached, compared, or tested alone. Here,
  every per-backend choice is a pass after the starting form.
- **Builtins and connected members by key.** A call of a builtin or a
  connected method names a stable key (as `String.isEmpty`); each
  backend's table maps keys to its runtime. Connected types are
  declarations marked connected, so a backend skips them and all their
  members (design.md, "Declaration facts in one place").

### 4.4 Arena layout, like the tree

Proposed: nodes in arrays with `u32` ids, children as ranges, payloads
in side tables, interned strings and types, as design.md section 3's
tree. That keeps passes fast, makes the binary form a near-direct dump,
and keeps order deterministic. Passes rebuild into a fresh arena rather
than editing in place, which keeps each pass simple and the old program
intact for comparison in tests.

### 4.5 Types

Proposed; section 11.2 has the structure. Nominal types with arguments,
invariant type parameters (typing.md), nullable types, `Void`,
`Never<T>`, and function types that carry their `@fun` interface's name
(Decided, design.md "Function types and fun interfaces"). Bubbling is an
effect on a function type, not a type; Kotlin's `Result<T, Bubble>` is
one backend's lowering. Classes are final; only interfaces are
extended, and they may carry method bodies.

### 4.6 Object literals, builders, and no named arguments

Decided earlier (design.md, "Object literals, not named arguments"):
tmpl has a constructor call with its arguments already placed, never
symbols or named arguments. Whether a public constructor gets a builder
(Java, Rust) or named-argument form (JS, Python) is a `surface` query,
answered once, so every backend agrees.

### 4.7 Module initialization: order and threads

**Decided (robustness):** tempest's backends should be more robust than
Kotlin's where they can.

- **Decided (2026-10-29): module initialization runs in source order
  now,** fixing Kotlin's bug (types sorted before values, so C++ reads
  zero and Python raises `NameError`). How top-level declarations are
  sorted is left to the team; tmpl takes the frontend's order.
- **A profile picks the init strategy** (Proposed): eager on import,
  on first use behind a once-guard (api-compat.md proposal 2), or class
  initialization.
- **Module-level `var`s are marked,** with reads and writes as their own
  nodes, so a pass can make them thread-safe where a target has threads,
  fixing the race Kotlin's C++ has.

Section 11.4 has the init list's structure.

### 4.8 Errors stay values

Unresolved code arrives as a typed `Error` node with its message, as in
design.md section 6; every backend turns it into a run-time failure.
The validator treats it as valid in any profile.

### 4.9 The public surface is first-class

**Decided (2026-10-29):** tmpl carries enough intent about a library's
public surface that each backend can present an idiomatic API, not just
correct code.

- **Decided (2026-10-29): the model stores only what the source says**
  about the public surface; shared helpers derive the rest.
- **Body lowerings and surface choices are kept apart** (Proposed),
  since a backend may lower a body construct and still expose its
  idiomatic form (a Java state machine behind an `Iterator`, Rust
  results behind a public `Result`).
- **Public API compatibility** ([api-compat.md](api-compat.md)) is
  checked at this layer: the surface a backend derives from tmpl is
  what its API diff compares.

Section 11.6 has the structure.

### 4.10 Mutability and isolation, for later

**Decided (2026-10-29):** don't distort the design for actors and
isolates (BEAM, Haskell), but give general solutions a shape that would
allow them (testing-and-milestones.md, "Process isolation").

- **Decided (2026-10-29, revised the same day): immutability facts come
  from the source where it declares them, and an analysis fills in the
  rest.** Temper already has `@imu` (a type, or a type parameter, that
  must be deeply immutable) and `@partialImu` (a generic type that is
  deeply immutable when its type arguments are, as core's `List`). tmpl
  records these marks; the analysis derives the rest; a check confirms
  the marks hold, as Kotlin's `ImuChecker` does.
- **Where mutable state lives is explicit** (Proposed): module `var`s,
  `var` fields and setters, builder types, and assigned captures.
- **No global identity assumptions** in passes: state a pass adds at
  module level is marked like a user's module `var`.

Section 11.7 has the structure.

## 5. Formats

**Decided (the direction):** a binary and a human-friendly but complete
text form, each round-tripping losslessly; one schema defining both;
one canonical, complete text form; an optional viewer that elides
derivable detail; binary also serving the library cache.

- **Decided (2026-10-29): canonical text is Temper-like,** with a
  distinct header and an obviously explicit style, so it's never
  mistaken for Temper. It leaves out types that the three local rules
  give back (a literal's type, a name's type from its declaration, and
  a call's result from its callee's signature with explicit type
  arguments); the reader restores them and the validator checks them.
- **The schema is Rust code in the crate** (Proposed), a `macro_rules!`
  table that declares each node kind and its fields once, and generates
  the node types, a field walker, and the text and binary readers and
  writers. No proc macros or new crates (code-style.md).
- **Spans in a trailing section,** optional on reading: golden tests
  strip it, and text without it reads with unknown spans.
- **The viewer** elides ids, conversions, capture lists, and obvious
  types for people. It never needs to read back.
- **Binary** can come later without changing the model, written to a
  spec so another implementation (Kotlin Temper, as a bridge) could
  read it.

Section 11.9 has the structure and an example.

### Where the goals conflict, and the recommendations

The user's priorities: (1) complete and explicit for every backend, (2)
profiles and lowering, (3) text round-trip, (4) binary.

- **Explicit versus readable text (1 against 3):** derivation rules plus
  a trailing spans section keep canonical text complete yet close to
  source; the viewer covers reading.
- **Explicit versus idiomatic output (1 against design.md's naming
  rules):** order is a guarantee and purity a fact; temporaries come
  only from a profile's lowering.
- **One model across levels versus simple passes (2):** node families
  with default structural walks, so a pass touches only its kinds.
- **One starting form versus per-backend choices (1 and 2):** where a
  choice is purely a target's spelling (Java's boxed type parameters,
  Rust's `Option`), it belongs in that backend's translator, not a pass.
- **Binary later versus a stable cache (4):** versioning starts on day
  one, in the text header too (section 7).

## 6. Targets

**Decided:** primary targets are Kotlin Temper's backends (C++, C#,
Java, JavaScript, Lua, Python, Rust). Python is tempest's first backend
and Rust right after, so the design keeps both ends in view: Python
needs few lowerings but exact capture and scope facts, and Rust needs
results, state machines, explicit conversions, nullable as `Option`,
and ownership-friendly facts.

The stress list, kept in mind but not central:

| Target | What it stresses | How the design copes |
|---|---|---|
| C99 | No GC, generics, closures, classes, or exceptions | Error codes from the explicit bubbling paths; closure conversion; erasure or monomorphizing; structs and vtables. Reference counting relies on no cycles ([acyclic.md](acyclic.md)) and cleanup on every exit path. Possible, not cheap. |
| PHP | `use (&$x)` captures; arrays copied by value | Capture lists; lists as runtime objects. Copes. |
| Go | No exceptions or inheritance, limited generics | Results as `(T, error)`; interfaces plus delegation for interfaces' method bodies; generic methods as functions. Copes, with some passes. |
| Swift | ARC, value types, protocols, `try` at each call | Every bubbling call is marked; ARC relies on no cycles. Copes. |
| Dart, Ruby, Kotlin | Little | Copes. |
| Elixir, BEAM; Haskell | Immutability, processes, no loops; purity, laziness | Rebinding and recursion are generic passes; 4.10's facts say what may cross processes and what is plain data. Shared mutable objects still need emulating. Room is left. |
| Actors and isolates generally | What state is per isolate, what may be sent | All mutable state is explicit (4.10). |
| MATLAB | 1-based indexing, value semantics | Indexing stays in runtime operations; handle classes; cells. Partly. |
| WASM | Structured control only | Labeled blocks and loops fit `br`; otherwise the C99 path, or WasmGC. Copes via the low end. |

## 7. Versioning and compatibility

Proposed:

- **Two versions in every file's header:** the format's (how it's
  encoded) and the model's (what nodes and fields mean). Text and binary
  share the model version.
- **At first, exact match only:** a cache is valid only for the compiler
  that wrote it ([library-cache.md](library-cache.md)), so the model can
  change freely until tempest's backends settle.
- **Later, for published caches:** new node kinds and fields only with
  defaults; meaning changes bump the major version. Recorded in
  `temper.keep` alongside other backward-compatibility facts.
- **Only the starting tmpl is ever stored.** Lowered forms are internal
  to a build, so passes can change without a version bump.

## 8. Testing

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

## 9. Lessons from Kotlin Temper's tmpl

From Kotlin Temper's `tmpL.out-grammar`, `TmpLTranslator.kt`,
`SupportNetwork.kt`, its `EqMacro`, and the Python, Rust, and C++
backends:

- **Avoided here:** nodes holding frontend objects, so tmpl can't exist
  without the frontend; per-backend choices made during translation;
  and two of several things (old and new type nodes).
- **What backends had to recompute, now stored:** captures,
  conversions, `for … of` (a `forEach` with a lambda there, so `yield`
  in its body fails), and module state shared across threads.
- **Bugs it let through, each with a test here:** types sorted before
  values; Boolean `==`; C++'s race on lazily set module `var`s; Rust's
  indexing that panics where Temper bubbles; C++ templates declared
  after their users.
- **Kept:** `Error` nodes; bubble and coroutine strategies, as profile
  choices; a computed jump node for state machines; equality without a
  generic operator, as Kotlin's 2026-09 relational-operator rework
  began.

## 10. Open questions, with recommendations

Answered on 2026-10-29 and folded in above: profile shape, canonical
text syntax and type elision, `for … of`, surface facts stored, where
immutability facts come from (revised: `@imu` and `@partialImu` plus
analysis), and init order; also backend variants and pretty names
everywhere. Still open from Level 1:

1. **Schema mechanism:** `macro_rules!` or a `build.rs` generator?
   Recommend `macro_rules!` until it hurts.
2. **Captures in the tree or computed?** Recommend in the tree, checked
   by recomputation, so passes and text show them.
3. **Ownership facts** for Rust, Swift, and C (clones, last uses):
   stored or computed? Recommend computed (11.8) until a backend proves
   otherwise.
4. **Bubbling payloads:** reserve a value on failure now? Recommend
   reserving the field, unused until Temper decides.
5. **Builtin keys:** one namespace for builtins, operators, and
   connected members, listed in the crate so backends' tables are
   checked for completeness? Recommend yes.
6. **Statements inside expressions** at the high level? Recommend yes,
   with a statement-izing pass; Rust keeps them.
7. **Spans:** file id and byte range, with line and column computed
   only for source maps? Recommend yes.
8. **Generics for C-like targets:** erasure or monomorphizing? Defer;
   keep full generics in the model.
9. **The tmpl interpreter** for compile-time evaluation of cached
   dependencies? Recommend deciding with the cache, after milestone 6.
10. **Surface choices in profiles:** one struct with body and surface
    sections? Recommend yes, so a backend's whole stance is in one
    place.
11. **What may cross an isolate?** Defer until an isolation model
    exists.

New from Level 2:

12. **Identity's spelling in Temper.** Temper has none today: `===` and
    `!==` lex as operators, but nothing gives them a meaning, and the
    docs don't mention identity. Recommend a core function,
    `identical(a, b)` (Dart's name), allowed only on class and
    interface types, with `===` left unused. It's explicit, greppable,
    and one typo away from nothing, where `===` is one character from
    `==` and means something else in JS. A language change for the
    team; tmpl's `Identical` node doesn't depend on the spelling.
13. **`==` without an operator method:** on a generic `T`, `AnyValue`,
    a function value, or an interface without one. Recommend an error
    in all these, never a silent identity check; a bound such as `T
    extends Equatable` calls the bound's method.
14. **Host equality protocols:** should a class's `@operator("==")`
    method also implement Python's `__eq__`, Rust's `PartialEq`, and
    C++'s `operator==`? Recommend yes for those three; for Java and C#,
    keep a plain `eq` method until Temper has hashing, since `equals`
    without a matching `hashCode` breaks hash maps.
15. **Null-safe `==` between nullable values:** one wrapper node,
    expanded by a pass, or expanded by the frontend as Kotlin's
    `EqMacro` does? Recommend the wrapper (`EqNullable`): Python and
    Rust compare nullable values natively, and the expansion is long.
16. **Comparisons:** `<` on builtin types as per-type keys (`Int64.lt`)
    or as `<=>` then `< 0`, as Kotlin now desugars? Recommend per-type
    keys, since every primary target has them natively; classes' `<`
    stays `compareTo(…) < 0`, which a backend may resugar.
17. **Fixed-type nodes in text:** may text also leave out types that a
    node's kind fixes (`Boolean` for `&&`, null checks, and
    `Identical`)? Recommend yes, as a fourth local rule; it's as
    trivial as a literal's.
18. **References in text:** `name#id` everywhere, or `#id` after the
    definition? Recommend `name#id` everywhere, the name checked by the
    reader, so text reads and diffs well.
19. **Init order through calls:** an initializer that reads a later
    module value directly is an error; through a function it calls?
    Recommend a warning from the reaches-module-state analysis, since
    it can't be exact.
20. **Rust's `async`:** native `async fn` over Temper's `Promise`, or
    the state machine as for generators? Recommend native, as
    milestone-3.md's `Future` plan suggests; generators get the state
    machine.
21. **Where `@imu` is checked:** tempest's frontend doesn't check it
    yet. Recommend the frontend reports violations in Kotlin's words (a
    small milestone 4 item, near slice 8, which needs the same facts),
    and the tmpl validator rechecks them as an invariant.
22. **The first Python variant:** Kotlin-compatible or idiomatic?
    Recommend Kotlin-compatible first, so milestone 5's API diff
    against Kotlin is empty by design and existing users can switch;
    the idiomatic variant follows as profile, naming, and surface data,
    with its own API baseline.
23. **Natural alternatives before numbers:** in what order? Recommend
    the hint's own candidates, then a name shared with a local whose
    lifetime doesn't overlap (where the target's scoping allows), then
    the shortest suffix (`x2`), and a golden-test count of suffixed
    names across the functional tests, std, Chromarite, and Ordable,
    kept near zero.

## 11. Level 2: structure

Everything here is Proposed unless marked Decided. Names are
placeholders until Level 3 fixes each node's fields.

### 11.1 Node families

Every node has a kind, a type, a span, a range of children, and maybe
a payload (an id, a literal, a key). Kinds are grouped into families;
each family has a default structural walk, so a pass touches only the
kinds it changes. The schema says, for each kind, its family, its
children's roles, and the profile feature it needs, if any, so the
validator's "allowed in this profile" rule is generic.

**Declarations** (each defines one `DeclId`): `Module`, `Class`,
`Interface`, `Function` (top-level or local), `Method`, `Constructor`,
`Getter`, `Setter`, `Field`, `ModuleLet` (with `var` marked), `Param`,
`TypeParam`, and `Test`. A connected declaration has no body. Locals
and labels are declared by the statements below.

**Statements:**

- Binding: `Let` (local, maybe `var`, maybe without an initializer).
- Writes: `Assign` to a local, `SetField`, `SetStatic`, and
  `SetModuleVar` (4.7: module `var`s have their own read and write).
- Control: `Block`, `If`, `While`, `DoWhile`, `ForOf` (Decided
  2026-10-29: a real loop, so `yield` and `await` in its body can work
  later; backends may map it to native loops or `forEach`),
  `Labeled`, `Break` and `Continue` (naming a label's declaration),
  `Return`, and `ExprStatement`.
- `When`, as the frontend settles it: cases on values or on types.

**Expressions:**

- Literals: Boolean, `Int32`, `Int64`, `Float64`, String, `Null` (typed
  by its place), and `Void`.
- Reads: `Local`, `This`, `GetField`, `GetStatic`, `GetModuleVar`, and
  `FunctionRef` (a declared function or method as a value, maybe
  bound to a receiver).
- Calls: `CallFunction` and `CallMethod` (a `DeclId` callee; a method
  call says whether it's virtual), `CallValue` (a function value or a
  `@fun` interface's value), `New` (a class, its type arguments, and
  placed arguments; object literals arrive as this), and `Builtin` (a
  key and typed operands: every operator and every core member).
  Arguments for omitted optional parameters are an explicit `Omitted`,
  since defaults apply inside the callee.
- Equality and comparison, below.
- Conversions (4.2): `Upcast`, `ToFun` and `FromFun`, `AsNullable`,
  `NonNull` (checked), and `FromNever`; and run-time checks `Is` and
  `As` (which may bubble).
- Functions: `FunctionLiteral`, with its parameters, captures, and
  body, typed by its `@fun` interface or bare function type.
- Control expressions: `IfValue`, `BlockValue`, `And`, `Or`, `OrElse`
  (a body and a fallback), `Bubble`, and `Panic`.
- Generators and async: `Yield` and `Await`, in functions marked
  generator or `async`.
- `Error`, valid anywhere (4.8).

**Lowered families,** which never appear in starting tmpl, each
introduced by one pass:

- Exceptions: `Try` (a body and a handler) and `Throw`.
- Results: the `Result<T>` builtin type for lowered signatures; `Ok`,
  `Fail`, `Propagate` (Rust's `?`), and `ResultOr` (a single bubbling
  call's `orelse`); a general `orelse` becomes a labeled block whose
  checks break to the fallback.
- Statements from expressions: compiler-made `Let`s with role hints;
  no new kinds.
- Cells for assigned captures: `NewCell`, `GetCell`, and `SetCell`.
- Control: `Switch` on integers or strings (already where the profile
  has it), `Goto` and `Label` (Lua's `continue`, C#'s labeled break),
  and the state machine's `Dispatch` (a computed jump).
- Module init: `EnsureInit(module)` at entry points, and
  `SyncGetModuleVar` and `SyncSetModuleVar` for threads.
- Named function types: synthesized `@fun` interface declarations; no
  new kinds.

**Invariants** the validator checks:

- Every expression has a type, and every child's type fits its role
  (an argument's its parameter's, maybe through an explicit
  conversion; there are no implicit ones).
- Every reference names a declaration in scope or an external one.
  Every `Break` and `Continue` names an enclosing label. Every
  may-bubble call is marked and has a handler; only marked calls may
  bubble.
- Children are in evaluation order. A node has one parent: the tree is
  a tree, not a DAG.
- Starting tmpl has no lowered-family node; a lowered program has only
  kinds its profile allows.
- Stored facts (captures, may-bubble marks, the init list) match their
  recomputation.

#### Equality

**Decided (2026-10-29):** Temper's `==` is always type-specific, so tmpl
has no generic `==` node:

- **A class's or interface's `==`** is a `CallMethod` of its method
  marked `@operator("==")`, its own or the nearest inherited one,
  virtual as any method call. A class with `@auto("==")` and no such
  method gets a synthesized `eq` declaration, marked `@auto("==")` and
  compiler-made, whose body compares each property with its own type's
  equality, as Kotlin's `AddAutoMembers` does.
- **Each builtin type's equality is its own builtin:** `Boolean.eq`,
  `Int32.eq`, `Int64.eq`, `Float64.eq`, and `String.eq`.
- **A comparison with `null`** is `IsNull` (or `IsNotNull`), never a
  call.
- **Identity** is `Identical(a, b)`, only for class and interface
  types, and only where the source asks for it (question 12). Backends
  map it to `is` (Python), `===` (JS), `ReferenceEquals` (C#), `==` on
  references (Java), `Arc::ptr_eq` (Rust), pointer comparison (C++), and
  `rawequal` (Lua).
- **`!=`** is `Builtin(Boolean.not)` around the equality, which a
  translator may print as `!=`.
- **Nullable on both sides** (Proposed, question 15): `EqNullable(a, b)`
  names the equality for the non-null values (a builtin key or a
  method) and means `IsNull(a) ? IsNull(b) : !IsNull(b) && eq(a!, b!)`.
  A pass expands it where the profile says the target's own `==` isn't
  null-safe.
- **Comparisons** (Proposed, question 16): builtin types have their own
  keys (`String.lt`, `Int32.cmp`); a class's `a < b` is
  `Builtin(Int32.lt, CallMethod(compareTo), 0)` with the method marked
  `@operator("<=>")`.

The method keeps its own name (`eq`, `compareTo`); whether a backend
also presents it as its host's equality protocol is a surface question
(11.6).

**Gaps in tempest's typing today** (typing.md, "Operators defined in
Temper"; milestone-4.md, "Operators defined by methods"), for slice 7:

- **Not every `==` is resolved.** Typing checks a class's operator
  method only for a non-null first operand of a class or interface it
  knows. A generic `T`, `AnyValue`, or function operand, or a class
  with an unknown supertype, is neither checked nor resolved (as in
  Kotlin), and the interpreter then compares by identity
  (`objects_equal` in `interp.rs`): the generic identity `==` this
  design removes. Each must become a resolved call or an error
  (question 13) before `to_tmpl` can translate it.
- **The choice isn't recorded.** Typing computes which method or
  builtin variant an operator uses, but `to_tmpl` will need it as a
  fact on the node, as for other calls.
- **`@auto("==")` has no declaration.** Typing and the interpreter
  special-case it; tmpl needs a real `eq` declaration (above).
- **Auto equality compares fields wrongly.** The interpreter's
  `objects_equal` recurses structurally into fields that are objects,
  so a field whose class has its own `@operator("==")` is compared
  without calling it. A small interpreter bug to fix with the above.
- **Nullable operands:** typing skips `==` with a nullable side, as
  Kotlin does; `to_tmpl` must produce `IsNull` or `EqNullable` from
  the operands' types.
- **No identity operation** exists in Temper, the interpreter, or
  typing (question 12).

### 11.2 The type model

The `TypeTable` interns every type, so equal types have equal ids. Its
variants:

- `Nominal { decl, args }`: classes, interfaces (`@fun` ones too), and
  builtin types, which are external declarations from core with keys
  (`Int32`, `String`, `List`, `Map`, `Generator`, `Promise`, …).
- `TypeParam(decl)`, with its bounds on the declaration.
- `Nullable(T)`, never of a nullable `T`.
- `Function { params, optional, result, bubbles }`: bare function
  types, kept while tests use them; Decided to go in favor of named
  `@fun` interfaces.
- `Void`, `Never(T)` (bottom with its fallback type), and `AnyValue`
  (the top).
- `Result(T)`: only in programs lowered to results (11.1).

Relationships, all answered by `analysis` helpers rather than stored
twice: supertypes from each declaration's `extends` with arguments
substituted; subtyping (nominal, invariant arguments, `Nullable` and
`Never` rules); a member's type as seen from a receiver type; and a
`@fun` interface's signature.

Invariants: no inference variables or unknown types (an `Error` node
is typed `Never` of what its place wants); explicit type arguments on
every generic call and `New`; every type an export mentions is
exported or external.

Per-backend type spelling (Java's boxing, Rust's `Option` and `Arc`,
C#'s nullable value types) is the translator's, never a pass's.

### 11.3 Declarations, names, and linking

The `DeclTable` holds every declared thing, `DeclId` dense from 0. Each
entry:

- **Kind:** module, class, interface, function, method, constructor,
  getter, setter, field, module `let`, local, parameter, type parameter,
  label, or test.
- **Name:** the source name, or for compiler-made declarations a role
  hint (`result`, `index`, `done`, or "copy of" another declaration).
  With kind and visibility, this is the `NameRequest` (design.md section
  8).
- **Owner:** the enclosing declaration; owners form a tree rooted at
  modules.
- **Facts read once** (design.md, "Declaration facts in one place"):
  visibility, exported, static, abstract, `var`, connected key,
  builtin key, `@fun`, `@operator` marks, `@auto("==")`, optional (for
  parameters with defaults), generator, `async`, may bubble, `@imu`
  and `@partialImu` (on types and type parameters), and compiler-made.
- **Type,** span, and docs.

**External declarations** stand for what this library uses from
another, core included: same entry, no body, plus a link key. A link
key is the library's name, the module's path, and the qualified source
name within the module (`Point.eq`). Temper has no overloading, so that
path is unique; the declaration's kind and type are kept too, so a
reader checks them against the dependency's exports and reports a
mismatch rather than mis-linking. Within a library, modules share one
table and refer to each other's declarations by `DeclId`; the import
list says which (11.4).

**Pretty names everywhere, inside functions too** (Decided 2026-10-29;
design.md section 8: no numbers except to resolve a real conflict).
Users have complained even about suffixed locals, so tmpl makes this
achievable:

- **Every local, parameter, label, and temporary is a declaration with
  a name:** its source name, or for one a pass introduces, a hint
  saying what it holds. The validator rejects a compiler-made
  declaration without a hint, so no pass can fall back to `tmp`.
- **Hints come with candidates,** in preference order, from shared
  helpers in `naming::hints`, so passes name what they add the same
  way: the parameter a value feeds (`x` for `new Point(x = …)`), the
  property or local it copies (`items`), the call it holds without
  `get` or `to` (`parsed`, `int32`), its type (`point`), or a role
  (`result`, `index`, `done`, `state`).
- **Scopes and shadowing are explicit:** every declaration's owner is
  its enclosing block, function, class, or module, and every reference
  is a `DeclId`, so two source locals named `x` are told apart without
  renaming them in tmpl.
- **The naming pass renames only on a real conflict,** as the target
  sees it: `NamingRules` describe its scoping as data (Python's and
  JS `var`'s function scope, Python's class bodies not enclosing
  methods, C#'s ban on shadowing an outer local, Rust's free
  shadowing). Order: exported names first, never suffixed (a clash
  between exports is an error reported once, not renamed), then source
  names in source order, then compiler-made names. On a conflict it
  tries the hint's next candidate, then, in function-scoped targets,
  sharing the name with a local whose lifetime doesn't overlap and
  isn't captured (liveness, 11.8), and only then the shortest suffix
  (question 23).

The naming pass returns a `NameMap` from `DeclId` to string; tmpl
itself stores no output names.

### 11.4 Modules: imports, exports, and the init list

A `Module` has its path within the library, its imports, its
declarations in source order, its init list, and its tests.

- **Imports:** each names a module of this library or an external
  library's module, and the declarations used from it. Temper rejects
  import cycles, so modules form an acyclic graph, and `Program`
  lists them in dependency order. Unused imports are dropped by the
  frontend.
- **Exports** are the declarations flagged exported; the module's
  export list is derived, in source order.
- **The init list** (Decided 2026-10-29: source order) holds, in the
  order the frontend gives: each module `let`'s initializer as an
  `InitLet`, each top-level statement, and each class's static
  initializers where the class appears. Type and function declarations
  aren't in it. An initializer that reads a module value initialized
  later is an error; through a call, a warning (question 19).
- **Tests** are a separate list of `Test` declarations, run after
  init.

Init strategy is a profile choice (11.5). The first-use pass adds
`EnsureInit` where an exported function, method, or getter may reach
the module's state, by the reaches-module-state analysis; a module
with no state has no guard.

### 11.5 Profiles: features and choices

**Decided (2026-10-29):** a set of independent features plus a few
either-or choices. Proposed shape, plain data:

```rust
pub struct Profile {
    pub native: Features,          // bit set: each feature the target keeps
    pub failure: Failure,          // Exceptions | Results (| ErrorCodes, later)
    pub function_types: FnTypes,   // Plain | Named
    pub module_init: ModuleInit,   // Eager | FirstUse | ClassInit
    pub module_vars: ModuleVars,   // Plain | Synchronized
    pub surface: Surface,          // 11.6
}
```

`Profile::full()` keeps everything; it describes starting tmpl. A
backend crate's variant (section 2) is a profile, `NamingRules`, and the
surface section, so a Kotlin-compatible variant and an idiomatic one
of the same language differ only in data. The tables below are each
language's first variant.

**Features, and Python's and Rust's first profiles** (native means
kept; otherwise lowered):

| Feature | Python | Rust | Others |
|---|---|---|---|
| Statements inside expressions | no: temporaries | native | only Rust |
| `ForOf` | native | native | all primary targets; `forEach` or index loops are the translator's choice |
| Labeled `break` and `continue` | no: flags or restructuring | native | Java, JS native; C# and Lua (`continue`) by `goto` |
| `Switch` (from `When`) | no: `if` chains | native (`match`) | C#, C++, Java, JS native; Lua `if` chains |
| Generators | native | no: state machine | C#, JS, Lua (coroutines) native; C++, Java state machine |
| `async` | native | native (question 20) | C#, JS native; others state machine |
| Assigned captures | native (`nonlocal`) | no: cells | C#, JS, Lua native; C++, Java cells |
| Properties | native (`@property`) | no: methods | C#, JS native; C++, Java, Lua methods |
| `Void` as a value | no | native (`()`) | JS, Lua native |
| Left-to-right arguments | native | native | all but C++ |
| Null-safe `EqNullable` | native | native (`Option`) | JS for builtin types; others expanded |

| Choice | Python | Rust | Others |
|---|---|---|---|
| Failure | exceptions | results | exceptions everywhere else (Lua's `error` and `pcall`) |
| Function types | plain | plain | named in Java; plain elsewhere |
| Module init | eager | first use | C++ first use; Java, C# class init; JS, Lua eager |
| Module `var`s | plain | synchronized | C++, C#, Java synchronized; JS, Lua plain |

`Int32` wrapping isn't a pass: the key stays, and each translator
spells it, with helpers where the value-range analysis can't rule out
overflow (JS, Lua, Python).

**Pass order,** fixed: named function types; properties to methods;
generators and `async` to state machines (they add labels and
switches); `ForOf` (only for targets without loops over iterables);
failure; `EqNullable`; `Switch` to `if`; labeled jumps; statements out
of expressions; evaluation-order temporaries; cells; module `var`
synchronization; module init; `Void` values. Each declares what it
needs and what it may add, and the validator checks both.

### 11.6 The public surface: intent and per-backend choices

**Stored** (Decided 2026-10-29: only what the source says), on
declarations: exported and visibility; source names and kinds; docs;
fields versus getters and setters, and which are computed; constructor
parameters and which have defaults; `@fun` interfaces by name; may
bubble; generator and `async`; `@operator`, `@auto("==")`, and `@imu`
marks;
connected keys; abstract, sealed, and type parameters with bounds.

**Derived** by `surface` helpers, each answered once per declaration
and profile:

- **Constructor form:** plain, builder (Java, Rust), named arguments
  (Python, JS options), or option struct, from the parameter count and
  defaults (4.6).
- **Property form:** native property, getter and setter methods, or a
  field (api-compat.md's per-backend rules).
- **Failure in signatures:** exceptions or `Result`, even where a body
  was lowered differently.
- **Iteration:** a generator presented as the host's iterator
  (`Iterator`, `IEnumerable`, Python generators, Rust `Iterator`).
- **Equality:** whether the `@operator("==")` method also implements
  the host's protocol (question 14), and `!=` with it where needed.
- **Mutability:** whether a Rust class wrapper needs a lock
  (api-compat.md proposal 4), from 11.7's analysis.

Profiles carry the per-variant choices as data in their `surface`
section (a Kotlin-compatible Python variant keeps Kotlin's shapes; an
idiomatic one may choose differently, each with its own API baseline), and naming conventions come from `NamingRules`. Passes that
change a body never change surface facts; a pass that changes how a
public declaration works records which surface form it presents (a
state-machine generator still presents an iterator). The API diff
compares what these helpers derive.

### 11.7 Mutable state and isolation facts

**Stored,** because the source says them: `var` on module `let`s,
fields, and locals; setters; assigned captures; builder types
(`ListBuilder`, `MapBuilder`, …) flagged mutable by key; module state
that passes add, marked like users'; and `@imu` and `@partialImu` on
types and type parameters (Decided 2026-10-29), as core marks `List`,
`Null`, and `GeneratorResult`, and std marks `Date`.

**Checked:** each mark holds. An `@imu` type has no `var` property, and
every property's type is deeply immutable (a type parameter only if
it's `@imu` itself); a `@partialImu` type has type parameters, and its
properties are deeply immutable given deeply immutable arguments; a
subtype doesn't break a supertype's mark. Kotlin's `ImuChecker` has
these rules and their messages; where tempest reports them is question
21.

**Derived** (Decided 2026-10-29: by analysis, where the source says
nothing), and agreeing with the marks where it does:

- A class is **mutable** if it has a `var` field or setter, or extends
  an interface that does.
- A type is **deeply immutable** if it's final, not mutable, and every
  type it holds is deeply immutable; type parameters count by their
  arguments.
- A function **touches module state** if it may read or write a module
  `var` or call something that does (11.8).

First users: Rust's lock or no lock (soon), `Rc` versus `Arc` (later),
and isolates (much later). The frontend's acyclic check (slice 8) needs
similar facts; it should share the definitions written down here, not
the code, since the frontend runs first.

### 11.8 Analyses shared by passes

Each is a function from `&Program` to a side table keyed by node or
declaration id. Programs are immutable, so a result is valid for the
program it came from; stored facts are rechecked by recomputing them.

- **Captures:** which declarations each function literal captures,
  and which it or its owner assigns after capture.
- **Purity:** whether an expression may have side effects or bubble,
  for evaluation-order temporaries.
- **Bubbling:** may-bubble calls and their handlers.
- **Call graph and reaches-module-state:** for init guards and init
  order.
- **Definite assignment:** for `Let`s without initializers (Java, Rust,
  and C# reject reads they can't prove assigned).
- **Mutability** (11.7).
- **Value ranges:** where `Int32` arithmetic can't overflow, for JS,
  Lua, and Python helpers.
- **Liveness and last use:** for letting locals with lifetimes that
  don't overlap share a name, and later for Rust's clones (question
  3).
- **Naming hints** (`naming::hints`): candidate names for any
  expression a pass hoists or copies (11.3), so every pass and backend
  names what it adds well, and function bodies read as hand-written.
- **Type queries:** subtyping, substitution, member lookup, operator
  methods, override chains, and which interface method bodies a class
  inherits (for Go and Rust trait defaults).

### 11.9 Format structure

**Text** (Decided 2026-10-29: Temper-like, explicit, with a distinct
header). Its shape:

```text
file    := header uses* module* spans?
header  := "tmpl" FORMAT-VERSION "model" MODEL-VERSION "library" NAME VERSION
uses    := "uses" LIBRARY VERSION "{" external-decl* "}"
module  := "module" NAME#ID "{" import* decl* "init" "{" stmt* "}" test* "}"
```

Its style, so it's never mistaken for Temper:

- Every declaration is written `name#id`; compiler-made ones are
  `~role#id`. References are `name#id` too, the name checked by the
  reader (question 18).
- Declarations always carry their types; expressions carry them only
  where the local rules don't give them back, as `(e): T`.
- Control nodes use Temper's syntax (`if`, `while`, `for`, `&&`,
  `orelse`, `return`); every builtin is a call written `%Key(…)`, so
  `%Int32.add(i#4, 1)`, never `i + 1`.
- A call that may bubble is marked `^`.
- Bodies are explicit: constructors assign their fields, and a
  function's last value is a `return`.
- Spans follow in a trailing `spans { … }` section, keyed by each
  node's position in writing order.

**The viewer** writes the same shape with options: hide ids where a
name is unique, hide conversions and capture lists, show `==` and `+`
for their builtins and operator methods, and show spans as
`file:line:column`. A CLI command (`tempest tmpl FILE --profile py
--view`) prints starting or lowered tmpl.

**Binary:** a header (magic `TMPL`, format and model versions, the
library's identity and its dependencies' versions), then a section
table, so a reader can skip docs or spans, and sections for strings,
types, declarations, external links, nodes stored column-wise (kinds,
types, child ranges, payloads), docs, and spans delta-encoded. Integers
are LEB128 varints and every reference an index. Equal programs give
equal bytes, so caches can compare by hash.

#### Worked example

Temper source:

```temper
export class Point(public x: Int, public y: Int) {
  @operator("==")
  public eq(other: Point): Boolean { x == other.x && y == other.y }
}

export let firstIndex(points: List<Point>, text: String): Int {
  let wanted: Point = { x: text.toInt32() orelse 0, y: 0 };
  var i = 0;
  for (let p of points) {
    if (p == wanted) { return i; }
    i += 1;
  }
  -1
}
```

Starting tmpl (canonical text, spans left out):

```text
tmpl 1 model 1 library example 0.1.0
uses temper-core 0.1.0 {
  class List#90<T#91>;
  class String#92 { method toInt32#93(): Int32 throws Bubble = %String.toInt32; }
}

module example#1 {
  export class Point#2 {
    public field x#3: Int32;
    public field y#4: Int32;

    public constructor new#5(x#6: Int32, y#7: Int32) {
      this.x#3 = x#6;
      this.y#4 = y#7;
    }

    @operator("==") public method eq#8(other#9: Point#2): Boolean {
      return %Int32.eq(this.x#3, other#9.x#3) && %Int32.eq(this.y#4, other#9.y#4);
    }
  }

  export fn firstIndex#10(points#11: List#90<Point#2>, text#12: String#92): Int32 {
    let wanted#13: Point#2 = new Point#2((%String.toInt32(text#12)^ orelse 0): Int32, 0);
    var i#14: Int32 = 0;
    for (let p#15: Point#2 of points#11) {
      if (p#15.eq#8(wanted#13)) {
        return i#14;
      }
      i#14 = %Int32.add(i#14, 1);
    }
    return -1;
  }

  init {}
}
```

The object literal is a `New` with placed arguments; `p == wanted` is a
call of `eq#8`; the field reads inside `eq#8` use `Int32`'s own
equality; and `orelse`'s type is written, since no local rule gives it.

**Lowered for Python** (exceptions; no statements inside expressions;
everything else native). Only `firstIndex`'s first statement changes:

```text
    var ~x#16: Int32;
    try {
      ~x#16 = %String.toInt32(text#12)^;
    } catch {
      ~x#16 = 0;
    }
    let wanted#13: Point#2 = new Point#2(~x#16, 0);
```

The temporary's hint comes from the constructor parameter it feeds,
`x`, with `parsed` (from `toInt32`) as its next candidate. Python's
locals are then `points`, `text`, `x`, `wanted`, `i`, and `p`, with no
numbers. Had `firstIndex` already a local `x` alive here, the temporary
would be `parsed`, not `x2`. Evaluation order needs no other
temporary, since `0` is pure. If question 14 lands as recommended,
`eq#8` presents as `__eq__` too, and the translator may print `p ==
wanted`; `%Int32.add` prints as `i + 1` only where value ranges rule
out overflow.

**Lowered for Rust** (results; everything else in this example native):

```text
    let wanted#13: Point#2 = new Point#2(result_or(%String.toInt32(text#12), 0), 0);
```

`%String.toInt32` now has the lowered signature, returning
`Result<Int32>`, so the call needs no `^`; `ResultOr` replaces the
`orelse` over one call, and the translator may print `.unwrap_or(0)`.
`firstIndex` doesn't bubble, so its signature is unchanged. No
temporary is added, so Rust's locals are the source's: `points`,
`text`, `wanted`, `i`, and `p`. The module
has no state, so first-use init adds no guard. `Point`'s `x` and `y`
stay fields in bodies; the surface presents them as getters, and the
properties-to-methods pass rewrites only reads from outside `Point`.

## Level 3: details (placeholders)

### Each node's fields and invariants

### Each lowering pass: preconditions, output, and examples

### The naming pass and each backend's `NamingRules`

### The validator's rules
