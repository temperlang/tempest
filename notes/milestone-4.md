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
   below; and the generic ones (`ls.length`, `m[k]`), in slice 5
   (2026-10-28).
4. **Classes:** members' types, `this`, method calls, field initializers
   and `new`, subtyping through `extends`, and the class checks left from
   milestone 3 (signatures of overrides, "No callee matches" for
   inherited ties). A function-typed field called as `obj.f(x)` types as
   a call of the field's value (Kotlin rejects it), with a case in
   `tests/settle`. First part done (2026-10-27), `@operator`,
   abstract members' parameters, overrides' signatures, `is`
   narrowing to types, Kotlin's errors for `is` and `as` checks, and
   nullable targets, `x is T?` (2026-10-28); status below. Left: what waits for slice 5 or for
   loading core, listed there.
5. **Generics:** type arguments, and generic calls solved per call site,
   including `Never<T>` carrying its `T`, with Kotlin's rules
   (typing.md), in place of the plain bottom type. First step done
   (2026-10-28): generic types written out, as `List<Int>`, and type
   parameters as types; then run-time checks of generic types, calls
   of generic functions solved from their arguments and type
   arguments, generic classes and methods, with calls solved from
   the type wanted, and builtin generic types' members read from core
   (2026-10-28); and users' generic supertypes, as `class Sub<T>
   extends Sup<T>` (2026-10-29); status below, with what's left.
6. **Target typing** for object, list and function literals, which Kotlin
   Temper doesn't do yet, with trailing blocks placed while typing rather
   than at run time (design.md, "Object literals, not named arguments").
   Wanted by the user (2026-10-28). The expected type, from a declared
   variable, field, or return type, a known callee's parameter, or a
   list's element type, picks an object literal's class (an interface
   narrows the classes to search), types a list literal and its
   elements (`let xs: List<Point> = []`), and gives a function literal
   its parameter and return types. Also under this slice: generic calls
   and constructors with no arguments to solve from, such as `new Box()`
   or an empty list, take their type arguments from the expected type;
   and `null`, `bubble()`, and `panic()` take the expected type, as they
   must with no Null type. (Temper has no map literals.) Some of this is
   Kotlin Temper's intended behavior already, if not all its actual
   behavior; match what it does where it does, and note where tempest
   goes further.
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

Left from milestone 3 that needs types: `case` patterns, and string
tags that evaluate to a class other than through names. "Void
expressions cannot be used as values" (as for a `do … while` used as a
value) is done (2026-10-28); status below. Both others wait, as of
2026-10-28: `case` patterns need a design for how a subject's type
decomposes them, which Kotlin Temper hasn't either (it keeps a
`postponedCase` call for a macro no type defines), so they're for the
team first; and a string tag is told apart while macros expand, before
typing, so a tag that isn't a name, such as `Builder<String>"…"` (which
also needs generics), needs macros that wait for types (design.md,
"Macros must be able to wait for types"). No Temper sources scanned
(Kotlin Temper's repo, Chromarite and Ordable) use either.

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
is wanted isn't reported, as in Kotlin. (Since the generics slice's
first step, below, classes, type parameters and generic types given
arguments are rejected where they can't fit too.)

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

- Methods of generic builtin types weren't typed, so neither their
  arguments nor their bubbling (`m[k]`) were checked; they are since
  2026-10-28 (below, "Builtin generic types' members"), as are
  the final builtin types' methods and `x as T`'s bubbling (below).
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
- Builtin types' methods: done for the final ones and, in slice 5, the
  generic ones, as `ls.length` and `m[k]` (below); core's other
  classes, as `StringIndex`, wait for loading core as a library.
- `is` narrowing to a type, so interface downcasts can be reported, and
  operators with an interface's value as first operand: done (below).
- Kotlin's errors for `is` and `as` checks: done (below), but for
  generic and `@connected` types.

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

- Generic results, as `s.split(",")`'s `List<String>`, were unknown
  until slice 5 (below, "Builtin generic types' members"); core's
  classes other than the builtin types read, as `StringIndex`, still
  are, so `let t: String = s.end` isn't reported.
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
- Kotlin's errors for the `is` checks themselves are reported since
  2026-10-28 (below).

**Tests:** in `tests/settle/flow.cases` ("is checks narrow to types")
and `class-types.cases` (an interface's `==` moved there from
`builtins.cases`' unchecked operators), whose inputs were also built with Kotlin
Temper: every error matches in message and position, apart from
narrowing beyond Kotlin. The functional tests, std, Chromarite and
Ordable get no new errors. Of the 722 directories of Temper sources
scanned (Kotlin Temper's repo, Chromarite and Ordable), one gets new
ones, both also reported by Kotlin (stage test `define/when-block`).

## Status: `is` and `as` checks

A feature. `typing/rtti.rs` checks each `x is T` and `x as T` (and `is
T` cases of `when`) as Kotlin's `TypeFuns.kt` (`problems`) does, and
`typing/bubbles.rs` reports `x as T` that may bubble; the rules are in
[typing.md](typing.md), "Run-time type checks" and "What may bubble".

**Now reported**, as Kotlin does, in its words and at its positions,
with classes named as written:

| Message | Where |
|---|---|
| Runtime type check from Int32 to String can never succeed | between two different final types: builtin types other than `AnyValue`, and classes |
| Unrelated types cannot be targeted with is or as runtime type checks: <[J]> from I | neither type a subtype of the other, as `i is J`, `c is I` for a class `C` that doesn't extend `I`, `x is Void` for `x: AnyValue`, or a type parameter from a type that's no supertype of its bound |
| Types marked @mayDowncastTo(false) cannot be targeted with is or as runtime type checks because they may not be distinct on all backends: <[Int32]> from AnyValue | a downcast to a final builtin type |
| Type parameters cannot be targeted with is or as runtime type checks: <[T]> from AnyValue | a downcast to a type parameter |
| Unnecessary type check to I from expression with type D which is a subtype | information: `is` to a supertype, or `as` to the same type, of a non-null value |
| Expected subtype of AnyValue?, but got Void | a `Void` value checked |
| Cannot bubble from a function without Bubble in its return type | `x as T` that may fail, a downcast or of a nullable value, in a function that can't bubble |

A name Kotlin folds to a literal is checked as the literal, as Kotlin
does: `let s: AnyValue = "a"; s is String` is unnecessary, not a
downcast to `String`. And an `is` check between unrelated types no
longer narrows the value to the target type, so `if (i is J) {
takeI(i) }` reports only the check, not `takeI(i)` too.

**Differences from Kotlin:**

- Kotlin reports each error for an `if`'s `is` condition a second time,
  at an empty position at the end of the branch it narrows; tempest
  reports it once, at the check.
- `x as T`'s bubbling is reported at the check, once. Kotlin reports it
  at an empty position at `x` for a check of a nullable value (`x as
  Int` for `x: Int?`), and at both that and the check for some
  downcasts (`i as D` for `i: I?`, `x as D` for `x: AnyValue`); it also
  reports an unnecessary `x as AnyValue` for `x: AnyValue`, at an empty
  position, and a disallowed one, which tempest doesn't.
- "Unnecessary type check" isn't said where flow facts narrowed the
  value, since they narrow more than Kotlin, so `if (i is D) { i as D }`
  says nothing.
- Not yet: `Never<Int>?` and bare generic types (`List`), slice 5;
  generic types given arguments are done since (below, "Run-time checks
  of generic types"); and `@connected` types: core's aren't typed until core loads
  as a library, and Kotlin fails on a user's (`@connected("Y") interface
  Y` says "No declaration for Y").

**Tests:** in `tests/settle/class-types.cases` ("is and as checks, as
Kotlin Temper reports them", "as may bubble"), and three cases in
`run.cases` and `flow.cases` that now get these, whose inputs were also
built with Kotlin Temper: every error matches in message and position,
apart from class names and the differences above. The functional tests,
std, Chromarite and Ordable get no new errors or information. Of the 722
directories of Temper sources scanned (Kotlin Temper's repo, Chromarite
and Ordable), one gets new errors, all of them expected by Kotlin
(stage test `generate-code/invalid-rtti`, which also expects some for
generic types, not reported yet), and `invalid-rtti-not-inlined` gets
Kotlin's one piece of information.

## Status: nullable targets

A semantic fix. `x is T?` is now true for null and for a `T`, and `x as
T?` lets null through, in the interpreter (`Val::NullableType` in
`interp.rs`, where `T?` was `T`), in checks (`typing/rtti.rs`), in
bubbling and in narrowing (`flow.rs`); the rules are in
[typing.md](typing.md), "Nullable targets". Kotlin's interpreter and
generated code already do this, but its type checker drops the `?`, as
tempest's checks and interpreter did
([kotlin-differences.md](kotlin-differences.md#changes-we-recommend-to-temper)).

**Search first:** of the Temper sources in Kotlin Temper's repo
(functional tests, std, stage and other tests), Chromarite, Ordable and
tempest's own tests, only three stage tests and tempest's own cases use
`is T?` or `as T?`. `as-and-is-simplification3` and `4` expect generated
code that passes null, as tempest now does; `invalid-rtti` expects
nothing for `bon as Boolean?`, which tempest now says is unnecessary
(information). Nothing depends on the `?` being dropped.

**Changed:** "Unnecessary type check" now counts null and names the
target as written (`to String? from expression with type String`), so
`s is String?` for `s: String?` and `i as I?` for `i: I?` are reported,
and `s is String?` for `s: String` names `String?`; `x as D?` for `x:
I?` may bubble; a false `a is T?` narrows `a` to non-null. A true one
doesn't narrow `a` to `T?` yet.

**Tests:** in `tests/settle/class-types.cases` ("is and as checks with
nullable targets, which null passes"), with run output; and three
updated outputs in `class-types.cases` and `flow.cases`.

## Status: `Void` as a value

A feature, from milestone 3's leftovers. `check_not_void` in
`typing.rs` reports a `Void` value stored or passed; the rules are in
[typing.md](typing.md), "`Void` as a value".

**Now reported**, as Kotlin does, in its words and at its positions:

| Message | Where |
|---|---|
| Void expressions cannot be used as values | a local or top-level declaration's `Void` initializer (`let v = do { … } while (c)`, `let x = b()`), at it; an assignment of one to a local, at the assignment; a `Void` argument of any call, at it, unless an error is reported there already |

**Differences from Kotlin:**

- Where Kotlin reports at an empty position after a temporary it made
  (`[b()]`, `b() == b()`, `"${b()}"`), tempest reports at the
  expression.
- `b().toString()` says this at `b()`; Kotlin says "No member toString
  in Void".
- Not yet through generics: `a[0]` for `a: List<Void>`, and Kotlin's
  "Type formal <listT extends AnyValue> cannot bind to Void" for
  `[b()]` (slice 5).

**Tests:** in `tests/settle/types.cases` ("Void expressions as values,
as Kotlin Temper reports them"), whose inputs were also built with
Kotlin Temper: every error matches in message and position, apart from
the differences above; and two cases in `macros.cases` (`let v = do { 1
} while (false)`) and `run.cases` (`void == void`) that now get it. The
functional tests, std, Chromarite and Ordable get no new errors. Of the
722 directories of Temper sources scanned (Kotlin Temper's repo,
Chromarite and Ordable), only Kotlin's stage test
`generate-code/void-not-a-value` gets new ones in its sources, three of
the four it expects (the fourth, `a[0]`, needs generics), and some of
Kotlin's lowered outputs that its stage tests expect, which aren't
valid sources anyway.

## Status: generic types, first step

The generics slice's first step: `types.rs` has type parameters
(`TypeData::Param`, with their bounds) and generic types given
arguments (`TypeData::Applied`), and `typing.rs` reads them from type
expressions. The rules are in [typing.md](typing.md), "Generic types".

**Now reported**, as Kotlin does, in its words and at its positions,
apart from names (Kotlin writes `T__0` and `Box__3`, tempest `T` and
`Box`): "Cannot assign to List<String> from List<Int32>" and the like
wherever a value's type is checked, between generic types with an
unrelated pair of arguments (`List<T>` and `List<Int32>`,
`List<List<Int32>>` and `List<List<String>>`, `Box<Int32>` and
`Box<String>`), between builtin generic types that core doesn't say
extend one another (`ListBuilder<Int32>` as a `List<Int32>`,
`Listed<Int32>` as a `List<Int32>`) or that do but with unrelated
arguments (`List<Int32>` as a `Listed<String>`), between a generic type
and a final type (`Int32`, `Void`, but `String` is a `MapKey<String>`),
and for a nullable one where its non-null type is wanted; and
between a type parameter and anything but its bound and the bound's
supertypes, both ways ("Cannot assign to Int32 from T", "Cannot assign
to T from Int32", "Cannot assign to U from T", "Cannot assign to T from
T?", "Cannot assign to Dog from T" for `T extends Animal`), with null
reported as "Expected subtype of T, but got T?".

**Left for this slice,** in rough order:

- Builtin generic types' members read from core (`ls.length`, `m[k]`),
  with the type's arguments substituted: done (below, "Builtin generic
  types' members"), but for their generic methods' own type parameters
  (`ls.map(f)`). Done for users' generic classes, with generic methods
  and solving from the type wanted (below, "Generic classes and
  methods").
- Users' supertypes with arguments: `class D extends Base<Int>`, and a
  user's class implementing a builtin interface, as `Generator<T>`:
  done (below, "Users' generic supertypes"), but for overrides'
  signatures in generic classes, and a builtin interface's members
  inherited by a user's class.
- Variance: Kotlin honors `@out` and `@in`, and so reports `Box<Dog>` as
  a `Box<Animal>` for an undecorated `class Box<T>`, and `List<Int32?>`
  as a `List<Int32>`. Tempest keeps parameters invariant, but reports
  only pairs no variance accepts until that's settled; the parser
  doesn't keep type parameters' decorations yet.
- Function types compared part by part (Kotlin reports `fn (Int32):
  List<Int32>` as a `fn (Int32): List<String>`).
- `Never<T>` carrying its `T` (typing.md), list literals' types, and
  Kotlin's "Type formal <listT extends AnyValue> cannot bind to Void" for
  `[b()]`; bounds that are type parameters (`<T, U extends T>`).
- Run-time checks that need users' supertypes with arguments, as Kotlin's
  stage test `generate-code/invalid-rtti-type-args` has (below, "Run-time
  checks of generic types"): done (below, "Users' generic supertypes").

**Tests:** in `tests/settle/types.cases` ("generic types and type
parameters, as Kotlin Temper reports them"), whose input was also built
with Kotlin Temper: every error matches in message and position, apart
from names and what's left above; and a unit test of subtyping in
`types.rs`. The functional tests, std, Chromarite and Ordable get no new
errors. Of the 722 directories of Temper sources scanned (Kotlin
Temper's repo, Chromarite and Ordable), one gets a new error, which
Kotlin also reports: stage test `syntax-macro/generic-function-in-docs`,
`let f<T, U extends T>(x: T): U { x }`, which only tests an earlier
stage.

## Status: run-time checks of generic types

A feature. `typing/rtti.rs` now checks `x is T` and `x as T` where
either type is generic; the rules are in [typing.md](typing.md),
"Run-time type checks".

**Decided:** a type parameter's value is judged by its bound, all that's
known of it at run time, so `x is Dog` for `x: T`, `T extends Animal`,
is a downcast, allowed, and narrows `x` to `Dog`, as before. Kotlin
says the types are unrelated, which reads as a bug
([kotlin-differences.md](kotlin-differences.md#bugs-in-kotlin-temper)):
the check is on the value's class, which erasure doesn't touch, and it
works as `a is Dog` for `a: Animal` does. So `x is Int` for an unbounded
`T` is the `@mayDowncastTo(false)` error, "from T", where Kotlin says
unrelated. What erasure does stop is checking a type argument, so those
errors now say so.

**Now reported**, as Kotlin does, at its positions, with classes named
as written:

| Message | Where |
|---|---|
| Type arguments cannot be introduced with is or as runtime type checks, because a type argument's run-time type can't be checked: <[Box<Int32>]> from AnyValue | a downcast to a generic type with an argument the checked type doesn't have; not for a parameter marked `@out` or `@in`, as Kotlin |
| Types marked @mayDowncastTo(false) cannot be targeted … <[List<Int32>]> from Listed<Int32> | a downcast to `List`, `ListBuilder`, `Map` or `MapBuilder`, or from a type parameter's value to a final builtin type |
| Runtime type check from String to Box<String> can never succeed | between generic types of different classes, or one and a final type |
| Unrelated types cannot be targeted … <[Box<String>]> from Box<Int32> | generic types whose arguments set them apart; `String` from `MapKey<Int32>` |
| Cannot bubble from a function without Bubble in its return type | `x as Box<Int>`, and `x as Dog` for `x: T`, that may fail |
| Unnecessary type check to Box<T> from expression with type Box<T> which is a subtype | information, as before, now for generic types and `x is T` for `x: T` |

"Type parameters cannot be targeted …" also gains ", because a type
argument's run-time type can't be checked", as the erasure error does;
Kotlin's wording ends at "checks" for both
([kotlin-differences.md](kotlin-differences.md#changes-we-recommend-to-temper)).
The parser now keeps `@out` and `@in` on type parameters
(`TYPE_PARAM_VARIANT` in `core.rs`) for this, and `types.rs` knows which
of core's generic types are classes, which are `@mayDowncastTo(false)`,
and which parameters core marks `@out` or `@in`, checked against core
by a unit test. `List<Int32>` is now known to be a `Listed<Int32>`, and
`String` a `MapKey<String>`.

**Not yet:** `Never<T>`'s checks (`invalid-rtti`'s "from Null to
Int32"). Checks that need users' supertypes with arguments, as
`Sub<String>` from `Sup<T>` for `class Sub<T> extends Sup<T>` (four of
the five errors Kotlin's stage test `generate-code/invalid-rtti-type-args`
expects), came later (below, "Users' generic supertypes").

**Tests:** in `tests/settle/class-types.cases` ("is and as checks of
type parameters' values and generic types"), whose input was also built
with Kotlin Temper: every error matches in message and position, apart
from names, the decision above, the added reason, and the known
differences in "`is` and `as` checks"; and a unit test in `types.rs`.
The functional tests, std, Chromarite and Ordable get no new errors. Of
the 722 directories of Temper sources scanned (Kotlin Temper's repo,
Chromarite and Ordable), two get new errors, all of them expected by
Kotlin: `invalid-rtti` now gets all four of its `MapKey<Int32>` ones,
and `invalid-rtti-type-args` the first of its five.

## Status: generic calls solved per call

A feature, the generics slice's next step. `typing/generic_calls.rs`
solves each call of a generic function declared by `let` from its type
arguments, `id<Int>(x)`, or else its arguments' types (`unify` in
`types.rs`), so the call's type is the result with the bindings put in,
and checks the bindings and arguments as Kotlin's solver does. The rules
are in [typing.md](typing.md), "Generic calls".

**Now reported**, as Kotlin does, in its words and at its positions,
apart from names (Kotlin writes `T__0`, tempest `T`):

| Message | Where |
|---|---|
| Cannot assign to String from Int32 | wherever a solved call's result is checked, as `let c: String = id(1)` |
| Type formal <T extends Animal> cannot bind to Int32 which does not fit upper bounds [Animal] | a binding outside its bound, at the call |
| No callee matches inputs [Int32] among [<T extends AnyValue>(T, Int32) -> T] | a generic call's wrong number of arguments, at the call, where tempest said "Wrong number of arguments.  Expected 2" |
| Actual arguments do not match signature: <T extends AnyValue>(T, Int32) -> T expected [String, Int32], but got [Int32, String] | arguments that don't fit, from the first to the last, where tempest said "Expected subtype of Int32, but got String" for a parameter that isn't generic |

**Differences from Kotlin:**

- `let xs: List<String> = wrap(1)`, for `let wrap<T>(x: T): List<T>`, is
  "Cannot assign to List<String> from List<Int32>" at the initializer,
  where Kotlin solves from the type wanted first and reports the
  argument: "Actual arguments do not match signature: … expected
  [String], but got [Int32]". Since matched (below, "Generic classes
  and methods").
- Kotlin also says "Expected function type, but got Invalid" at the
  callee of a call with type arguments whose arguments don't fit, as
  `id<String>(n)`; tempest doesn't.
- Not reported yet: Kotlin's "Expected subtype of AnyValue, but got
  AnyValue?" for `id(null)`, and what a function literal argument would
  bind, as `ap(1, fn (x: Int): Int { x })`'s result.

**Not yet:** builtin generic methods (`ls.map(f)`), whose own type
parameters core's signatures leave unknown so far; bounds naming the function's own type
parameters. Solving from the type wanted, generic methods, and generic
classes' constructors and members came next (below).

**Tests:** in `tests/settle/types.cases` ("generic calls solved per
call, as Kotlin Temper reports them"), whose input was also built with
Kotlin Temper: every error matches in message and position, apart from
names and the differences above; and the earlier generic types case,
whose `let c1: String = id(1)` is now reported, as Kotlin does. The
functional tests, std, Chromarite and Ordable get no new errors, nor do
any of the 722 directories of Temper sources scanned (Kotlin Temper's
repo, Chromarite and Ordable).

## Status: generic classes and methods

A feature, the generics slice's next step: generic classes' values and
members, generic methods, and generic calls solved from the type their
place wants (`typing/members.rs`, `typing/generic_calls.rs`). The rules
are in [typing.md](typing.md), "Generic types" and "Generic calls".

**Now reported**, as Kotlin does, in its words and at its positions,
apart from names (Kotlin writes `T__0` and `Box__2`, tempest `T` and
`Box`):

| Message | Where |
|---|---|
| Actual arguments do not match signature: <T extends AnyValue>(T) -> Box<T> expected [String], but got [Int32] | `new Box<String>(1)`'s arguments, as a generic function's |
| Cannot assign to String from Int32 | wherever a generic class's member's type is checked: `let s: String = bx.item`, `bx.get()`, for `bx: Box<Int>` |
| Expected subtype of Int32, but got String | `bx.put("x")`, and `bx.count = "y"` for a `var` field |
| Type formal <T extends Animal> cannot bind to Int32 …, No callee matches inputs [Order, Int32] among [<T extends AnyValue>(Order, T, Int32) -> T], and Actual arguments do not match signature: <U extends AnyValue>(Box<Int32>, U, Int32) -> U expected [Box<Int32>, Int32, Int32], but got [Box<Int32>, Int32, String] | generic methods' calls, as a generic function's, with an instance's receiver shown first |
| Actual arguments do not match signature: <T extends AnyValue>(T) -> List<T> expected [String], but got [Int32] | `let xs: List<String> = wrap(1)`, at the argument, where tempest said "Cannot assign to List<String> from List<Int32>"; likewise a function's result, `return`, assignment, and `new Box(1)` where a `Box<String>` is wanted |

**Differences from Kotlin:**

- A method's explicit type arguments, `o.first<String>(ss)`, bind as
  written; Kotlin fails ("Member first defined in Order incompatible
  with usage", "Expected function type, but got Invalid"), a bug
  ([kotlin-differences.md](kotlin-differences.md#bugs-in-kotlin-temper)).
- A generic call that's an argument, `takeLS(wrap(1))` for `takeLS(xs:
  List<String>)`, is "Expected subtype of List<String>, but got
  List<Int32>"; Kotlin reports nothing, a bug (same place).
- `bx.item = "y"` for a field that isn't `var` is "Expected subtype of
  Int32, but got String" at the value, where Kotlin says "Member item
  defined in Box incompatible with usage" at the assignment; tempest
  reports that only for `this.x` and bare names, for non-generic
  classes too, so far.
- `let b: Box<Animal> = new Box(new Dog())` passes, as before, where
  Kotlin, with a user's class's parameters invariant unless marked
  `@out` or `@in`, reports "Cannot assign to Box<Animal> from Box<Dog>"
  for a `k: Box<Dog>`. Tempest could report that too now that calls
  are solved from the type wanted, but it's left for a step of its own.
- Kotlin's "Expected function type, but got Invalid", at a static
  generic method's call solved from the type wanted, as `let e:
  List<String> = Order.make(1)`, isn't reported.

**Not yet:** members inherited from a generic supertype, whose type
parameters are unknown as users' supertypes with arguments aren't typed
yet (since done, below, "Users' generic supertypes"); and builtin
generic types' members (`ls.length`), done next (below).

**Tests:** in `tests/settle/types.cases` ("generic classes and methods"
and "generic calls solved from the type wanted", as Kotlin Temper
reports them), whose input was also built with Kotlin Temper: every
error matches in message and position, apart from names and the
differences above; the earlier generic calls case, whose `let c5:
List<String> = wrap(1)` is now reported at the argument, as Kotlin does;
and `macros.cases`, whose `c is C<Int, I<Int>>` now gets Kotlin's
"Unnecessary type check" information. The functional tests, std,
Chromarite and Ordable get no new errors, nor do any of the 722
directories of Temper sources scanned (Kotlin Temper's repo, Chromarite
and Ordable).

## Status: builtin generic types' members

A feature, the generics slice's next step. `typing/core_members.rs` now
reads the builtin generic types' declarations from core too (`List`,
`ListBuilder`, `Listed`, `Map`, `MapBuilder`, `Mapped`, `Pair`, `Deque`,
the generator types, `Promise` and `PromiseBuilder`), their signatures
as types that name the type's own type parameters, and
`typing/members.rs` types `x.m(…)` and `x.p` for a value of one with
the receiver's type arguments put in, looking up an inherited member in
`Listed` or `Mapped`. `a[i]` is typed and checked as `a.get(i)`, for a
class's value too, as Kotlin desugars it. The rules are in
[typing.md](typing.md), "Builtin types' members".

**Now reported**, as Kotlin does, in its words and at its positions:
results where stored ("Cannot assign to String from Int32" for `ls[0]`,
`ls.length`, `m.get(k)`; "Cannot assign to List<Int32> from
List<String>" for `s.split(",")`, which was unknown); arguments
("Expected subtype of String, but got Int32" for `lb.add(1)`, `lb[0] =
2`, `m[1]`, `mb["k"] = "v"`; "Expected subtype of Int32?, but got
String" for a parameter with a default, `lb.add("x", "y")`); and
bubbling, "Cannot bubble from a function without Bubble in its return
type", for `m[k]`, `m.get(k)`, `mb.remove(k)`, and a generator's
`g.next()`.

**Differences from Kotlin:**

- A builtin generic method's own type parameters are unknown, so
  `ls.map(f)` is a `List<?>`, where Kotlin solves `O`, from the type
  wanted too ("Actual arguments do not match signature: <O extends
  AnyValue>(Listed<Int32>, fn (Int32): O) -> List<O> …").
- Function types aren't compared part by part yet, so `ls.filter(f)` for
  an `f: fn (String): Boolean` isn't reported (Kotlin: "Expected subtype
  of fn (Int32): Boolean, but got fn (String): Boolean").
- `List.of<String>("a")` (core's `of = builtins._listify`) is unknown.
- A nullable receiver's members are unknown, as for classes; Kotlin says
  "No member length in AnyValue" for `xs.length`, `xs: List<Int>?`.
- `let b: Listed<Animal> = lb` for `lb: ListBuilder<Dog>` passes, as
  before; Kotlin reports it, as `ListBuilder`'s `T` isn't `@out`
  (variance, "Left for this slice", above).

**Tests:** in `tests/settle/builtins.cases` ("generic builtin types'
members have core's signatures"), whose input was also built with
Kotlin Temper: every error matches in message and position, apart from
the differences above; the earlier builtin methods case, whose `let b6:
String = s.split(",")` is now reported, as Kotlin does; and two cases
in `run.cases` whose generators' `next()` and `as ValueResult<Int>` now
bubble where they can't, as Kotlin reports (checked); and unit tests of
reading core's signatures. The functional tests, std, Chromarite and
Ordable get no new errors. Of the 722 directories of Temper sources
scanned (Kotlin Temper's repo, Chromarite and Ordable), two get new
ones, each also reported by Kotlin: stage test
`generate-code/bubble-or-else-not`, which now gets all three of the
errors it expects (`nums[index]` for a `Map`), and
`generate-code/generator-results-used`, whose `generator.next()` in a
`Void` function Kotlin also reports, though that stage test only
checks a later stage's output.

## Status: users' generic supertypes

A feature, the generics slice's next step: what a user's class or
interface extends, with type arguments, as `class Sub<T> extends
Sup<T>` or `class Sub4 extends Sup<String>`, is known to subtyping
(`ClassInfo` and `supertype_as` in `types.rs`, registered by
`class_symbol` in `typing/members.rs`). A value is compared with a
generic type as its supertype with that type's base sees it, with its
own type arguments put in: a `Sub2<Int>`, for `class Sub2<T> extends
Sup2<T, String>` and `interface Sup2<T, U> extends Sup<U>`, is a
`Sup2<Int, String>` and a `Sup<String>`. The same view solves generic
calls (`unwrap(sh)` binds `T` to `String` for `unwrap<T>(x:
Holder<T>)` and `class StrHolder extends Holder<String>`), types
inherited members (`sh.describe(x: T)` takes a `String`), and judges
run-time checks. The rules are in [typing.md](typing.md), "Generic
types". `Types::is_subtype` and `unify` now take `&mut self`, as
seeing a supertype may make a new type.

**Now reported**, as Kotlin does, in its words and at its positions,
apart from names (Kotlin writes `Sup__0` and `T__12`, tempest `Sup` and
`T`):

| Message | Where |
|---|---|
| Cannot assign to Sup<Int32> from Sub<String> | a value of a user's class or interface where a supertype it extends with other arguments, or one it doesn't extend, is wanted, at any site of the first slice (`Expected subtype of Sup<Int32>, but got Sub4` as an argument) |
| Cannot assign to Int32 from String, Expected subtype of String, but got Int32 | a member inherited from a generic supertype, read, called, or given an argument, with the receiver's arguments put in |
| Actual arguments do not match signature: <T extends AnyValue>(Holder<T>) -> List<T> expected [Holder<Int32>], but got [StrHolder] | a generic call solved from the type wanted whose argument's supertype doesn't fit |
| Unrelated types cannot be targeted with is or as runtime type checks: <[Sub4]> from Sup<T>, and "Type arguments cannot be introduced …: <[Sup2<T, U>]> from Sup<U>" | run-time checks between a user's generic types, all five of Kotlin's stage test `generate-code/invalid-rtti-type-args` |

**Differences from Kotlin:**

- Kotlin can't bind a type parameter from a non-generic class's generic
  supertype, so it rejects `unwrap(sh)` for `sh: StrHolder` ("… expected
  [Holder<AnyValue>], but got [StrHolder]"), and for `let n: Int =
  unwrap(sh)` reports the argument; tempest binds `T` to `String` and
  reports "Cannot assign to Int32 from String" at the initializer, as
  for a `GenHolder<String>`, which Kotlin binds
  ([kotlin-differences.md](kotlin-differences.md#bugs-in-kotlin-temper)).
- A type error about a use an `is` check narrows, as `let y: Sup<Int> =
  s` in `if (s is Sub<String>) { … }`, is at the use, where Kotlin
  reports it at an empty position at the end of the branch (same
  place).
- As before, a pair of arguments that only variance tells apart, as
  `Sub<Dog>` where a `Sup<Animal>` is wanted, isn't reported (typing.md,
  "Type parameters are invariant"). A scan with such pairs reported
  found no Temper sources that rely on it.

**Not yet:** overrides' signatures in generic classes or against
generic supertypes, as `class BooleanJsonAdapter extends
JsonAdapter<Boolean>`'s methods (`typing/overrides.rs` skips both); a
builtin interface's members inherited by a user's class; and Kotlin's
argument error for a bare `T` result solved from the type wanted where
no argument binds `T`, as `let n: Int = firstOf(lb)` for `firstOf<T>(x:
List<T>): T` and `lb: ListBuilder<String>` ("… expected [List<Int32>],
but got [ListBuilder<String>]"), which tempest leaves unreported.

**Tests:** in `tests/settle/types.cases` ("users' generic supertypes")
and `class-types.cases` ("is and as checks through users' generic
supertypes", Kotlin's `invalid-rtti-type-args`), whose inputs were also
built with Kotlin Temper: every error matches in message and position,
apart from names and the differences above; and a unit test in
`types.rs`. The functional tests, std, Chromarite, and Ordable get no new
errors. Of the 722 directories of Temper sources scanned (Kotlin
Temper's repo, Chromarite, and Ordable), only
`generate-code/invalid-rtti-type-args` gets new ones, now all five it
expects.
