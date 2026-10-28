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
   return type". Operators and `console.log` done (2026-10-27), and the
   final builtin types' methods, read from core (2026-10-28); status
   below. Generic ones (`ls.length`, `m[k]`) wait for slice 5.
4. **Classes:** members' types, `this`, method calls, field initializers
   and `new`, subtyping through `extends`, and the class checks left from
   milestone 3 (signatures of overrides, "No callee matches" for
   inherited ties). A function-typed field called as `obj.f(x)` types as
   a call of the field's value (Kotlin rejects it), with a case in
   `tests/settle`. First part done (2026-10-27), `@operator`,
   abstract members' parameters, overrides' signatures, and `is`
   narrowing to types (2026-10-28); status below. Left: Kotlin's errors
   for `is` and `as` checks that can't succeed or aren't allowed, and
   what waits for slice 5 or for loading core, listed there.
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
narrows to its type too since 2026-10-28 (below). `panic()` doesn't yet
count as always leaving.

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
| Wrong number of arguments.  Expected 1 | `console.log` without exactly one argument (Kotlin counts `this`, saying "Expected 2") |
| Cannot bubble from a function without Bubble in its return type | `bubble()`, a call of a function value that `throws Bubble`, or a division that may fail, in a function whose declared return type doesn't `throw Bubble` |

An operator's result now has the type of the variant picked, even if its
arguments don't fit it, so `let f(): Int { 1.0 + 1 }` also says "Cannot
assign to Int32 from Float64", as in Kotlin; `**` is always a `Float64`.
Functions' types record `throws Bubble`, written as Kotlin does,
`fn (Int32): Int32` and with no parameters `fn: Int32`, but bubbling as
Temper writes it, `fn (Int32): Int32 throws Bubble`, since 2026-10-28
(typing.md, "Checking semantics and messages"), where Kotlin writes
`fn (Int32): (Int32 | Bubble)`.

**Differences from Kotlin,** where it reports more, until later slices:

- Methods of generic builtin types aren't typed, so neither their
  arguments nor their bubbling (`m[k]`) are checked; nor is `x as T`'s
  bubbling. The final builtin types' methods are, since 2026-10-28
  (below).
- An operand of a function, generic or `AnyValue` type isn't checked;
  Kotlin's messages name its types as `Fn__74<Int32>` and `Box__9<Int32>`.
  One of a class or interface is, since 2026-10-28 (below).
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

## Status: classes, first part

`typing/members.rs` has classes' types; the rules are in
[typing.md](typing.md), "Classes' types". A class or interface without
type parameters is a type (`TypeData::Class`), with what it extends
registered for subtyping; `this`, `new C(…)`, members read and called
(`c.x`, `c.m(…)`, `C.m(…)`, bare names in the class), and member
assignments are typed and checked.

**Now reported**, as Kotlin does, in its words and at its positions,
with classes named as written (Kotlin says `C__0`):

| Message | Where |
|---|---|
| Cannot assign to I from C | a class's value where an unrelated class or interface, or a final builtin type, is wanted, at any site of the first slice; and a final builtin type's value where a class is wanted |
| Expected subtype of Int32, but got String | a method's or constructor's argument; assigning a member (`c.x = v`, `this.x = v`, bare `x = v`), at the value; a field's initializer |
| Expected subtype of String?, but got Int32 | an argument for a method's or constructor's parameter with a default, typed as nullable, as in Kotlin |
| Wrong number of arguments.  Expected 1 | a method or constructor call with too few or too many arguments; Kotlin counts `this` for methods, saying "Expected 2" |
| No callee matches inputs [T, Int32] among [(A, Int32) -> Int32, (B, Int32) -> Int32] | calling a method that a class inherits from more than one supertype (a tie) |
| Cannot bubble from a function without Bubble in its return type | a call of a method that `throws Bubble` |

`console.log()`'s arity error now says "Expected 1" too.

**Differences from Kotlin,** where it reports more, until later:

- A value of an interface where an unrelated class or interface is
  wanted (`let d: D = i`, a downcast, or `let j: J = i`) is reported
  since 2026-10-28, when `is` began narrowing to types (below).
- A function where an interface is wanted, and a class's value where a
  function is, aren't reported, since a function may be a `@fun`
  interface's value.
- Generic classes (`new Box(1)`, `bx.item`) are unknown (slice 5).
- A tie where only one method fits the arguments has an unknown type,
  where Kotlin picks that one; a tie called by a bare name in the class
  isn't reported (Kotlin: "No declaration for a").
- A member of a nullable value (`g.n` for `g: P?`) is unknown; Kotlin's
  messages for it are odd ("No type for subject of .n").

Kotlin reports `obj.f(x)` for a field `f` holding a function as
"Member f defined in C incompatible with usage"; tempest types it as a
call of the field's value ([kotlin-differences.md](kotlin-differences.md#bugs-in-kotlin-temper)).

**Left for later parts of this slice:**

- **Abstract members' parameters** and **overrides' signatures:** done
  (below).
- **`@operator`** methods for `==` and `<=>`: done (below).
- Builtin types' methods: done for the final ones (below); generic
  ones, as `ls.length` and `m[k]`, wait for generics (slice 5), and
  core's other classes, as `StringIndex`, for loading core as a library.
- `is` narrowing to a type, so interface downcasts can be reported, and
  operators with an interface's value as first operand: done (below).
- Kotlin's errors for `is` and `as` checks: "Runtime type check from
  Int32 to String can never succeed", "Unrelated types cannot be
  targeted with is or as runtime type checks: <[J]> from I", the
  `@mayDowncastTo(false)`, connected-type and type-parameter ones, and
  "Unnecessary type check" (Kotlin's `TypeFuns.kt`, `problems`). Kotlin
  also reports each but the last a second time at an empty position.

**Tests:** `tests/settle/class-types.cases`, whose inputs were also
built with Kotlin Temper: every error matches in message and position,
apart from class names, `this` in arity counts, and the differences
above. The functional tests, std, Chromarite and Ordable get no new
errors. Of the 649 directories of Temper sources in Kotlin Temper's
repo, two get new ones, each also reported by Kotlin
(`getter-setters-var-or-not` and `compound-ops-with-getter-and-setter`).

## Status: builtin types' methods

`typing/core_members.rs` reads the members of the final builtin types
(`Boolean`, `Int32`, `Int64`, `Float64` and `String`) from the vendored
`core.temper`, parsing just those classes once per process, when typing
first needs one; `typing/members.rs` types `x.m(…)`, `x.p` and static
`T.m(…)` and `T.p` with them. The rules are in [typing.md](typing.md),
"Builtin types' members". Tempest doesn't load core as a library yet;
this reads only signatures, and the interpreter still runs these methods
natively.

**Now reported**, as Kotlin does, in its words and at its positions, for
methods such as `s.toInt32()`, `x.toString()`, `x.min(y)` and `s.split(sep)`,
getters such as `s.isEmpty`, and statics such as `String.fromCodePoint(c)`
and `Float64.pi`: their results where stored ("Cannot assign to Int32
from String"), their arguments ("Expected subtype of Int32, but got
String", or `Int32?` for a parameter with a default), their arity
("Expected 1", where Kotlin counts `this`, "Expected 2"), and their
bubbling (`s.toInt32()` in a function without `throws Bubble`).

**What may bubble,** found while matching Kotlin here, for all calls:
a call with the wrong number of arguments isn't reported as bubbling,
nor is anything in the arguments of an operator they don't fit, as in
`1.0 + h(y)` (the operator itself still may be); a call with an argument
that doesn't fit is, even with constant arguments, since Kotlin doesn't
fold it; and a method of a value is never folded (`"1".toInt32()` is
reported), but a builtin type's static one may be
(`String.fromCodePoint(65)` isn't).

**Differences from Kotlin,** where it reports more, until later:

- Generic results, as `s.split(",")`'s `List<String>`, are unknown
  (slice 5), as are core's classes other than the final builtin types,
  as `StringIndex`, so `let t: String = s.end` isn't reported.
- A static call Kotlin folds and finds fails, as
  `String.fromCodePoint(0x110000)`, isn't reported as bubbling.

**Tests:** in `tests/settle/builtins.cases`, whose inputs were also
built with Kotlin Temper: every error matches in message and position,
apart from the arity count and the differences above; and a unit test of
reading core. The functional tests, std, Chromarite and Ordable get no
new errors, nor do the 723 directories of Temper sources in Kotlin
Temper's repo.

## Status: operators defined by methods

`@operator("==")` and `@operator("<=>")` mark a method
(`MEMBER_OPERATOR_EQ` and `MEMBER_OPERATOR_CMP` in `core.rs`), and `==`,
`!=`, `<=>` and the comparisons with an object as first operand call it,
in the interpreter (`class_operator` in `interp.rs`) and in typing
(`typing/signatures.rs`). The rules are in [typing.md](typing.md),
"Operators defined in Temper". Before, the interpreter compared objects
by identity (or `@auto("==")`), and `<` on objects failed ("Can't compare
these"); Kotlin's JS backend calls the methods, as tempest now does.

**Now reported**, as Kotlin does, in its words and at its positions,
with classes named as written:

| Message | Where |
|---|---|
| Actual arguments do not match signature: (H, H) -> Boolean expected [H, H], but got [H, G] | `h == g`, `h != g`, `c < 1` and the like, for a first operand of a class with such a method, own or inherited, or `@auto("==")`; at the second operand |
| Actual arguments do not match signature: (Int32, Int32) -> Boolean expected [Int32, Int32], but got [K, K] | an operator with a class's or interface's value where its builtin variants apply: `k == k` for a class without such a method, `1 - i`, `c + 1`, `-c`, a nullable class's value (`[C?, Int32]`) |
| Expected subtype of Boolean, but got C | a class's value as `!`'s or `**`'s argument |

**What may bubble** in an operator's arguments now matches Kotlin in
more cases: an argument that fits isn't skipped any more because another
doesn't (`mi(n) - "a"`), and where the first argument fits no variant,
all are looked into (`"a" - mi(n)`, `k == mk(n)`); typing.md, "What may
bubble".

**Differences from Kotlin,** where it reports more, until later:

- An interface's value as first operand (`i == i`) is checked since
  2026-10-28, when `is` began narrowing to types (below).
- A function's, generic class's or `AnyValue` operand (above).
- `when (k) { k -> … }` on a class without such a method is reported at
  the case, where Kotlin spans from the subject to the case
  ([kotlin-differences.md](kotlin-differences.md#bugs-in-kotlin-temper)).

**Tests:** in `tests/settle/class-types.cases` and `builtins.cases`, and
two cases in `run.cases` whose `==` on a class without `eq` is now
reported; inputs also built with Kotlin Temper: every error matches in
message and position, apart from class names and the differences above.
The functional tests, std, Chromarite and Ordable get no new errors, nor
do the 723 directories of Temper sources in Kotlin Temper's repo.

## Status: abstract members' parameters

A restructuring, with no change to what typing reports: an abstract or
bodiless `@connected` method's, getter's or setter's `Let` now holds a
`Fn` without a body (`FN_NO_BODY` in `core.rs`, printed as `Signature`
in trees), with its type parameters, parameters and return type, where
before it held only the return type as its type. `core::member_fn`
gives a member's `Fn` only if it has a body, for what runs it. A `@fun`
interface's `apply` keeps its parameters the same way. Names in such a
signature now resolve in its own scope, so a bodiless generic method,
as core's `map<O>(transform: fn (T): O): List<O>`, no longer reports
"O has not been declared".

## Status: abstract members' calls and overrides' signatures

A feature, on top of the restructuring above. `typing/members.rs` now
types a bodiless method from its signature, as any other, and
`typing/overrides.rs` checks overrides; the rules are in
[typing.md](typing.md), "Classes' types".

**Now reported**, as Kotlin does, in its words and at its positions,
with classes named as written:

| Message | Where |
|---|---|
| Expected subtype of Int32, but got String | an argument of an abstract or bodiless `@connected` method, or of a `@fun` interface's `apply`; assigning an abstract setter (`i.p = "s"`) |
| Wrong number of arguments.  Expected 2 | such a call with too few or too many arguments (Kotlin counts `this`, saying "Expected 3") |
| Actual arguments do not match signature: (I, I) -> Boolean expected [I, I], but got [D, H] | `d == h` for `D extends I` with `I`'s abstract `@operator("==")` method |
| Type B has method f with signature f(x: Int32): Void, but it should be f(x: String): Void to correctly override from I | a method, getter, setter or field whose signature differs from one it overrides, spanning it |

**Differences from Kotlin:**

- Kotlin checks overrides only in a module with exports, while
  generating code; tempest checks every module, as it already did for
  missing implementations and narrowed visibility (milestone 3).
- A field that overrides a getter is reported spanning the field, where
  Kotlin's position is empty, at its name.
- Generic classes, supertypes and methods aren't compared yet (slice 5),
  so Kotlin's stage test `error-message-on-bad-override`, whose
  interface is generic, gets none of its six errors.

**Tests:** in `tests/settle/class-types.cases`, whose inputs were also
built with Kotlin Temper (with an export added): every error matches in
message and position, apart from the arity count and field positions
above. The functional tests, std, Chromarite and Ordable get no new
errors, nor do the 712 directories of Temper sources scanned (Kotlin
Temper's repo, Chromarite and Ordable).

## Status: `is` narrowing to types

A feature. Flow facts (`flow.rs`) now also record, for a use that an
`is` check reaches, the type it checked, and typing (`narrowed` in
`typing.rs`) gives the use that type; the rules are in
[typing.md](typing.md), "Null-check and `is` narrowing: the rules".
With that, a class's or interface's value is a subtype only of what it
extends (`types.rs`), and an operator whose first operand is an
interface's value is checked as one of a class's (`operator_method` in
`typing/members.rs`).

**Now reported**, as Kotlin does, in its words and at its positions,
with classes named as written:

| Message | Where |
|---|---|
| Cannot assign to D from I | an interface's value where a class or interface it doesn't extend is wanted, at any site of the first slice, as a downcast `let d: D = i` that no `is` check narrows, or `let j: J = i` |
| Expected subtype of D, but got I | the same, as an argument, as `takeD(i)` in the `else` branch of `if (i is D)`, or for a `var` |
| Actual arguments do not match signature: (Int32, Int32) -> Boolean expected [Int32, Int32], but got [I, I] | `i == j` for an interface without `@operator("==")`, own or inherited |

A use that `if (i is D)` narrows is a `D`, so `takeD(i)`, `i.dm()` and
`i == d` there are typed and checked as for a `D`.

**Differences from Kotlin:**

- Narrowing reaches as far as null checks' does (typing.md), past
  `&&`, `||`, double negation and early exits, where Kotlin reports
  "Expected subtype of D, but got I".
- `d is I` for `d: D` already an `I` keeps `d` a `D`; Kotlin makes it an
  `I` ("No member dm in I"), after reporting the check as unnecessary.
- `when (i) { 4 -> … }` on an interface without `@operator("==")` is
  reported at the case, where Kotlin reports at the subject, as for
  classes (above).
- Kotlin's errors for the `is` checks themselves aren't reported yet
  (above, "Left").

**Tests:** in `tests/settle/flow.cases` ("is checks narrow to types")
and `class-types.cases` (an interface's `==` moved there from
`builtins.cases`' unchecked operators), whose inputs were also built with Kotlin
Temper: every error matches in message and position, apart from
narrowing beyond Kotlin. The functional tests, std, Chromarite and
Ordable get no new errors. Of the 722 directories of Temper sources
scanned (Kotlin Temper's repo, Chromarite and Ordable), one gets new
ones, both also reported by Kotlin (stage test `define/when-block`).
