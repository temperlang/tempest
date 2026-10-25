# Compile-time evaluation: macros, constants and inlining

Part of the [rewrite plan](README.md).

Temper runs code at compile time: macros (built-in and written in Temper),
constants, and the evaluation that inlining and some typing decisions rely on.
Kotlin Temper does much of this by partially interpreting whole modules at
several stages (`InterpretiveDanceStage`), with a cap to stop infinite loops.
Tempest needs the same semantics, without paying to interpret everything,
and with a structure that stays easy to follow.

## Principle: evaluate only on demand

Nothing is interpreted unless the settle loop needs a value:

- a macro call whose macro is known and whose inputs are ready,
- a constant that something else needs (an array size, a type argument, a
  config export, a value another macro inspects),
- a call the inliner decided to fold, with known arguments.

Everything else stays as tree and goes to code generation untouched. Most code
in a library is never evaluated at compile time.

## Two kinds of macros

1. **Built-in macros** (`let`, `fn`, `if`, `while`, `class`, `when`, `test`,
   `assert`, and the rest) are Rust functions in a table:
   `fn(&mut SettleCx, NodeId) -> Step`. They inspect and rewrite tree nodes
   directly. No interpreter is involved, so they cost about as much as a
   parser rule. These are almost all of the macro calls in practice.
2. **Macros written in Temper** run in the interpreter. They receive tree
   fragments as values and return new fragments, which replace their call
   node.

Both kinds expand exactly once: the expansion replaces the call node, so it's
never revisited.

## A cheap interpreter

The interpreter is its own small crate, `temper-eval`, with one entry point
the settle loop calls:

```rust
fn eval(cx: &mut EvalCx, function: FnId, args: &[Value], fuel: &mut Fuel) -> Result<Value, EvalError>
```

What keeps it fast:

- **Compile once, run many.** The first time a function is evaluated, its
  settled body is lowered to a compact form (a flat list of simple
  instructions with local *slot numbers*, not names) and cached by `FnId`.
  Later calls reuse it. No tree walking or name lookup during execution.
- **Locals in slots,** in a `Vec<Value>` per call frame, not hash maps.
- **Small values:** `Value` is a compact enum (int, float, bool, interned
  string, `Rc` list/map/object, function id, tree fragment), so copying one
  is cheap.
- **No speculative evaluation.** Only demanded calls run (above).
- **Only evaluable code:** pure Temper code, plus connected builtins marked
  safe at compile time. Anything else (I/O, time, randomness) is a
  compile-time error with a clear message, as today.

## Budgets and loop protection

- **Fuel:** every instruction costs one unit. Each demanded evaluation gets a
  budget, and a build has an overall budget too. Running out is an error at
  the call site that started it, naming the macro or function, and the build
  still continues (errors as values).
- **Expansion depth:** macros that expand into more macro calls get a depth
  limit, with the chain of macros reported when it's hit.
- **Cycle detection:** if the same macro is expanding the same input
  (structurally identical, by hash) somewhere further up the same chain, report
  a cycle right away instead of burning fuel.

The limits are generous by default and configurable, so legitimate heavy
macros still work.

The interpreter already takes an optional step budget
(`interp::run(settled, fuel)`); plain runs (`run-temper`) default to none,
so the budget is the compile-time evaluator's to set.

## Caching

- **Within a build:** expansions of the same Temper macro on structurally
  identical input, with the same relevant facts, are memoized, so repeated
  patterns (like many similar declarations) run the macro once.
- **Across watch rebuilds:** libraries that didn't change keep their settled
  summaries, so none of their macros or constants run again. For the library
  being edited, unchanged functions keep their lowered interpreter code
  (keyed by a content hash), so only edited code is lowered again.

## Inlining

Deciding what to inline doesn't need the interpreter. It's a size and usage
heuristic on settled code: small functions, functions called once, and
functions whose arguments are known constants. Inlining itself is a tree
rewrite: copy the callee's nodes into the arena with fresh ids, then
substitute the arguments. Folding the result is ordinary constant evaluation,
on demand.

## Measuring

`--verbose` (or a stats flag) reports per build: macro expansions (built-in vs.
Temper), evaluations, instructions run, time in evaluation, and cache hits.
Check this on the functional tests, std and Chromarite from the start, so cost
regressions show up early. Compare against Kotlin Temper's time for the same
inputs, back to back, since timing on this machine is noisy.

## Why this stays clear

- One place decides *when* to evaluate (the settle loop, by demand).
- One crate decides *how* (`temper-eval`, with one entry point).
- Built-in macros are plain Rust functions over the tree, readable one at a
  time.
- Budgets, depth limits and caches live in the evaluator and the settle loop,
  not scattered through macros.

## Caching and inlining rules (decided 2026-10-22)

- Cache or fold a compile-time result only when it's immutable and
  guaranteed to be the same value it would be at run time. Anything that
  depends on run-time state, or is mutable, stays a run-time computation.
- There should eventually be a way to prevent inlining in some contexts
  (an annotation or option), for code size, debugging, or semantics that
  depend on a call staying a call.
- Folding a call can make closures that didn't exist before it ran
  (Kotlin's `control-flow/for-of-folded` test: `for … of` and `forEach`
  bodies assigning the caller's locals). Only the call's result may be
  kept, and only if it's immutable; the closures and the locals they
  captured must not leak into the folded tree.
