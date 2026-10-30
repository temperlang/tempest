# Temper in Rust: rewrite plan

Tempest, a skunkworks reimplementation of Temper. It keeps
Temper's language, philosophy, functional-test behavior and generated public
APIs, with a new implementation that is much simpler, smaller and faster. Kotlin
Temper stays the product meanwhile; core fixes keep landing there.

This folder holds the plan in parts:

- [design.md](design.md): the tree, parsing, the settle loop, errors, tmpl,
  naming, backends, and CLI/watch/LSP.
- [api-compat.md](api-compat.md): the public API each backend must keep, what
  may change, and proposed deliberate changes.
- [size-and-speed.md](size-and-speed.md): code size today vs. target,
  speed and memory targets, and what the measurements in
  [size-and-speed.csv](size-and-speed.csv) show.
- [compile-time.md](compile-time.md): macros, constants and inlining:
  on-demand evaluation, a cheap interpreter, budgets and caching.
- [typing.md](typing.md): type inference, and flow facts (narrowing, value
  ranges) from structured passes without a control-flow graph.
- [testing-and-milestones.md](testing-and-milestones.md): how compatibility is
  checked, including semantics and error messages; milestones; risks.
- [milestone-3.md](milestone-3.md): the settle loop: core constructs, tree
  layout, names, and waiting and waking; and its status.
- [milestone-4.md](milestone-4.md): types in the settle loop: its slices,
  and their status.
- [acyclic.md](acyclic.md): ruling out reference cycles at compile time,
  despite mutability (a design sketch).
- [code-style.md](code-style.md): ownership without lifetime annotations,
  structure, and the dependency policy.
- [library-cache.md](library-cache.md): a stable cache of frontend results
  (a preliminary assessment).
- [test-speed.md](test-speed.md): fast `tempest test` on every backend,
  with a fast path kept in agreement with the package path (a proposal).
- [tmpl/](tmpl/README.md): tmpl, the backend IR: its crate, nodes,
  types, profiles and lowering, formats, and targets, as a layered
  design with one file per topic.
- [kotlin-differences.md](kotlin-differences.md): where tempest differs
  from Kotlin Temper on purpose, and Kotlin Temper's known bugs.

## 1. Goals and non-goals

Goals:

- **Same language.** Every functional test passes on every backend, and
  Temper source written today keeps working, including the flexible grammar
  where chained blocks become macro or function calls.
- **Same public APIs** in generated libraries, so library users can't tell the
  difference. A *few* deliberate changes are fine where there's good reason,
  since there are few users today. Each one gets written down with its reason.
- **Much simpler.** Fewer concepts and representations, plainly written code.
  Smaller should follow from simpler, not from being terse.
- **Fast and lean.** Far fewer allocations, good data locality, a small single
  binary, little memory.
- **Same philosophy:** report errors and keep going, all the way through code
  generation.

Non-goals:

- Matching Kotlin Temper's internal representations, stages, intermediate
  forms, or private generated details. Kotlin's frontend tests are useful only
  as hints about semantics.
- Matching non-exported names, private members or function bodies in
  generated code. These should get *prettier*, for example without numbered
  names like `x__123`.

## 2. How it works, end to end

```text
source files (.temper, .temper.md)
  │ literate split: which lines are code, which are prose
  │ lexer: tokens with byte spans
  │ parser: operator precedence, straight into the tree
  ▼
tree (one per library build, in an arena)
  │ settle loop: names, macro calls, types, inlining, compile-time values,
  │              until nothing changes; leftovers become error nodes
  ▼
tmpl (backend IR: settled, typed, named for output)
  │ shared lowering: control flow, failure handling, coroutines per backend needs
  │ shared naming: readable, collision-free names
  ▼
output tree (shared, with source spans) → printer → source files + source maps
  + copied runtime libraries
```

There are only three data structures a reader needs to learn: tokens, the
tree, and tmpl.
