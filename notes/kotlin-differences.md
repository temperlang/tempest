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
  named arguments").
- **Null narrowing from control flow,** through `&&`, `||`, `!`, `while`,
  and after an early `return`, `break`, `continue` or `bubble()`; Kotlin
  narrows only the branches of an `if` on one check. Done for names that
  aren't `var` ([typing.md](typing.md), "Null-check narrowing: the rules").
- **Type errors name declared types,** not the types of folded constants
  (Kotlin's `let s: String? = null; let q: Int = s` says "got Int32?").
  Done ([milestone-4.md](milestone-4.md), first slice).
- **One type representation** (`types.rs`), where Kotlin has `type/` and
  `type2/` bridged by `hackMapOldStyleToNew`. Done.
- **Core's generic interface methods** (`Listed.map<O>`, `reduceFrom<O>`,
  `reduceFromIndex<O>`, `Mapped.toListWith<T>`) as `@extension`
  functions, so core needs no exemption from "overridable methods don't
  allow generics". Planned ([typing.md](typing.md), "To explore later").
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
  assigns it, even `n++` inside a log, so output can scramble. Reorder
  only declarations, or reject assigning a top-level `var` outside
  functions. Tempest matches Kotlin for now, pending agreement
  ([milestone-3.md](milestone-3.md), "Top-level order").
- **NFKC for every non-ASCII word.** Kotlin checks only words with a code
  point past Latin-1, so `ªb` and `ab` are different names. Tempest
  matches for now (testing-and-milestones.md, "NFKC, as Kotlin").
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

## Bugs in Kotlin Temper

### Language and types

- **A function in a field, `obj.f(x)`,** fails ("incompatible with
  usage"); Kotlin's own `functions/constructor-callbacks` keeps its use
  inert. Tempest runs it (`call_method` in `interp.rs` falls back to the
  property); not yet tested, and typing must keep it (milestone 4 slice 4).
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
  `throws Bubble`. Tempest fails the same way; decide with builtins'
  signatures (milestone-4.md, slice 3).
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
