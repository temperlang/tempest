# tmpl: modules and initialization

Part of the [tmpl design](README.md).

## Level 1: Module initialization: order and threads

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

Level 2, below, has the init list's structure.

## Level 2: Modules: imports, exports, and the init list

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
  later is an error; through a call, a warning ([question 19](open-questions.md)).
- **Tests** are a separate list of `Test` declarations, run after
  init.

Init strategy is a profile choice ([profiles.md](profiles.md)). The first-use pass adds
`EnsureInit` where an exported function, method, or getter may reach
the module's state, by the reaches-module-state analysis; a module
with no state has no guard.
