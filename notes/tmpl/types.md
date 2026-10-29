# tmpl: types

Part of the [tmpl design](README.md).

## Level 1: Types

Proposed; Level 2, below, has the structure. Nominal types with arguments,
invariant type parameters (typing.md), nullable types, `Void`,
`Never<T>`, and function types that carry their `@fun` interface's name
(Decided, design.md "Function types and fun interfaces"). Bubbling is an
effect on a function type, not a type; Kotlin's `Result<T, Bubble>` is
one backend's lowering. Classes are final; only interfaces are
extended, and they may carry method bodies.

## Level 2: The type model

The `TypeTable` interns every type, so equal types have equal ids. Its
variants:

- `Nominal { decl, args }`: classes, interfaces (`@fun` ones too), and
  builtin types, which are external declarations from core, linked by
  qname (`Int32`, `String`, `List`, `Map`, `Generator`, `Promise`, …).
- `TypeParam(decl)`, with its bounds on the declaration.
- `Nullable(T)`, never of a nullable `T`.
- `Function { params, optional, result, bubbles }`: bare function
  types, kept while tests use them; Decided to go in favor of named
  `@fun` interfaces.
- `Void`, `Never(T)` (bottom with its fallback type), and `AnyValue`
  (the top).
- `Result(T)`: only in programs lowered to results ([nodes.md](nodes.md)).

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

## Level 3: Types for subset A

Proposed.

```rust
pub struct TypeId(u32);           // TypeId::NONE (u32::MAX) for statements
pub struct TypeTable {
    data: Vec<TypeData>,          // interned: equal types, equal ids
    lists: Vec<TypeId>,           // TypeListId ranges, for arguments and parameters
}
pub enum TypeData {
    Scalar(Scalar),               // ids 0 to 6, the same in every program
    Function { params: TypeListId, result: TypeId, bubbles: bool },
    // Subset B: Nominal, TypeParam, Nullable, Never; subset C: Result.
}
pub enum Scalar { Void, Boolean, Int32, Int64, Float64, String, AnyValue }
```

- **Scalar builtin types are fixed entries,** not external
  declarations as Level 2 had them: every program has them, they need
  no link key, and text writes them bare (`Int32`, not `Int32#7`).
  Generic builtin types (`List`, `Map`, …) stay external declarations,
  linked by qname, since they have type parameters and members to link
  ([question 26](open-questions.md)).
- **Temper's `Int` is `Int32`,** in text and everywhere else.
- **A function declaration's type is its signature,** a `Function`
  entry: parameter types, result, and whether it may bubble. This
  stays when bare function types go away, since it's only how
  signatures are stored.
- **Subtyping in subset A is equality;** with no classes or nullable
  types, a child fits its role only by having the role's type.
- **Operators have fixed signatures,** in the crate's operator table
  ([analyses.md](analyses.md#level-3-analyses-for-subset-a)), so an
  `Op`'s type comes from its operator. Core's class for a scalar type
  (`core.type Int32`) is still an external declaration, owning its
  members ([nodes.md](nodes.md#level-3-ops-and-core-as-declarations)).
