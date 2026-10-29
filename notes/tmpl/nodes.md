# tmpl: nodes

Part of the [tmpl design](README.md).

## Level 1: Errors stay values

Unresolved code arrives as a typed `Error` node with its message, as in
design.md section 6; every backend turns it into a run-time failure.
The validator treats it as valid in any profile.

## Level 2: Node families

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
  placed arguments; object literals arrive as this), and `Builtin` (a
  key and typed operands: every operator and every core member).
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
- **Each builtin type's equality is its own builtin:** `Boolean.eq`,
  `Int32.eq`, `Int64.eq`, `Float64.eq`, and `String.eq`.
- **A comparison with `null`** is `IsNull` (or `IsNotNull`), never a
  call.
- **Identity** is `Identical(a, b)`, only for class and interface
  types, and only where the source asks for it ([question 12](open-questions.md)). Backends
  map it to `is` (Python), `===` (JS), `ReferenceEquals` (C#), `==` on
  references (Java), `Arc::ptr_eq` (Rust), pointer comparison (C++), and
  `rawequal` (Lua).
- **`!=`** is `Builtin(Boolean.not)` around the equality, which a
  translator may print as `!=`.
- **Nullable on both sides** (Proposed, [question 15](open-questions.md)): `EqNullable(a, b)`
  names the equality for the non-null values (a builtin key or a
  method) and means `IsNull(a) ? IsNull(b) : !IsNull(b) && eq(a!, b!)`.
  A pass expands it where the profile says the target's own `==` isn't
  null-safe.
- **Comparisons** (Proposed, [question 16](open-questions.md)): builtin types have their own
  keys (`String.lt`, `Int32.cmp`); a class's `a < b` is
  `Builtin(Int32.lt, CallMethod(compareTo), 0)` with the method marked
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
  ([question 13](open-questions.md)) before `to_tmpl` can translate it.
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

## Level 3: The validator's rules
