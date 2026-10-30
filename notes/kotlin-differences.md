# Differences from Kotlin Temper

Part of the [rewrite plan](README.md). Where tempest deliberately differs
from Kotlin Temper as it is today, and the bugs we know of in Kotlin
Temper, with what tempest does about each. Details live in the linked
notes. The parser's differences are listed in full in
[testing-and-milestones.md](testing-and-milestones.md) ("Deliberate
deviations from Kotlin" and "Kotlin bugs found"), so only the main ones
are here. Update this when either side changes.

## How far to differ

- **Stay compatible** with Kotlin Temper's tests, the functional tests
  above all, wherever at all possible.
- **Do obvious improvements** that stay compatible, such as accepting
  more of what people expect (flow typing that narrows through `&&`, or
  after an early `return`) and clearer errors where Kotlin crashes.
- **Error positions and wording:** match Kotlin's by default, but fix
  clearly bad ones (empty positions, spans in the wrong place, messages
  that count hidden arguments), and note arguable ones here for review
  ([typing.md](typing.md), "Checking semantics and messages").
- **Nothing too exciting without the Temper team:** a new rule, error,
  or language feature is proposed here first, as a recommended change.
- **Backend translation** may differ in ways that make libraries
  incompatible only for really strong reasons, recommended strongly as a
  change to Kotlin Temper's backends before Temper has many users, and
  maybe still with a compatibility mode. Its main use: running existing
  callers of Temper libraries unchanged, such as Chromarite's and
  Ordable's tours, to prove tempest is a Temper
  ([testing-and-milestones.md](testing-and-milestones.md), "Library
  tours"). Libraries update to an improved translation only once the
  team adopts it, never just to work with tempest.

## Changes we recommend to Temper

Language and frontend:

- **Target typing** for object, list and function literals: the expected
  type picks an object literal's class, ahead of matching constructor
  names. Planned, milestone 4 slice 6 (design.md, "Object literals, not
  named arguments"). Begun with list literals: `[d]` where a
  `List<Animal>` is wanted is a `List<Animal>`, and `[]` where a
  `List<Int>` is, a `List<Int32>` ([typing.md](typing.md), "List
  literals and target typing"). Function literals' parameters and
  results are typed from the type wanted as Kotlin types them, with
  trailing blocks placed while typing ([typing.md](typing.md),
  "Function literals and target typing"); a literal's result should
  also bind a generic callee's type parameter, which Kotlin binds to its
  bound, so `ls.map { x => x * 2 }` is a `List<AnyValue>` there. Not yet
  in tempest either, which leaves it unknown. Object literals take their
  class from the type wanted (2026-10-29; [typing.md](typing.md),
  "Object literals and target typing"): `let a: A = { x: 1 }` makes an
  `A` where Kotlin says "Multiple types have matching constructors: A,
  B", and where the names fit only another class, tempest says "A is
  wanted here, but these properties fit Mark's constructor" where Kotlin
  makes a `Mark` and says "Cannot assign to A from Mark", at the same
  position. Kotlin also reports "Void expressions cannot be used as
  values" after "No signature matches" for an object literal no class
  fits, a follow-on error tempest doesn't report.
- **Named `@fun` interfaces are nominal** (decided 2026-10-29, done;
  [typing.md](typing.md), "Fun interfaces"): a `Pred` isn't a `Check`,
  even where their `apply`s' signatures match, and the error suggests
  the adapter: "A Pred isn't a Check; to adapt it, wrap it: `fn (y) {
  p(y) }`". Kotlin accepts it, as it compares them as function types.
  Function literals and references to declared functions and methods
  take the interface wanted, so no adapter is needed there; bare
  function types stay structural. A function literal that initializes a
  `let` without a type is an error too ("f needs a declared type, such
  as a @fun interface, …"), which Kotlin accepts. And Kotlin's
  documented rules for declaring one (only `AnyValue` as a supertype,
  one `apply`, no other members, no `@json`, no `is` or `as`) are
  checked, where Kotlin checks none but `apply`'s type parameters.
- **Null and `is` narrowing from control flow,** through `&&`, `||`,
  `!`, `while`, and after an early `return`, `break`, `continue` or
  `bubble()`; Kotlin narrows only the branches of an `if` on one check.
  Done for names that aren't `var` ([typing.md](typing.md), "Null-check
  and `is` narrowing: the rules"). Also, `d is I` for a `d` already an
  `I` keeps its more precise type, where Kotlin narrows it to `I`.
- **Type errors name declared types,** not the types of folded constants
  (Kotlin's `let s: String? = null; let q: Int = s` says "got Int32?",
  and a name it folds to a function, `opt`, gets `opt`'s type, "from fn
  (Int32, _?: Int32): Int32", where the name's is `fn (Int32, Int32):
  Int32`). Done ([milestone-4.md](milestone-4.md), first slice, and
  "Function types compared part by part").
- **A function literal's result reported at the result,** "Expected
  subtype of Int32, but got String" for `{ x => let y = x; "a" }` where
  a `fn (Int32): Int32` is wanted at the `"a"`, where Kotlin's position
  is empty, at the start of the body (here, the `let`); likewise a
  value it returns, at the value, each `return` on its own, where
  Kotlin's position is empty, just after the `return`, and its type is
  all the literal's results joined. Done ([typing.md](typing.md),
  "Function literals and target typing").
- **One type representation** (`types.rs`), where Kotlin has `type/` and
  `type2/` bridged by `hackMapOldStyleToNew`. Done.
- **Core's generic interface methods** (`Listed.map<O>`, `reduceFrom<O>`,
  `reduceFromIndex<O>`, `Mapped.toListWith<T>`) as `@extension`
  functions, so core needs no exemption from "overridable methods don't
  allow generics". Planned ([typing.md](typing.md), "To explore later").
- **Generic overrides matched by position.** Kotlin compares methods'
  type parameters by identity, so an override of a generic method, as
  `h<U>(x: U): U` for an interface's `h<T>(x: T): T`, always differs
  ("… h(x: U__5): U__5, but it should be h(x: T__1): T__1 …"), on top of
  "Overridable methods don't allow generics" for the interface's method.
  Tempest pairs them by position and reports only real differences. Done
  ([typing.md](typing.md), "Classes' types").
- **Generators in any function form** (named functions, `fn` literals,
  methods, not only block lambdas), and `return` ends a generator, where
  Kotlin fails typing and panics. Done ([milestone-3.md](milestone-3.md),
  flat frames, step 2).
- **Private members of other instances** of the same class are usable in
  its methods, as a Kotlin Temper PR allows; Kotlin today says only
  "Cannot translate Invalid" and emits code that fails. Done, checked at
  run time ([milestone-3.md](milestone-3.md), "Classes").
- **`yield` and `await` in `for … of` bodies.** Kotlin lowers a body to a
  `forEach` lambda, so a `yield` there fails at run time. Idea: iterables
  provide a generator or iterator, so `yield`, `await`, `break` and
  `return` just work, backends still map lists to native loops, and
  `forEach` stays an optional fast path; costs an iterator object where
  there's no native mapping. Tempest says "Not supported yet" for now
  ([milestone-3.md](milestone-3.md), flat frames).
- **Top-level statements in source order.** Kotlin's `sortTopLevels`
  makes every reader of a top-level `var` wait for each statement that
  assigns it, even `n++` inside a log, so output can scramble. This is
  outright broken, so Temper must settle it. Options: reorder only
  declarations, keeping statements in source order; or require top-level
  side effects, such as assigning a top-level `var`, to be in a top-level
  `do { … }` block, as the team has discussed but not enforced (stricter,
  so maybe unfriendly, though the REPL could wrap input for people).
  Tempest matches Kotlin for now, pending agreement
  ([milestone-3.md](milestone-3.md), "Top-level order").
- **NFKC for every non-ASCII word.** Kotlin checks only words with a code
  point past Latin-1, so `ªb` and `ab` are different names. Tempest
  matches for now (testing-and-milestones.md, "NFKC, as Kotlin").
- **Types that may bubble written as in Temper's syntax,** `Int32
  throws Bubble`, in every message. Kotlin writes its internal types,
  `Result<Int32, Bubble>` in operators' and overrides' signatures and
  `fn (Int32): (Int32 | Bubble)` in function types, though Temper has no
  `Result` type or `|` in its syntax. Done ([typing.md](typing.md),
  "Checking semantics and messages").
- **`x is T?` passes null in type checks too.** Kotlin's interpreter
  and generated code already make `null is T?` true and let `null as
  T?` through, but its type checker drops the `?`: `s is String?` for
  `s: String` is "Unnecessary type check to String", `s is String?` for
  `s: String?` (always true) and `bon as Boolean?` for `bon: Boolean?`
  aren't reported, `x as D?` for `x: D` is, and a false `a is T?` doesn't
  narrow `a` to non-null. Tempest's checks count null, naming the target
  as written ("Unnecessary type check to String? from expression with
  type String?"), and its interpreter, which dropped the `?` too, now
  passes null. Kotlin's stage test `generate-code/invalid-rtti` expects
  nothing for its `bon as Boolean?`, which its comment calls a no-op;
  tempest says it's unnecessary. Done ([typing.md](typing.md),
  "Nullable targets").
- **Say why erased checks fail:** "Type arguments cannot be introduced
  with is or as runtime type checks" (`x is Box<Int>` from `AnyValue`)
  and "Type parameters cannot be targeted …" (`x is T`) end ", because
  a type argument's run-time type can't be checked" in tempest, before
  the types. Done ([typing.md](typing.md), "Run-time type checks").
- **Float64 division by a nonzero constant** (`x / 2.0`) can't fail, so
  it needn't count as bubbling, as `Int` division by one doesn't. Tempest
  matches Kotlin for now ([milestone-4.md](milestone-4.md), builtins'
  signatures).
- **An `internal` visibility** (module or library) between `public` and
  `private`, so module functions can use a class's hidden parts. Design it
  from what JS can express (only public and `#private`). Not planned yet.

Literate source:

- **Code in markdown block quotes is prose.** It can't work past one line
  in Kotlin anyway. Done.
- **Multi-quoted strings** follow clearer rules of our own. Done
  (testing-and-milestones.md, "Multi-quoted strings").
- **A warning for an indented code block that continues a list item.**
  CommonMark makes it prose, so code silently disappears (seen: a class
  lost its first lines). Not planned yet.

Backends, planned: no numbered names where visible (design.md, "Naming";
api-compat.md proposal 1); no required `init` in Rust and C++
(api-compat.md proposal 2); fun interfaces as named function types
(design.md, "Function types and fun interfaces"); tmpl independent of the
frontend (milestone-3.md, "tmpl: a separate schema crate"); value ranges
so Python and Lua skip 32-bit wrapping helpers ([typing.md](typing.md),
"What flow facts are used for").
- **Shared backend tests.** Kotlin's backend tests are separate per
  backend, so most inputs are checked on only some backends. One shared
  set of inputs with an expected output per backend, plus golden files of
  generated code for the functional tests, would catch more; tempest
  plans it that way ([testing-and-milestones.md](testing-and-milestones.md),
  "Backend tests, shared from the start").

## Bugs in Kotlin Temper

### Language and types

- **`x is D` for a type parameter's value,** `x: T` with `T extends
  Animal` and `D` a class extending `Animal`, is reported as between
  unrelated types, where `a is D` for `a: Animal` is a downcast. Tempest
  judges a type parameter's value by its bound, all that's known of it
  at run time, so this is a downcast, allowed, narrowing `x` to `D`; and
  `x is Int` for an unbounded `T` is the `@mayDowncastTo(false)` error,
  "from T", where Kotlin says unrelated ([typing.md](typing.md),
  "Run-time type checks").

- **A function in a field, `obj.f(x)`,** fails ("incompatible with
  usage"); Kotlin's own `functions/constructor-callbacks` keeps its use
  inert. Tempest runs it (`call_method` in `interp.rs` falls back to the
  property) and types it as a call of the field's value
  (`tests/settle/class-types.cases`).
- **A trailing block placed past a skipped optional parameter** crashes
  Kotlin's typer (a `NullPointerException` in `Typer.typeCallByParts2`,
  on the null put in for the skipped parameter): `h(1) { … }`, for `let
  h(n: Int, s: String = "d", g: (fn (): Void)? = null)`, a method's
  too. Tempest passes the block as `g`, and `s` takes its default
  ([typing.md](typing.md), "Function literals and target typing";
  `tests/settle/run.cases`).
- **An empty body of a non-`Void` function,** `let f(): Int { }`, isn't
  reported, and calls then fail at run time. Tempest doesn't report it
  yet either (`typing.rs` skips `Void` results).
- **`assert(list == null)`** with a nullable list fails with "Cannot
  assign to Invalid from String". Tempest's `assert` runs it; not yet
  tested with types.
- **An interface's backed property** isn't reported, then fails at run
  time. Tempest reports it ([milestone-3.md](milestone-3.md), "Classes").
- **A missing setter** is hidden by an implemented getter
  (`checkAllMethodsOverridden` groups by name only). Tempest reports it,
  and copies the message's "it's visibility" to match.
- **`case` patterns** give unrelated type errors, then fail when run.
  Tempest says "Not supported yet"; they come with types (milestone-4.md).
- **A function tag on a string with `:` statement lines** crashes the
  compiler on a `TODO`. Tempest says "Not supported yet"
  ([milestone-3.md](milestone-3.md), "Statements in strings").
- **Labelled `continue outer`** from a nested `for … of` fails at run
  time with no message. Tempest runs it.
- **Type checks depend on constant folding:** `take(maybe(1))` is
  accepted, `take(maybe(y))` isn't. Tempest matches, reporting no more
  than Kotlin (`may_fold`; [milestone-4.md](milestone-4.md), flow facts).
- **A nullable function folded to null,** `let g: fn (Int): Boolean =
  h` for `let h: (fn (Int): Boolean)? = null`, is reported with internal
  names: "Expected subtype of Fn__1006<Int32, Boolean>, but got
  Fn__1006<Int32, Boolean>?". Tempest doesn't report it yet, as for any
  nullable name Kotlin might fold ([milestone-4.md](milestone-4.md),
  "Function types compared part by part").
- **Builtins' errors:** `1 < 1.0` names `<=>`'s signature, `(Int32,
  Int32) -> Int32`; `console.log()` says "Expected 2", counting `this`;
  "Cannot bubble …" for `x as T` has an empty position; and a `fn` with
  no return type is checked for `1 / d` but not `bubble()`. Tempest
  copies the first, says "Expected 1", reports `x as T` at the check,
  once (below), and doesn't report the last yet. Naming `<`'s own signature rather than `<=>`'s is arguable, for
  review.
- **Classes' errors:** messages name classes by internal names, as
  `C__0` in "Cannot assign to String from C__0" and "No callee matches
  inputs [T__10] among [(A__6) -> Int32, …]", and a method call's arity
  error counts `this` ("Expected 2" for one parameter). Tempest names
  classes as written and doesn't count `this`
  ([milestone-4.md](milestone-4.md), classes). Arguable, kept for now:
  a method's or constructor's parameter with a default shows as
  nullable ("Expected subtype of String?, but got Int32"), since null
  means the default, while a function's shows as `String`.
- **`==` in `when`:** `when (k) { k -> … }` on a class or interface
  without `@operator("==")` is reported spanning from the subject to the
  case, and `when (k) { 4 -> … }` at the subject; tempest reports both
  at the case, the value compared. Arguable, for
  review ([milestone-4.md](milestone-4.md), operators defined by
  methods).
- **`is` checks' errors,** as "Unrelated types cannot be targeted with
  is or as runtime type checks: <[J]> from I" and "Runtime type check
  from Int32 to String can never succeed", are each reported twice: at
  the check, and again at an empty position at the end of the branch it
  narrows. Tempest reports each once, at the check
  ([milestone-4.md](milestone-4.md), "`is` and `as` checks").
- **`as` checks' bubbling** is reported at odd positions: at an empty
  position at `x` for `x as Int` with `x: Int?`, at both that and the
  check for `i as D` with `i: I?`, and only at the check for `i as D`
  with `i: I`; also for an unnecessary `x as AnyValue`. Tempest reports
  a check that may fail once, at the check, and an unnecessary one not
  at all ([typing.md](typing.md), "What may bubble").
- **A user's `@connected` type** can't be a check's target: `x is X`
  for `@connected("X") class X` says "Expected value of type Type not
  `X`", and `@connected("Y") interface Y` isn't declared at all. Tempest
  treats them as ordinary classes for now ([milestone-4.md](milestone-4.md),
  "`is` and `as` checks").
- **Bubbling in operators' arguments** depends on how they fit: Kotlin
  doesn't report a call that may bubble in an argument that doesn't fit
  the operator's signature (`1 == mk(n)`), unless the first argument
  fits no variant (`k == mk(n)`, `"a" - mi(n)`). Harmless, since the
  misfit is reported, but odd; tempest matches ([typing.md](typing.md),
  "What may bubble").
- **Overrides with the wrong signature** ("Type B has method f with
  signature …, but it should be …"), like Kotlin's other
  `TypeDeclChecker` errors, are reported only while generating code,
  which a module without exports skips, so its bad overrides pass.
  Tempest checks every module. A field that overrides a getter with the
  wrong type is reported at an empty position at its name; tempest
  spans the field.
- **"Void expressions cannot be used as values"** is reported at an
  empty position after a temporary Kotlin made, for `[b()]`, `b() ==
  b()` and `"${b()}"`; tempest reports it at the expression. For
  `b().toString()`, Kotlin says "No member toString in Void"; tempest
  says this at `b()` ([typing.md](typing.md), "`Void` as a value").
- **Generic calls:** a call with type arguments whose arguments don't
  fit, as `id<String>(n)` for `n: Int` in a function, is also "Expected
  function type, but got Invalid" at the callee, which tempest doesn't
  say; at top level, a call of constants is folded first, so
  `id<Int>("a")` passes as a `String`, which tempest matches. `let xs:
  List<String> = wrap(1)` reports the argument ("Actual arguments do
  not match signature: … expected [String], but got [Int32]"), solving
  from the type wanted first, where `let c: String = id(1)` reports the
  assignment; tempest matches both ([typing.md](typing.md), "Generic
  calls").
- **A generic call that's an argument** isn't checked against the
  parameter: `takeLS(wrap(1))`, for `takeLS(xs: List<String>)` and
  `wrap<T>(x: T): List<T>`, passes, where `let xs: List<String> =
  wrap(1)` is an error. Tempest reports it, "Expected subtype of
  List<String>, but got List<Int32>" ([milestone-4.md](milestone-4.md),
  "Generic classes and methods").
- **A method's explicit type arguments** fail: `o.first<String>(ss)`,
  for `first<T>(items: List<T>): T`, is "Member first defined in Order
  incompatible with usage", then "Expected function type, but got
  Invalid", even where they fit, as is a builtin one's,
  `ls.map<String>(toStr)` ("Member map defined in List | Listed …").
  Tempest binds them as written, as for a function's.
- **A non-generic class's generic supertype** doesn't bind a generic
  call's type parameter: for `class StrHolder extends Holder<String>`
  and `unwrap<T>(x: Holder<T>): T`, `unwrap(sh)` is "Actual arguments do
  not match signature: … expected [Holder<AnyValue>], but got
  [StrHolder]", though a `GenHolder<String>` (`class GenHolder<U>
  extends Holder<U>`) binds `T` to `String`. Tempest binds both, so
  `let n: Int = unwrap(sh)` is "Cannot assign to Int32 from String"
  ([milestone-4.md](milestone-4.md), "Users' generic supertypes").
- **A narrowed use's type error,** as `let j: J = i` in `if (i is D) {
  … }`, is reported at an empty position at the end of the branch.
  Tempest reports it at the use.
- **An `if` value that doesn't fit,** as `let s: String = if (c) { 1 }
  else { return -1; }`, is "Cannot assign to String from Int32" at an
  empty position at the end of the first branch. Tempest reports it at
  the `if` ([milestone-4.md](milestone-4.md), "Every expression typed,
  first part").
- **`m is MapBuilder`** for `m: Map<Int32, String>`, a check that can
  never succeed, is "Could not infer type actuals for (AnyValue?, Type)
  -> Boolean given [Map<Int32, String>, Type] in context unknown", a
  solver's failure, and fails to translate. Tempest says "Runtime type
  check from Map<Int32, String> to MapBuilder can never succeed", at the
  same span.
- **A member of a nullable value** gets an odd message: `xs.length`
  for `xs: List<Int>?` is "No member length in AnyValue", and `g.n` for
  `g: P?` "No type for subject of .n"; each should say the value may be
  null. Tempest doesn't report these yet ([milestone-4.md](milestone-4.md),
  "Builtin generic types' members").
- **A list literal with a null or nullable element** is rejected:
  `[1, null]` is "Expected subtype of Int32, but got Int32?", even
  where a `List<Int?>` is wanted, and `[n, 1]` for `n: Int?` there is
  "Actual arguments do not match signature". Tempest accepts both, as
  `List<Int32?>`s. A list literal as a function's argument, as `g([1])`
  for `g(xs: List<String>)`, isn't checked, though a constructor's is;
  tempest reports it. And an inner list typed from the type wanted gets
  odd errors: `[[], [1]]` where a `List<List<String>>` is wanted blames
  the `[]` ([milestone-4.md](milestone-4.md), "List literals and target
  typing").
- **Function literals' errors** ([typing.md](typing.md), "Function
  literals and target typing"): a block's result that isn't `Void`
  where `Void` is wanted is reported for users' functions and
  `Mapped.forEach`, but not for builtin methods with an
  `@inlineUnrealizedGoal` parameter, as `Listed.forEach`, nor users'
  methods with one; tempest reports none of them yet. A null, `Void`, or
  `panic()` result where an `Int` is wanted is "but got Never<Empty>?",
  "but got Void" (with "Void expressions cannot be used as values"), or
  "but got Top", so tempest doesn't report those yet. Some say "No type
  for expr" instead of what's wrong, as a literal whose declared result
  isn't the one wanted where that's a `String` or nullable (`chk5(true) {
  (x): Int => 1 }`, for `chk5(c: Boolean, m: fn (Int): String)`), a list
  literal result that doesn't fit, or `wiI { x => "" }`, for `wiI(m: fn
  (Int): Int): Int { 1 }`; tempest says what's wrong. And valid code
  fails: `opts(fn (x: Int): Int { x }) { x => x.length }`, for `opts(f:
  fn (Int): Int, g: fn (String): String)`, is "No type for expr" and "No
  type for subject of .length"; tempest accepts it. A `return panic()`
  in a literal is "but got Top", and `return null` "but got
  Never<Empty>?"; tempest accepts the first and says "but got Int32?"
  for the second.
- **`return` in a block lambda** is an "unrealized goal" in Kotlin, a
  jump to the function around the lambda, which works only where the
  lambda is inlined, at an `@inlineUnrealizedGoal` parameter: so
  `ls.forEach { x => if (x == 1) { return 5; } }; 3` in a function
  returns 5, and `return "a"` there is "Cannot assign to Int32 from
  String" where that function returns an `Int`. Elsewhere, as in `apply
  { x => return 2; }`, it's "Expected function type, but got Function",
  with "Expected subtype of Int32, but got Void" and "Void expressions
  cannot be used as values" (or "Expected function type, but got Int32"
  where the lambda declares its result). Checked with `temper build` and
  `temper run -b js`, 2026-10-29. Tempest returns from the block lambda,
  as from a `fn` literal, so the `forEach` above returns 3, and checks
  the value against the lambda's result: a difference at run time.
  Decided 2026-10-29: match Kotlin at `@inlineUnrealizedGoal` parameters,
  by an equivalent mechanism, and give a clear error elsewhere; not done
  yet ([milestone-4.md](milestone-4.md), "Order and decisions";
  [typing.md](typing.md), "Function literals and target typing"). A `return` in a `fn` literal leaves the literal in both.
- **Function arguments go unchecked** where Kotlin can run the call
  while compiling, so `wiI(str)` or `wiI(fn (x: Int): String { "" })`,
  for `wiI` above, which never calls its argument, passes; and in
  `assert(true) { (): Int => 1 }` as the last expression of a `test`
  block (with a `;` after it, it's reported). Tempest reports each, as
  Kotlin does for a function that calls its argument.
- **A function where a generic `@fun` interface is wanted** in a
  generic call fails: `applyM(1, isPos)`, for `let applyM<A, B>(x: A,
  m: Mapper<A, B>): B` and `@fun interface Mapper<A, B>(a: A): B;`, is
  "Expected function type, but got Invalid" and "Actual arguments do not
  match signature: … expected [Int32, Mapper__3<Int32, AnyValue>], but
  got [Int32, Fn__1006<Int32, Boolean>]"; a trailing block there works.
  And a function literal in a list literal where a `List<Mapper<Int,
  Boolean>>` is wanted isn't typed from it ("Explicit return type
  required"). Checked with `temper build`, 2026-10-29. Tempest accepts
  both ([typing.md](typing.md), "Fun interfaces").
- **`@fun` interfaces' rules are documented but not checked:**
  `FunDecorator.kt` says one may extend only `AnyValue`, declare only
  `apply`, and not be used with `is` or `as`, but `temper build`
  accepts each (checked 2026-10-29), and also a `Pred` where a `Check`
  is wanted. Tempest reports them (above, "Named `@fun` interfaces are
  nominal").
- **Method references fail:** a method read as a value, `k.isBig` or a
  bare `isBig` in its class, is "Member isBig defined in K incompatible
  with usage" (checked 2026-10-29). Tempest types it as the method's
  function type, or as the `@fun` interface wanted, as Java's method
  references.
- **Positions and wording:** a bare `return` in a non-`Void` function is
  reported past the function's end (tempest: at the `return`); a cycle of
  inferred types says "Cannot assign to String from Top" (tempest: no
  error yet; it should say the type can't be inferred).

### Interpreter, core and std

- **`Float64.toString` cuts at 16 digits** in the interpreter
  (`FormatDouble.kt`), so `0.30000000000000004` prints as
  `0.3000000000000000`, which doesn't read back; the Python and JS
  runtimes round-trip. Tempest matches the interpreter for now
  ([milestone-3.md](milestone-3.md), "Numbers"); it should round-trip.
- **`String.toFloat64`** takes JVM syntax beyond its docs (`1f`, `-.5`,
  `0x1p3`, `02`, `+Infinity`). Tempest follows the docs.
- **`String.toInt32` and `toInt64`** take more than the documented JSON
  integers, differently per backend. Tempest takes only ASCII digits, but
  still a `+` and surrounding whitespace, as Kotlin's interpreter does.
- **`s[i]` past the end** fails with a bubble in the interpreter, which
  its signature doesn't declare; the docs and runtimes panic, as tempest
  does.
- **`Int.toString(radix)`** fails outside 2 to 36, but core declares no
  `throws Bubble`. Tempest fails the same way; decide with builtin
  types' methods' signatures (milestone-4.md, slice 3).
- **A failing `async` block is silent** in the interpreter
  (`interpretAndWaitForAsyncTasksToSettle` checks the wrong result).
  Tempest reports the first failure.
- **std `net`'s `post`** assigns `bodyMimeType` to itself, so no content
  type is sent. Tempest uses std as is, so the same until std is fixed.
- **temper-regex-parser drops the code before a trailing dash** in a set:
  `/[a-]/` parses as `[-]`. Tempest keeps both (`regex.rs`).

### Lexer and parser

Details in testing-and-milestones.md, "Kotlin bugs found". Tempest fixes:
crashes on several malformed inputs (`Parse.kt`, `StringFixer`,
`buildTaggedRange`, `Lift.kt`), stack overflow at about 300 nested calls,
exponential time on nesting, odd spans from `Tree.replace`, an unclosed
fence at the end of a file losing its last line, and `"\` splitting a
surrogate pair. It copies, until the comparisons with Kotlin end, two
that affect only broken input: close brackets that can't look past
others, and semicolon insertion counting a `{` twice.

## Later (backends and tooling)

Bugs tempest would inherit with its backends (milestones 5 and 6) if it
copied Kotlin's:

- **`==` and `!=` between Booleans** don't translate in Lua and JS, and
  Python emits code that fails only when run.
- **Initialization order:** type declarations go before module-level
  values, so a static can read a value not set yet (C++ reads zero,
  Python fails with `NameError`); Python's statics also precede the
  functions they use. tmpl should keep source order. Rust and C++ need
  `init` first, and init timing differs per backend (api-compat.md,
  proposal 2).
- **Rust list indexing panics** where Temper bubbles (`ListedTrait::get`
  unwraps); use a checked get.
- **Rust module `var`s lose concurrent updates:** `n = n + 1` reads
  `n` under one lock and writes it under another, so two threads can
  both write the same value. Safe in Rust's sense, but not what a
  counter wants. Tempest's Rust backend reads and writes such an update
  under one write lock (done 2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-rust)).
  Kotlin's Rust output also fails clippy's deny-level
  `self_assignment` on `zero = zero` despite its `#![allow(warnings)]`,
  which allows only warn-level lints; tempest's too, being faithful,
  so its test allows that lint.
- **JS source maps that tools don't find:** Kotlin writes a `.js.map`
  beside each module but no `//# sourceMappingURL=` comment, so Node's
  `--enable-source-maps` and browsers never load it; its `sourceRoot`
  is `-work/src/` (the work root's own name, not a path from the map),
  so its sources don't resolve; and its `file` is relative to
  `temper.out`, not to the map. Tempest writes the comment and names
  sources by a path from the map to them, which Node follows (tested;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "source maps for JS").
  Kotlin includes `sourcesContent`; tempest leaves it out, since tmpl
  carries no source text.
- **String ordering by code point (intended difference):** Temper orders
  strings by code point, as the interpreter does. Kotlin's Java output
  compares strings with `compareTo`, which orders UTF-16 code units, so
  `"\u{FF61}" < "\u{1F600}"` is false there, and its `<=>` gives the
  units' difference (10020), not -1, 0, or 1. Kotlin's JS orders code
  points (`cmpString`), but its `<=>` on strings and on floats gives
  `cmpString`'s or `cmpFloat64`'s difference too (`"z" <=> "a"` is 25,
  `2.5 <=> 1.0` is 1.5). Kotlin's C# is right: it calls the runtime's
  `StringUtil.CompareStringsByCodePoint`. Tempest's Java backend calls a
  small code-point helper that each module's class gets (its
  `temper-core` has none), its JS backend takes `Math.sign` of those
  differences, and its C# backend calls the runtime's helper, as
  Kotlin's; Python, Rust, C++ (whose `std::string` compares bytes as
  `unsigned char`, which for UTF-8 is code-point order), and Lua (whose
  `<` compares bytes with `strcoll`, the same in the C locale it starts
  in) order code points natively. One program, checked against the
  interpreter, runs on all seven (done 2026-10-30;
  [tmpl/roadmap.md](tmpl/roadmap.md#state)).
- **C# formats numbers by the current culture:** Kotlin's C# writes
  `Int32.toString()` as the runtime's `Core.ConvertToString(n)`, which
  is `n.ToString()`, so under `sv-SE` (and other cultures whose minus
  sign is U+2212) `-1` logs as `−1`; `Boolean.toString()` lowercases by
  the current culture too, harmlessly for `True` and `False`. Tempest's
  C# backend formats in the invariant culture
  (`n.ToString(CultureInfo.InvariantCulture)`), and its tests run under
  `sv-SE` with the .NET analyzers' culture rules as errors (done
  2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-c)).
- **C# `Boolean` comparisons:** Kotlin's C# writes `a < b` on booleans
  (it turns `a.CompareTo(b) < 0` into `<`), which C# doesn't have, so
  the library doesn't compile (CS0019). Tempest's keeps
  `a.CompareTo(b) < 0`, and its C# tests compile and run it.
- **C# bodies:** besides numbered names, Kotlin's C# wraps a function
  that returns early in labeled blocks with a `goto` and a `return__N`
  variable, keeps `zero = zero` (C# warning CS1717), and names
  namespaces by one-letter aliases (`C::Core`). Tempest's returns
  directly, drops self-assignments, and writes `using`s.
- **Java module `var`s are plain statics,** so a function's update from
  several threads can be lost and other threads may read a stale value.
  Tempest's Java backend makes a `var` that functions write `volatile`,
  and locks the class for an update from itself alone (done 2026-10-30;
  [tmpl/profiles.md](tmpl/profiles.md#as-built-java)).
- **C++ module state isn't thread-safe:** Kotlin's `global_init_init()`
  checks and sets a plain `static bool`, so two threads calling it at
  once can both run the module's top level, and module `var`s are plain
  globals, so functions updating them from several threads race
  (undefined behavior in C++): ThreadSanitizer reports it, and 8
  threads × 1,000 updates of `ticks = ticks + 1` give about 6,500.
  Tempest's C++ runs init through `std::call_once` and reads and writes
  such `var`s in functions only under a mutex, a run of updates under
  one lock (done 2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-c-1)).
- **C++ order of evaluation:** Kotlin's C++, at the vendored commit,
  passes operands straight to calls and to its `cat`, whose order C++
  leaves unspecified, so `module-state`'s `"${bump()} … ${total}"` logs
  `hi 1 of 3 10` under g++, which reads `total` first. (Kotlin's main
  has since hoisted operands into a lambda's locals.) Tempest's computes
  an operand into a local first only where the order shows (done
  2026-10-30).
- **C++ shifts and comparisons:** Kotlin's C++ writes `a << b` and
  `a >> b` for `Int`, which C++14 leaves undefined for a negative `a`
  or a count past 31, where Temper masks the count; `<=>` on two string
  literals as `Compare::cmp("b", "a")`, which compares pointers; and
  fails on `Boolean`'s `<=>` ("unsupported C++ binary operator:
  Compare::cmp"). Tempest's uses the runtime's `Int::shl` and
  `Int::shr` (C++'s `>>` only for a literal count in range), compares
  literals as `std::string`s, and calls `Compare::cmp` for `Boolean`.
- **C++ bodies and headers:** besides numbered names, Kotlin's C++ wraps
  every loop and branch body in an extra block, a function that returns
  early in a labeled block with a `goto` (whose unused labels fail
  `-Wall -Werror`), keeps `zero = zero`, writes every module-level value
  as a global even if only the top level uses it, builds strings
  through a `std::stringstream` (`cat`), and declares every function in
  the header, exported or not. Tempest's returns directly, uses `+` and
  `std::to_string`, keeps top-level-only values as the init's locals,
  and declares only exports.
- **C++ temporaries:** Kotlin's C++ returns lists as `shared_ptr`s, so
  iterating one in place (`for (auto const &x : *obj->list())`) reads a
  vector already freed. Tempest's C++ will name such a list first
  ([tmpl/profiles.md](tmpl/profiles.md#as-built-c-1), "Temporaries that
  die in use"); subset A has no lists yet.
- **Lua exports are copied once:** Kotlin's Lua module fills its
  exports table after its top level runs (`exports.ticks = ticks`), so
  an exported `var` that the library's functions write stays at its
  first value for users: after `tick()` twice, `require("…").ticks` is
  still 0. Tempest's Lua keeps such a `var` in the exports table itself
  (`exports.ticks = …` in `tick`), and its test reads it back (done
  2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-lua)).
- **Lua `Boolean` comparisons:** Kotlin's Lua writes `a < b` on booleans,
  as its C# does, which Lua refuses at run time ("attempt to compare two
  boolean values"). Tempest's writes the order as logic (`not a and b`),
  or through the runtime's `bool_cmp` where the right side has effects
  that `and` would skip.
- **Lua's `goto`:** Kotlin's list of Lua keywords leaves out `goto`
  (reserved since Lua 5.2), so a Temper name `goto` gives Lua that
  doesn't parse; tempest reserves it. And Kotlin says it supports Lua
  5.1 (its docs, and `lua >= 5.1` in its rockspecs, which tempest keeps
  for compatibility), but its output uses `goto` for every early return
  and loop exit, which 5.1 can't parse (LuaJIT can). Tempest's uses
  `goto` only for `continue` and for leaving an outer loop, so a program
  without those parses on 5.1 too.
- **Lua bodies:** besides numbered names, Kotlin's Lua declares every
  module-level name in one `local` line at the top and assigns each
  later, keeps a `0.0` for the console, wraps early returns in labels
  and `goto`s, logs through `temper.log`, concatenates through
  `temper.concat`, and escapes every non-ASCII byte in strings
  (`'\xc3\xa9a'`). Tempest's declares locals and local functions where
  they're first given a value, returns directly, and writes `print`,
  `..`, and strings as UTF-8. Kotlin also writes the rockspec's
  `source.url` as an absolute `file://` path to a zip it makes; tempest
  names the zip relative, as Kotlin does when it has no path, and makes
  none.
- **C++ template methods** can't call module-level generic functions,
  declared after types in the header.
- **Stale files in `temper.out`** after renames break builds; record the
  files each backend writes and delete only those no longer written.
- **Generated APIs:** proposals in api-compat.md cover numbered names,
  Lua's null sentinel, Rust's optional parameters, Java's `to_` and
  `<Lib>Global`. Not yet covered: C# `ToString` without `override`, C#
  `A98_Rgb`, public C# private constructors and module-private classes,
  Java map literals too slow to infer, Lua rockspecs that don't install,
  and C++ headers exposing private members.
- **Variance (recommended change):** Kotlin Temper honors `@out` and
  `@in` on type parameters, so it accepts a `List<Dog>` as a
  `List<Animal>`. Temper plans invariant type parameters, since C++ and
  Rust, among others, can't express variance well, and tempest stays
  invariant ([typing.md](typing.md), "Type parameters are invariant").
  Recommended: Kotlin Temper drops variance too. Tempest won't report
  what only variance accepts until code relying on it is found and
  changed.

- **Kotlin Temper's move to its newer types** (PR #500, in review
  2026-10-29) carries `Never<T>` with its fallback type into its tmpl,
  as tempest's tmpl does, and keeps "may bubble" as a result type beside
  a separate signature, which tempest avoids by treating bubbling as an
  effect. Its ad hoc function types now also separate whether a method
  has `this`, a question to answer whenever tempest interns function
  types. Notes only; no change to tempest's plans.
