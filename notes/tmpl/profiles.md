# tmpl: profiles and lowering

Part of the [tmpl design](README.md).

## Level 1: One model, high-level to low-level, chosen by profile

**Decided (the direction):** one data model whose constructs range from
high-level (`for … of`, generators, `orelse` and bubbling, closures,
classes with properties) to low-level (plain loops, explicit error
results, state machines, closure conversion). Each backend declares a
profile of what it handles natively; shared passes lower only what it
lacks; a validator checks a tree against a profile. State machines are
only a tmpl post-pass (milestone-3.md, flat frames).

- **Decided (2026-10-29): a profile is a set of independent features
  plus a few either-or choices.** Targets don't line up on one scale:
  Go has closures but no exceptions, Lua has coroutines but no
  `continue`, C99 has `goto` but no closures. The choices are things
  like bubbling as exceptions or results (Level 2, below).
- **Passes run in one fixed order** (Proposed), each removing one
  feature and declaring what it may introduce (closure conversion may
  add classes; the state-machine pass needs labeled loops and a
  switch). `lower` picks the needed subset; the order never varies, so
  output is deterministic.
- **Lowered output is still tmpl,** checked by the validator against the
  profile after every pass in debug builds and tests.
- **Low-level nodes are defined when a target needs them.** Error codes,
  closure conversion and `goto`-like jumps serve only the [stress list](targets.md)
  today; the model leaves room rather than carrying dead code.

## Level 2: Profiles: features and choices

**Decided (2026-10-29):** a set of independent features plus a few
either-or choices. Proposed shape, plain data:

```rust
pub struct Profile {
    pub native: Features,          // bit set: each feature the target keeps
    pub failure: Failure,          // Exceptions | Results (| ErrorCodes, later)
    pub function_types: FnTypes,   // Plain | Named
    pub module_init: ModuleInit,   // Eager | FirstUse | ClassInit
    pub module_vars: ModuleVars,   // Plain | Synchronized
    pub surface: Surface,          // surface.md
}
```

`Profile::full()` keeps everything; it describes starting tmpl. A
backend crate's variant ([overview.md](overview.md#boundaries)) is a profile, `NamingRules`, and the
surface section, so a Kotlin-compatible variant and an idiomatic one
of the same language differ only in data. The tables below are each
language's first variant.

**Features, and Python's and Rust's first profiles** (native means
kept; otherwise lowered):

| Feature | Python | Rust | Others |
|---|---|---|---|
| Statements inside expressions | no: temporaries | native | only Rust |
| `ForOf` | native | native | all primary targets; `forEach` or index loops are the translator's choice |
| Labeled `break` and `continue` | no: flags or restructuring | native | Java, JS native; C# and Lua (`continue`) by `goto` |
| `Switch` (from `When`) | no: `if` chains | native (`match`) | C#, C++, Java, JS native; Lua `if` chains |
| Generators | native | no: state machine | C#, JS, Lua (coroutines) native; C++, Java state machine |
| `async` | native | native ([question 20](open-questions.md)) | C#, JS native; others state machine |
| Assigned captures | native (`nonlocal`) | no: cells | C#, JS, Lua native; C++, Java cells |
| Properties | native (`@property`) | no: methods | C#, JS native; C++, Java, Lua methods |
| `Void` as a value | no | native (`()`) | JS, Lua native |
| Left-to-right arguments | native | native | all but C++ |
| Null-safe `EqNullable` | native | native (`Option`) | JS for builtin types; others expanded |

| Choice | Python | Rust | Others |
|---|---|---|---|
| Failure | exceptions | results | exceptions everywhere else (Lua's `error` and `pcall`) |
| Function types | plain | plain | named in Java; plain elsewhere |
| Module init | eager | first use | C++ first use; Java, C# class init; JS, Lua eager |
| Module `var`s | plain | synchronized | C++, C#, Java synchronized; JS, Lua plain |

`Int32` wrapping isn't a pass: the key stays, and each translator
spells it, with helpers where the value-range analysis can't rule out
overflow (JS, Lua, Python).

**Pass order,** fixed: connected lowering (each `Lower` hook the
variant's connection table names, Proposed 2026-10-29,
[names.md](names.md#level-3-connected-declarations)); named function types; properties to methods;
generators and `async` to state machines (they add labels and
switches); `ForOf` (only for targets without loops over iterables);
failure; `EqNullable`; `Switch` to `if`; labeled jumps; statements out
of expressions; evaluation-order temporaries; cells; module `var`
synchronization; module init; `Void` values. Each declares what it
needs and what it may add, and the validator checks both.

## Level 3: Lowering for subset A

### Variants and the first profiles

**Decided (2026-10-29): every backend's first variant is
Kotlin-compatible:** its public surface (layout, public names,
signatures, and runtime library) matches Kotlin Temper's, as each
variant's API diff checks, so Chromarite's and Ordable's tours keep
working. Private names and bodies still get prettier. Other variants
follow as data, each with its own API baseline.

Proposed: variants are named on the command line, the first one by
the language's short name (`py`, `rust`, …), and later ones by a
suffix (`py-idiomatic`). A program's text header names the profile
it's lowered for.

Subset A's features, for `py`: everything native except statements
inside expressions. Its choices: exceptions, plain function types,
eager init, and plain module `var`s. Unlike the Level 2 table, Python
keeps `Void` as a value natively, as `None` ([question 30](open-questions.md)).

As built (2026-10-30, in `crates/temper-be-py`): a variant is a
`Variant { name, profile, rules, runtime_version }`, and the first is
`PY`, with no native features (statements inside expressions are
lowered) and Python's `NamingRules`. `lower` and the hoisting pass
aren't built yet: none of the 7 translated tests needs one, and a
program that does is reported as not yet. The either-or choices
(failure, module init, and the rest) wait for the passes that read
them.

### As built: Rust

As built (2026-10-30, in `crates/temper-be-rust`): the first variant is
`RUST`, a `Variant { name, profile, rules, runtime_version,
rust_version }` (`temper-core` 0.6.0 and `rust-version` 1.71.1, as
Kotlin's), with statements inside expressions native (as Rust blocks),
so subset A needs no lowering. Its choices, as Level 2's table has
them, are made by the translator for now, since no pass reads them yet:

- **Failure: results,** without a pass. In subset A only operators and
  calls bubble, and nothing catches, so a function that may bubble
  returns `temper_core::Result<T>`, each call or operator that may
  bubble takes `?`, and `Error` returns `Err`. The results pass waits
  for `orelse` (subset B and C).
- **Module init: once, on `init`,** as Kotlin's: `lib.rs`'s `init`
  calls the module's, which runs the init block in a `OnceLock`
  (`INIT_ONCE`) and keeps its result, so later calls give the same
  `Ok` or `Err`. Not first use: an exported function called before
  `init` panics reading module state, as Kotlin's does.
- **Module `var`s: synchronized,** where shared. A module `let` or
  `var` that only the init block uses is a local of it, which runs
  once on one thread. One that a function uses, or that's exported, is
  a `static` with a getter function (`pub` if exported), as Kotlin's
  "toppers": a `let` a `OnceLock<T>`, set once by init, and a `var` an
  `RwLock<Option<T>>`, `None` until init sets it. A read clones the
  value under the read lock, in the getter, so no guard outlives it.
  Rust's types make these safe across threads (no data race is
  possible in safe code), as Kotlin's are. Where Kotlin isn't quite:
  `count = count + 1` reads under one lock and writes under another,
  so concurrent updates can be lost. Here, a `var` assigned a value
  that reads it and else only locals and literals, through operators,
  is read and written under one write lock (the `module-state` test
  runs 8,000 such updates from 8 threads and gets 8,000). A value that
  calls anything, or reads other module state, is computed first and
  stored under the lock after, as Kotlin's, since holding the lock
  across a call could deadlock with a call that takes it again.

### As built: JS

As built (2026-10-30, in `crates/temper-be-js`): the first variant is
`JS`, a `Variant { name, profile, rules, runtime_version }`, with no
native features, as Python's: statements inside expressions wait for
the hoisting pass, and a program that needs it is reported as not yet.
JS keeps natively everything else subset A has: labeled `break` and
`continue`, `do … while` with `continue`, and `Void` as `undefined`.
Its choices, made by the translator for now, as Level 2's table has
them:

- **Failure: exceptions.** A bubble is a thrown error: the runtime's
  `bubble()` for `Error`, and its `divIntInt` and `modIntInt`, or
  BigInt's own `/` and `%`, for a zero divisor.
- **Module init: eager,** as Kotlin's: a module's code runs once, when
  it's first imported. A module `let` or `var` that the init block
  initializes at its top level is declared there (`const` or `let`,
  with its value); others are declared first, with a JSDoc type and no
  value (`analysis::module_state` says which, shared with Rust).
- **Module `var`s: plain,** as `let`s; JS runs a module's code on one
  thread, and an exported one is a live binding, as Kotlin's.

**Kotlin-compatible surface:** `package.json` (`"type": "module"`,
`exports` for `.` and `./NAME`, and `@temperlang/core` as a
dependency), `index.js` re-exporting the module, the module's public
file `NAME.js` re-exporting its exports, in source order, from
`NAME.internal.js`, which holds the code; names keep their case. One
difference, in what isn't public: Kotlin's `.internal.js` exports every
top-level, so other modules can import them, while this exports only
what the source does, until there are several modules. A `Console` is
erased, as Rust's is: `console.log(…)` is the global console, which
Kotlin's `globalConsole` is too.

**Naming:** block-scoped (`NamingRules::block_scoped`,
[names.md](names.md#as-built-2026-10-30)): two locals of a function share
a name only when their blocks don't nest, since `let` can't declare a
name twice in one block, and an inner one would shadow an outer one from
the start of its block. A tmpl block inside a block keeps its braces.

**Versions** (Proposed): `@temperlang/core` 0.6.0, as Kotlin's; generated
code needs ES2020 (BigInt) and ES modules, so Node 14 or later, and is
checked as ES2022 under Node's module rules (`--module nodenext`); the
tests use Node 24 locally and in CI, and TypeScript's latest (7.0.2
locally).

### As built: Java

As built (2026-10-30, in `crates/temper-be-java`): the first variant is
`JAVA`, a `Variant { name, profile, rules, runtime_version,
java_release }`, with no native features, as JS's: statements inside
expressions wait for the hoisting pass, and a program that needs it is
reported as not yet. Java keeps natively everything else subset A has:
labeled `break` and `continue`, `do … while` with `continue`, and `?:`
for `IfValue`. Its choices, made by the translator for now, as Level 2's
table has them:

- **Failure: exceptions.** A bubble is an unchecked `RuntimeException`
  from the runtime's `Core.bubble()`, thrown by `throw Core.bubble();`
  where `Error` is a statement or a `return`'s value, or by
  `Core.throwBubble()` in an expression; `Core.divIntInt` and
  `Core.modIntInt` throw it for a zero divisor.
- **Module init: class init,** as Kotlin's: a module is a final class of
  statics, and its init block the class's static initializer, which the
  JVM runs once, on one thread, when the class is first used (the main
  class loads it with `Class.forName`).
- **Module `var`s: synchronized,** lightly. Module `let`s and `var`s that
  functions use, or that are exported, are static fields (`final` for a
  `let`), declared without a value and assigned in the static
  initializer, as Kotlin's are; the rest are the initializer's locals. A
  `var` that a function writes is `volatile`, so every thread reads its
  latest value, and a function's update of one from itself alone
  (`analysis::reads_only_itself`, shared with Rust) takes the class's
  lock, so `ticks = ticks + 1` from 8 threads, 1,000 times each, gives
  8,000. Kotlin's are plain statics. Other updates aren't atomic, as in
  Rust.

**Kotlin-compatible surface:** `pom.xml` (group the package, artifact
the library, `temper-core` 0.6.0, Java 17 by `<release>`, and the main
class for the jar and `exec:java`, otherwise Kotlin's template), and a
package named for the library with `_` for `-` (`hello_world`), holding
`HelloWorldGlobal` (the module) and `HelloWorldMain` (`main`: simple
logging to stdout, the module's class loaded, async tasks waited for).
Exports are `public static`, the rest package-private, with the source's
names. Logging goes through `temper-core`'s `Console`, from
`Core.getConsole(Logger.getLogger(PACKAGE))`, as Kotlin's, not erased
as in JS and Rust, since `java.util.logging` configures where it goes.

**Naming:** block-scoped, as JS's: Java forbids a local that shadows
another local. The types the translator names (`Math`, `Integer`,
`Core`, `Logger`, …) are reserved, since a local of that name would
obscure them.

**Versions** (Proposed, 2026-10-30): generated code targets **Java 17**
(`javac --release 17`), as Kotlin's `java` backend does (its `java8`
backend is a separate one, not carried forward yet); nothing in subset
A needs more, and 17 is the oldest LTS still widely supported. The
runtime, `temper-core` 0.6.0, compiles for Java 8, as its own
`pom.xml` says. The tests use JDK 21 (Temurin 21.0.4 locally, the
latest 21 in CI).

### As built: C#

As built (2026-10-30, in `crates/temper-be-csharp`): the first variant
is `CSHARP`, a `Variant { name, profile, rules, runtime_version,
target_framework }`, with no native features, as Java's: statements
inside expressions wait for the hoisting pass, and a program that needs
it is reported as not yet. C# keeps natively everything else subset A
has but labeled jumps: a `break` or `continue` of a loop that isn't the
innermost is a `goto` to a label after the loop (`outerEnd: ;`) or at
the end of its body (`outerNext: ;`), as Level 2's table has it. Its
choices, made by the translator for now:

- **Failure: exceptions.** A bubble is the runtime's `Core.Bubble()`,
  which throws; in a function with a result, `return Core.Bubble<T>();`,
  so C# sees the function end, and in an expression `Core.Bubble<T>()`.
  `Core.Div` and `Core.Mod` bubble on a zero divisor.
- **Module init: class init,** as Kotlin's: a module is a static class,
  and its init block the class's static constructor, which .NET runs
  once, on one thread, before the class is first used (the program runs
  it with `RuntimeHelpers.RunClassConstructor`).
- **Module `var`s: synchronized,** lightly, as Java's: statics that
  functions use, or that are exported, are static fields (`readonly` for
  a `let`), assigned in the static constructor; a `var` that a function
  writes is `volatile` (but a `long` or `double`, which C# doesn't
  allow), and a function's update of one from itself alone takes the
  module's lock, a private `object` (not `lock (typeof(…))`, which other
  code could take too). 8 threads × 1,000 ticks give 8,000.

**Kotlin-compatible surface:** per library, `src/ROOT.csproj` (.NET 6,
`RootNamespace`, `IsPackable`, `Nullable` enabled, `InternalsVisibleTo`
its `ROOTTest` project, and a project reference to
`../../temper-core/TemperLang.Core.csproj`; `<Version>` only for a
library that sets one), `src/ROOTGlobal.cs` (the root module, a public
static class in namespace `ROOT`, the library's name in Pascal case),
`src/Logging.cs` (Kotlin's template, in `ROOT.Support`, through which a
host sends logs where it likes), and `program/ROOTProgram.csproj` with
Kotlin's `Program.cs`. Exports are `public static`, the rest
`internal`, as Kotlin's; functions and exported values are in Pascal
case by Kotlin's conversion (`Fib`, `Greeting`, and Kotlin's
`A98_Rgb`), the rest as the source has them. A `Console` is the
runtime's `ILoggingConsole`, from the library's
`Support.Logging.LoggingConsoleFactory.CreateConsole(ROOT)`, as
Kotlin's.

**Culture** (C# only): C#'s string and number operations follow the
current culture unless told otherwise, so the translator never leaves
it to them: `Int32.toString()` is `n.ToString(CultureInfo.
InvariantCulture)`, or with two or more numbers in a concatenation
`string.Create(CultureInfo.InvariantCulture, $"…")`, never `+` on a
number; string equality is `==` and comparison the runtime's code-point
`StringUtil.CompareStringsByCodePoint`, both ordinal; `Boolean.toString()`
is `b ? "true" : "false"`; and `isEmpty` is `s.Length == 0`. The tests'
analyzers make any culture-sensitive call an error, and their runs use a
culture whose minus sign isn't `-`.

**Parameter names** are API in C# (named arguments), so they're exactly
the source's, with `@` for a keyword. The Roslyn analyzer that is to warn
on named arguments in calls into a library, but for constructors and
factories ([surface.md](surface.md#parameter-names-where-languages-have-named-arguments-proposed-2026-10-30)),
isn't built yet: it belongs in the NuGet package, which waits for
packing.

**Versions** (Proposed, 2026-10-30): generated code targets **.NET 6
(`net6.0`, C# 10)**, as Kotlin's projects do; its runtime's project
targets `net48;net6.0`, and the tests build its .NET 6 side. .NET 6 is
out of Microsoft's support (since 2024-11), so a later variant, or a
change made with Kotlin's, may move to .NET 8, whose SDK also brings
C# 12 and `>>>`. The tests use the local .NET 6.0.402 SDK (6.0.x in
CI), its Roslyn and .NET analyzers.

### Passes

```rust
pub struct Pass {
    pub name: &'static str,
    pub removes: Feature,       // what the profile lacks
    pub needs: Features,        // what must already be gone
    pub may_add: Features,      // checked by the validator afterwards
    pub run: fn(Program, &Profile) -> Program,
}
```

`lower` runs, in the fixed order, each pass whose feature the profile
lacks and the program uses, and validates after each in tests and
debug builds. Subset A needs one pass for Python.

### Statements out of expressions (`hoist`)

- **Precondition:** the profile lacks statements inside expressions.
- **Output:** no `BlockValue`, and no statement under an expression;
  `IfValue` remains only with plain expressions in both branches,
  which Python writes as `a if c else b`.
- **Method:** for a statement `S` holding an expression `E` that holds
  statements:
  1. Hoist each operand left of `E` that order requires (the
     order-sensitive operands analysis) into a `let` temporary, named
     by `naming::hints`.
  2. Turn `E` into statements that assign its value to a target, and
     put them before `S`, then put a read of the target in `E`'s place.
  3. Choose the target without a temporary where possible: a `Let x =
     E` assigns `x` itself (a `Let` without an initializer); a `Return
     E` returns in each branch; an `ExprStatement` discards the value.
     Otherwise the target is a new `let` temporary with hints.
  4. An `And` or `Or` whose right side holds statements becomes an
     `IfValue` first; a `while` condition that holds statements
     becomes `while (true) { …; if (!c) break; … }`.
- **Spans:** hoisted statements keep theirs; a temporary's
  declaration, assignments, and read take the hoisted expression's,
  marked derived.

Example: `count` is a module function, impure, so its call is hoisted
before `"counted"` is logged:

```text
~console#6.log#10(%Int32.add(count#2(), (value { ~console#6.log#10("counted"); 1 }): Int32).toString#8(omitted));
```

becomes:

```text
let ~count|int32#9: Int32 = count#2();
~console#6.log#10("counted");
~console#6.log#10(%Int32.add(~count#9, 1).toString#8(omitted));
```

The receiver `~console#6` stays in place: a module `let` that isn't
`var` can't change once initialized.

In Python, `count` is a module function the enclosing function calls,
so the temporary takes its second candidate and is named `int32`.
