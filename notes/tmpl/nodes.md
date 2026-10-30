# tmpl: nodes

Part of the [tmpl design](README.md).

## Level 1: Errors stay values

Unresolved code arrives as a typed `Error` node with its message, as in
design.md section 6; every backend turns it into a run-time failure.
The validator treats it as valid in any profile.

## Level 2: Node families

Every node has a kind, a type, a span, a range of children, and maybe
a payload (an id, a literal, an operator). Kinds are grouped into families;
each family has a default structural walk, so a pass touches only the
kinds it changes. The schema says, for each kind, its family, its
children's roles, and the profile feature it needs, if any, so the
validator's "allowed in this profile" rule is generic.

**Declarations** (each defines one `DeclId`): `Module`, `Class`,
`Interface`, `Function` (top-level or local), `Method`, `Constructor`,
`Getter`, `Setter`, `Field`, `ModuleLet` (with `var` marked), `Param`,
`TypeParam`, and `Test`. A connected declaration's body, if any, is
its pure Temper fallback ([names.md](names.md#level-3-connected-declarations)). Locals
and labels are declared by the statements below.

**Statements:**

- Binding: `Let` (local, maybe `var`, maybe without an initializer).
- Writes: `Assign` to a local, `SetField`, `SetStatic`, and
  `SetModuleVar` ([modules.md](modules.md): module `var`s have their own read and write).
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
  placed arguments; object literals arrive as this), and `Op` (an
  operator on builtin types, and typed operands). Core's and std's
  members are ordinary calls of external declarations, never `Op`s
  (Proposed 2026-10-29, [below](#level-3-ops-and-core-as-declarations)).
  Arguments for omitted optional parameters are an explicit `Omitted`,
  since defaults apply inside the callee.
- Equality and comparison, below.
- Conversions ([overview.md](overview.md#fully-explicit)): `Upcast`, `ToFun` and `FromFun`, `AsNullable`,
  `NonNull` (checked), and `FromNever`; and run-time checks `Is` and
  `As` (which may bubble).
- Functions: `FunctionLiteral`, with its parameters, captures, and
  body, typed by its `@fun` interface or bare function type.
- Control expressions: `IfValue`, `BlockValue`, `And`, `Or`, `OrElse`
  (a body and a fallback), `Bubble`, and `Panic`.
- Generators and async: `Yield` and `Await`, in functions marked
  generator or `async`.
- `Error`, valid anywhere (Level 1, above).

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

### Equality

**Decided (2026-10-29):** Temper's `==` is always type-specific, so tmpl
has no generic `==` node:

- **A class's or interface's `==`** is a `CallMethod` of its method
  marked `@operator("==")`, its own or the nearest inherited one,
  virtual as any method call. A class with `@auto("==")` and no such
  method gets a synthesized `eq` declaration, marked `@auto("==")` and
  compiler-made, whose body compares each property with its own type's
  equality, as Kotlin's `AddAutoMembers` does.
- **Each builtin type's equality is its own operator:** `Boolean.eq`,
  `Int32.eq`, `Int64.eq`, `Float64.eq`, and `String.eq`.
- **A comparison with `null`** is `IsNull` (or `IsNotNull`), never a
  call.
- **Identity** is `Identical(a, b)`, only for class and interface
  types, and only where the source asks for it (Decided 2026-10-29,
  [question 12](open-questions.md)). Backends
  map it to `is` (Python), `===` (JS), `ReferenceEquals` (C#), `==` on
  references (Java), `Arc::ptr_eq` (Rust), pointer comparison (C++), and
  `rawequal` (Lua).
- **`!=`** is `Op(Boolean.not)` around the equality, which a
  translator may print as `!=`.
- **Nullable on both sides** (Decided 2026-10-29, [question 15](open-questions.md)): `EqNullable(a, b)`
  names the equality for the non-null values (an `Op` or a
  method) and means `IsNull(a) ? IsNull(b) : !IsNull(b) && eq(a!, b!)`.
  A pass expands it where the profile says the target's own `==` isn't
  null-safe.
- **Comparisons** (Proposed, [question 16](open-questions.md)): builtin types have their own
  operators (`String.lt`, `Int32.cmp`); a class's `a < b` is
  `Op(Int32.lt, CallMethod(compareTo), 0)` with the method marked
  `@operator("<=>")`.

The method keeps its own name (`eq`, `compareTo`); whether a backend
also presents it as its host's equality protocol is a surface question
([surface.md](surface.md)).

**Gaps in tempest's typing today** (typing.md, "Operators defined in
Temper"; milestone-4.md, "Operators defined by methods"), for slice 7:

- **Not every `==` is resolved.** Typing checks a class's operator
  method only for a non-null first operand of a class or interface it
  knows. A generic `T`, `AnyValue`, or function operand, or a class
  with an unknown supertype, is neither checked nor resolved (as in
  Kotlin), and the interpreter then compares by identity
  (`objects_equal` in `interp.rs`): the generic identity `==` this
  design removes. Each must become a resolved call or an error
  (Decided 2026-10-29, [question 13](open-questions.md)) before `to_tmpl`
  can translate it.
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
  typing ([question 12](open-questions.md)).

## Level 3: Each node's fields and invariants

Subset A (basics) first; everything here is Proposed unless marked
Decided. [roadmap.md](roadmap.md#level-3-after-subset-a) sketches what
subsets B, C, and D add.

### The node record

Nodes live in columns, one entry per `NodeId` (a `u32`):

```rust
pub struct Nodes {
    kind: Vec<Kind>,          // u8, from the schema
    ty: Vec<TypeId>,          // TypeId::NONE for statements and declarations
    span: Vec<Span>,          // "Source positions", below
    kids: Vec<(u32, u32)>,    // start and length in `kid_list`
    a: Vec<u32>,              // payload slots; the schema names each
    b: Vec<u32>,              //   kind's use of them
    flags: Vec<Flags>,        // u8: MAY_BUBBLE, VAR, …
    kid_list: Vec<NodeId>,
}
```

The builder appends a node after its children, so a child's id is
always lower than its parent's. Payloads are ids (`DeclId`, `StrId`,
`Op`, `TypeListId`) or literal bits; `0` in `b` means none.

As built (2026-10-29, Proposed): `var` is a fact on the declaration
only, not a node flag, so the `VAR` flags in the tables below are the
declaration's; a node's flags are `MAY_BUBBLE` and `DERIVED` (its
span's bit). Type arguments in `b` wait for subset B's generics. The
schema's table is `crates/temper-tmpl/src/schema.rs`
([roadmap.md](roadmap.md#state)).

### Subset A's kinds

Declarations, each defining the `DeclId` in `a`:

| Kind | Children | Payload and flags |
|---|---|---|
| `Function` | `Param`s, then the body `Block` | `a`: the function |
| `Param` | none (a default value comes with subset B) | `a`: the parameter |
| `ModuleLet` | none; its initializer is an `InitLet` | `a`: the module `let`; `VAR` |
| `BodilessFunction` | `Param`s | `a`: a connected function without a body (built 2026-10-29) |

Statements (no type):

| Kind | Children | Payload and flags |
|---|---|---|
| `Block` | statements | |
| `Let` | the initializer, or none | `a`: the local; `VAR` |
| `Assign` | the value | `a`: a local |
| `SetModuleLet` | the value | `a`: a module `var` |
| `InitLet` | the value | `a`: a module `let`; only in an init block |
| `If` | condition, then `Block`, maybe else `Block` | |
| `While` | condition, body `Block` | `a`: its label |
| `DoWhile` | body `Block`, condition | `a`: its label |
| `Break`, `Continue` | none | `a`: a loop's label |
| `Return` | the value, or none in a `Void` function | |
| `ExprStatement` | an expression | |

Expressions, with where their type comes from (the local rules are
[formats.md](formats.md#level-1-formats)'s, plus [question 17](open-questions.md)'s fourth):

| Kind | Children | Payload and flags | Type |
|---|---|---|---|
| `BoolLit` | none | `a`: 0 or 1 | `Boolean` |
| `Int32Lit` | none | `a`: the bits | `Int32` |
| `Int64Lit`, `Float64Lit` | none | `a`, `b`: low and high bits | `Int64`, `Float64` |
| `StringLit` | none | `a`: a `StrId` | `String` |
| `VoidLit` | none | | `Void` |
| `Local` | none | `a`: a local or parameter | its declaration's |
| `GetModuleLet` | none | `a`: a module `let` | its declaration's |
| `CallFunction` | arguments | `a`: the function; `b`: type arguments; `MAY_BUBBLE` | its signature's result |
| `Op` | operands | `a`: the operator; `MAY_BUBBLE` | its operator's result |
| `CallMethod` | receiver, then arguments | `a`: a method or getter; `MAY_BUBBLE` | its signature's result |
| `Omitted` | none | | its parameter's |
| `And`, `Or` | left, right | | `Boolean` |
| `IfValue` | condition, then, else | | written |
| `BlockValue` | statements, then the value | | written |
| `Error` | none | `a`: the message | written |

Changes from Level 2's names: module `let`s, `var` or not, are read by
`GetModuleLet` and written by `SetModuleLet` (the declaration says
`var`, and only a `var` is written); every loop declares a label,
compiler-made (`~loop`) when the source has none, so `Break` and
`Continue` always name one; and `Error` is typed as its place wants,
not `Never` of it ([questions 28, 29, and 27](open-questions.md)).

Invariants, per kind (the validator's rule codes are below):

- **`Let`** declares a local owned by the enclosing function; its
  scope is the rest of the enclosing `Block`. Without an initializer,
  it's definitely assigned before each read; without `VAR`, at most
  once on any path.
- **`Assign`** targets a local of the same function, `VAR` or not yet
  assigned (subset A has no captures), or a parameter marked `VAR`
  (As built, 2026-10-30: R2 accepts one, and R6 rejects assigning a
  parameter that isn't `VAR`).
- **Conditions** are `Boolean`.
- **`Break` and `Continue`** name an enclosing loop of the same
  function.
- **`Return`** has a value exactly when the function's result isn't
  `Void`, and the value's type is the result. A function whose result
  isn't `Void` never reaches the end of its body.
- **Calls** have one argument per parameter, each of exactly the
  parameter's type after substituting the type arguments (subset A has
  no conversions, since it has no subtypes), or `Omitted` for an
  optional one. `MAY_BUBBLE` is set exactly when the callee or
  operator may bubble.
- **Bubbling:** in subset A, a call that may bubble has no handler
  (`orelse` comes with B), so its function must be marked may-bubble.
  In an init block, it fails the module's initialization
  ([question 35](open-questions.md)).
- **`IfValue`'s branches** and **`BlockValue`'s value** have the
  node's own type.
- **`Error`** is valid anywhere, with any type (Level 1, above).

### Source positions

**Decided (2026-10-29):** source positions are carried, for source
maps and similar tools; backends emit source maps from them. Proposed
content:

```rust
pub struct Span { pub file: FileId, pub start: u32, pub end: u32 } // bytes, end exclusive
pub struct SourceFile {
    pub path: StrId,              // relative to the library's root
    pub hash: [u8; 16],           // of the text, to match it later
    pub line_starts: Vec<u32>,    // byte offsets, for lines and columns
    pub wide_lines: Vec<(u32, StrId)>, // text of lines with non-ASCII characters
}
```

- **Every node and declaration has a span.** A declaration has two:
  the whole declaration, and its name (for a source map's `names` and
  for tools such as go-to-definition).
- **Each node's span is its own or derived,** one bit per node.
  Derived marks a node that the frontend or a pass made, which points
  at the source construct that caused it.
- **Columns:** source maps count UTF-16 units, so a backend converts
  byte columns using `wide_lines`; ASCII lines need no text, and the
  sources needn't be present ([question 34](open-questions.md)).
- **Unknown spans** come only from reading text without its spans
  section; such a program is marked as having no spans, and backends
  then write no source map.

What each node points at:

1. **The frontend:** each node takes the span of the settled node it
   translates. Nodes it synthesizes are derived and take the span of
   what asked for them: an `@auto("==")` method's body takes the
   annotation's span, a constructor's field assignments their
   parameters', and a loop's compiler-made label the loop's.
2. **Passes:** a node a pass keeps or rebuilds keeps its span and bit.
   A node a pass adds is derived and takes the span of the node it
   stands for: a temporary's declaration, assignments, and reads take
   the hoisted expression's; a `Try` and its handler take the
   `orelse`'s; a result check takes its call's; cell operations take
   the variable use they replace; a state machine's dispatch and state
   take the function's; and an init guard takes the declaration it
   guards.
3. **Backends:** the output tree copies each node's span; the printer
   records a mapping on entering a node whose span differs from its
   parent's, and a declaration's name with its source name. Derived
   spans map too, so stepping through a temporary lands on the
   expression it holds; a variant may leave them out.

As built (2026-10-30), for JS: a source file's line table comes from
its text in `to_tmpl` (`Builder::set_file_text`); text read back has
none, so it maps nothing until the reader finds the sources. Wide
lines keep their text as a `String`, as `path` does. Rather than an
output tree that copies every node's span, the JS tree carries spans
on statements and functions' names only, and its printer writes into
`source_map::MappedText`, which records the byte offset where each
span's output starts; `source_map::source_map` turns those into lines
and UTF-16 columns. Mapped so far: statements (and so `else if`),
functions' JSDoc, `function`, and names (with the source name in
`names`). Maps leave out `sourcesContent`.

## Level 3: Ops, and core as declarations

**Decided (2026-10-29):** customization is general, by naming: only
operator lookups and connected declarations
([names.md](names.md#level-3-connected-declarations)), never a
hardcoded table of core functions such as `%console.log`.

Proposed, replacing subset A's 113-key table:

- **`Op` is only operators on builtin types,** a closed set of 65,
  as Kotlin's `BuiltinOperatorId`s are: per type, `eq`, `lt`, `le`,
  `gt`, `ge`, and `cmp` (`Boolean`, `Int32`, `Int64`, `Float64`, and
  `String`); `add`, `sub`, `mul`, `div`, `mod`, and `neg` (the numeric
  types); `bitAnd`, `bitOr`, `bitXor`, `bitNot`, `shl`, `shr`, and
  `ushr` (the integer types); `Float64.pow`; `Boolean.not`; and
  `String.cat`. They belong to the language, not a library, and passes
  and analyses (purity, value ranges, hoisting) match them by enum,
  never by string. Each variant spells every one.
- **Everything else is a declaration of core or std,** referenced by
  `CallFunction` or `CallMethod` like any other: the other 48 keys
  (`Int32.toString`, `Float64.sqrt`, `String.toInt32`, `String.isEmpty`
  as a getter, …) are `@connected` members of core's classes, with
  qnames such as `core.type Int32.toString()`. Their signatures are
  core's, defaults included (`toString`'s radix), so calls pass
  `Omitted` for them.
- **`console`, as Kotlin does it:** the frontend declares a
  compiler-made module `let`, `~console`, initialized by core's
  connected `getConsole()`, and `console.log(s)` is a virtual
  `CallMethod` of `Console`'s `log` on it.
- **An explicit `a.eq(b)` on a builtin type** is its `Op`, as `==` is,
  though core declares `Int32.eq` as a connected `@operator("==")`
  method.
- **Scalar types stay fixed type entries** ([question 26](open-questions.md)):
  core's class declaration for one, such as `Int32`, is an external
  declaration owning its members, linked to the fixed entry by its
  qname.

## Level 3: The validator's rules

`validate(&Program, &Profile)` returns problems, each with a rule
code, a node or declaration, and a message, sorted by writing order,
so output is deterministic. Structure runs first; the other groups run
only on a structurally sound program, so one mistake isn't reported
many times. Golden tests list problems as `RULE @node: message`.

- **S, structure.** S1: each kind's children match the schema's
  roles and count. S2: every node has one parent. S3: statements are
  in statement places and expressions in expression places. S4: the
  profile allows each kind and flag, and starting tmpl has no
  lowered-family kind.
- **T, types.** T1: every expression has a type, and every
  declaration its declared type. T2: a node the local rules cover has
  exactly their type. T3: every child's type is its role's exactly;
  there are no implicit conversions. T4: type arguments match the
  callee's type parameters in number. T5: every type is well formed
  (no `Nullable` of a nullable, `Result` only in a lowered program).
- **R, references.** R1: every id exists and isn't removed. R2: a
  reference's declaration is of the kind its node wants (`Local` a
  local or parameter, `CallFunction` a function). R3: a local is read
  only in its scope, within its function. R4: labels enclose their
  jumps. R5: a `Let` without an initializer is definitely assigned
  before each read. R6: only `VAR` declarations are assigned twice.
- **F, flow.** F1: a function whose result isn't `Void` never falls
  off its end. F2: a `Return` has a value exactly when the result
  isn't `Void`. F3: `MAY_BUBBLE` is set exactly when the callee may
  bubble. F4: a call that may bubble has a handler, or its function
  may bubble. F5: an initializer doesn't read a module `let` that's
  initialized later (through a call, it's a warning, [question 19](open-questions.md)).
- **D, declarations.** D1: every compiler-made declaration has a hint
  with at least one candidate. D2: owners form a tree rooted at
  modules. D3: every type an export mentions is exported or external.
  D4: `@imu` and `@partialImu` marks hold (subset B,
  [mutability.md](mutability.md)). D5: stored facts (may-bubble marks,
  and from subset B captures) match their recomputation.
- **Q, qnames and connected** (Proposed 2026-10-29, built;
  [names.md](names.md#level-3-qnames)). Q1: qnames are unique within a
  library. Q2: every declaration but compiler-made ones has one. Q3: a
  function or method without a body is connected or external (or,
  from subset B, abstract). Q4: a connected declaration with a backend
  list has a body, its fallback.
- **P, positions.** P1: in a program with spans, every node and
  declaration has a known span inside its file.

`Error` nodes are exempt from T2 and T3.

As built (2026-10-29, Proposed), with core's declarations:

- **S3:** `Omitted` goes only in a call's arguments; a module's items
  may be `BodilessFunction`s.
- **T1 and T5:** a class's or interface's type is its own `Nominal`
  type, or for core's class of a scalar (`core.type Int32`) the
  scalar; a getter takes no arguments; a parameter's `OPTIONAL` fact
  matches its signature; a `Nominal` type names a class or interface.
- **T3:** a `CallMethod`'s receiver has its method's owner's type, and
  `Omitted` fills only an optional parameter. Operators have none.
- **R2:** `CallMethod` names a method or getter; `CallFunction` a
  function, a static one included.
- **D2:** a used library is listed in `Program::uses` and has no owner;
  an external class or interface is owned by its library, a method or
  getter by a class or interface, and a function by a module, library,
  class, or interface.
- **Q1 and Q2** check `QNames::derive`; Q2 exempts compiler-made
  declarations and what they own, labels, modules, and used libraries,
  which Kotlin gives no qname, and reports an override that doesn't
  parse. Q3 is checked on `BodilessFunction`; external declarations
  never have bodies. Q4 also rejects a backend list on a declaration
  that isn't connected.
- **P1** exempts other libraries' declarations, whose spans, if any,
  point into their own files.
- **Virtual isn't stored:** a `CallMethod` is virtual exactly when its
  method's owner is an interface (`Program::is_virtual`), so there's no
  `VIRTUAL` flag to keep in step.
- **Room for failure kinds** ([question 44](open-questions.md)): a
  signature's `bubbles` and a node's `MAY_BUBBLE` stay booleans, and a
  failure kind would be an added, optional field beside them (a
  `Nominal` declaration, by qname); nothing here depends on `Bubble`
  being the only one.
