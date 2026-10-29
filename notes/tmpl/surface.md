# tmpl: the public surface

Part of the [tmpl design](README.md).

## Level 1: The public surface is first-class

**Decided (2026-10-29):** tmpl carries enough intent about a library's
public surface that each backend can present an idiomatic API, not just
correct code.

- **Decided (2026-10-29): the model stores only what the source says**
  about the public surface; shared helpers derive the rest.
- **Body lowerings and surface choices are kept apart** (Proposed),
  since a backend may lower a body construct and still expose its
  idiomatic form (a Java state machine behind an `Iterator`, Rust
  results behind a public `Result`).
- **Public API compatibility** ([api-compat.md](../api-compat.md)) is
  checked at this layer: the surface a backend derives from tmpl is
  what its API diff compares.

Level 2, below, has the structure.

### Object literals, builders, and no named arguments

Decided earlier (design.md, "Object literals, not named arguments"):
tmpl has a constructor call with its arguments already placed, never
symbols or named arguments. Whether a public constructor gets a builder
(Java, Rust) or named-argument form (JS, Python) is a `surface` query,
answered once, so every backend agrees.

## Level 2: The public surface: intent and per-backend choices

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
  defaults ([Level 1, above](#object-literals-builders-and-no-named-arguments)).
- **Property form:** native property, getter and setter methods, or a
  field (api-compat.md's per-backend rules).
- **Failure in signatures:** exceptions or `Result`, even where a body
  was lowered differently.
- **Iteration:** a generator presented as the host's iterator
  (`Iterator`, `IEnumerable`, Python generators, Rust `Iterator`).
- **Equality:** whether the `@operator("==")` method also implements
  the host's protocol ([question 14](open-questions.md)), and `!=` with it where needed.
- **Mutability:** whether a Rust class wrapper needs a lock
  (api-compat.md proposal 4), from [mutability.md](mutability.md)'s analysis.

Profiles carry the per-variant choices as data in their `surface`
section (a Kotlin-compatible Python variant keeps Kotlin's shapes; an
idiomatic one may choose differently, each with its own API baseline), and naming conventions come from `NamingRules`. Passes that
change a body never change surface facts; a pass that changes how a
public declaration works records which surface form it presents (a
state-machine generator still presents an iterator). The API diff
compares what these helpers derive.
