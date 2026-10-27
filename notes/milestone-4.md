# Milestone 4 plan: types in the settle loop

Part of the [rewrite plan](README.md). Types become facts in the settle
loop, enough for most functional tests in-process, with the type errors
Kotlin Temper reports, in its words, and what backends will need from
types. How typing should work is in [typing.md](typing.md); how the loop
works, in [milestone-3.md](milestone-3.md).

## Slices

Each its own chunk, roughly in this order:

1. **Declared and inferred types** of literals, locals, parameters, calls
   of known functions and results, with Kotlin's errors for them. Done
   (2026-10-27); status below.
2. **Flow facts** (typing.md): null checks narrowing locals, so a nullable
   value where a non-null one is wanted can be reported as Kotlin does.
   Done (2026-10-27) for locals that aren't `var`; status below.
3. **Builtins' signatures:** operators, `console.log`, and the builtin
   types' methods, with Kotlin's "Actual arguments do not match
   signature" and "Cannot bubble from a function without Bubble in its
   return type". Operators and `console.log` done (2026-10-27); status
   below. Left: the builtin types' methods (`s.toInt32()`, `ls.length`,
   `m[k]`), whose signatures are in Kotlin's `core.temper`, which tempest
   doesn't load yet; likely with slice 4, since they're members.
4. **Classes:** members' types, `this`, method calls, field initializers
   and `new`, subtyping through `extends`, and the class checks left from
   milestone 3 (signatures of overrides, "No callee matches" for
   inherited ties). A function-typed field called as `obj.f(x)` types as
   a call of the field's value (Kotlin rejects it), with a case in
   `tests/settle`.
5. **Generics:** type arguments, and generic calls solved per call site,
   including `Never<T>` carrying its `T`, with Kotlin's rules
   (typing.md), in place of the plain bottom type.
6. **Target typing** for object, list and function literals, which Kotlin
   Temper doesn't do yet, with trailing blocks placed while typing rather
   than at run time.
7. **Every expression typed,** as tmpl and backends need. Including a
   `Void` result where a value is wanted, such as the empty body of a
   non-`Void` function (`let f(): Int { }`), which Kotlin misses and
   which fails only at run time; and a declaration whose type a cycle
   leaves unknown, reported as such rather than Kotlin's "Cannot assign
   to String from Top".
8. **No reference cycles** ([acyclic.md](acyclic.md)): an error for a
   mutable reference whose type could lead back to its owner, and for
   `this` escaping a constructor. Temper has always planned this, but
   Kotlin Temper doesn't do it yet, so the rules need agreeing with the
   team first; maybe warnings at first.

Left from milestone 3 that needs types: `case` patterns, "Void
expressions cannot be used as values" (as for a `do … while` used as a
value), and string tags that evaluate to a class other than through
names.

## Status: the first slice

`types.rs` has interned types (`TypeId`s for builtin types, classes,
nullable types and function types), subtyping and how messages write
types; `typing.rs` has the steps.

**Facts.** Typing starts once expansion has settled, as a second round
of the loop. Each declaration whose type comes from its initializer, as
`let x = f(1)`, is a work item that publishes `Fact::Type(decl)` once it
knows that type, waiting on the declarations its initializer reads, in
any order; a cycle (`let a = b; let b = a`) is forced, leaving the type
unknown. Declared types and functions' signatures come from annotations,
needing no waiting. A local takes its initializer's type, as in Kotlin
(`var a = 1; a = "s"` is an error). Then a pass over each file checks
what it can, children first, as Kotlin's `TypeChecker` does. Types of
expressions go into a side table by node, so the tree doesn't change.
Macros that need types will interleave with this round once there are
any, so keep typing's results facts that a waiting macro can be woken by
(design.md, "Macros must be able to wait for types").

**Unknown is quiet.** A type typing hasn't worked out, such as a
member's, a method call's or a generic one's, never causes an error, and
nor does a type that may have subtypes it doesn't know: a class, a type
parameter, a generic type. So the only values rejected are of the final
builtin types (`Boolean`, `Int32`, `Int64`, `Float64`, `String`, `Void`),
null, and functions, where they can't fit. `AnyValue` where a final type
is wanted isn't reported, as in Kotlin.

**Errors reported**, as Kotlin does on the same inputs:

| Message | Where |
|---|---|
| Cannot assign to Int32 from String | a declaration's initializer; an assignment (all of it); `return v` (all of it); a function's result (each branch's last expression, through `if`, `when` and `orelse`), `Void` functions included |
| Expected subtype of Int32, but got String | a call's argument, where the callee is a known function or a parameter of function type |
| Expected subtype of String, but got String? | `null` where a non-null type is wanted, in any of those places |
| Wrong number of arguments.  Expected 2 | a call of a known function with too few or too many arguments; parameters with defaults are optional |
| Expected value of type Boolean not Int32 | a condition, of `if`, `while` and the like |
| Explicit return type required | a function declared by `let`, a `fn` that initializes an untyped `let`, a method or a getter without a return type; not one passed as an argument, whose parameter gives it a type |

Calls with a function literal argument or type arguments aren't checked
yet, since trailing blocks are still placed at run time. Type errors
don't change what runs, as Kotlin's "not fatal" static errors in
`semantics/type-checked-locals`, which now reports exactly Kotlin's five.
The functional tests, std, Chromarite and Ordable get no others.

**Differences from Kotlin**, until later slices:

- A nullable local or parameter where its non-null type is wanted
  (`let f(a: String?): String { a }`) is reported since the second
  slice, below.
- A `Void` result where another type is wanted isn't reported. Kotlin
  reports "Cannot assign to Int32 from Void", past the function's end
  when the last statement is a `let`.
- A bare `return` in a non-`Void` function is reported at the `return`;
  Kotlin's position is past the function's end.
- Types left unknown by a cycle aren't reported; Kotlin says "Cannot
  assign to String from Top".
- Parameters' defaults aren't checked yet.
- Kotlin folds `let` constants before checking, so its messages may name
  the value's type rather than the declared one (in
  [kotlin-differences.md](kotlin-differences.md#changes-we-recommend-to-temper)); ours use the declared type.

`x++`'s result now spans `x`, where Kotlin reports a type error about it.

**Tests:** `tests/settle/types.cases`, whose inputs were also built with
Kotlin Temper (`temper build`): every error matches in message and
position, apart from the differences above; and a unit test of subtyping
in `types.rs`. The fuzz test (`mutated_inputs_settle`) covers typing too.

## Status: flow facts

`flow.rs` has the pass (typing.md's flow facts, for null checks only so
far): before inferring, one walk over each file, in evaluation order,
carries the set of declarations known not to be null, merging where
control flow merges, and marks the uses it reaches with a fact. A use of
`a: String?` so marked is a `String`. Only locals, parameters and
top-level names that aren't `var` are narrowed, as in Kotlin, so a fact
never stops holding: loops need no fixed point, and closures keep the
facts where they're made. A function or class declared in a block gets
only the facts at the block's start, since it may be used before its
declaration; after a loop, or a block that a `break` leaves, only the
facts from before it hold.

**Narrowed:** the branches of `if` (and `when` and `??`, which
expand to it) on `a == null`, `a != null`, `null == a` and `a is T` for
a non-nullable `T`, through `!`, `&&` and `||`; `while` bodies; and
after an `if` whose other branch always leaves, by `return`, `break`,
`continue` or `bubble()`.

**Now reported**, as Kotlin does, in its words and at its positions: a
nullable value where its non-null type is wanted, at every site of the
first slice (`Cannot assign to String from String?`, `Expected subtype
of String, but got String?`), including an argument for a parameter
with a default.

**Not reported, as in Kotlin:** a nullable value where an `AnyValue` is
wanted (only `null` itself is). And since Kotlin folds constants before
checking, a value it might work out while compiling, as `let c: String?
= "q"` or a call `maybe(1)` with constant arguments, isn't reported
either: only one that reads a parameter or an assigned `var` is
(`may_fold` in `typing.rs`).

**Differences from Kotlin:** Kotlin narrows only in the branches of an
`if` whose condition is a null check or `is` check of a name, negated at
most once, so it reports errors for narrowing through `&&`, `||`, `while`
or double negation, and after an early `return`, which this accepts (in
[kotlin-differences.md](kotlin-differences.md#changes-we-recommend-to-temper)). A `var` isn't narrowed in either; that
needs facts that assignments and closures undo (typing.md). `is`
narrows only away from null, not to its type, which matters once classes
are typed. `panic()` doesn't yet count as always leaving.

**Tests:** `tests/settle/flow.cases`, whose inputs were also built with
Kotlin Temper: every error matches in message and position, and Kotlin
reports others only where it narrows less, as above. The functional
tests (`control-flow/bubble` needs the narrowing), std, Chromarite and
Ordable get no new errors, nor do the 652 directories of Temper sources
in Kotlin Temper's repo.

## Status: builtins' signatures, first part

`typing/signatures.rs` has the operators' variants, as Kotlin declares
them, and `console.log`'s; `typing/bubbles.rs`, what may bubble. The
rules are in [typing.md](typing.md), "Builtins' signatures" and "What
may bubble".

**Now reported**, as Kotlin does, in its words and at its positions:

| Message | Where |
|---|---|
| Actual arguments do not match signature: (Int32, Int32) -> Int32 expected [Int32, Int32], but got [Int32, String] | an operator with several variants (`+`, `-`, `*`, `/`, `%`, bitwise operators, shifts, comparisons, `<=>`, `==` and `!=`), from the first argument that doesn't fit the variant picked to the last |
| Expected subtype of Boolean, but got String | an argument of `!`, `**` or `console.log`, which have one signature each |
| Wrong number of arguments.  Expected 2 | `console.log` without exactly one argument (Kotlin counts `this`) |
| Cannot bubble from a function without Bubble in its return type | `bubble()`, a call of a function value that `throws Bubble`, or a division that may fail, in a function whose declared return type doesn't `throw Bubble` |

An operator's result now has the type of the variant picked, even if its
arguments don't fit it, so `let f(): Int { 1.0 + 1 }` also says "Cannot
assign to Int32 from Float64", as in Kotlin; `**` is always a `Float64`.
Functions' types record `throws Bubble`, written as Kotlin does:
`fn (Int32): (Int32 | Bubble)`, and with no parameters, `fn: Int32`.

**Differences from Kotlin,** where it reports more, until later slices:

- Methods of builtin types aren't typed, so neither their arguments nor
  their bubbling (`s.toInt32()`, `m[k]`) are checked; nor is `x as T`'s
  bubbling.
- An operand of a class, function, generic or `AnyValue` type isn't
  checked; Kotlin's messages name its types as `C__1` and `Fn__74<Int32>`.
- `==` and `!=` with a nullable side aren't checked: Kotlin first expands
  them to null checks, and may still report the rest.
- A block lambda, as `run { x => x / d }`, isn't checked for bubbling,
  nor is a `fn` without a return type, nor an `async` block or a
  generator; these get their types from where they're passed, which is
  target typing (slice 6).
- A divisor Kotlin folds to a constant that makes division fail, as in
  `let z = 0; … a % z`, isn't reported, nor is a branch of an `if` whose
  condition Kotlin might fold.

**Tests:** `tests/settle/builtins.cases`, whose inputs were also built
with Kotlin Temper: every error matches in message and position, and
Kotlin reports others only as above. The functional tests, std,
Chromarite and Ordable get no new errors. Of the 652 directories of
Temper sources in Kotlin Temper's repo, four get new ones, each also
reported by Kotlin there (stage tests `bubble-or-else-not`,
`unary-plus-washes-out`, `increment-in-do-block` and
`dot-operation-desugaring`). A case in `macros.cases` whose function
bubbled without `throws Bubble` now declares it.

