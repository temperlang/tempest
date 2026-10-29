# tmpl: declarations, names, and linking

Part of the [tmpl design](README.md).

## Level 2: Declarations, names, and linking

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
list says which ([modules.md](modules.md)).

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
  isn't captured (liveness, [analyses.md](analyses.md)), and only then the shortest suffix
  ([question 23](open-questions.md)).

The naming pass returns a `NameMap` from `DeclId` to string; tmpl
itself stores no output names.

## Level 3: The naming pass and each backend's `NamingRules`
