# tmpl: the backend IR

Part of the [rewrite plan](../README.md). tmpl is the one form between
the frontend and every backend: a settled, typed, fully explicit
program that a backend can translate without asking anything of the
frontend. It refines design.md section 7 ("tmpl, the backend IR") and
section 8 ("Naming").

## How the levels work

A fractal design: each topic file grows by level, and each level
unlocks code ([roadmap.md](roadmap.md)).

- **Level 1** (2026-10-29) is purpose, boundaries, the API's shape,
  core decisions, targets, formats, versioning, and testing.
- **Level 2** (2026-10-29, after the user's review of Level 1) is
  structure: node families, types, declarations, modules, profiles, the
  public surface, analyses, and the formats' shape. In Level 2,
  everything is Proposed unless marked Decided, and names are
  placeholders until Level 3 fixes each node's fields.
- **Level 3** (each node's fields, each pass) goes subset by subset.
  Subset A (basics) is drafted (2026-10-29): node fields, the
  validator's rules, source positions, the text grammar and view, the
  naming pass, the hoisting pass, operators and protocols, and golden
  tests. Subsets B, C, and D are sketched in the roadmap.

**Decided** marks the user's direction, with its date; everything else
is **Proposed**, for review.

## Files

- [roadmap.md](roadmap.md): the levels and subsets, interleaved with
  milestone 4.
- [overview.md](overview.md): purpose, boundaries, the crate's API,
  core decisions, and lessons from Kotlin Temper's tmpl.
- [nodes.md](nodes.md): node families, equality, and invariants.
- [types.md](types.md): the type model.
- [names.md](names.md): declarations, names, and linking.
- [modules.md](modules.md): imports, exports, and initialization.
- [profiles.md](profiles.md): profiles, features, and lowering passes.
- [surface.md](surface.md): the public surface and per-backend choices.
- [mutability.md](mutability.md): mutable state and isolation facts.
- [analyses.md](analyses.md): analyses shared by passes.
- [formats.md](formats.md): text, viewer, and binary, versioning, and a
  worked example.
- [targets.md](targets.md): primary targets and the stress list.
- [testing.md](testing.md): round trips, goldens, and fuzzing.
- [open-questions.md](open-questions.md): numbered questions with
  recommendations.
