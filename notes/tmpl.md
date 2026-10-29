# tmpl: the backend IR

Part of the [rewrite plan](README.md). A fractal design: Level 1 (this
draft, 2026-10-29) is purpose, boundaries, the API's shape, core
decisions, targets, formats, versioning, and testing. Level 2 (node
families, types, names, modules, profiles, format structure) and Level 3
(each node's fields, each pass) come after review; their headings are
placeholders at the end. It refines design.md section 7 ("tmpl, the
backend IR") and section 8 ("Naming").

**Decided** marks the user's direction (2026-10-29); everything else is
**Proposed**, for review.

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
  the model, builder, validator, profiles, lowering passes, the naming
  pass and `NamingRules` (design.md section 8), the text reader, writer
  and viewer, and later the binary codec. No dependencies, not even on
  `temper-syntax`.
- **In the frontend:** the translation from settled trees into tmpl, in
  `temper-frontend` (it depends on `temper-tmpl`, never the reverse).
- **In each backend crate:** its profile, `NamingRules`, the table from
  builtin and connected keys to its runtime, and the translator to the
  shared output tree. Backends depend on `temper-tmpl` and the printing
  crate only.
- **Not in tmpl:** output syntax, precedence, layout, source map
  encoding, runtime libraries, or anything chosen per backend before
  lowering.

## 3. The crate's public API, in shape

Proposed; names are placeholders until Level 2.

```rust
/// One library's program: modules plus the tables they share.
pub struct Program {
    pub library: LibraryInfo,      // name, version, dependencies
    pub modules: Vec<Module>,      // in a stable, deterministic order
    pub nodes: Nodes,              // arena: kinds, children, payloads
    pub types: TypeTable,          // interned: equal types, equal ids
    pub decls: DeclTable,          // every declared thing, by DeclId
    pub strings: Interner,
    pub spans: SpanTable,          // per node: file and byte range
    pub docs: DocTable,            // per declaration
}

pub struct Builder { … }           // the frontend's only way in; checks as it goes

pub struct Profile { … }           // what a backend handles natively
impl Profile { pub fn full() -> Profile; } // everything high-level: the starting tmpl

pub fn validate(p: &Program, profile: &Profile) -> Vec<Problem>;
pub fn lower(p: Program, profile: &Profile) -> Program; // only the passes it needs

pub mod passes;   // each lowering, also callable alone (tests, tools)
pub mod naming;   // NameRequest, NamingRules, assign() -> NameMap
pub mod analysis; // captures, purity, may-bubble, liveness: shared queries
pub mod text;     // write, read, view(ViewOptions)
pub mod binary;   // write, read (later)
```

A backend's whole use of the crate: `lower(program, &MY_PROFILE)`, then
`naming::assign(&lowered, &MY_RULES)`, then walk the result.

## 4. Core decisions

### 4.1 One model, high-level to low-level, chosen by profile

**Decided (the direction):** one data model whose constructs range from
high-level (`for … of`, generators, `orelse` and bubbling, closures,
classes with properties) to low-level (plain loops, explicit error
results, state machines, closure conversion). Each backend declares a
profile of what it handles natively; shared passes lower only what it
lacks; a validator checks a tree against a profile. State machines are
only a tmpl post-pass (milestone-3.md, flat frames).

**Proposed refinements:**

- **A profile is a set of independent features, not a level.** Targets
  don't line up on one scale: Go has closures but no exceptions, Lua has
  coroutines but no `continue`, C99 has `goto` but no closures. So each
  feature (labeled `continue`, `orelse` as exceptions, generators,
  closures, …) is lowered or kept on its own.
- **Some features are choices, not absences.** Bubbling as exceptions,
  result values, or error codes; function types as plain types or named
  interfaces; module initialization eager, on first use, or by class
  init. A profile holds these as enums, and the pass for each choice is
  shared.
- **Passes run in one fixed order,** each removing one feature and
  declaring what it may introduce (closure conversion may add classes;
  the state-machine pass needs labeled loops and a switch). `lower`
  picks the needed subset; the order never varies, so output is
  deterministic.
- **Lowered output is still tmpl,** checked by the validator against the
  profile after every pass in debug builds and tests.
- **Low-level nodes are defined when a target needs them.** Error codes,
  closure conversion and `goto`-like jumps serve only the stress list
  today; define and build them when such a target is real. The model
  should leave room, not carry dead code.

### 4.2 Fully explicit

**Decided:** more explicit than the frontend, since tmpl must be enough
for every backend. Proposed meaning, item by item:

- **Types on every expression,** as interned `TypeId`s (a `u32` each).
  A backend never infers a type.
- **Resolved names:** every reference points at a `DeclId`, unique in
  the program, which carries its source name, kind (local, parameter,
  field, method, type, module-level), visibility, and whether it's
  exported: exactly a `NameRequest` (design.md section 8). Compiler-made
  declarations carry a role hint instead of a source name. tmpl holds no
  output names; the naming pass returns them in a side table.
- **Conversions are nodes.** Upcasts to an interface, a function
  becoming a `@fun` interface's value and back, adapting between
  interfaces, a non-null value used as nullable, a checked not-null,
  `Never<T>` used as a `T`, and numeric widening if Temper adds any.
  Kotlin's Rust translator works these out again from types (its
  `to_listed` casts); here they're in the tree.
- **Captures are listed** on each function literal: each captured
  declaration, and whether it's assigned after capture (Python's
  `nonlocal`, Rust's and Java's shared cells, PHP's `use (&$x)`). Kept in
  the tree and checked by the validator against a recomputation.
- **Evaluation order** is left to right, as Temper's, and a fact rather
  than a shape: tmpl keeps nested expressions, and a pass hoists into
  temporaries only where a profile says the target doesn't guarantee
  the order (C++ arguments) and only for operands that aren't pure
  (an analysis in the crate). This honors design.md's "fewer
  temporaries" rule.
- **Bubbling paths:** every call that may bubble is marked, with its
  handler: the nearest enclosing `orelse`, or the function's exit. So
  every lowering of failure (exceptions, results, error codes, and
  reference-count cleanup in a C target) reads the same facts.
- **Operators are typed builtins.** `==` on `Boolean` is its own builtin,
  not a generic `==` a backend may forget (Kotlin's Lua, JS and Python
  bug); each builtin is a stable key with explicit operand types.
- **Source positions** on every node (file and byte range), and **doc
  comments** on every declaration, for source maps and generated docs.

### 4.3 Independent of the frontend, and one starting form

**Decided:** nothing from the frontend in the model. Proposed in detail:

- **Its own types, names, values and metadata,** all ids into its own
  tables. Literals are a small enum (Boolean, the numeric types, String,
  null, Void), never interpreter values. No closures or host objects in
  the tree.
- **One starting tmpl for all backends.** Kotlin consults the backend's
  `SupportNetwork` (bubble strategy, coroutine strategy, support code)
  while translating from the frontend, so its tmpl is per backend and
  can't be cached, compared, or tested alone. Here, every per-backend
  choice is a pass after the starting form.
- **Builtins and connected members by key.** A call of a builtin or a
  connected method names a stable key (as `String.isEmpty`'s connected
  key); each backend's table maps keys to its runtime. Connected types
  are declarations marked connected, with their key, so a backend skips
  them and all their members (design.md, "Declaration facts in one
  place").

### 4.4 Arena layout, like the tree

Proposed: nodes in arrays with `u32` ids, children as ranges, payloads
in side tables, interned strings and types, as design.md section 3's
tree. That keeps passes fast, makes the binary form a near-direct dump,
and keeps order deterministic. Passes rebuild into a fresh arena rather
than editing in place, which keeps each pass simple and the old program
intact for comparison in tests.

### 4.5 Types

Proposed; Level 2 details them.

- Nominal types with arguments, type parameters (invariant, as typing.md
  decides), nullable types, `Void`, `Never<T>` with its fallback type,
  and function types.
- **Function types carry their `@fun` interface's name** when they have
  one (Decided, design.md "Function types and fun interfaces"). A profile
  chooses plain function types (C++, JS, Lua, Python, Rust, likely C#) or
  named interfaces (Java); the pass for the latter gives any remaining
  bare function type a synthesized interface. Bare function types stay
  in the model while tests still use them.
- **Bubbling is an effect, not a type:** a function type says whether it
  may bubble, as `throws Bubble` does in Temper, and calls are marked
  (4.2). Kotlin's `Result<T, Bubble>` union is one backend's lowering,
  not the model. The failure itself has room for a payload later.
- **Classes are final;** only interfaces are extended, and they may carry
  method bodies. Properties, getters and setters stay high-level, and a
  pass makes them methods where a target needs that.

### 4.6 Object literals, builders, and no named arguments

Decided earlier (design.md, "Object literals, not named arguments"):
tmpl has a constructor call with its arguments already placed, never
symbols or named arguments. Proposed: whether a public constructor
gets a builder (Java, Rust) or named-argument form (JS, Python) is a
helper query in the crate, answered once, so every backend agrees on
which constructors get one.

### 4.7 Module initialization: order and threads

**Decided (robustness):** tempest's backends should be more robust than
Kotlin's where they can. Proposed for tmpl:

- **Each module has one ordered init list:** its top-level statements
  and static initializers in source order (after whatever top-level
  ordering Temper settles on; kotlin-differences.md, "Top-level
  statements in source order"). Declarations of types and functions
  aren't in the list, so nothing initializes before a value it reads.
  This fixes Kotlin's order bug (types before values; C++ reading zero,
  Python's `NameError`). The validator rejects an initializer that
  reads a module value initialized later.
- **A profile picks the init strategy:** eager on import (Python, JS,
  Lua), on first use behind a once-guard (Rust, C++; api-compat.md
  proposal 2), or class initialization (Java, C#).
- **Module-level `var`s are marked,** with reads and writes as their own
  nodes, so a pass can make them thread-safe where a target has threads
  (atomic `shared_ptr` access in C++, a lock or atomic in Rust, Java and
  C#), fixing the race Kotlin's C++ has.

### 4.8 Errors stay values

Unresolved code arrives as a typed `Error` node with its message, as in
design.md section 6; every backend turns it into a run-time failure.
The validator treats it as valid in any profile.

### 4.9 The public surface is first-class

**Decided (2026-10-29):** high versus low level isn't only about
constructs inside bodies. tmpl carries enough intent about a library's
public surface that each backend can present an idiomatic API, not just
correct code. Proposed:

- **Two kinds of profile choice:** body lowerings (section 5) and
  surface choices, kept apart, since a backend may lower a body
  construct and still expose its idiomatic form (a Java state machine
  behind an `Iterator`, Rust results behind a public `Result`).
- **Intent, not spelling, in the model.** Exported declarations keep
  what they mean: a property (with getter, setter, or both, and
  whether it's computed), a constructor that takes an object literal, a
  `@fun` interface by name, a function that may bubble, a generator or
  iterable, `async` results, optional parameters, docs, and source
  names with their kind. Lowering never erases these from the surface;
  passes that change a body record which surface form it presents.
- **Surface choices, per backend,** as data in its profile: properties
  as fields, getters and setters, or native properties; builders,
  named arguments, or option structs for constructors; named interfaces
  or plain function types; exceptions or `Result` in signatures;
  native iterators or generators (`Iterator`, `IEnumerable`, Python
  generators, Rust `Iterator`); and naming conventions, through
  `NamingRules` (design.md section 8). Shared helpers answer each once
  (4.6's builder query is one), so backends agree.
- **Public API compatibility** ([api-compat.md](api-compat.md)) is
  checked at this layer: the surface a backend derives from tmpl is
  what its API diff compares.

### 4.10 Mutability and isolation, for later

**Decided (2026-10-29):** don't distort the design for actors and
isolates (BEAM, Haskell), but give general solutions a shape that would
allow them (testing-and-milestones.md, "Process isolation"). Proposed:

- **Where mutable state lives is explicit:** module-level `var`s are
  already marked (4.7); a class records whether it's mutable (any `var`
  property or setter), as Rust's wrappers already want (api-compat.md
  proposal 4); builders (`ListBuilder`, `MapBuilder`) are known types;
  assigned captures are listed (4.2).
- **Facts that would decide what crosses an isolate boundary:** a type
  is deeply immutable when it's final, has no mutable state, and holds
  only deeply immutable types. Proposed as an analysis in the crate,
  not a stored field, until a backend needs it; Rust's `Arc` versus
  `Rc` and lock choices could use it sooner.
- **No global identity assumptions** in passes: a pass that adds
  module-level state (caches, interned helpers) marks it like a user's
  module `var`, so a future per-isolate lowering sees it too.

## 5. Profiles and lowering, first cut

What each primary backend handles natively, and the shared lowering
otherwise. Proposed; Level 2 makes this the profile's real fields, and
Kotlin's choices are the starting point (its `SupportNetwork`s).

| Feature | Native in | Lowering otherwise |
|---|---|---|
| `orelse` and bubbling | — (a choice) | exceptions (C++, C#, Java, JS, Lua, Python); results (Rust); error codes (later: C99, Go) |
| Generators, `async` | C#, JS, Lua, Python; Rust's `async` | state machine (C++, Java, Rust generators) |
| `for … of` | most, over lists | index loop, or `forEach` with a body function |
| Labeled `break`, `continue` | C#, Java, JS, Rust | flags or restructuring (Python); `goto` (Lua `continue`) |
| Switch on an integer | C#, C++, Java, JS, Rust | `if` chains (Lua, Python) |
| Expressions with statements inside | Rust | statements and temporaries (the rest) |
| Assigned captures | JS, Lua, Python (`nonlocal`), C# | shared cells (C++, Java, Rust) |
| Properties with getters and setters | C#, JS, Python | methods (C++, Java, Lua, Rust) |
| Function types | all but Java | named interfaces (Java) |
| `Void` as a value | JS, Lua, Rust | not reified (C#, C++, Java, Python) |
| Left-to-right evaluation | all but C++ | temporaries for impure operands (C++) |
| `Int32` wrapping | C#, C++, Java, Rust | helpers where value ranges can't rule it out (JS, Lua, Python) |
| Module-level `var` across threads | JS, Lua, Python (no shared threads) | atomics or locks (C#, C++, Java, Rust) |

## 6. Formats

**Decided (the direction):** a binary and a human-friendly but complete
text form, each round-tripping losslessly; one schema defining both;
one canonical, complete text form; an optional viewer that elides
derivable detail; binary also serving the library cache.

Proposed:

- **The schema is Rust code in the crate,** a `macro_rules!` table that
  declares each node kind and its fields once, and generates the node
  types, a field walker, and the text and binary readers and writers. No
  proc macros or new crates (code-style.md). If the macro gets unwieldy,
  fall back to a `build.rs` generator over a schema file.
- **Canonical text is complete but not redundant.** Every field is
  written, except a type that a few local rules give: a literal's type,
  a name's type from its declaration, and a call's result from its
  callee's signature with explicit type arguments. The reader fills
  those by the same rules, and the validator checks every stored type
  against them, so dropping them loses nothing. No inference is
  involved.
- **Temper-like syntax** for statements and expressions, with
  declarations named `name#id` once and referenced by id, so it reads
  like the source and diffs well. A golden test reads like the program.
- **Spans in a trailing section,** optional on reading: golden tests
  strip it so edits elsewhere don't churn expectations, and text without
  it still reads, with unknown spans. With it, the round trip is exact.
- **The viewer** elides what's derivable or noisy (ids where names are
  unique, conversions, capture lists, types that match the obvious
  ones) for people. It never needs to read back.
- **Binary** (can come later without changing the model): a header with
  magic and versions, a string table, the type and declaration tables,
  node arrays in varints, and spans; written to a spec so another
  implementation (Kotlin Temper, as a bridge) could read it.

### Where the goals conflict, and the recommendations

The user's priorities: (1) complete and explicit for every backend, (2)
profiles and lowering, (3) text round-trip, (4) binary.

- **Explicit versus readable text (1 against 3).** Every type, id,
  conversion, and span written out makes text hard to read. Recommended:
  derivation rules plus a trailing spans section (above) keep canonical
  text complete yet close to source, and the viewer covers reading.
- **Explicit versus idiomatic output (1 against design.md's naming
  rules).** Spelling out evaluation order as temporaries would bloat
  generated code. Recommended: order is a guarantee and purity a fact;
  temporaries only come from a profile's lowering.
- **One model across levels versus simple passes (2).** Every pass must
  walk every node kind, and the node set grows with low-level forms.
  Recommended: node families with default structural walks, so a pass
  touches only its kinds, and low-level nodes only when a target needs
  them.
- **One starting form versus per-backend choices (1 and 2).** Kotlin made
  some choices during translation that tempest moves into passes (Java's
  boxed type parameters, Rust's `Equatable` bounds). Where a choice is
  purely a target's spelling, it belongs in that backend's translator,
  not a pass.
- **Binary later versus a stable cache (4).** No conflict in the model,
  but versioning starts on day one, in the text header too (section 8).

## 7. Targets

**Decided:** primary targets are Kotlin Temper's backends (C++, C#,
Java, JavaScript, Lua, Python, Rust). Python is tempest's first backend
and Rust right after, so the design keeps both ends in view: Python
needs few lowerings but exact capture and scope facts, and Rust needs
results, state machines, explicit conversions, nullable as `Option`,
and ownership-friendly facts.

The stress list, kept in mind but not central:

| Target | What it stresses | How the design copes |
|---|---|---|
| C99 | No GC, generics, closures, classes, or exceptions | Error codes from the explicit bubbling paths; closure conversion; erasure or monomorphizing; structs and vtables. Reference counting relies on no cycles ([acyclic.md](acyclic.md)) and explicit cleanup on every exit path, a large new pass. Possible, not cheap. |
| PHP | `use (&$x)` captures; arrays copied by value | Capture lists give the first; lists as runtime objects cover the second. Copes. |
| Go | No exceptions or inheritance, limited generics (no generic methods) | Result-style lowering to `(T, error)`; Temper's classes are final, so interfaces plus delegation for interfaces' method bodies; generic methods as functions. Copes, with some passes. |
| Swift | ARC, value types, protocols, `try` at each call | Every bubbling call is already marked; ARC relies on no cycles; Temper's objects are classes, not structs. Copes. |
| Dart | Little: exceptions, `sync*`, `async`, null safety | Copes with today's features. |
| Elixir, BEAM | Immutability, processes, no loops | Locals by rebinding and loops as recursion are generic passes, and 4.10's facts say which values may cross processes. Shared mutable objects still need a heap emulated in processes or tables. Doesn't cope cheaply; room is left. |
| Haskell | Purity, laziness, no mutation without `IO` or `ST` | Same facts as BEAM; deeply immutable types become plain data, and mutable state goes through `IORef`-like cells, with effects marked per function by the analysis. Doesn't cope cheaply; room is left. |
| Actors and isolates generally | What state is per isolate, what may be sent | Module-level state, mutable classes, builders and captures are all explicit (4.10); a future isolation model chooses per-isolate state or forbids it. |
| MATLAB | 1-based indexing, value semantics, captures by value | Indexing stays in runtime operations, never raw arithmetic in tmpl; handle classes for objects; cells for assigned captures. Partly. |
| Ruby | Little: blocks, exceptions, `Enumerator` | Copes. |
| Kotlin | Little: `fun interface` native, sequences | Copes; named function types map directly. |
| WASM | No high-level anything; structured control only | Labeled blocks and loops fit `br` directly; otherwise the C99 path, or WasmGC for memory. Copes via the low end. |

## 8. Versioning and compatibility

Proposed:

- **Two versions in every file's header:** the format's (how it's
  encoded) and the model's (what nodes and fields mean). Text and binary
  share the model version.
- **At first, exact match only:** a cache is valid only for the compiler
  that wrote it ([library-cache.md](library-cache.md)), so the model can
  change freely until tempest's backends settle.
- **Later, for published caches:** new node kinds and fields only with
  defaults, read by older minor versions as errors only where used;
  meaning changes bump the major version. Recorded in `temper.keep`
  alongside other backward-compatibility facts.
- **Only the starting tmpl is ever stored.** Lowered forms are internal
  to a build, so passes can change without a version bump.

## 9. Testing

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
- **Fuzzing:** settle and translate mutated inputs as
  `mutated_inputs_settle` does, checking that translation never panics,
  output validates, and passes are deterministic; and random valid tmpl
  from the schema, lowered under random profiles.
- **A tmpl interpreter, later** (milestone-3.md): the semantic reference
  for passes. Run a program before and after lowering and compare
  output, and compare with the frontend's interpreter on the functional
  tests to check the translation itself.

## 10. Lessons from Kotlin Temper's tmpl

From Kotlin Temper's `tmpL.out-grammar` (which generates `TmpL.kt`),
`TmpLTranslator.kt`, `TypeTranslator.kt`, `FunctionTypeStrategy.kt`,
`SupportNetwork.kt`, and the Python, Rust, and C++ backends. Read for
lessons, not as a template.

- **Frontend leaks:** nodes hold frontend objects (`TypeShape`,
  `Signature2`, `Type2`, `Value`, `ResolvedName`, `NamingContext`,
  library configurations), and the Rust backend calls the frontend's
  override finder. tmpl can't exist without the frontend.
- **Per-backend from the start:** the `SupportNetwork` is consulted
  during translation, and support code objects sit in the tree.
- **Two of several things:** old and new type nodes side by side
  (`AType`), and ids that are either frontend names or target names.
- **What it can't express, so backends recompute it:** captures
  (Python's `nonlocal`, Rust's clones), conversions (Rust's casts
  between list types), `for … of` (already a `forEach` with a lambda,
  so `yield` in its body fails), values carried by failure, and
  whether module-level state is shared across threads.
- **Bugs it lets through:** top-level sorting that puts types before
  values; Boolean `==`; C++'s race on lazily set module `var`s; Rust's
  indexing that panics where Temper bubbles; C++ templates declared
  after the types that use them.
- **What works and is kept:** `Error` (its "garbage") nodes, so errors
  reach run time; the coroutine and bubble strategies as ideas, now
  profile choices; marking where a function becomes a fun interface's
  value; a computed jump node for state machines.

## 11. Open questions for Level 2, with recommendations

1. **Canonical text syntax:** Temper-like or s-expressions? Recommend
   Temper-like, with a small strict grammar, for golden tests' sake.
2. **Schema mechanism:** `macro_rules!` or a `build.rs` generator?
   Recommend `macro_rules!` until it hurts.
3. **Type elision rules:** are the three local rules (literals, names,
   calls) enough, and do they keep the reader simple? Recommend those
   three only.
4. **Top-level order:** keep Kotlin's sorting until the team decides, or
   source order now? Recommend source order for statements, with
   declarations hoisted, behind the frontend's ordering, so tmpl doesn't
   decide it.
5. **`for … of`:** a loop over an iterable in tmpl, lowered per target,
   rather than Kotlin's `forEach` lambda? Recommend yes; it's what lets
   `yield` work in loop bodies later.
6. **Captures in the tree or computed?** Recommend in the tree, checked
   by recomputation, so passes and text show them.
7. **Ownership facts for Rust, Swift, and C** (clones, last uses,
   retains): in the model or computed by passes? Recommend computed, not
   stored, until a backend proves otherwise.
8. **Bubbling payloads:** reserve a value on failure now? Recommend
   reserving the field, unused until Temper decides.
9. **Profile shape:** a struct of booleans and enums with a `full()`
   default? Recommend yes, plain data, no traits.
10. **Builtin keys:** one namespace for builtins, operators, and
    connected members, as strings interned per program? Recommend yes,
    with the keys listed in the crate so backends' tables are checked
    for completeness.
11. **Statements inside expressions** (`if` and blocks as values, as
    the frontend's core constructs allow) at the high level? Recommend
    yes, with a statement-izing pass; Rust keeps them.
12. **Spans:** file id and byte range, with line and column computed
    only for source maps? Recommend yes, as the frontend does.
13. **Generics for C-like targets:** erasure or monomorphizing? Defer;
    keep full generics in the model.
14. **The tmpl interpreter:** also for compile-time evaluation of
    cached dependencies (library-cache.md's open question)? Recommend
    deciding with the cache, after milestone 6.
15. **Surface intent:** which public-surface facts does the model keep
    (4.9), and which are derived by shared helpers? Recommend storing
    only what source says (property, getter and setter, `@fun` name,
    may bubble, generator, optional parameters) and deriving the rest,
    such as builders and iterator adapters.
16. **Surface choices in profiles:** one profile with body and surface
    sections, or two structs? Recommend one struct, two sections, so a
    backend's whole stance is in one place.
17. **Immutability facts** for isolates, Rust's `Rc`, and locks:
    computed per class by an analysis, or declared in Temper (a
    `@immutable` check)? Recommend computed now; a declared form is a
    language change for the team.
18. **What may cross an isolate:** deeply immutable values only, or
    also copied values? Defer until an isolation model exists; keep
    module state, builders, and mutable classes explicit meanwhile.

## Level 2: structure (placeholders)

### Node families

### The type model

### Declarations, names, and linking across modules and libraries

### Modules: imports, exports, and the init list

### Profiles: features and choices

### The public surface: intent and per-backend choices

### Mutable state and isolation facts

### Analyses shared by passes

### Format structure: text grammar, viewer, and binary layout

## Level 3: details (placeholders)

### Each node's fields and invariants

### Each lowering pass: preconditions, output, and examples

### The naming pass and each backend's `NamingRules`

### The validator's rules
