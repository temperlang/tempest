# Differences from Kotlin Temper

Part of the [rewrite plan](README.md). Where tempest deliberately differs
from Kotlin Temper as it is today, and the bugs we know of in Kotlin
Temper, with what tempest does about each. Details live in the linked
notes. The parser's differences are listed in full in
[testing-and-milestones.md](testing-and-milestones.md) ("Deliberate
deviations from Kotlin" and "Kotlin bugs found"), so only the main ones
are here. Update this when either side changes.

Each entry where tempest fixes or avoids a Kotlin bug ends with
"Test:", naming what would fail if Kotlin's behavior came back. Paths
are under `crates/`: `settle/X.cases` is
`temper-frontend/tests/settle/X.cases`, its cases named by their `===`
titles; `programs/X` is the backends' shared program in
`temper-be-testkit/programs/`; a funtest is a vendored functional test,
and `kit::X` a list of them, that each backend's `tests/functional.rs`
runs. Entries without one aren't fixed yet, match Kotlin, or are
proposals. A new fix gets its test in the same change.

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
  Test: settle/types.cases, "list literals typed from the type wanted",
  "function literals typed from the type wanted", and "object literals
  take their class from the type wanted".
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
  Test: settle/types.cases, "named @fun interfaces are nominal", "@fun
  interfaces keep Kotlin Temper's rules for them", and "a function
  literal needs a type wanted".
- **Fields first** (decided by the Temper team 2026-11-02, done;
  [typing.md](typing.md), "Fields first"): a constructor, and the field
  initializers before it, may use `this` only to set fields and read
  those already set until every field is set on every path, and every
  normal return sets every field. Tempest reports an error where Kotlin
  accepts the program, a deliberate, team-approved difference: "Calling
  this.sum() before b is set", "Property b not initialized in
  constructor" for an explicit constructor, and the like. Kotlin checks
  only a class without a constructor, with that last message, and its
  Rust backend fails to compile a field set after `this` is used
  (rustc's E0063), while its other backends read the unset field.
  Test: settle/fields-first.cases, every case.
- **Null and `is` narrowing from control flow,** through `&&`, `||`,
  `!`, `while`, and after an early `return`, `break`, `continue` or
  `bubble()`; Kotlin narrows only the branches of an `if` on one check.
  Done for names that aren't `var` ([typing.md](typing.md), "Null-check
  and `is` narrowing: the rules"). Also, `d is I` for a `d` already an
  `I` keeps its more precise type, where Kotlin narrows it to `I`.
  Test: settle/flow.cases, "narrowed beyond Kotlin Temper" and "is
  checks narrow to types".
- **Type errors name declared types,** not the types of folded constants
  (Kotlin's `let s: String? = null; let q: Int = s` says "got Int32?",
  and a name it folds to a function, `opt`, gets `opt`'s type, "from fn
  (Int32, _?: Int32): Int32", where the name's is `fn (Int32, Int32):
  Int32`). Done ([milestone-4.md](milestone-4.md), first slice, and
  "Function types compared part by part").
  Test: settle/types.cases, "null where a type isn't nullable" and
  "function types compared part by part".
- **A function literal's result reported at the result,** "Expected
  subtype of Int32, but got String" for `{ x => let y = x; "a" }` where
  a `fn (Int32): Int32` is wanted at the `"a"`, where Kotlin's position
  is empty, at the start of the body (here, the `let`); likewise a
  value it returns, at the value, each `return` on its own, where
  Kotlin's position is empty, just after the `return`, and its type is
  all the literal's results joined. Done ([typing.md](typing.md),
  "Function literals and target typing").
  Test: settle/types.cases, "function literals typed from the type
  wanted" and "returns and bubbling in function literals".
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
  Test: settle/class-types.cases, "overrides of generic supertypes'
  members" and "interfaces' generic methods, which Kotlin Temper doesn't
  allow".
- **Generators in any function form** (named functions, `fn` literals,
  methods, not only block lambdas), and `return` ends a generator, where
  Kotlin fails typing and panics. Done ([milestone-3.md](milestone-3.md),
  flat frames, step 2).
  Test: settle/run.cases, "generators from named functions and methods,
  which Kotlin Temper lacks" and "return ends a generator, and closures
  keep its locals".
- **Private members of other instances** of the same class are usable in
  its methods, as Kotlin Temper's #625 allows (merged to its main
  2026-11-03; before it, Kotlin said only "Cannot translate Invalid" and
  emitted code that fails). Done, checked at run time
  ([milestone-3.md](milestone-3.md), "Classes"); no longer a difference
  once Kotlin's next release has it.
  Test: settle/classes.cases, "private members are readable on any
  instance of their class"; programs/classes.temper.md on every backend.
- **`yield` and `await` in `for … of` bodies.** Kotlin lowers a body to a
  `forEach` lambda, so a `yield` there fails at run time. Idea: iterables
  provide a generator or iterator, so `yield`, `await`, `break` and
  `return` just work, backends still map lists to native loops, and
  `forEach` stays an optional fast path; costs an iterator object where
  there's no native mapping. Tempest says "Not supported yet" for now
  ([milestone-3.md](milestone-3.md), flat frames).
  Test: settle/run.cases, "yield in a for … of body isn't supported yet"
  (the message).
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
  Test: settle/types.cases, "types that may bubble are written as Temper
  writes them".
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
  Test: settle/class-types.cases, "is and as checks with nullable
  targets, which null passes".
- **Say why erased checks fail:** "Type arguments cannot be introduced
  with is or as runtime type checks" (`x is Box<Int>` from `AnyValue`)
  and "Type parameters cannot be targeted …" (`x is T`) end ", because
  a type argument's run-time type can't be checked" in tempest, before
  the types. Done ([typing.md](typing.md), "Run-time type checks").
  Test: settle/class-types.cases, "is and as checks of type parameters'
  values and generic types".
- **Float64 division by a nonzero constant** (`x / 2.0`) can't fail, so
  it needn't count as bubbling, as `Int` division by one doesn't. Tempest
  matches Kotlin for now ([milestone-4.md](milestone-4.md), builtins'
  signatures).
- **An `internal` visibility** (module or library) between `public` and
  `private`, so module functions can use a class's hidden parts. Design it
  from what JS can express (only public and `#private`). Not planned yet.
- **A test-only marker** (`@forTest`, name open) for helper
  declarations: the frontend reports an error if one is exported or used
  outside tests (test blocks, test modules, or other test-only
  declarations), so readers can see which helpers exist only for tests,
  and backends can leave them out of production output. Idea from the
  user, 2026-11-02; needs team agreement. Not planned yet.

Literate source:

- **Code in markdown block quotes is prose.** It can't work past one line
  in Kotlin anyway. Done.
  Test: temper-syntax/tests/edge-cases.cases, "blockquote with indented
  code then a lazy line, which crashed Kotlin".
- **Multi-quoted strings** follow clearer rules of our own. Done
  (testing-and-milestones.md, "Multi-quoted strings").
  Test: temper-syntax/tests/kotlin-lexer-tests.cases (the multi-quoted
  cases) and edge-cases.cases.
- **A warning for an indented code block that continues a list item.**
  CommonMark makes it prose, so code silently disappears (seen: a class
  lost its first lines). Not planned yet.

Backends, planned: no numbered names where visible (design.md, "Naming";
api-compat.md proposal 1); no required `init` in Rust and C++
(api-compat.md proposal 2; in Kotlin since #636, and in tempest since
2026-10-31); fun interfaces as named function types
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

- **`@auto("==")`'s `eq`** (2026-10-30) is generated as Kotlin's
  `AddAutoMembers` does, with its "multiple supertype(s)" error, but
  tempest doesn't check yet that the hierarchy is sealed (Kotlin's
  "unsealed subtype(s)" and "not sealed" errors), nor make one for a
  generic class, nor rename it past another member named `eq`
  ([milestone-4.md](milestone-4.md), "`@auto("==")` members
  generated").
  Test: settle/run.cases, "auto equality compares public fields", "auto
  equality from a sealed supertype", and "auto equality calls each
  field's own ==".

- **`x is D` for a type parameter's value,** `x: T` with `T extends
  Animal` and `D` a class extending `Animal`, is reported as between
  unrelated types, where `a is D` for `a: Animal` is a downcast. Tempest
  judges a type parameter's value by its bound, all that's known of it
  at run time, so this is a downcast, allowed, narrowing `x` to `D`; and
  `x is Int` for an unbounded `T` is the `@mayDowncastTo(false)` error,
  "from T", where Kotlin says unrelated ([typing.md](typing.md),
  "Run-time type checks").
  Test: settle/class-types.cases, "is and as checks of type parameters'
  values and generic types".

- **A function in a field, `obj.f(x)`,** fails ("incompatible with
  usage"); Kotlin's own `functions/constructor-callbacks` keeps its use
  inert. Tempest runs it (`call_method` in `interp.rs` falls back to the
  property) and types it as a call of the field's value
  (`tests/settle/class-types.cases`).
  Test: settle/class-types.cases, "a function in a field, called as a
  method"; settle/run.cases, "a functional interface, called through a
  field".
- **A trailing block placed past a skipped optional parameter** crashes
  Kotlin's typer (a `NullPointerException` in `Typer.typeCallByParts2`,
  on the null put in for the skipped parameter): `h(1) { … }`, for `let
  h(n: Int, s: String = "d", g: (fn (): Void)? = null)`, a method's
  too. Tempest passes the block as `g`, and `s` takes its default
  ([typing.md](typing.md), "Function literals and target typing";
  `tests/settle/run.cases`).
  Test: settle/run.cases, "a trailing block goes to the last function
  parameter" and "a method's trailing block goes where typing places
  it".
- **An empty body of a non-`Void` function,** `let f(): Int { }`, isn't
  reported, and calls then fail at run time. Tempest doesn't report it
  yet either (`typing.rs` skips `Void` results).
- **`assert(list == null)`** with a nullable list fails with "Cannot
  assign to Invalid from String". Not fixed in tempest either: the
  message `assert` makes, as Kotlin's, interpolates both sides
  (`expected none == (${none}) not (…)`), and a `List<Int>?` has no
  `toString`, so typing leaves those reads unresolved and tmpl can't
  translate the test (checked 2026-11-04 in `every_expression_is_typed`).
  A fix: a message that shows only values that can be shown. Deferred,
  as a message of one's own works around it: `assert(list == null) {
  "list isn't null" }` types, translates, and runs.
  Test: settle.rs, `every_expression_is_typed` (the workaround).
- **An interface's backed property** isn't reported, then fails at run
  time. Tempest reports it ([milestone-3.md](milestone-3.md), "Classes").
  Test: settle/classes.cases, "interfaces have no constructor parameters
  or backed properties".
- **A missing setter** is hidden by an implemented getter
  (`checkAllMethodsOverridden` groups by name only). Tempest reports it.
  Test: settle/classes.cases, "a concrete class implements what its
  supertypes leave abstract" (`P`).
- **`case` patterns** give unrelated type errors, then fail when run.
  Tempest says "Not supported yet"; they come with types (milestone-4.md).
  Test: settle/macros.cases, "when with case patterns, which need types,
  and invalid cases" (the message).
- **A function tag on a string with `:` statement lines** crashes the
  compiler on a `TODO`. Tempest says "Not supported yet"
  ([milestone-3.md](milestone-3.md), "Statements in strings").
  Test: settle/macros.cases, "a tag that names a class is an accumulator
  type" (the message).
- **Labelled `continue outer`** from a nested `for … of` fails at run
  time with no message. Tempest runs it.
  Test: settle/run.cases, "for of, with jumps (Kotlin fails continue
  outer at run time)".
- **Errors about `Invalid`** follow others since #500 (in Kotlin's stage
  tests' expectations): after "No declaration for what" or "Member ap …
  not publicly accessible", also "Expected subtype of String, but got
  Invalid", and "Void expressions cannot be used as values" becomes
  "Expected subtype of List<Void>, but got Invalid". Tempest reports
  the first error only, and keeps the Void one.
  Test: settle/names.cases, "undeclared names are errors, which fail
  when run"; settle/classes.cases, "private members aren't usable
  outside their class"; settle/types.cases, "Void expressions as
  values".
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
  Test: settle/builtins.cases, "operators pick a variant by their
  arguments, in order" and "builtins with one signature";
  settle/class-types.cases, "as may bubble".
- **Classes' errors:** messages name classes by internal names, as
  `C__0` in "Cannot assign to String from C__0" and "No callee matches
  inputs [T__10] among [(A__6) -> Int32, …]", and a method call's arity
  error counts `this` ("Expected 2" for one parameter). Tempest names
  classes as written and doesn't count `this`
  ([milestone-4.md](milestone-4.md), classes). Arguable, kept for now:
  a method's or constructor's parameter with a default shows as
  nullable ("Expected subtype of String?, but got Int32"), since null
  means the default, while a function's shows as `String`.
  Test: settle/class-types.cases, "values of classes and interfaces" and
  "members read, called and assigned".
- **`==` in `when`:** `when (k) { k -> … }` on a class or interface
  without `@operator("==")` is reported spanning from the subject to the
  case, and `when (k) { 4 -> … }` at the subject; tempest reports both
  at the case, the value compared. Arguable, for
  review ([milestone-4.md](milestone-4.md), operators defined by
  methods).
  Test: settle/class-types.cases, "operators defined by methods" (`b2`,
  `b3`).
- **`is` checks' errors,** as "Unrelated types cannot be targeted with
  is or as runtime type checks: <[J]> from I" and "Runtime type check
  from Int32 to String can never succeed", are each reported twice: at
  the check, and again at an empty position at the end of the branch it
  narrows. Tempest reports each once, at the check
  ([milestone-4.md](milestone-4.md), "`is` and `as` checks").
  Test: settle/class-types.cases, "is and as checks, as Kotlin Temper
  reports them".
- **`as` checks' bubbling** is reported at odd positions: at an empty
  position at `x` for `x as Int` with `x: Int?`, at both that and the
  check for `i as D` with `i: I?`, and only at the check for `i as D`
  with `i: I`; also for an unnecessary `x as AnyValue`. Tempest reports
  a check that may fail once, at the check, and an unnecessary one not
  at all ([typing.md](typing.md), "What may bubble").
  Test: settle/class-types.cases, "as may bubble".
- **A user's `@connected` type:** since #633, user code may connect a
  concrete class (bare `@connected`, or with a list of backends), but
  not an interface ("At this time, users can connect only classes or
  top-level functions"); `@connected("X")` with a key, as core's, is an
  error there ("@connected expects a list of backend names"); and a
  connected class can be the target of `is` or `as` only from a sealed
  supertype ("Connected types cannot be targeted with is or as runtime
  type checks …"). Tempest treats them as ordinary classes for now and
  reports none of these ([milestone-4.md](milestone-4.md), "`is` and
  `as` checks"). Since Kotlin's `connected-type-fallback` branch, a
  connected class may have a complete Temper implementation, a body for
  every method, which the interpreter, backends not in its
  `@connected([...])` list, and `--no-connected` use. Tempest matches
  in the interpreter, and reports Kotlin's errors: a partial
  implementation ("Method two of a @connected class needs a Temper
  body, since other methods have one, …"), a backend list without one
  ("… that lists backends needs a Temper body for the backends not
  listed"), and default arguments ("Method scale of a connected class
  can't have default arguments, …"), which tempest reported nowhere
  before. Kotlin reports these only while generating code; tempest
  reports them while typing, in every module, skipping std's
  connected classes and core-style ones that connect members. A bodiless
  method of a user's connected class fails in the interpreter as "No
  connected implementation of Counter.add", where it said "Abstract
  method". Every backend reads a library's host files and chooses host
  code or the Temper implementation as Kotlin's `ConnectedChoice.kt`
  does, with `--no-connected`, and Java and C# leave out a replaced
  `Counter.java` or `Counter.cs` ([tmpl/roadmap.md](tmpl/roadmap.md),
  "State"). Where a backend needs host code that isn't there, tempest
  says so, naming the file, where Kotlin leaves it to the target's
  compiler or the first call.
  Test: settle/classes.cases, "a connected class's Temper implementation
  must be complete", "a connected class without a Temper implementation
  runs only host code", and "a connected class's constructor can't have
  defaults either"; each backend's `missing_host_code` and
  `kit::CONNECTED` runs, as is and under `--no-connected`.
- **Connected wrappers' null defaults in JS:** Kotlin's wrapper for
  `@connected let f(s: String? = null)` passes `undefined` to host code
  for a left-out `s`, which strict `tsc` rejects against host code
  typed `string | null`; tempest passes `null`.
  Test: temper-be-js/tests/goldens/functions-connected.js, from funtest
  functions/connected, which the JS `runs` checks with `tsc`.
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
  Test: settle/class-types.cases, "overrides' signatures".
- **"Void expressions cannot be used as values"** is reported at an
  empty position after a temporary Kotlin made, for `[b()]`, `b() ==
  b()` and `"${b()}"`; tempest reports it at the expression. For
  `b().toString()`, Kotlin says "No member toString in Void"; tempest
  says this at `b()` ([typing.md](typing.md), "`Void` as a value").
  Test: settle/types.cases, "Void expressions as values, as Kotlin
  Temper reports them".
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
  Test: settle/types.cases, "generic calls solved from the type wanted"
  (`takeLS`).
- **A method's explicit type arguments** fail: `o.first<String>(ss)`,
  for `first<T>(items: List<T>): T`, is "Member first defined in Order
  incompatible with usage", then "Expected function type, but got
  Invalid", even where they fit, as is a builtin one's,
  `ls.map<String>(toStr)` ("Member map defined in List | Listed …").
  Tempest binds them as written, as for a function's.
  Test: settle/types.cases, "generic classes and methods"
  (`o.first<String>(ss)`).
- **A non-generic class's generic supertype** doesn't bind a generic
  call's type parameter: for `class StrHolder extends Holder<String>`
  and `unwrap<T>(x: Holder<T>): T`, `unwrap(sh)` is "Actual arguments do
  not match signature: … expected [Holder<AnyValue>], but got
  [StrHolder]", though a `GenHolder<String>` (`class GenHolder<U>
  extends Holder<U>`) binds `T` to `String`. Tempest binds both, so
  `let n: Int = unwrap(sh)` is "Cannot assign to Int32 from String"
  ([milestone-4.md](milestone-4.md), "Users' generic supertypes").
  Test: settle/types.cases, "users' generic supertypes".
- **A narrowed use's type error,** as `let j: J = i` in `if (i is D) {
  … }`, is reported at an empty position at the end of the branch.
  Tempest reports it at the use.
  Test: settle/types.cases, "where Kotlin Temper's errors are missing,
  misplaced, or wrong" (`n1`).
- **An `if` value that doesn't fit,** as `let s: String = if (c) { 1 }
  else { return -1; }`, is "Cannot assign to String from Int32" at an
  empty position at the end of the first branch. Tempest reports it at
  the `if` ([milestone-4.md](milestone-4.md), "Every expression typed,
  first part").
  Test: settle/types.cases, "where Kotlin Temper's errors are missing,
  misplaced, or wrong" (`s1`).
- **`m is MapBuilder`** for `m: Map<Int32, String>`, a check that can
  never succeed, is "Could not infer type actuals for (AnyValue?, Type)
  -> Boolean given [Map<Int32, String>, Type] in context unknown", a
  solver's failure, and fails to translate. Tempest says "Runtime type
  check from Map<Int32, String> to MapBuilder can never succeed", at the
  same span.
  Test: settle/run.cases, "Map, MapBuilder and Pair".
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
  Test: settle/types.cases, "list literals' types" and "list literals
  typed from the type wanted".
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
  Test: settle/types.cases, "function literals typed from the type
  wanted", "returns and bubbling in function literals", and "where
  Kotlin Temper's errors are missing, misplaced, or wrong" (`opts`,
  `chk5`).
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
  Test: settle/types.cases, "where Kotlin Temper's errors are missing,
  misplaced, or wrong" (`wiI`).
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
  Test: settle/types.cases, "@fun interfaces as the function types they
  stand for"; settle/run.cases, "a trailing block goes to a @fun
  interface's parameter".
- **`@fun` interfaces' rules are documented but not checked:**
  `FunDecorator.kt` says one may extend only `AnyValue`, declare only
  `apply`, and not be used with `is` or `as`, but `temper build`
  accepts each (checked 2026-10-29), and also a `Pred` where a `Check`
  is wanted. Tempest reports them (above, "Named `@fun` interfaces are
  nominal").
  Test: settle/types.cases, "@fun interfaces keep Kotlin Temper's rules
  for them" and "named @fun interfaces are nominal".
- **Method references fail:** a method read as a value, `k.isBig` or a
  bare `isBig` in its class, is "Member isBig defined in K incompatible
  with usage" (checked 2026-10-29). Tempest types it as the method's
  function type, or as the `@fun` interface wanted, as Java's method
  references.
  Test: settle/types.cases, "named @fun interfaces are nominal"
  (`k.isBig`, `isBig`).
- **Positions and wording:** a bare `return` in a non-`Void` function is
  reported past the function's end (tempest: at the `return`); a cycle of
  inferred types says "Cannot assign to String from Top" (tempest: no
  error yet; it should say the type can't be inferred).
  Test: settle/types.cases, "function results" (`nothing`), for the
  first.

### Interpreter, core and std

- **`Float64.toString` cuts at 16 digits** in the interpreter
  (`FormatDouble.kt`), so `0.30000000000000004` prints as
  `0.3000000000000000`, which doesn't read back; the Python and JS
  runtimes round-trip. Tempest matches the interpreter for now
  ([milestone-3.md](milestone-3.md), "Numbers"); it should round-trip.
- **`String.toFloat64`** takes JVM syntax beyond its docs (`1f`, `-.5`,
  `0x1p3`, `02`, `+Infinity`). Tempest follows the docs.
  Test: settle/run.cases, "Float64 formatting, order and methods, as
  Kotlin Temper's".
- **`String.toInt32` and `toInt64`** take more than the documented JSON
  integers, differently per backend. Tempest takes only ASCII digits, but
  still a `+` and surrounding whitespace, as Kotlin's interpreter does.
  Test: settle/run.cases, "strings and string indices, as Kotlin
  Temper's".
- **`s[i]` past the end** fails with a bubble in the interpreter, which
  its signature doesn't declare; the docs and runtimes panic, as tempest
  does.
  Test: settle/run.cases, "a string index past the end panics, as
  String.get's docs say".
- **`Int.toString(radix)`** fails outside 2 to 36, but core declares no
  `throws Bubble`. Tempest fails the same way; decide with builtin
  types' methods' signatures (milestone-4.md, slice 3).
- **A failing `async` block is silent** in the interpreter
  (`interpretAndWaitForAsyncTasksToSettle` checks the wrong result).
  Tempest reports the first failure.
  Test: settle/run.cases, "a failing async block doesn't stop the
  others".
- **temper-regex-parser drops the code before a trailing dash** in a set:
  `/[a-]/` parses as `[-]`. Tempest keeps both (`regex.rs`).
  Test: temper-frontend/src/regex.rs, `parses_the_dialect`.

### Lexer and parser

Details in testing-and-milestones.md, "Kotlin bugs found". Tempest fixes:
crashes on several malformed inputs (`Parse.kt`, `StringFixer`,
`buildTaggedRange`, `Lift.kt`), stack overflow at about 300 nested calls,
exponential time on nesting, odd spans from `Tree.replace`, an unclosed
fence at the end of a file losing its last line, and `"\` splitting a
surrogate pair. It copies, until the comparisons with Kotlin end, two
that affect only broken input: close brackets that can't look past
others, and semicolon insertion counting a `{` twice.
Test: the crashes in temper-syntax/tests/parse/edge-cases.cases and
ast/edge-cases.cases ("… which crashes Kotlin"), checked again under
edits by `mutated_inputs_keep_parser_properties`; the stack overflow and
exponential time in parse.rs, `deep_and_nested_inputs_parse`; the
unclosed fence and `"\` in temper-syntax/tests/edge-cases.cases.

## Later (backends and tooling)

- **Generators and `async`** (2026-11-05), each as Kotlin's strategy
  but:
  - **Python:** a generator's `def` is typed (`do_await:
    Callable[[Future[Any]], Any]`, `-> Generator[Any, Any, None]`),
    where Kotlin's leaves `do_await` untyped and returns
    `Generator[empty, None, None]`, naming a function as a type, which
    strict mypy and pyright reject; and a name a later block declares
    again is annotated once, as mypy's `no-redef` wants.
  - **Rust:** each state's code is in an `orelse`, where Kotlin's wraps
    only an `await`'s result; a state machine with one state has no
    loop (clippy's `never_loop`); and `Promise` is a `Future`
    (api-compat.md, proposal 2).
  - **JS:** a generator is a `function*` declared before its statement,
    made a factory by `adaptAwaiter(fn)` where it's used, where Kotlin's
    is `const fn = adaptAwaiter(function* fn(…) {…})`; it's typed
    `@returns {Generator<any>}`, as what it yields is the runtime's, and
    `yield;` yields nothing, where Kotlin's `yield null` fails strict
    tsc against `Generator<{}>`.
  - **Lua:** the runtime has promises and `async` (vendored patch 0006),
    which Kotlin's Lua calls but lacks (its issue 144); a block starts
    at once and runs to its first pause.
  - **C#:** `Async` is `global::TemperLang.Core.Async`, in Program.cs
    too, as a library named `async` hides it; a `yield` yields
    `default!`, where Kotlin's `null` warns.
  - **Java:** `awakeUpon` is `whenComplete` with an expression lambda,
    where Kotlin's is `handle` with a block one; a state machine's
    lasting locals are cells, as tempest's captured locals are, where
    Kotlin's share one class.
  - **C++:** settling a promise from outside any `async` block drains
    the queue (vendored patch 0005), so `async-broken-early` and
    `await-in-loop` pass, which Kotlin's C++ fails (its note 520); and
    C++20 code can `co_await` a Temper promise.
  - **State machines** (Rust, Java, C++) are tmpl's `if`/`else if`
    chains, where Kotlin's are `switch`es; as Kotlin's, a step sets its
    state to `-1` before it runs, so a step that failed is done.
  - **Closures in a generator's loop** each get their own copy of a
    loop's `let` they capture, as the interpreter and generator
    backends give each pass its own binding. Kotlin's state machines
    share one binding where the `let` is used after a pause (a bug);
    tempest moves every local up, so it copies, and fails on a captured
    `let` assigned after it's set, which a copy can't share.
  - **Python:** `nextSafe` is `next(g, None)`, where Kotlin's bare
    `next` raises `StopIteration` once the generator is done (a bug).
  Test: funtests `control-flow/async`, `async-waiters`,
  `async-broken-early`, `await-in-loop`, and `actor-run`;
  programs/functions.temper.md's generator, on every backend; and
  settle/run.cases, "closures made in a generator's loop, each pass its
  own as in a function".

Bugs tempest would inherit with its backends (milestones 5 and 6) if it
copied Kotlin's:

- **`==` and `!=` between Booleans** don't translate in Lua and JS, and
  Python emits code that fails only when run.
  Test: programs/operators.temper.md (`same`), on every backend.
- **Initialization order:** type declarations go before module-level
  values, so a static can read a value not set yet (C++ reads zero,
  Python fails with `NameError`); Python's statics also precede the
  functions they use. tmpl should keep source order. Rust and C++
  needed `init` first until #636, which initializes them on first use,
  as tempest's do too (api-compat.md, proposal 2); init timing still
  differs per backend.
  Test: programs/classes.temper.md (`Rect.unit`) and funtests
  classes/static-properties and classes/static-properties-scope, on
  every backend; Rust's and C++'s `FIRST_USE` runs.
- **Types as values** (`whatever(): Type { …; Void }`) come out
  inconsistently, and mostly wrong: Python returns `NotImplemented`
  after a string describing the value, JS the string `"Void: Type"`,
  C++ `0` as a `std::shared_ptr<void*>`, Lua `nil`, and Java a raw
  `Class` (`Void.class`, which `-Xlint` warns on); only C#'s
  `typeof(void)` and Rust's `TypeId::of::<()>()` are values of a type
  type. Harmless in the one test, which discards the value. Tempest
  should give each backend a real type value (not yet, 2026-11-04).
  Test: funtest classes/angle-call (not yet translating).
- **Rust list indexing panics,** as `ListedTrait::get` unwraps. Not a
  bug after all: core documents `Listed.get` past the end as a panic,
  as tempest's interpreter and backends give (checked 2026-11-04).
- **Rust integer literals as receivers** (`x & (0x7F >> n)`) give
  `127.wrapping_shr(…)`, which rustc rejects ("can't call method
  `wrapping_shr` on ambiguous numeric type", E0689). Tempest's gives the
  literal a suffix, `127i32.wrapping_shr(…)`.
  Test: temper-be-rust/tests/functional.rs, `literal_shifts`.
- **Rust first use reads a library partly initialized:** since #636 a
  read of a module-level value initializes the library if the value
  isn't set (`temper_core::initialized`), but reads a set one at once,
  even while another thread's init is still running the rest of the top
  level. So a thread that calls a library while another's first use
  initializes it can run alongside the init and change what the init
  logs or computes: Kotlin's `module-state` from 8 threads with no
  `init` logged `hi 19 of 3 200` and the like, where the init's second
  `report()` should give `hi 2 of 3 13`, in 9 of 300 runs (tempest's: 0
  of 300). Tempest's reads go through the crate's
  `support::initialized`, which reads at once only where the library's
  init has finished or runs on this thread, and otherwise initializes
  it first or waits for the thread that is, through the same
  `temper_core::initialized` (done 2026-10-31). It costs one atomic
  load per read. Its locked self-update (`n = n + 1`, below) goes
  through it too, the update as the read, so a function's update
  initializes the library as Kotlin's getter read does. C++'s entry points wait in `init_once`, so Kotlin's
  C++ doesn't have this. Kotlin's Rust writes `init_on_first_use()` in
  every crate, which its `#![allow(warnings)]` keeps quiet where nothing
  calls it; tempest's, which builds without warnings, writes it only
  where something does.
  Test: temper-be-rust/tests/functional.rs, `runs_and_lints`
  (`FIRST_USE`, `module-state` from 8 threads).
- **Rust module `var`s lose concurrent updates:** `n = n + 1` reads
  `n` under one lock and writes it under another, so two threads can
  both write the same value. Safe in Rust's sense, but not what a
  counter wants. Tempest's Rust backend reads and writes such an update
  under one write lock (done 2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-rust)).
  Kotlin's Rust output also fails clippy's deny-level
  `self_assignment` on `zero = zero` despite its `#![allow(warnings)]`,
  which allows only warn-level lints; tempest's too, being faithful,
  so its test allows that lint.
  Test: temper-be-rust/tests/functional.rs, `runs_and_lints`
  (`THREADS`).
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
  Test: temper-be-js/tests/functional.rs, `source_maps`;
  tempest/tests/cli.rs, `js_writes_a_package`.
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
  Test: programs/string-order.temper.md, on every backend.
- **C# formats numbers by the current culture:** Kotlin's C# writes
  `Int32.toString()` as the runtime's `Core.ConvertToString(n)`, which
  is `n.ToString()`, so under `sv-SE` (and other cultures whose minus
  sign is U+2212) `-1` logs as `−1`; `Boolean.toString()` lowercases by
  the current culture too, harmlessly for `True` and `False`. Tempest's
  C# backend formats in the invariant culture
  (`n.ToString(CultureInfo.InvariantCulture)`), and its tests run under
  `sv-SE` with the .NET analyzers' culture rules as errors (done
  2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-c)).
  Test: temper-be-csharp/tests/functional.rs, `runs` (under `sv-SE`,
  analyzers' culture rules as errors).
- **C# `Boolean` comparisons:** Kotlin's C# writes `a < b` on booleans
  (it turns `a.CompareTo(b) < 0` into `<`), which C# doesn't have, so
  the library doesn't compile (CS0019). Tempest's keeps
  `a.CompareTo(b) < 0`, and its C# tests compile and run it.
  Test: temper-be-csharp/tests/functional.rs, `HIDING` (`before`);
  programs/operators.temper.md (`same`).
- **C# bodies:** besides numbered names, Kotlin's C# wraps a function
  that returns early in labeled blocks with a `goto` and a `return__N`
  variable, keeps `zero = zero` (C# warning CS1717), and names
  namespaces by one-letter aliases (`C::Core`). Tempest's returns
  directly, drops self-assignments, and writes `using`s.
  Test: temper-be-csharp/tests/goldens/.
- **Java module `var`s are plain statics,** so a function's update from
  several threads can be lost and other threads may read a stale value.
  Tempest's Java backend makes a `var` that functions write `volatile`,
  and locks the class for an update from itself alone (done 2026-10-30;
  [tmpl/profiles.md](tmpl/profiles.md#as-built-java)).
  Test: temper-be-java/tests/functional.rs, `runs` (`THREADS`).
- **C++ module `var`s aren't thread-safe:** they're plain storage
  (since #636, `GlobalSlot`s read and written directly), so functions
  updating them from several threads race (undefined behavior in C++):
  ThreadSanitizer reports it, and 8 threads × 1,000 updates of `ticks =
  ticks + 1` give about 6,500. Tempest's C++ reads and writes such
  `var`s in functions only under a mutex, a run of updates under one
  lock (done 2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-c-1)).
  Init itself was a plain `static bool` check before #636; Kotlin's is
  now `init_once`, thread-safe, which tempest's uses too (2026-10-31),
  where it had `std::call_once`, which would deadlock now that the
  init's own calls of exported functions run it again. Since #636 an
  exported `var` is read through an accessor, `x()`, giving a `const
  &`, which races with the functions' writes as the variable did;
  tempest's accessor for a `var` that functions write gives a copy,
  read under the module's lock.
  Test: temper-be-cpp/tests/functional.rs, `runs` (`module-state` from 8
  threads, under ThreadSanitizer).
- **C++ statics give `const &`s to mutable objects:** since #636 a
  public static is read through `Type::x()`, a `const &`, but a
  `shared_ptr<T>` static's object isn't `const`, so any caller can
  change a shared object through it (`Type::policy()->set_limit(0)`),
  as through a module-level `let` of a class type. Tempest's is the
  same for now (a Kotlin fix to make).
- **C++ order of evaluation:** Kotlin's C++ before #601 passed operands
  straight to calls and to its `cat`, whose order C++ leaves
  unspecified, so `module-state`'s `"${bump()} … ${total}"` logged `hi 1
  of 3 10` under g++, which reads `total` first. Kotlin's now hoists
  every call's operands into a lambda's locals; tempest's computes an
  operand into a local first only where the order shows (done
  2026-10-30).
  Test: programs/module-state.temper.md (`hi 1 of 3 11`) on C++.
- **C++ shifts and comparisons:** Kotlin's C++ writes `a << b` and
  `a >> b` for `Int`, which C++14 leaves undefined for a negative `a`
  or a count past 31, where Temper masks the count; `<=>` on two string
  literals as `Compare::cmp("b", "a")`, which compares pointers; and
  fails on `Boolean`'s `<=>` ("unsupported C++ binary operator:
  Compare::cmp"). Tempest's uses the runtime's `Int::shl` and
  `Int::shr` (C++'s `>>` only for a literal count in range), compares
  literals as `std::string`s, and calls `Compare::cmp` for `Boolean`.
  A count in range doesn't make `<<` defined either: `5 << 30` on an
  `Int` overflows, undefined before C++20 (checked 2026-11-04);
  `Int::shl` shifts as `uint32_t` and casts back, wrapping as Temper
  does, and never writes `<<` itself.
  Test: funtest types/int/shifty (`shl32(0x05, 30)`, `-1 << n`, and
  `Int64`'s) under UndefinedBehaviorSanitizer, which reports a signed
  shift's overflow; temper-be-cpp/tests/functional.rs, `HIDING`
  (`shifts(33)`); programs/operators.temper.md (`"b" <=> "a"`, `same`).
- **C++ argument-dependent lookup** (found 2026-11-04 in Kotlin's
  be-cpp): a library function called unqualified, with an argument
  whose type is `std`'s (`std::string`, `shared_ptr`, `vector`), is
  also looked up in `std`, which can pick `std`'s function by the same
  name, as `swap(xs, ys)` for two lists finds `std::swap(shared_ptr&,
  shared_ptr&)`, better than a `const &` parameter, and `move(xs)`
  `std::move`, both failing to compile, or worse, compiling to the wrong
  call. Tempest's C++ did the same until 2026-11-04; it now qualifies a
  call of the library's own function with its namespace
  (`hiding::swap(xs, ys)`) wherever an argument isn't a scalar, which
  turns that lookup off. `quoted(s)` for a `std::string` was safe, as
  overloading prefers the library's non-template over `std::quoted`.
  Test: temper-be-cpp/tests/functional.rs, `HIDING` (`quoted`, `swap`,
  `move`).
- **C++ bodies and headers:** besides numbered names, Kotlin's C++ wraps
  every loop and branch body in an extra block, a function that returns
  early in a labeled block with a `goto` (whose unused labels fail
  `-Wall -Werror`), keeps `zero = zero`, writes every module-level value
  as a global even if only the top level uses it, builds strings
  through a `std::stringstream` (`cat`), and declares every function in
  the header, exported or not. Tempest's returns directly, uses `+` and
  `std::to_string`, keeps top-level-only values as the init's locals,
  and declares only exports.
  Test: temper-be-cpp/tests/goldens/, and `runs` with `-Wall -Werror`
  and the rest of `STRICT`.
- **C++ temporaries:** Kotlin's C++ returns lists as `shared_ptr`s, so
  iterating one in place (`for (auto const &x : *obj->list())`) reads a
  vector already freed. Tempest's C++ will name such a list first
  ([tmpl/profiles.md](tmpl/profiles.md#as-built-c-1), "Temporaries that
  die in use"); tmpl has no lists yet. With classes (2026-10-30),
  chained calls on returned objects stay inline, safe in C++, and an
  argument read from a `var` field is copied first (below), both
  checked under ASan.
  Test: temper-be-cpp/tests/functional.rs, `LIFETIMES`, under ASan.
- **Lua exports are copied once:** Kotlin's Lua module fills its
  exports table after its top level runs (`exports.ticks = ticks`), so
  an exported `var` that the library's functions write stays at its
  first value for users: after `tick()` twice, `require("…").ticks` is
  still 0. Tempest's Lua keeps such a `var` in the exports table itself
  (`exports.ticks = …` in `tick`), and its test reads it back (done
  2026-10-30; [tmpl/profiles.md](tmpl/profiles.md#as-built-lua)).
  Test: temper-be-lua/tests/functional.rs, `runs` (`LIVE`).
- **Lua `Boolean` comparisons:** Kotlin's Lua writes `a < b` on booleans,
  as its C# does, which Lua refuses at run time ("attempt to compare two
  boolean values"). Tempest's writes the order as logic (`not a and b`),
  or through the runtime's `bool_cmp` where the right side has effects
  that `and` would skip.
  Test: temper-be-lua/tests/functional.rs, `HIDING` (`print`, `math`);
  programs/operators.temper.md (`same`).
- **Lua's `goto`:** Kotlin's list of Lua keywords leaves out `goto`
  (reserved since Lua 5.2), so a Temper name `goto` gives Lua that
  doesn't parse; tempest reserves it. And Kotlin says it supports Lua
  5.1 (its docs, and `lua >= 5.1` in its rockspecs, which tempest keeps
  for compatibility), but its output uses `goto` for every early return
  and loop exit, which 5.1 can't parse (LuaJIT can). Tempest's uses
  `goto` only for `continue` and for leaving an outer loop, so a program
  without those parses on 5.1 too.
  Test: temper-be-lua/tests/functional.rs, `HIDING` (`end(goto: Int)`),
  for the name; Lua 5.1 isn't tested.
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
  Test: temper-be-lua/tests/goldens/.
- **Stale files in `temper.out`** after renames break builds; record the
  files each backend writes and delete only those no longer written. Tempest empties a library's directory before writing it
  (2026-10-30), which also drops what builds and test runs left there.
  Test: tempest/tests/cli.rs, `build_writes_temper_out`.
- **`tempest build` and `temper.out` (2026-10-30):** the same layout,
  `temper.out/BACKEND/LIBRARY/…`, package files, and runtime copy
  (`temper-core`, without its dotfiles and own tests, Lua's rockspec named
  for the version), checked against Kotlin's for the functional tests
  (`crates/tempest/tests/kotlin_layout.rs`). Intended differences: source
  maps only for JS's modules so far; no zip for LuaRocks; a
  `CMakeLists.txt` per C++ library (decided 2026-10-30); a library's
  directory emptied before it's written, so no stale file survives (the
  bug above); and no `data` backend, so the default backends are
  Kotlin's but that one. A library's config is read for its `export let
  KEY = "VALUE";` lines only, where Kotlin runs it as Temper.
  Test: tempest/tests/kotlin_layout.rs.
- **An imported `var` that its module writes later** (2026-10-30):
  Kotlin's backends disagree on what the importer reads, Python a copy
  made at import (as the interpreter's `let` gives), JS the current
  value (a live `import` binding). Tempest copies it when the import
  runs, explicitly in tmpl, so every backend reads what the interpreter
  does ([tmpl/modules.md](tmpl/modules.md)).
  Test: programs/modules/, on every backend.
- **Several modules' init** (2026-10-30): Kotlin's Java and C# load
  only the root module, leaving imported modules to lazy class
  initialization, and Lua's root doesn't require modules it doesn't
  import; C++ names each module's init for its path's last part, so
  two can clash; and C++ needs std for a library whose tests only use
  it. Tempest runs every module's init in Temper's order on every
  backend, and keeps tests' needs out of the library's own build. Its
  package files name what's imported, where Kotlin's name what's left
  after folding.
  Test: programs/modules/ and programs/imports-library/, on every
  backend; temper-tmpl/tests/text/problems.cases, "module init order
  (F5, R6)".
- **Importing a library by its bare name:** Kotlin resolves
  `import("other-lib/sub")` by the library's name, but, by reading its
  `ModuleAdvancer`, a bare `import("other-lib")` looks for a directory
  named `other-lib` inside that library's root, since the path after the
  name is taken from index `slash + 1` with no slash found (not run;
  `import("other-lib/")` works). Tempest takes both to mean the library's
  root module.
  Test: programs/imports-library/ (`import("modules")`), on every
  backend.
- **Generated APIs:** proposals in api-compat.md cover numbered names,
  Lua's null sentinel, Rust's optional parameters, Java's `to_` and
  `<Lib>Global`. Not yet covered: C#
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
  Test: settle/types.cases, "function types compared part by part" and
  "list literals typed from the type wanted".

- **Kotlin Temper's move to its newer types** (PR #500, in review
  2026-10-29) carries `Never<T>` with its fallback type into its tmpl,
  as tempest's tmpl does, and keeps "may bubble" as a result type beside
  a separate signature, which tempest avoids by treating bubbling as an
  effect. Its ad hoc function types now also separate whether a method
  has `this`, a question to answer whenever tempest interns function
  types. Notes only; no change to tempest's plans.
- **Classes in the backends** (2026-10-30; [tmpl/roadmap.md](tmpl/roadmap.md#state),
  "classes"): Kotlin-compatible in names, constructors and factories
  (Java and JS builders and `new(props)`, Rust `builders`, C++ `make`),
  properties (Python `@property`, Java `getX`/`setX`, C# properties, JS
  accessors, Rust and C++ getter methods, Lua `T.get`/`T.set`), and
  layout (Java's and C#'s file per type), with these fixes and
  differences on purpose:
  Test: programs/classes.temper.md and `kit::CLASSES` on every backend,
  with each backend's goldens; the items below name more.
  - **Private members read on another instance** work on every backend;
    Kotlin's emit a getter or attribute that doesn't exist (Java
    `other.getHidden()`, Python `other.hidden`).
    Test: programs/classes.temper.md (`Counter`).
  - **Statics initialize in Temper's order,** in the module's init where
    the class is: Kotlin's Python ran them before functions they call
    (`NameError`), its C++ at namespace scope in no order across files
    (since #636 in the module's init too), and its Java and C# when the class is first touched. Java keeps
    Kotlin's `static final` field where the initializer can't tell when
    it runs; C# reads a static through a property of the module class.
    Test: programs/classes.temper.md; funtests classes/static-properties
    and classes/static-properties-scope.
  - **Abstract members are abstract:** Python `@abstractmethod`s, with
    abstract properties declared; C++ `= 0` with `override` on
    overriders; JS type-only fields, so a mixin can't copy a stub over
    an inherited default (Kotlin's JS logs `undefined` for an abstract
    `a()` beside another interface's default); Lua none, where Kotlin's
    call a `temper.virtual()` its runtime doesn't define.
    Test: funtests interfaces/pure-virtual and
    interfaces/property-members, with the type checkers and
    `-Wsuggest-override`.
  - **Diamond defaults** follow Temper's pick (nearest, breadth-first,
    then `extends` order) on every backend, by forwarders only where the
    host would pick otherwise; C++ interfaces are virtual bases (Kotlin's
    can duplicate one); C# hiding members say `new`.
    Test: temper-be-csharp/tests/functional.rs, `DISPATCH`;
    settle/run.cases, "an interface's body beats another's abstract
    member".
  - **C++ aliasing:** an argument read from a `var` field is copied into
    a local first, since Kotlin's `const &` lets a callee that reassigns
    the field read freed memory (checked under ASan).
    Test: temper-be-cpp/tests/functional.rs, `LIFETIMES`, under ASan.
  - **Lua:** `is` tells apart same-named classes of two modules (Kotlin's
    compares bare names); a static named like a runtime key (`methods`)
    gets a `_` rather than replacing it; a plain `toString` is
    `__tostring`.
    Test: temper-be-lua/tests/functional.rs, `MEMBERS` and
    `MEMBERS_READ`.
  - **Python:** a library named like a standard module (`types`) gets a
    `_` (`types_`), which Kotlin's hides; `@operator("==")` also defines
    `__eq__` (surface.md). A plain `toString` is `__str__`, as Kotlin's
    is since #641.
    Test: temper-be-py/tests/goldens/imports-types.py (`types_`).
  - **Rust:** `Display` for a plain `toString`, `PartialEq` for `==`
    (surface.md); only `var` fields are locked, each in its own
    `RwLock`, read through getters so no guard outlives a read.
  - **Java:** builders null-check reference parameters; C# keeps braces
    around every `if`, so no `else` attaches to the wrong one. C#'s
    `ToString` is an `override`, as Kotlin's is since #641.
  - **Kept, not dropped:** declarations nothing uses (an unused interface,
    a class whose statics Kotlin inlines) stay in tempest's output.
- **Nullables in the backends** (2026-10-30; [tmpl/roadmap.md](tmpl/roadmap.md#state),
  "nullable types"): Kotlin-compatible spellings (Python `T | None`, JS
  `T | null`, Rust `Option<T>`, Java `@Nullable` boxes, C# `T?`, C++
  `NullableParam` or a null `shared_ptr`, Lua `temper.null`), `orelse`
  as each host's `try` (Rust a closure's `Result`, Lua `pcall`), with
  these fixes and differences on purpose:
  Test: programs/nullables.temper.md and `kit::NULLABLES` on every
  backend.
  - **`panic()` isn't caught by `orelse`:** Kotlin's Python, Java, C#,
    C++, Lua, and JS throw the same thing for `panic()` as for
    `bubble()`, so `let p(): Int throws Bubble { panic() }` and `let
    g(): Int { p() orelse 2 }` give 2, silently, where Temper's docs
    call a panic unrecoverable. Tempest's Python raises
    `BaseException`, Java an `AssertionError`, C# `Environment.FailFast`,
    C++ the runtime's `panic()`, and Rust `panic!()`, none of which
    `orelse` catches. Not yet in JS and Lua, which still match Kotlin's
    (their runtimes' panic is their bubble).
    Test: each backend's nullables golden (`checked`).
  - **`(do { panic(); 1 }) orelse 2`** fails Kotlin's build ("does not
    export symbol return__3"); tempest translates it, Rust leaving out
    the value after the panic (2026-11-04), which rustc's
    `unreachable_code` warned of.
    Test: programs/nullables.temper.md (`stops`), on every backend.
  - **Lua `Int64.toInt32()`** calls a runtime function only Lua before
    5.3 has, so on 5.4 `x.toInt32() orelse -1` silently gives -1;
    tempest uses the range-checked Float64 conversion.
    Test: funtest types/int/limits (`Actually big: no`) on Lua 5.4.
  - **C# numbers by the invariant culture,** formatting and parsing
    (`Float64.toString`, `String.toFloat64`), where Kotlin's runtime
    under `sv-SE` logs `1.5` as `1,5.0` and fails to parse `"1.5"` (#641
    made its case folding, `StartsWith`, and `IndexOf` culture-free,
    but not these); braces kept on every `if`, so no dangling `else`
    (above).
    Test: temper-be-csharp/tests/functional.rs, `runs`, under `sv-SE`
    (funtests types/float/*), and `DISPATCH` for the braces.
  - **Java:** `==` on two narrowed boxes unboxes one, and a conditional
    mixing a null box with a scalar boxes the scalar, as Kotlin's does
    by writing `if`s; `==` with a side that may be null is
    `Objects.equals`.
    Test: programs/nullables.temper.md (`sameInt`) on Java.
  - **Narrowing after an early return** (above, "Null and `is`
    narrowing") is used by the shared `nullables` program, so Kotlin
    rejects that program as written; an intended difference.
  - **`??` on a `var`** reports "Cannot assign to Int32 from Int32?" in
    both, since the macro reuses the name and a `var` isn't narrowed.
    Recommended for both: a temporary for a `var` subject.
- **Generics in the backends** (2026-10-31; [tmpl/roadmap.md](tmpl/roadmap.md#state),
  "generics"): Kotlin's surface for generic functions, classes, and
  interfaces and core's lists, maps, and pairs (Python `TypeVar`s and
  `Sequence`, JS `@template` and frozen arrays, Rust bounded type
  parameters and `temper_core`'s `List`, Java and C# generics with
  `java.util` and `System.Collections.Generic` types, C++ templates,
  Lua tables), with these fixes and differences on purpose:
  Test: programs/generics.temper.md and `kit::GENERICS` on every
  backend.
  - **JS `getOr` of a map with null values** gives the fallback in
    Kotlin's runtime (`mappedGetOr` is `map.get(key) ?? fallback`), so a
    `MapBuilder<String, Int?>` with `"n"` set to null logs `0` for
    `mb.getOr("n", 0) ?? -2`, silently, where Temper gives null and
    `-2`. Tempest writes `m.has(k) ? m.get(k) : fallback` where the value
    may be null.
    Test: programs/generics.temper.md (`mb.getOr("n", 0) ?? -2`).
  - **Rust:** a class bound (`T extends Named`) names a trait Kotlin
    never writes (`NamedTrait`), and a string literal for a type
    parameter that's `String` stays `&str` (`Box::new("boxed")` for a
    `Box<Arc<String>>`); neither compiles. Tempest bounds a class's type
    parameter by `Into` the class, and writes the literal as the
    `Arc<String>` the type wants.
    Test: programs/generics.temper.md, under clippy.
  - **C++ `const &` aliasing of a list field:** Kotlin passes
    `this->list` by reference to a method that reassigns the field, so
    the parameter then reads the new list (`"fresh"` where Temper gives
    `"kept"`), silently. Tempest copies the `shared_ptr` first, as it
    does for objects; the ASan-checked `lifetimes` program covers lists
    too.
    Test: temper-be-cpp/tests/functional.rs, `LIFETIMES` (`Names`),
    under ASan.
  - **Java `==` on two boxes from generic code** (`a[0] == b[0]` for
    `List<Int>`s) compares references in Kotlin's (`Core.listGet(a, 0)
    == Core.listGet(b, 0)`, false for 1000); tempest unboxes one.
    Test: programs/generics.temper.md (`firstSame`) on Java.
  - **Type checkers:** Kotlin's JS fails `tsc --checkJs --strict`
    (frozen arrays as `Array<T>`, `mapBuilderConstructor()` without the
    argument its declaration wants); tempest types `List` as `readonly
    T[]`, casting for the runtime's helpers. Python's type variables are
    invariant, as Temper's are, where Kotlin makes an interface's
    covariant.
    Test: the JS `runs` (`tsc --checkJs --strict`) and Python `runs`
    (mypy and pyright, strict).
  - **C# `new TemperLang.Core.OrderedDictionary<K, V>()`,** qualified,
    since .NET 9's `System.Collections.Generic` has one too.
  - **Not compared:** the Kotlin layout test (`kotlin_layout.rs`)
    doesn't cover the generics tests yet.
- **Core's other types in the backends** (2026-11-02;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "core's other types"):
  Kotlin's surface for string indices, `StringBuilder`, `Empty`,
  `Deque`, `DenseBitVector`, and more of `String`'s and `Float64`'s
  members ([tmpl/profiles.md](tmpl/profiles.md), "As built, core's
  other types"), with these fixes:
  Test: `kit::CORE_TYPES` on every backend, with its checkers: funtest
  types/list/empty for Python's `Empty`, types/string/indices for C++'s
  `Compare::eq`, types/string/build for Rust's `build-main`.
  - **Python annotates `Empty` as `'_empty'`,** which nothing defines,
    so mypy and pyright reject it; tempest writes `tuple[()]`. Two
    locals that share a name in a Python function now share a type too
    (with or without null), and one declared inside another's
    initializer gets its own, since checkers give a name one declared
    type, and a walrus assigns before the outer annotation.
  - **JS's JSDoc names core types by names nothing defines**
    (`Deque_2<string>`, `Console_1`), which `tsc --checkJs` rejects.
  - **C++ writes `StringIndexOption.eq` as a bare `==`,** so comparing
    an index with itself fails `-Wtautological-compare` under
    `-Werror`; tempest calls `Compare::eq`. A private function's
    default-argument overloads are written only for the argument counts
    calls use, so none is unused.
  - **Rust names a library's binary for the library,** and cargo
    forbids one named `build`; tempest names it `build-main`. Generated
    code also passes clippy where Kotlin's only allows warnings.
  - **C# analyzers** (CA1834, CA1806, CS0219): one-unit appends as
    chars, `_ = new C(…)` for a constructor called for effect, and an
    unread module `let` of a literal left out.
  - **Named arguments that skip core's optional parameters** pass
    `Omitted` in tmpl, and each connection writes the language's null
    there, as Kotlin's output does.
  - Not fixed: Java's `round` is `Math.round`, which saturates and
    makes NaN 0, and Python's rounds half to even; Kotlin's Lua runtime
    miscounts and misparses on LuaJIT and Lua 5.1 without `utf8`, so
    `types/float/ops` and `types/string/read` run only on Lua 5.3 or
    later.
- **Divisions that can't fail, and maps, in the backends** (2026-11-03;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "divisions that can't
  fail"): `/` by an integer constant other than 0, and `%` by a
  positive one, is tmpl's `divSafe` or `modSafe`, as Kotlin's
  `DivIntIntSafe` and the like, wherever it is, so a function that
  can't bubble may use it, and each backend spells it as Kotlin's (Java's
  own `/`, Rust's `wrapping_div`, C#'s `Core.DivSafe`, C++'s
  `Int::div_safe` or its own `/`, JS's `a / b | 0`). Typing takes a
  builtin generic type's constructor's parameters from the type
  arguments `new` gives it, so `new Map<Int, Int>([])`'s `[]` is a
  `List<Pair<Int32, Int32>>`, as Kotlin types it. With these fixes:
  Test: `kit::DIVISIONS` on every backend, with its checkers.
  - **JS:** the runtime declares `listedReduceFrom`'s and
    `mappedToListWith`'s functions with the wrong parameters, and
    `mappedToMap`'s result as a `FreezeMap`, which isn't a `Map` to
    `tsc`; tempest's copy of the runtime fixes their JSDoc
    (`vendor/temper-patches/`), so calls need no casts.
  - **Python:** a one-expression literal passed where core's method
    wants a function that gives back the receiver's type parameter
    (`reduce`'s `fn (T, T): T`) is a `def`, as all of Kotlin's are,
    since pyright narrows `(2, 3, 4)` to literals and then rejects the
    lambda's `int`; a local made empty by `MapBuilder` or `ListBuilder`
    is annotated, as Kotlin annotates every local.
  - **Java and C++:** a function literal in the init block doesn't take
    the name of a local of the init around it (`(sum2, n) -> sum2 + n`
    in `int sum = …`), which Java rejects and C++'s `-Wshadow` reports;
    Kotlin numbers every name. C++ names no lambda parameter the body
    doesn't read (`-Wunused-parameter`), as it already did for
    functions, and passes a string literal where a runtime template
    deduces a type parameter as a `std::string` (`reduceFrom(vals,
    std::string(""), …)`), where `const char*` wouldn't compile.
  - **Java** reduces to another kind than the items' through Core's
    reducers (`listedReduceIntToObj`), as Kotlin's; to another object
    type, not yet. Java, C#, and C++ drop `orelse void`'s `void`.
  - **Lua** runs `types/int/shifty` only on Lua 5.3 or later: it needs
    64-bit integers and `countBetween`, which Kotlin's runtime gets
    wrong on LuaJIT. Rust's clippy allows `map`'s unused locals, which
    the test declares on purpose.
- **Running tests with std's `testing`, in the backends** (2026-11-03;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "backends running tests"):
  std's `Test.bail()` is each backend's connection, as Kotlin's, and its
  private method isn't declared: Python's `assert False,
  str(test.messages_combined())`; Rust's `return
  Err(temper_core::Error::with_optional_message(…))`, where Kotlin's
  binds the `Err` to a local and returns it if it isn't `Ok`; JS's
  `strict.fail(test.messagesCombined() ?? undefined)`, as Node's `fail`
  takes no null, where Kotlin's passes the null on (a `tsc` error with
  `@types/node`); Java's `Core.throwAssertionError(…)`; and Lua's
  runtime functions, named from every std key as Kotlin's are
  (`temper.test_assert`). Java's is `Core.throwAssertionError(…)`; C#'s
  `throw new AssertFailedException(test.MessagesCombined() ?? "")`, as
  Kotlin's; C++'s the runtime's `testBail(test)`. Names match Kotlin's,
  Rust's `TestStruct` fields included (`passing`, `failed_on_assert`,
  `messages`; 2026-11-03, after a chunk that numbered them): Rust's
  fields are named apart from methods (`fields_apart`), and only the
  private accessor functions tempest adds for `var` fields, which
  Kotlin's Rust doesn't have, take a number where a member has the name
  (`passing2()` beside the getter `passing()`). Python's private storage
  is `_messages`, as its naming knows fields' storage and private
  members take a `_`. The
  test runners are the tests' own where Kotlin's need packages: Mocha's
  built-in `xunit` reporter where Kotlin's uses `mocha-junit-reporter`;
  Java's JUnit launcher and C#'s small reflection runner over MSTest's
  attributes, where Kotlin's run Maven's surefire and `dotnet test`.
  std's own Java and C# names are Kotlin's: package `temper.std`, Maven
  `dev.temperlang:temper-std` with JUnit, and namespace
  `TemperLang.Std` with MSTest's packages (`TemperLang.Std.csproj` was
  `Std.csproj`). In C#, a name a namespace beside std's hides (`Core`,
  inside `TemperLang.Std`) is written `global::TemperLang.Core.Core`;
  Kotlin's grammar notes the case but doesn't handle it. C# gives a
  conditional with a block's value or an `orelse` in a branch as an
  `if` assigning the place, and `FormatFloat64` calls
  `MemoryExtensions.AsSpan` by its class, so it needs no `using
  System`.
  Test: programs/testing.temper.md and `kit::TESTS` on every backend.
- **Python's module-level names** (2026-11-04): a library module binds
  no public-looking name but its exports. Its imports are private
  aliases (`from temper_core import mapped_for_each as
  _mapped_for_each`, `from modules import sum_ as _sum_`), as are its
  `TypeVar`s (`_T`), function literals lifted to module level (`def
  _fn9`, where Kotlin writes `_fn_97`), loop labels there (`_outer`),
  and hoisted temporaries (`_t_0`, as before); names inside functions
  stay as they were. Kotlin's private declarations and temporaries
  already take `_`, but its imports (`TypeVar as TypeVar2`) and type
  variables (`T_1`) don't, so `from lib.mod import *` and completion
  offer them as if they were the library's. Each module also sets
  `__all__` to exactly its exports, which Kotlin's doesn't: compatible,
  since `import *` then gives only names that were public anyway (a
  module's submodules, which are attributes of a parent module once
  imported, are left out too). The library's `__init__.py` keeps
  star-importing the root module and naming its modules, and tests
  files, which nothing imports, keep plain names.
  Test: temper-be-py/tests/functional.rs, `runs`, which checks every
  generated library module and std's with `tests/module_names.py`.
- **std's `regex`, in Python and JS** (2026-11-03;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "std's regex"): std's
  connected regex members are each backend's runtime functions, as
  Kotlin's (`temper_core.regex.regex_compiled_found(self, compiled,
  text)`, `regexCompiledFound(this, …)`), and aren't declared in their
  classes. `doPure { … }` is its block's body in place, as Kotlin's
  `DoPureFn` erases the call. Python imports every name under a private
  alias (`from collections.abc import Sequence as _Sequence`, beside
  std's regex class `Sequence`; below, "Python's module-level names"),
  where Kotlin's aliases every import with a number (`Sequence as
  Sequence17`). Kotlin's runtime types four of its regex
  helpers wrong for `tsc` (`compiled` as a `RegExp`, `regexCompiledSplit`
  without `regexRefs`, and `out` as `[string]`); tempest's copy fixes
  their JSDoc (`vendor/temper-patches/`), so std calls them plainly.
  Test: `kit::WITH_STD` (regex/match, regex/zero-advance), with `tsc`
  and the Python checkers.
- **std's `regex`, in Rust, Java, C#, and C++** (2026-11-03;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "std's regex on four more
  backends"): each backend writes Kotlin's support code into std's
  output (Rust's `src/regex/support.rs`, Java's
  `temper/std/regex/Core.java`, C#'s `Regex/RegexSupport.cs` and
  `IntRangeSet.cs`; C++'s is in its runtime), and connects the same
  keys Kotlin's support networks do; std's crate's `Cargo.toml` has
  `regex = { version = "=1.12.2", optional = true }`, and a crate that
  imports `std/regex` asks for its feature, as Kotlin's. Rust writes
  `mod support; use support::*;`, where Kotlin's `pub use` re-exports
  nothing (the functions are `pub(crate)`) and rustc warns of it, and
  passes `self`, not `&self`, as clippy's `needless_borrow` wants;
  std's crate allows `dead_code` and `unused_variables` (std declares
  what it never uses, and `adjustCodeSet`'s body ignores `regexRefs`),
  where Kotlin's allows every warning in every crate. A statement
  `orelse` whose body jumps (`RegexFormatter.pushCode`'s `return`s) runs
  the body in place in a labeled block, as Kotlin's (`'ok: { 'orelse:
  { … } fallback }`), each `?` in it `if let Ok(v) = e { v } else {
  break 'orelse; }`. A constructor may run statements that don't touch
  `this` among its fields' assignments (`Regex`'s `let formatted`).
  Java calls `temper.std.regex.Core` by its full name everywhere, where
  Kotlin's does only where `temper.core.Core` is imported. Java's and
  C#'s naming keeps types apart from functions and module `let`s
  (`NamingRules::types_apart`), so std's local class `Digit` is `Digit`
  beside its module `let` `Digit`, as Kotlin's, which `RegexSupport.cs`
  names. C# names a class that a namespace this code is in hides from
  `global::` (a library `match`'s namespace `Match` and std's class
  `Match`), which Kotlin's doesn't. A Rust unused closure parameter
  named by a raw identifier is `_match`, not `_r#match`. Java connects
  neither `adjustCodeSet` nor `pushCaptureName`, whose entries Kotlin's
  Java network has commented out (tempest's table had listed them).
  Test: `kit::WITH_STD` on those backends, Rust's under clippy.
- **std's `regex`, in Lua** (2026-11-03;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "std's regex on Lua"): a
  `return`, `break`, or `continue` out of an `orelse` body returns a
  signal from the function `temper.pcall` runs, and an `if` after it
  redoes the jump, as Kotlin's `TryStatement` (`local ok, value, kind =
  temper.pcall(…)`; `return v, "return"`, `return "break", "flow"`).
  Kotlin's builds the checks for an unlabeled `break` and `continue`
  but drops them, so those fall through after the `orelse`; tempest's
  redoes every jump, in one `if … elseif …` chain, where Kotlin's nests
  them (`kind == "return"` first, then the jumps' names, where Kotlin's
  checks `kind == "flow"`).
  Test: `kit::WITH_STD` on Lua.
- **std's `temporal`, `Date` as a connected type** (2026-11-04;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "std's temporal"): Python,
  JS, Java, and C# connect `Date` to `datetime.date`, `globalThis.Date`,
  `LocalDate`, and `DateTime`, and leave std's class out, as Kotlin's;
  but where Kotlin keeps its members that aren't connected
  (`encodeToJson`, `decodeFromJson`) as private module functions, with
  `@json`'s `DateJsonAdapter`, tempest has neither yet (it makes no
  `@json` adapters). Rust, C++, and Lua declare the class, as Kotlin's.
  Differences that fix Kotlin's bugs (Kotlin fixes to make):
  C#'s `dayOfWeek` is `((int)d.DayOfWeek + 6) % 7 + 1`, where Kotlin's
  `TemporalSupport.IsoWeekdayNum` gives Sunday 0, and C++'s adds the
  same to the runtime's `getDayOfWeek`, which also gives Sunday 0; C#'s
  `toString()` formats in the invariant culture, where Kotlin's uses the
  current culture's calendar. Rust's `today()` is tempest's own support
  code, std only, where Kotlin's uses the `time` crate (and its
  `powerfmt`), so std's crate has no new dependency; its `temporal`
  feature is declared empty, and `today()` is re-exported (`pub use
  support::*`) as Kotlin's. JS calls `dateConstructor` for every
  `new Date(…)`, where Kotlin inlines `Date.UTC` for a literal year of
  100 or more. Kotlin's bugs kept, as the runtimes' own: C++'s
  `makeDate` and JS's constructor don't check the date (Feb 30 rolls
  over rather than bubbling), JS's `fromIsoString` gives an invalid
  `Date` rather than bubbling, and C++'s `today()` is local, not UTC.
  Test: funtest types/date on every backend, C#'s under `sv-SE`.
- **std's `json`, in Java, C#, C++, and Lua** (2026-11-04;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "std's json on four more
  backends"): mostly as Kotlin's. Java boxes an override's result where
  the overridden method's is a type parameter (`Boolean
  decodeFromJson`) by returning the box, Java boxing at `return`, where
  Kotlin also moves the body into a private helper; for a boxed
  parameter both do, tempest's helper named `encodeToJsonUnboxed`
  (numbered past the class's names) where Kotlin's is the name made
  distinct (`encodeToJson__12`), a private name either way. C# has
  Kotlin's `Optional<T>` for `T?`, wrapping and unwrapping at calls; its
  tests allow the analyzers' CA1707 and CA1725, which std's names, as
  Kotlin gives them, break. C++ writes `static_cast<void>(x);` after a
  scalar local nothing reads, where `-Wunused-variable` would stop the
  strict build. Lua casts to `Mapped` with `cast_to_map`, where Kotlin
  calls `cast_to_mapped`, which its runtime lacks (its second
  `cast_to_map` is that check), and writes a cast of a cast as one `@as`,
  the outer, where LuaLS would read only the first.
  Test: funtest types/json/syntax-tree on those backends, with their
  checkers.
- **std's `json`, in Python, JS, and Rust** (2026-11-04;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "std's json"): std's `json`
  is all Temper, as Kotlin connects none of it. A function that ends in
  `bubble()` ends in its statement (JS's `bubble();`, Rust's `return
  Err(…)` as the function's value), where tempest returned its value.
  Python casts to a core type by its Python class (`cast_by_type(x,
  dict)`), as Kotlin's `argless` types, and annotates a local a cast
  initializes, which pyright otherwise types as the cast's argument.
  `stringToInt64` gives a `bigint`, which Kotlin's JSDoc says is a
  `number`; tempest's copy of the runtime says `bigint`
  (`vendor/temper-patches/`), so `tsc` passes std's
  `JsonNumericToken.asInt64`. Rust
  writes a value `orelse` whose fallback bubbles as `if let Ok(v) = … {
  v } else { … }`, and a constructor's statements that read fields
  already set (`JsonTextProducer`'s `this.stack.add(…)`) among its
  fields' locals; std's crate allows clippy's `collapsible_if`,
  `manual_range_contains`, and `question_mark`, which its json source
  trips, where Kotlin's allows every warning.
  Test: funtest types/json/syntax-tree on those backends, with their
  checkers.
- **Builtin values as `AnyValue`, and `List.of`, in the backends**
  (2026-11-04; [tmpl/roadmap.md](tmpl/roadmap.md#state), "builtin
  values as `AnyValue`"): APIs as Kotlin's (`thing1: AnyValue` is
  Rust's `temper_core::AnyValue`, C++'s `shared_ptr<AnyValueBase>`,
  Java's `Object`, C#'s `object`); bodies differ where Kotlin's don't
  pass the checkers tempest runs.
  Test: `kit::ANY_VALUES` on every backend, with its checkers.
  - **Python** annotates `AnyValue` as `object`, where Kotlin's writes
    `typing.Any`: the same values fit, and a checker makes a reader
    check what it has. A cast to an interface with abstract members
    (`y as Apple`) passes mypy's `type-abstract` on that line, since
    tempest's interfaces have `@abstractmethod`s and Kotlin's are
    empty; `x as T` for `x: T?` is `cast_none(x)`, Kotlin's an `if x is
    None: raise`.
  - **C++** boxes a string literal as a `std::string`
    (`any_box(std::string("two"))`), where Kotlin's boxes the `const
    char*` (`any_box("two")`), an `AnyValueBox<const char*>` that a
    check for a boxed `std::string` wouldn't match (not seen to fail:
    no functional test casts a builtin back from `AnyValue`).
  - **JS:** be-js's `listBuilderSplice` JSDoc takes neither a null
    `index` nor a read-only `newValues`, so strict tsc rejects Kotlin's
    own calls of it; tempest patches the types
    (`vendor/temper-patches/0002-js-splice-jsdoc-types.patch`), a
    Kotlin fix to make.
  - **Rust** casts a nullable value with `x.and_then(cast::<T>)`, as
    Kotlin's, where tempest had passed the `Option`, and clippy's
    `approx_constant` is allowed in the tests for as-expr's `3.14`.
- **Labeled loops and classes declared in functions, in the backends**
  (2026-11-03; [tmpl/roadmap.md](tmpl/roadmap.md#state), "labeled
  loops' jumps"): a class declared in a function is declared at its
  module's level, as Kotlin's `ClosureConvertClasses` does, but only one
  that names none of the function's locals (Kotlin's captures them in a
  closure record; tempest's says "not yet"). Code after a `while (true)`
  that nothing breaks is left out, where javac and rustc's
  `unreachable_code` reject it. The `eq` of a field-less class below an
  `@auto("==")` interface is just `other is C`, which clippy's
  `needless_bool` wants. With these:
  Test: `kit::LOOPS_AND_CLASSES` on every backend, with its checkers.
  - **Python** jumps out of an inner loop through the runtime's `Label`
    and `LabelPair`, as Kotlin's, named `outer` (numbered past any other
    name) rather than for the source's label; a module's file is named
    as its import is (`extra_class.py` for `./extra-class`); an empty
    `Deque` is annotated, as `ListBuilder`'s are; and a local nothing
    reads has pyright's `reportUnusedVariable` ignored.
  - **Rust** builds a generic class's builder (`ChangeBuilder<T>`); a
    local nothing reads, or a function literal's parameter, is named
    with a leading `_`, where Kotlin's `#![allow(warnings)]` hides the
    warning; a local declared in a loop is `mut` only if that loop's body
    assigns it more than once on a path; and the printer follows rustfmt
    on long conditions (operators leading continuation lines, `{` on its
    own line), field chains over `chain_width`, a lone `g(…)?` argument
    (`f(g(\n…\n)?,)`), and a block under `&*` as a last argument.
  - **Java** builds a generic class's `Builder<T>`; **C#** writes a
    module function's `where` constraints, and runs a block value ahead
    of its statement past reads of `var`s that no block value assigns.
  - **JS** makes a generic class's `static new(props)` generic itself,
    as `tsc` rejects a static member naming the class's type parameters,
    and declares an interface's inherited abstract properties again,
    since `tsc` sees nothing through the runtime's `type(I)`, so `J
    extends I` satisfies a bound of `I`.
  - **Lua** bounds `---@generic IT: lib.I`, and types a local of a
    function's type parameter `any`, since LuaLS knows the `---@generic`
    name only in the function's own annotations.
- **Parameters' defaults in the backends** (2026-11-01;
  [tmpl/roadmap.md](tmpl/roadmap.md#state), "parameters' defaults"):
  Kotlin's surface for optional parameters (nullable, defaulting to null
  where the language can say so, overloads in Java and C++), with
  plainer bodies: the default set in place where the language allows
  (`if end is None: end = …`, `end ??= …`, `let end =
  end.unwrap_or(…)`) or one expression into a local (`int end2 = end ??
  …`), where Kotlin writes an `if`/`else` into a fresh local for each,
  and nothing for a default of null. Rust's options builder leaves out
  the builder field nothing reads when a constructor has no required
  parameters, which rustc warns of in Kotlin's. Not yet: tempest
  doesn't report Kotlin's "Optional arguments must follow required
  arguments", and translates such a parameter as Kotlin's types would
  (backends give only the trailing ones defaults); Kotlin's Rust backend
  throws on it.
  Test: programs/defaults/ and `kit::DEFAULTS` on every backend.
- **Function values in the backends** (2026-10-31; [tmpl/roadmap.md](tmpl/roadmap.md#state),
  "function values"): Kotlin's surface for function types and `@fun`
  interfaces (each backend's plain function type, Java's
  `java.util.function` interfaces by signature or one synthesized in
  `PKG.function`, as Kotlin's), literals, references to functions, and
  calls of values, with prettier bodies (one-expression literals inline
  as lambdas, arrows, or one-line functions where Kotlin hoists each to a
  named function), and these fixes:
  Test: programs/functions.temper.md and `kit::FUNCTIONS` on every
  backend.
  - **C++ captures an assigned local by reference** (`[=, &count]`), so
    a closure returned from its function writes a dead stack slot:
    undefined behavior, which ASan reports as a stack use after return.
    Tempest keeps such a local in a `std::shared_ptr` the lambda copies,
    and a lambda that uses `this` copies the member's `shared_ptr` to it.
    Test: programs/functions.temper.md (the counter), under ASan.
  - **A static method as a value doesn't compile** in Rust
    (`Scaler::triple()`, a call with no arguments), Java (`triple`, after
    `import static`, which names no value), or C++
    (`Scaler::temper_statics::triple.value`). Tempest writes
    `Arc::new(Scaler::triple)`, `Scaler::triple` (a method reference), and
    `Scaler::triple`.
    Test: programs/functions.temper.md (`Scaler.triple`).
  - **Java:** a function value whose interface differs from the
    parameter's (`Float64Float64Int32` where `ToIntBiFunction<Double,
    Double>` is wanted) is adapted by a method reference, as Kotlin's,
    and so is one for a generic parameter (`IntUnaryOperator` where
    `Function<T, T>` is), which Kotlin passes as it is.
  - **C#:** a lambda's parameters are typed, so a generic callee infers
    its type arguments (`MakeHi((int i) => i + j)`), which Kotlin's
    didn't compile (its test notes it); a method group is cast to its
    delegate only where a generic method takes it.
  - **Python:** `ListBuilder.sort` casts its list for the runtime's
    `list_builder_sort`, annotated `List[T]` where a `ListBuilder` is a
    `MutableSequence`, which `mypy --strict` rejects in Kotlin's output.
  - **JS:** Kotlin exports every top-level function its hoisting makes
    (`export function fn`) and its `forEach` temporaries (`export const
    this_`, `n`, `i`); tempest's literals stay inline or local, and its
    temporaries unexported.
  - **Python closures made in a loop share its `let`s:** Kotlin's
    nested `def` reads the loop's one variable, so three closures of
    `let k = i * 10` all give `20`, silently, where Temper's `let` is
    fresh each time round (`0 10 20`). Tempest makes such a closure by a
    factory that takes the loop's locals it captures, `make_fn(k)`.
    Test: programs/functions.temper.md (`let k = i * 10`).
  - **Not yet:** a local function that calls itself; `@fun` interfaces as named Java
    or C# interfaces (the Kotlin-compatible variant uses Kotlin's
    types); the Kotlin layout test for these tests.

