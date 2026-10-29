# tmpl: mutability and isolation

Part of the [tmpl design](README.md).

## Level 1: Mutability and isolation, for later

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

Level 2, below, has the structure.

## Level 2: Mutable state and isolation facts

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
these rules and their messages. Decided (2026-10-29, [question
21](open-questions.md)): tempest's frontend reports them in Kotlin's
words, and the tmpl validator rechecks them.

**Derived** (Decided 2026-10-29: by analysis, where the source says
nothing), and agreeing with the marks where it does:

- A class is **mutable** if it has a `var` field or setter, or extends
  an interface that does.
- A type is **deeply immutable** if it's final, not mutable, and every
  type it holds is deeply immutable; type parameters count by their
  arguments.
- A function **touches module state** if it may read or write a module
  `var` or call something that does ([analyses.md](analyses.md)).

First users: Rust's lock or no lock (soon), `Rc` versus `Arc` (later),
and isolates (much later). The frontend's acyclic check (slice 8) needs
similar facts; it should share the definitions written down here, not
the code, since the frontend runs first.

## Level 3: Mutability checks, from subset B

Subset A has no classes, so nothing here applies to it. Proposed for
subset B:

- **Facts:** `IMU` and `PARTIAL_IMU` bits on type and type parameter
  declarations, and `VAR` on fields, locals, and module `let`s.
- **The frontend checks the marks** in Kotlin's words (`ImuChecker.kt`)
  first (Decided 2026-10-29); the validator's rule D4 rechecks the
  same rules on tmpl, as an invariant that no pass may break.
- **`analysis::mutability`** returns the mutable classes as a bit set
  by declaration, and memoizes deep immutability by `TypeId`.
