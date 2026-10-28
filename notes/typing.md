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

  Kotlin Temper rejects this today; this design handles it naturally,
  and tempest does, for null checks so far (milestone-4.md).
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

## Null-check narrowing: the rules

What tempest does today (`flow.rs`); the status and how it was checked
are in [milestone-4.md](milestone-4.md). A *fact* says a declaration
isn't null. A use of `a: String?` where the fact holds is a `String`.

**Eligible names:** locals, parameters, and top-level names declared
without `var`, plus the compiler's temporaries, such as `??`'s. These are
never assigned again, so a fact, once true, holds for the rest of the
scope: nothing kills it, loops need no fixed point, and closures can keep
it. A `var` is never narrowed (`var b = a; if (b != null) { take(b) }`
reports `take(b)`), as in Kotlin Temper.

**Conditions.** Each condition gives the facts where it's true and where
it's false:

- `a == null` and `null == a`: `a` isn't null where it's false.
  `a != null` and `null != a`: where it's true.
- `a is T`: `a` isn't null where it's true, if `T` isn't nullable
  (`a is String?` gives nothing). It doesn't narrow to `T`.
- `!c` swaps `c`'s two.
- `x && y` (`if (x) y else false`): `y` is checked with `x`'s true
  facts. True: `x`'s and `y`'s true facts. False: what both `x` false and
  `y` false (after `x` true) give.
- `x || y` (`if (x) true else y`): `y` is checked with `x`'s false facts.
  False: `x`'s and `y`'s false facts. True: what both `x` true and `y`
  true (after `x` false) give, so `a != null || b != null` narrows
  neither.
- `true` and `false`: the side that can't happen holds everything.
- Anything else, such as a Boolean local (`let c = a != null; if (c)`),
  gives nothing.

**Statements.** Facts flow through a block in order. `if` checks its
then-branch with the condition's true facts and its else-branch with its
false ones; after it, what holds at the end of both branches
(intersection). A branch that always leaves, by `return`, `break`,
`continue`, or `bubble()`, counts as holding everything, so after `if (a
== null) { return; }`, `a` isn't null. `panic()` doesn't count yet. Uses
after a statement that always leaves aren't narrowed. `when` and `??`
expand to `if` and `==`, so `when (a) { null -> …; else -> take(a) }`
and `a ?? ""` narrow.

**Loops and blocks.** A loop's body starts with the facts from before
it, and a `while` condition's true facts hold in the body (it's the
loop's first `if`). After a loop, only the facts from before it hold,
even after `while (a == null) { return; }`. After a block that a `break`
leaves (`label: do { if (a == null) { break label; }; … }`), only the
facts from its start hold. `orelse`'s fallback gets the facts from
before the `orelse`; after it, what both sides end with.

**Functions.** A function or class declared in a block (any `let` whose
value is a `fn`) may be called before its declaration, so its body gets only
the facts at the block's start. A function value elsewhere, as an
argument, gets the facts where it's written. So after `if (a == null) {
return; };`, `let g(): String { a }` reports `a`, but `run(fn ():
String { a })` doesn't.

**Narrower than Kotlin Temper**, which narrows only the branches of an
`if` whose condition is a null check or `is` check of a name, negated at
most once. Tempest accepts these, which Kotlin rejects:

- `&&`: `if (a != null && b != null) { take(a); take(b) }`
- double negation: `if (!(a != null)) { return; }; take(a)`
- early exits: `if (a == null || a == "") { return; }; take(a)`
- `while`: `while (a != null) { take(a); break; }`
- `bubble()`: `if (a == null) { bubble() }; take(a)`
- functions and closures after an early exit:
  `run(fn (): String { a })`

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

## `Never<T>`: bottom with a fallback

Temper has no single bottom type. `core.temper` declares
`sealed interface Never<@out T>`, and the `T` says what a backend
without a bottom type (Java, C#, C++, …) should write in its place:
`Never<String>` becomes `String` there, and a bottom type where there is
one. These are Kotlin Temper's rules (`type2/TypeContext2.kt`,
`Reconciliation.kt`, `TypeSolver.kt`, `Type2.kt`, and
`value/NullaryNeverFn.kt`):

- **Subtyping:** `Never<T>` is a subtype of `T`, and so of `T`'s
  supertypes and `AnyValue`, but not of anything else, and `T` isn't a
  subtype of `Never<T>`. `Never<Never<T>>` normalizes to `Never<T>`.
- **Nullability moves out:** `Never<T?>` normalizes to `Never<T>?`, a
  type whose only value is `null`.
- **Joins and meets unwrap:** the join (least upper bound) of `Never<A>`
  and `B` is the join of `A` and `B`, and the meet is `Never<` the meet of
  `A` and `B` `>`. So an `if` whose one branch returns and the other is a
  `String` gives `String`.
- **Calls that never return:** `panic()` and `bubble()` have two
  signatures, `(): Never<Void>` (`bubble`'s with `throws Bubble`) and
  `<T>(): Never<T>`, so `T` is inferred from what the call's place
  wants, as in `let x: Int = panic()`. Kotlin's inserted
  `voidishPanic` (a missing `else` of a `when`) works the same way.
- **Inference prefers the plain type:** a type variable with lower
  bounds `Foo` and `Never<Foo>` gets `Foo`. One with only `Never` lower
  bounds waits for other constraints, and only then becomes `Never<…>` of
  what its bounds unwrap to. Seeing `Foo <: a` for an invariant concrete
  class `Foo` also adds `a <: Never<Foo>`.
- **Void-like:** `Never<Void>` counts as `Void`, as for `Result<Void,
  …>`, when choosing between overloads by whether a result is used.
- **Errors Kotlin means to add** (TODOs there): `Never<Never<T>>`
  written out, and `<T extends Never<T>>`.

**Tempest today** has one bottom, `TypeData::Never` (`types.rs`), a
subtype of every type, used for `return`, `panic()`, and the like and
for a bare `Never`. A written `Never<String>`, like any type with
arguments so far, is unknown. Both accept everything Kotlin accepts,
since Kotlin's `Never<T>` is a subtype of fewer types, but a backend
can't get its fallback type, and a `Never<A>` used as an unrelated `B`
isn't reported. Before code generation (milestone 5), or reporting that,
`Never` should carry its `T`, with the rules above, likely alongside the
generics slice in [milestone-4.md](milestone-4.md).

**Open: is `Never` covariant?** Kotlin honors its `@out`, so there
`Never<Sub>` is a `Never<Super>`; tempest ignores `@out` (above). Only
that one relation depends on it: `Never<Sub>` is still a `Sub`, and so a
`Super`, either way. Making `Never` the one exception looks safe, since
it has no values for variance to go wrong on, and a backend's fallback
`Sub` to `Super` is an ordinary upcast. Not yet decided; if `Never`
stays invariant, code relying on it would become an error that Kotlin
doesn't report.

## Inference

- **Locals without annotations** take the type of their initializer, as
  in Kotlin Temper, so `var a = 1; a = "s"` is an error. Widening to a
  declared supertype where later assignments need it would only accept
  more programs, so it can come later if wanted.
- **Generic calls** solve their type arguments from argument types and the
  expected result type at that call site. The solver is small: unify, collect
  bounds, pick the most specific type that satisfies them.
- **Function signatures** come from annotations, which Temper requires at
  module boundaries, so libraries typecheck independently. A function
  needs a return type unless its context gives it one, as a parameter
  does for a function passed to it ("Explicit return type required").
- **In the loop:** a declaration's inferred type is a fact
  (`Fact::Type`), published once its initializer's type is known;
  declared types need no waiting. Anything not worked out is unknown and
  causes no error, so typing can grow a piece at a time. Status and
  differences from Kotlin are in [milestone-4.md](milestone-4.md).

## Builtins' signatures

What tempest does today (`typing/signatures.rs`), as Kotlin Temper does;
status in [milestone-4.md](milestone-4.md). Operators have variants, in
this order (Kotlin's `BuiltinOperatorSpecs.kt`):

- `+`, `-` and `*`: `(Int32, Int32) -> Int32`, then `Int64` and
  `Float64` likewise; `+` and `-` with one operand, `(Int32) -> Int32`
  and so on.
- `/` and `%`: the same three, each returning `Result<T, Bubble>`.
- `&`, `|` and `^`: `Int32` and `Int64`; `~`: `(Int32) -> Int32` and
  `(Int64) -> Int64`; `<<`, `>>` and `>>>`: `(Int32, Int32) -> Int32` and
  `(Int64, Int32) -> Int64`.
- `<=>`, and `<`, `<=`, `>` and `>=`, which Kotlin desugars to it:
  `Int32`, `Int64`, `Float64`, `String` and `Boolean` pairs, each
  `-> Int32`.
- `==` and `!=`: the same five pairs, each `-> Boolean`.
- One each: `!`, `(Boolean) -> Boolean`; `**`, `(Float64, Float64) ->
  Float64`; and `console.log`, `(String) -> Void`.

**Picking a variant,** as Kotlin's solver does: go through the
arguments in order, and for each, drop the variants it can't fit, last
first, until one is left. An argument fits a parameter only if its type
is the parameter's: a nullable value or `null` fits none. So `"a" + 1`
picks `Int32` (the String rules out `Float64`, then `Int64`), and `1.0 +
1` picks `Float64`. The call's type is that variant's result.

**Errors.** An operator reports "Actual arguments do not match
signature: (Int32, Int32) -> Int32 expected [Int32, Int32], but got
[Int32, String]", spanning the first argument that doesn't fit to the
last, with `null` shown as the parameter's type made nullable. One with
a single signature reports each argument that doesn't fit as a call of a
function does ("Expected subtype of Boolean, but got String");
`console.log`'s arity error says "Expected 1", where Kotlin counts
`this` and says "Expected 2".

**Checked only where every argument's type is known as Kotlin knows
it:** a final builtin type, nullable or not, or `null`. Not for a class,
function, generic or `AnyValue` operand, nor `Never`; not for a nullable
argument Kotlin might fold to a constant (`may_fold`); not for `==` or
`!=` with a nullable or `Void` side, which Kotlin expands first; and not
for a compiler temporary, as `x++` reads. Then the result's type is
unknown too.

### Operators defined in Temper

Kotlin Temper has begun marking methods that implement an operator with
`@operator("…")`, so that eventually every operator's meaning can be
specified in Temper itself rather than in the compiler. Today that's
only `==` and `<=>`: `Equatable`'s `eq` and, for example,
`StringIndexOption`'s `eq` and `compareTo` in `core.temper`, and a few
`eq` methods in std's regex. The method keeps an explicit name of its
own (`eq`, `compareTo`), following Temper's general rule that everything
has an explicit, unique name by default, which eases translation into
each language; the operator is a second way to reach it. Tempest should
read `@operator` when it types class members (left from slice 4), resolving `a ==
b` on such a class to its `eq`, and keep the table above only for
builtin types until core declares those in Temper too.

### Builtin types' members

The final builtin types' members (`Boolean`, `Int32`, `Int64`, `Float64`
and `String`) have the signatures core declares for them, in Kotlin
Temper's `core.temper` (`typing/core_members.rs`): `x.m(…)` calls an
instance's public method, `x.p` reads its field or getter, and `T.m(…)`
and `T.p` a static one, as `String.fromCodePoint(c)` and `Float64.pi`.
A method's parameter with a default is optional and typed as nullable,
as for classes' methods, and its arguments, arity and bubbling are
checked as a function's are; arity doesn't count `this`. A type in these
signatures that isn't a builtin type's name, as `List<String>`, is
unknown, as is a member core doesn't declare (`x.foo()`). `Int.m` is
`Int32.m`, as core's `Int` is `Int32`.

## What may bubble

A function whose declared return type doesn't `throw Bubble` mustn't
bubble (`typing/bubbles.rs`, Kotlin's `BubblesAreNotFree`). In its body
and parameters' defaults, each of these is "Cannot bubble from a
function without Bubble in its return type", at the call:

- `bubble()`.
- A call of a function value whose type `throws Bubble`: a parameter,
  local, function, or method, including a builtin type's (`s.toInt32()`).
  A call Kotlin might fold, as `g(1)` or `String.fromCodePoint(65)`, isn't
  reported, but Kotlin never folds a method of a value (`"1".toInt32()`
  is reported), nor a call with an argument that doesn't fit its
  parameter. A call with the wrong number of arguments isn't reported,
  as Kotlin can't pick its callee.
- `a / b` and `a % b`, unless Kotlin might fold the call, or the divisor
  is an integer that's a constant (maybe negated) other than zero, or for
  `%`, positive, or the divisor is anything Kotlin might fold (it may
  fold to such a constant). Dividing `Float64`s always may bubble.

Not looked into: the arguments of an operator they don't fit, as `h(y)`
in `1.0 + h(y)` (the operator itself, as `x / y` with `x: Int?`, still
is); an `orelse`'s body (its fallback is), functions
inside (each is checked on its own), and the branches of an `if` whose condition Kotlin might fold, except
the one a literal `true` or `false` leaves. A `for … of` body is part of
the function around it. Functions checked: those that declare a return
type, apart from generators, and constructors and setters, which return
`Void`. Module-level code and `test` bodies may bubble.

## Classes' types

What tempest does today (`typing/members.rs`), as Kotlin Temper does;
status in [milestone-4.md](milestone-4.md).

**Values.** A class or interface without type parameters is a type:
`new C(…)` is a `C`, and so is `this` in `C`'s members. A generic one
stays unknown until generics are typed. A class or interface named as a
builtin type (`class Int32` in Kotlin's `core.temper`) is that type.

**Subtyping** is nominal: a class or interface is a subtype of what it
extends, theirs and so on. A class can't be extended, so its values are
nothing else. An interface's values may be of a class that also extends
something else, and `is` checks don't narrow to types yet, so a value of
an interface where an unrelated type is wanted is only rejected for a
final builtin type. Neither is a final builtin type's value an instance
of a class, nor null a non-nullable one's. Functions and classes don't
reject each other, as a function may be a `@fun` interface's value.

**Members.** `c.x` reads a field or getter of `c`'s class, own or
inherited, and `c.m(…)` calls a method (failing that, a field's
function, as `obj.f(x)`); `C.m(…)` calls a static method and `C.x`
reads a static field. A bare member name in its class means the same.
A field's type is declared, a getter's is its return type, and a
method's is its signature, with a parameter that has a default typed
as nullable, as Kotlin does. A call of an abstract method has its
return type, but its arguments aren't checked, as its parameters aren't
kept yet. More than one method of a name (overloads) is unknown.

**Errors**, in Kotlin's words and at its positions, with classes named as
written rather than Kotlin's `C__0`:

- What a value can't be, as for any type: "Cannot assign to I from C"
  where stored, "Expected subtype of I, but got C" as an argument.
- A method's or constructor's arguments, as a function's; arity errors
  don't count `this` ("Expected 1" for one parameter, where Kotlin
  says "Expected 2").
- Assigning a member, `c.x = v`, `this.x = v` or a bare `x = v`, and a
  field's initializer: "Expected subtype of Int32, but got String" at
  the value, as for a setter's argument.
- Calling a method that bubbles, in a function that can't (a bare call
  reads `this`, so is never folded).
- **Ties:** a method a class inherits from more than one supertype, as
  `class T extends A & B` with both defining `a`, is "No callee matches
  inputs [T, Int32] among [(A, Int32) -> Int32, (B, Int32) -> Int32]",
  spanning the call, as Kotlin's `FilterOutMaskedMembers.kt` and solver
  find it: not if the class defines its own, nor for one another
  supertype overrides, nor for one further from the class than another
  with the same signature (then the nearer wins), nor if the arguments
  fit only one. Supertypes are listed depth-first, in `extends` order.
  A tie's type is unknown. Only reported where every signature and
  argument type is known.

## Checking semantics and messages

The type checker's behavior and its error messages should match Kotlin
Temper's wherever practical. Sources to compare against: the frontend stage
tests, the backend tests, the REPL tests, and running Kotlin Temper itself,
including its REPL, on the same inputs. See
[testing-and-milestones.md](testing-and-milestones.md).

**Positions and wording are worth improving.** Kotlin has been more
careful with error positions in some places than others. Match its
messages and positions by default, but where one is clearly bad (an
empty position, a span past the function's end, a message that counts a
hidden argument), point at the right place and say the right thing,
and list the change in [kotlin-differences.md](kotlin-differences.md).
Where it's only arguable, keep Kotlin's and note it there for review.
