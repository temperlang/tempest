# Typing and flow facts

Part of the [rewrite plan](README.md).

Typing works in two layers that cooperate inside the settle loop:

1. **Declared and inferred types** of declarations and expressions, as
   constraints solved when their inputs are known (see the settle loop in
   [design.md](design.md)).
2. **Flow facts**: what's known at each *use* of a variable, such as "not null
   here", "is a `Circle` here", or "between 0 and 255 here". These come from
   structured passes over a function body, with no control-flow graph.

Static types may change what a program compiles to, such as which class an
object literal constructs, or what a backend emits.
The interpreter and dynamic backends like Python ignore types at run time,
so where types change meaning, they act on the tree before that, and the
interpreter runs the result.

## Flow facts without a CFG

Temper's settled tree has structured control flow: blocks, `if`, loops,
labeled `break`/`continue`, `return`, and failure (`bubble` with `orelse`).
So a pass can walk a function body in evaluation order and carry an
*environment* of facts, merging at the points where control flow merges,
instead of first building a graph.

The environment maps each local to what's known about it:

- **Type:** narrowed by `is`, casts and null checks.
- **Nullability:** null, not null, or either.
- **Value range,** for numbers: a constant, an interval like `0..=255`, or
  unknown. For strings and lists, possibly emptiness or length bounds.
- **Definitely assigned**, or not.

How each construct is handled:

- **Sequence:** facts flow through statements in order.
- **`if (c) A else B`:** the condition refines facts separately for each
  branch (`a != null` makes `a` non-null in `A` and null in `B`); after the
  `if`, merge the two outgoing environments. A branch that always exits
  (`return`, `throw`, `break`) contributes nothing to the merge, which is what
  makes the early-return pattern work:

  ```temper
  if (a == null) { return x; }
  a.length   // `a` is non-null here
  ```

  Kotlin Temper rejects this today; this design handles it naturally.
- **Loops:** iterate the body until the environment at the loop head stops
  changing. To guarantee that quickly, ranges *widen* after a couple of rounds
  (an interval that keeps growing becomes unknown in that direction).
- **`break` and `continue`:** carry their environment to the loop or label
  they target; that target merges everything that arrives.
- **`return` and failure:** carry their environment to the function exit,
  or to the nearest `orelse`, which merges whatever can fail inside it.
- **Closures:** a local that a closure might assign loses its narrowed facts
  wherever the closure might run. Locals no closure assigns keep them.

The cost is about linear in the function's size, plus a few rounds for loops.

### Why "semi-holistic"

Each function body is analyzed as a whole once its inputs have settled (the
types of the things it calls and reads), rather than fact by fact. If an
input changes later in the settle loop, which is rare, the function's facts
are recomputed. Results go into side tables keyed by node id, so the tree
itself doesn't change.

### Risks to design for

- **A small core of control constructs.** After macro expansion, everything
  must reduce to forms the flow pass knows: block, `if`, loop, `break` and
  `continue`, `return`, `bubble`/`orelse`, and function. Macros that leave
  other shapes behind would make the flow pass complicated. This is the main
  commitment of the design.
- **`orelse` merges many exits.** Every call that can bubble is an implicit
  exit, so the facts at `orelse` are the merge of the facts at each such
  call. Easy to get subtly wrong; test it well.
- **Flow facts and macros affect each other within a function.** A narrowed
  type can pick which method a call resolves to, and that can trigger a
  macro that needs types. So each function gets its own small, bounded fixed
  point: expand, type and flow, run late macros, flow again.

## What flow facts are used for

- **Typing:** the type of each use of a local, so null checks and `is` checks
  narrow the way people expect.
- **Errors:** null access, use before assignment, unreachable code, and casts
  that can never succeed, all with precise spans.
- **Generated code quality:** skipping runtime checks that facts prove
  unnecessary. The big one is integer overflow: Python and Lua route every
  `Int` operation through helper functions to emulate 32-bit wraparound, and
  ranges can show that most loop counters and indexes can't overflow, so they
  can use plain `+`. Also bounds checks and null checks in other languages.
- **Constant folding** where values are known.

## Type parameters are invariant

Temper plans to make all type parameters invariant, because C++ and Rust,
among others, can't express variance well. Tempest doesn't support
covariant or contravariant type parameters at all: `List<Sub>` is never a
`List<Super>`. Kotlin Temper's `@out` and `@in` annotations (about a dozen,
all in `core.temper`, as on `Generator<@out YIELD>`) parse as ordinary
decorations and are ignored, with no error. When copying `core.temper` and
the `std` library from Kotlin Temper, keep them as they are, so the files
stay easy to diff, and rely on this rule.

### To explore later: variance without runtime variance

Both ideas only accept more programs, so neither conflicts with starting
invariant.

1. **Use-site sugar for bounded generics.** Most wanted covariance is
   read-only use of a parameter. `let total(shapes: List<some Shape>)` could
   mean `let total<T extends Shape>(shapes: List<T>)`: monomorphized in Rust
   (`impl Trait`) and C++ (templates), and a wildcard or generic method in
   Java and C#. Zero runtime cost. Limits: parameters only, and not on
   overridable methods, since Rust's `dyn` traits and C++ virtual functions
   can't have generic methods. Temper already forbids generics on
   overridable methods ("Overridable methods don't allow generics"), so the
   sugar would follow the same rule. Classic Temper's Rust runtime takes the
   same approach for `map<O>` and friends: a non-generic trait, plus generic
   free functions over `&dyn ListedTrait<T>`. Core's own `Listed.map<O>`,
   `reduceFrom<O>`, `reduceFromIndex<O>` and `Mapped.toListWith<T>` break
   the rule (Kotlin exempts core). Tempest's typed core will declare them
   as extensions instead, keeping the same call syntax with static
   dispatch:
   `@extension("reduceFrom") let listedReduceFrom<T, O>(listed: Listed<T>, initial: O, accumulate: fn (O, T): O): O`,
   and `@connected` ones (`map`) as connected top-level functions. Then
   the rule holds everywhere, with no exemption. Accepted: Java and C#
   host code then sees these as free functions rather than methods,
   unlike classic Temper's output, which may make the same breaking
   change. Today the untyped
   interpreter runs them as built-in list methods (`interp.rs`), exercised
   by `types/list/reduce` and `imports/functions`.
2. **Adapters inserted at conversions,** for what's left: wrap a function
   value to accept a narrower argument (O(1), always valid); wrap an `@out`
   read-only interface in a view that upcasts elements on access, or an
   `@in` one in a view that upcasts inputs. The compiler knows each concrete
   cast, so it can emit it as a closure, which works in stable Rust; C++ can
   use converting constructors like `shared_ptr`'s. Costs: an allocation per
   conversion, dynamic dispatch on every access, and a view that isn't the
   same object as the original, so identity checks differ. Copying an
   immutable concrete list instead is correct but O(n). Only worth it if
   real code needs it.



- **Locals without annotations** take the type of their initializer, widened
  to a declared supertype only if later assignments need it.
- **Generic calls** solve their type arguments from argument types and the
  expected result type at that call site. The solver is small: unify, collect
  bounds, pick the most specific type that satisfies them.
- **Function signatures** come from annotations, which Temper requires at
  module boundaries, so libraries typecheck independently.

## Checking semantics and messages

The type checker's behavior and its error messages should match Kotlin
Temper's wherever practical. Sources to compare against: the frontend stage
tests, the backend tests, the REPL tests, and running Kotlin Temper itself,
including its REPL, on the same inputs. See
[testing-and-milestones.md](testing-and-milestones.md).
