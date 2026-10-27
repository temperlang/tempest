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
3. **Builtins' signatures:** operators, `console.log`, and the builtin
   types' methods, with Kotlin's "Actual arguments do not match
   signature" and "Cannot bubble from a function without Bubble in its
   return type".
4. **Classes:** members' types, `this`, method calls, field initializers
   and `new`, subtyping through `extends`, and the class checks left from
   milestone 3 (signatures of overrides, "No callee matches" for
   inherited ties).
5. **Generics:** type arguments, and generic calls solved per call site.
6. **Target typing** for object, list and function literals, which Kotlin
   Temper doesn't do yet, with trailing blocks placed while typing rather
   than at run time.
7. **Every expression typed,** as tmpl and backends need.

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
any.

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
  (`let f(a: String?): String { a }`) isn't reported, since without flow
  facts a null check wouldn't narrow it.
- A `Void` result where another type is wanted isn't reported. Kotlin
  reports "Cannot assign to Int32 from Void", past the function's end
  when the last statement is a `let`.
- A bare `return` in a non-`Void` function is reported at the `return`;
  Kotlin's position is past the function's end.
- Types left unknown by a cycle aren't reported; Kotlin says "Cannot
  assign to String from Top".
- Parameters' defaults aren't checked yet.
- Kotlin folds `let` constants before checking, so its messages may name
  the value's type rather than the declared one (a Kotlin
  fix to make); ours use the declared type.

`x++`'s result now spans `x`, where Kotlin reports a type error about it.

**Tests:** `tests/settle/types.cases`, whose inputs were also built with
Kotlin Temper (`temper build`): every error matches in message and
position, apart from the differences above; and a unit test of subtyping
in `types.rs`. The fuzz test (`mutated_inputs_settle`) covers typing too.
