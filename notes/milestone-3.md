# Milestone 3 plan: the settle loop

Part of the [rewrite plan](README.md). It settles what the rest depends
on: the core constructs every macro reduces to, the tree's layout, names,
and how waiting work wakes up. What's built so far, and where it differs,
is at the end.

## Core constructs

After macro expansion, every function body is made of these, and only these
need to be understood by flow typing ([typing.md](typing.md)), the
interpreter, and the lowering to tmpl:

| Construct | Notes |
|---|---|
| `Block(label?, items)` | Sequence; a label makes it a `break` target. |
| `If(cond, then, else)` | `else` may be empty. |
| `Loop(label?, body, next?)` | The only loop. `while`, `do … while` and C-style `for` add `If` and `Break`; `for … of` uses the iteration protocol. `next` runs after the body and on `continue`: a `for` loop's increment, or `do … while`'s check. |
| `Break(label)`, `Continue(label)` | Labels resolve to a `Block` or `Loop`. |
| `Return(value)` | Leaves the function. |
| `Orelse(body, fallback)` | Catches failure from `body`. |
| `Fail` | `bubble()`; may carry a value later. |
| `Yield(value)`, `Await(value)` | Suspension points in generators and async functions. |
| `Fn(params, body)` | Function values and declarations' bodies. |
| `Decl(name, type?, init?)`, `Assign(target, value)` | `let`, `var`, and assignment. (`Let` in the code, as `Fn` is, to keep them apart from the grammar step's `Decl` and `Fun`, whose layouts differ.) |
| `Call(callee, args)`, `Name`, `Literal`, `Error` | Everything else is a call, including operators and `panic()`. |

Lowered by macros into these: `when` (to `If` chains on a temporary), `&&`,
`||` and `??` (to `If`), `?.` (to `If` on null), and `for … of`.

Facts at `Orelse`: every `Call` that can fail is an implicit exit to the
nearest `Orelse`, so its facts there are the merge of the facts at each such
call ([typing.md](typing.md)).

Generators and async, from Kotlin Temper (`core.temper`, and the stage tests
in `frontend/.../stage-tests/convert-coro/`):

- `next()` takes no argument and `DoneResult` carries no value: values only
  flow out, through `yield(x)`, and `return` just ends the generator.
- `Generator.next()` propagates a bubble from the body; `SafeGenerator`'s
  `nextSafe()` can't fail. `close()` exists but is still a TODO.
- `yield` may appear inside `orelse`, nested (`nesting-orelse`).
- `async { }` takes a safe generator block and runs it later, on a runner
  the backend provides. `await(p)` suspends within it and can fail on a
  broken promise (`await(p) orelse "Broken"`). `PromiseBuilder` completes or
  breaks a promise.
- Strategy per backend (`be/.../tmpl/CoroutineStrategy.kt`): native
  generators for JS, Python, C# and Lua; a state machine from
  `frontend/.../coroutine/CoroutineConverter.kt` for C++, Java and Rust.

Here, `Yield` and `Await` stay ordinary nodes through typing, and a shared
state-machine pass runs late, only where a backend needs it. It needs
structured control flow (these constructs), `yield` inside `orelse`, and
which locals live across each suspension point; no sent or returned values.

Targets that could do better than classic Temper, with flexibility where
it's cheap:

- **C++:** state machines work on any C++ version. A small adapter header,
  compiled only under C++20, lets C++20 users `co_await` Temper promises and
  use generators natively. A backend option to emit `co_await` and
  `co_yield` directly can come later if there's a reason.
- **Rust:** `impl Future for Promise<T>` in the runtime (std only: `poll`
  returns the result or registers the waker through the existing callbacks),
  so Rust code can `.await` Temper promises on any executor, and an
  `AsyncRunner` can wrap the user's executor. Temper `async { }` blocks can
  become native Rust `async` blocks with `.await`, which is stable; plain
  `yield` generators keep the state machine, since Rust generators aren't.

## tmpl: a separate schema crate

tmpl is its own tree in its own crate: data types, a validator, and a
printer, with no dependency on the parser or frontend. The frontend produces
it, and backends depend only on it. That enforces the boundary classic
Temper lacks (its backends reach into frontend types), lets backends build
and test alone, and lets pieces ship separately, such as a wasm playground.
The cost is a translation step and a second printer, which is modest since
tmpl is small.

### Interpreters and backend crates

- **One interpreter on frontend trees,** needed anyway for compile-time
  evaluation in the settle loop, also runs whole programs: in-process tests,
  the REPL, and the playground.
- **A small tmpl interpreter crate, later (milestone 5 or after),** as a
  test oracle for lowerings: run the same program before and after lowering
  (state machines for generators, naming, other shared passes) and compare,
  without target toolchains. Worth it only once lowerings exist; a few
  thousand lines at most.
- **Each backend is its own crate,** depending only on the tmpl crate and a
  shared printing crate, never on the parser or frontend.

## Tree layout

One arena per library build, of plain structs indexed by `u32`:

- `nodes: Vec<Node>`, where a `Node` is a kind (`u16`), a source span
  (`u32` start and end), an origin (`NodeId` of the parse node it came from),
  and its children as a range into `children: Vec<NodeId>`.
- Payloads live in side tables keyed by `NodeId` or in small typed arrays:
  names (interned `NameId`), literal values, and later types and flow facts.
- A macro expansion rewrites its own call node in place: it changes the
  kind and points at newly appended children. Replaced children become
  garbage, freed with the arena; nothing is freed one node at a time.
- The parse result stays separate and unchanged (see design.md §4).

## Names

Much simpler than Kotlin Temper's `TemperName` hierarchy (`ParsedName`,
`ResolvedName`, `SourceName`, `Temporary`, `BuiltinName`, `ModularName`,
`ExportedName`, `OutName` and more):

- **Text** is an interned `NameId` (`u32`), as `names.rs` already does.
- **Resolution** is a side table from each use node to a `SymbolId` (`u32`).
- **Symbols** are one flat table of plain structs: text, a small kind enum
  (local, parameter, member, top level, builtin, synthetic), owner,
  declaring node, and whether it's exported. Compiler-made temporaries are
  synthetic symbols with a hint; their id is their number. Builtins are
  symbols in the builtin library's table, made once.
- **Stability across builds** comes from exports: a table from (library,
  name) to symbol, not from name objects.
- **Hygiene:** when a macro means a specific binding, it inserts a reference
  already resolved to a `SymbolId` instead of text, so user locals can't
  capture it. No separate parsed and resolved name types.
- **Names in generated code** belong to tmpl's naming pass for backends
  (design.md §8), not the frontend.

## Waiting and waking

Each unsettled node has a work item recording the one fact it waits on
(design.md §5): a binding, another node's type or value, a module's
exports, or a macro having run. An index from each fact to the items
waiting on it means a new fact wakes exactly those items; nothing is
rescanned.

- The ready queue is ordered by (phase, source position), so output and
  errors are deterministic.
- When the queue empties with items still waiting, the wait graph is
  checked for cycles. A cycle breaks at its earliest item in source order;
  anything else left waiting becomes an error that names its chain, from
  the same "why is this waiting?" query used for debugging.
- Facts are only added, never retracted, which these rules keep true:
  - **Outside-in expansion.** A call's arguments aren't processed until its
    callee resolves; if it's a macro, nothing beneath it is touched until it
    expands. Most macros (`let`, `fn`, `if`, `class`) need only syntax.
  - **A macro call publishes nothing until it has expanded:** no type, and
    no bindings visible outside it.
  - **Macros that need types** use their typed arguments only where they
    are. To re-scope an argument, a macro copies it, giving fresh node ids,
    so facts about the old nodes just become garbage.
  - **Debug check:** debug builds record which facts each step read and
    assert none came from a node that was later rewritten.
- Full dependency tracking with recomputation (like salsa) isn't needed
  within a build. Watch mode will need it at a coarser grain, per top-level
  declaration across edits.

## Done when

- The simplest functional tests run in-process through the interpreter,
  with `log` output matching Kotlin Temper's.
- `.cases` files for name resolution and macro expansion, with expected
  output, plus property tests on edited inputs (never panics, always
  settles within the step budget).
- Rows in the speed, size and memory tables.

## Status: the first slice

Built in `crates/temper-frontend`, with what changed from the plan above.

**Crates.** `temper-frontend` holds the library arena, the settle loop,
names, macros and the interpreter: one crate for "settled tree from
source", as notes/code-style.md suggests, since they share the tree and
symbol tables closely. The interpreter will move or split once tmpl
exists, if backends want it apart. tmpl gets its own crate later. Core
node kinds live in temper-syntax's `NodeKind`, since there's one tree
type; their layouts are documented in `temper_frontend::core`.

**Tree layout.** The arena reuses temper-syntax's `Tree`: parallel arrays
(kinds, spans, data, child ranges) rather than one `Vec<Node>` of structs,
with origins in a side array. `Tree::replace` rewrites a node in place,
appending its new children. Files are appended into one arena with shared
names (`library::Library`); spans are library-wide, each file's offsets
shifted by a base, so a span alone says which file it's in. Copying each
file's grammar tree into the arena costs a little time; building straight
into it is an easy later change.

**Names** follow the "Names" section: `scope::Symbol` is the flat table
(kinds builtin, top level, parameter, local, member and synthetic), with a side table from use nodes to `SymbolId`. Macros
insert names already resolved to builtins (`cat` for string
interpolation, `+` for `+=`). Temporaries start in the syntax step, as
call joins' delegate parameters: a `NameId` that's never interned, so it
can't equal any source name; declaring one makes a synthetic symbol.

**Waiting.** The scheduler (`engine`) is generic and tested on its own:
items wait on one fact each, publishing wakes exactly the waiters, the
queue is ordered by (phase, position, creation), there's a step budget,
and "why is this waiting" chains. Instead of turning leftover waits into
errors directly, the engine *forces* the earliest stuck item (one whose
chain of waits reaches a fact nobody will produce, or loops) and its step
goes on without the fact; that usually makes an error node, and may free
others. So far there's one kind of fact, a scope's declarations being
complete, and no cycles can arise from it; forcing is covered by the
engine's tests. Types, values and imports will add facts.

**Reading blocks.** A block's statements are read in order (phase 0),
each expanded outside-in only as far as needed to see what it declares;
then the scope is complete. Other lookups (phase 1) that reach an
incomplete scope wait for it, so no name resolves outward past a
declaration still to come: a block's declarations cover the whole block,
as in JavaScript. A statement's own macro name resolves against what's
declared so far; if a later declaration in that block would have
shadowed it, that's an error (`DeclaredAfterUse`) rather than a retracted
fact. Debug builds check that no step read a declaration from a node that
a later step rewrote.

**Call joins** arrive rewritten as Kotlin does for libraries
(`temper_syntax::module`); `if` reads the arguments the delegate function
passes on instead of calling it.

**Classes** (`classes.rs`): `class` and `interface` lower to a `Class`
node, whose members are `Let`s tagged with a role (field, header
parameter, method, getter, setter, constructor), visibility and
`static`. The body
is read by shape, not expanded as code, so the class's scope is complete
as soon as it's made: members may use each other in any order, after a
synthetic `this` symbol. A bare member name means that member of `this`,
dispatched by name at run time, as `this.name` is: own members first,
then supertypes breadth-first, so the nearest wins and ties go to the
first in `extends` order (what `call-override-from-subtype` expects).
Kotlin Temper agrees: its `OverrideLinker` walks supertypes
breadth-first, and `TyperTest.multipleInheritedMethodImplementations` and
`JavaBackendTest.multipleInheritance` have `D extends B & C`, with `B
extends A`, take `C`'s `a()` over `A`'s. A tie (`F extends C & E`, both
defining `a()`) is a type error there ("No callee matches"), though it
still runs the first; we should report it too once there are types.
Static members hang off the class value, which plays `this` for them. No context record or
parent links were needed: `this` finds its class by walking scopes. The
interpreter's objects hold their own class's fields; a member runs in a
frame for its declaring class's scope holding just `this`. Field
initializers run before the constructor's body, as in Kotlin Temper.
`new`, `as` and `is` are builtin functions. `obj.x += 1` and `obj.x++`
read `obj` once, through a temporary. A bare non-`var` field may be
assigned only in its class's constructor, if it has no initializer; that's
an error otherwise, reported, and the assignment isn't performed: its
value stands in for it (Kotlin Temper fails at run time instead). Not
yet: inherited fields, abstract members'
parameters, `super` calls, checks that a class implements its interfaces,
visibility checks, and the assignment check for `this.x = …`.

**Types:** none yet. The interpreter is dynamically typed, and type
expressions are only resolved. Type parameters (`class Box<T>`, `let
f<T extends I>(x: T)`, `fn<T>`, methods' own) lower to `Let`s with a
role, ahead of a `Class`'s supertypes or a `Fn`'s parameters, and are
declared in a scope of their own around the class's or function's
scope, so they cover bounds, supertypes, parameters and bodies.
Decorators on them are ignored; variance isn't in the grammar.
Type arguments stay in the tree, for typing and for backends that keep
them (C++ templates, Rust and C# generics); only the interpreter ignores
them, as some targets (Java, JS, Python) also erase them. `T<A>` is a
call of a hidden `<>` builtin, which the interpreter evaluates to `T`,
and `f<A>(x)` keeps `\typeArg A` among the call's arguments, which the
interpreter skips. Kotlin rejects type
parameters on overridable (interface) methods; we don't check that yet.

**Lists:** `[a, b]` and `List.of(a, b)` make an immutable `List`;
`new ListBuilder<T>()` a mutable one. `ls[i]` and `get` panic out of
bounds, and `ls[i] = v` is `ls.set(i, v)`. Both have `length`,
`isEmpty`, `get`, `getOr`, `slice`, `toList`, `toListBuilder`, `map`,
`filter`, `reduce`, `reduceFrom`, `join` and `sorted`; `List` has
`forEach`; `ListBuilder` has `add` and `addAll` (with an optional index),
`set`, `removeLast`, `reverse`, `clear`, `sort` and `splice`, where a
null argument takes its default. They're written in the interpreter for
now, as is `String.split`, `a <=> b` and `ignore(x)`. Sorting is a
stable merge sort that tolerates a comparison that fails or contradicts
itself (Rust's own sorts may panic on those). Callbacks see a copy of a
builder's items, so they may change it.

**`for (let x of xs) { body }`** lowers as Kotlin Temper does, to
`xs.forEach(fn (x) { body })`, wrapped as `Loop({ that; Break })` so that
`break` has a loop to leave. The body `Fn` is marked `FN_LOOP_BODY`, as
Kotlin marks `forEach`'s parameter `@inlineUnrealizedGoal`: `return` in
it returns from the enclosing function, `break` leaves the loop, and
`continue` ends the call. These are just jumps the interpreter passes
through the call. TODO (tmpl and backends): turn
`Loop({ xs.forEach(FN_LOOP_BODY fn) })` back into the target's own
for-each loop where it has one, as Kotlin's backends do through their
support networks, and inline it otherwise; the marker is kept for that.
Labeled `continue
outer` from an inner `for … of` works here, but Kotlin Temper fails at
run time with no message (worth reporting upstream). `ListBuilder` has
no `forEach` in Kotlin's core, so it can't be looped over directly,
which looks deliberate (a TODO there is about iterating while the body
changes it).

**`when`, `??` and `?.`** lower as Kotlin Temper's macros do. `when (x)
{ 0, 1 -> a; is T -> b; else -> c }` is `do { let t = x; if (t == 0 || t
== 1) { a } else if (t is T) { b } else { c } }`, with no temporary when
`x` is a name; without `else` it's void when nothing matches. `a ?? b`
is `do { let t = a; if (t == null) { b } else { t } }`, and `a?.m` and
`a?.m(args)` are `do { let t = a; if (t == null) { null } else { t.m(args)
} }`, so a null `a` skips the arguments too. As in Kotlin Temper, the
null skips only that one step: `a?.b.c` fails when `a` is null. The
macros refer to `==` and `is` through hidden builtins, so local names
can't capture them; every named builtin has one now. `case` patterns
(`case_case`) aren't supported yet. `T?` is a call of a hidden `?`
builtin, `T` to the interpreter. `is` and `as` now check builtin types
(`Int`, `String`, `List`…) instead of always passing.

**Defaults and null:** a parameter's default applies when its argument
is missing or null, in the callee, as Kotlin Temper does
(`functions/defaulting`).

**Numbers** (`interp/numbers.rs`) follow Kotlin Temper's
`FloatFns.kt`. Floats compare in a total order, as Java's
`Double.compare`: `-0.0 < 0.0`, `-0.0 != 0.0`, and `NaN == NaN`, above
everything. Float `/` and `%` fail for a zero divisor, as for integers.
`toString` is Java's shortest round-trip form with `e+N` exponents
(`1.0e+7`, `9.99e-4`, `4.9e-324`), cut, not rounded, at 16 digits after
the point, as `FormatDouble.kt` does; so `0.30000000000000004` shows as
`0.3000000000000000`, which doesn't read back as the same number (worth
raising upstream). `max` and `min` give NaN if either is, `round` rounds
ties to even, `sign` keeps zeros' signs, and `toInt32`, `toInt64` and
`Int64.toFloat64` fail where the value doesn't fit (beyond 2^53 - 1 for
the 64-bit ones). `String.toFloat64` takes JSON numbers plus `NaN` and
`±Infinity`, as its docs say; Kotlin Temper also takes whatever the JVM
does (`1f`, `-.5`, `0x1p3`), which looks accidental. `NaN`, `Infinity`,
`Float64.e` and `Float64.pi` are there, and `**` for floats. `Int` and
`Int64` have `toString(radix)`, failing outside radixes 2 to 36.

**Strings** (`interp/strings.rs`) follow Kotlin Temper's `StringFns.kt`
and `core.temper`: `s[i]`, `hasIndex`, `next`, `prev`, `step`,
`countBetween`, `hasAtLeast`, `slice`, `indexOf`, `split`, `forEach`
(so `for (let c of s)`), `toInt32` and `toInt64` (with a radix),
`String.fromCodePoint(s)`, `StringIndex.none` (a `NoStringIndex`,
sorting before every index), `compareTo` and `is`/`as` on
`StringIndex`, `NoStringIndex` and `StringIndexOption`. A
`StringIndex` is a UTF-8 offset where Kotlin's is a UTF-16 one; Temper
code can't tell, as indices are opaque. Indices past the end clamp to
it, as in Kotlin; one from another string that lands inside a
character counts as that character's start. `s[i]` past the end
panics, as `String.get`'s docs and the backends do (Kotlin's interpreter
fails instead). `toInt32` takes an optional sign and ASCII digits
around whitespace (Kotlin also takes other Unicode digits).
`StringBuilder` has `append`, `appendCodePoint`, `appendBetween`,
`clear`, `end` and `toString`. Tagged strings, `tag"a${b}c"`, lower as
Kotlin's `StringExprMacro` does for a function tag, to `tag(["a", "c"],
[b])` with escapes left undecoded; `char'a'` is the only builtin tag so
far. Statements in strings aren't supported yet.

**Destructuring** `let { a, b as c, d is T } = x` lowers as Kotlin's
`flattenMultiInit`, to `let t = x; let a = t.a; let c = t.b; let d: T
= t.d`, spliced into the enclosing block (no temporary when `x` is a
name). Only in blocks so far, and not for `import`, `var`, nested
patterns or `...`; a second `as` is just "Declaration is malformed",
where Kotlin says `MultipleRenames`.

**Top-level order** (`order.rs`): after settling, a module's top-level
statements run in dependency order, as Kotlin's `sortTopLevels`
(`frontend/.../syntax/SortTopLevels.kt`) arranges them. A statement
needs every top-level name it reads anywhere inside it, function bodies
included, and goes after those names' declarations and then every
statement that assigns them; a statement that assigns a name also goes
after its declaration. The order is a depth-first walk in source order,
so unrelated statements keep their order, and a cycle breaks where the
walk enters it (`let a = b + 1; let b = a + 1` runs `b`'s first and
fails reading `a`; Kotlin also reports that at compile time, which we
don't yet). Kotlin's rule scrambles statements that assign a top-level
`var` (any `n++` makes every reader of `n` wait for it); we match it,
and it's a Kotlin fix to make. Across files, the order is global: the
interpreter runs `Settled::top_levels`, and each file's root is
reordered to match for printing. Kotlin also puts imports first, for
when there are imports. Uses are counted after name resolution, so a
local that shadows a top-level name doesn't count, as in Kotlin, where
sorting follows `resolveNames`.

**Bitwise:** `~`, and `>>>` beside `<<` and `>>`, as Kotlin's
`BuiltinFuns.kt`: shift distances wrap at the width (`5 << 32` is 5 for
`Int`), and an `Int64`'s distance is an `Int`.

**Running without fuel:** `interp::run(settled, fuel)` takes an optional
step budget. `run-temper` defaults to none (`--fuel none|STEPS`), so a
program runs to completion as with any language's runner; tests and the
wasm playground pass `interp::FUEL` (50M steps), and compile-time
evaluation will budget its own. No fuel is `u64::MAX` steps, so the hot
path is unchanged.

### Functional tests

Picked for using only `let`, `var`, functions and closures, arithmetic,
`if`, loops, `console.log` and simple classes, with no lists, generics or
imports. 45 of 69 pass (`compat/check_functional.py`): `algos/hello-world`,
`algos/fibonacci`, `control-flow/if-return`, `control-flow/loop-reenterable`,
`control-flow/loops` (labels, `break`, `continue`), `functions/simple-locals`
(closures, block arguments), `names/nonascii`, `types/int/basics`,
`types/string/is-empty` (`if` as an expression), `semantics/constness`,
`semantics/type-checked-locals` (`do`, `orelse`, `bubble`; Kotlin also
reports type errors there, which we don't yet) and `semantics/broken`
(expected to fail at run time); with classes, `algos/hello-world-object`,
`algos/hello-from-class-to-top`, `casts/as-expr`, `classes/direct-getter`,
`classes/inherited-getter`, `classes/object-literals/extra-class`,
`classes/private-method`, `classes/property-order`, `classes/setters`,
`classes/static-properties`, `classes/static-properties-scope`,
`classes/write-back-through-field`,
`classes/call-override-from-subtype`, `interfaces/empty`,
`interfaces/pure-virtual` and `semantics/mutually-referencing-types`;
with type parameters and lists, `classes/angle-call`, `casts/specific`,
`functions/constructor-callbacks`, `regression/minimal-repro`,
`types/list/empty` and `types/list/reduce`; with list builders and `for
… of`, `types/list/operations` and `control-flow/for-of-folded`; with
`when`, `??`, `?.` and number methods, `control-flow/bubble`,
`functions/defaulting` and `types/float/basics`; with strings, string
indices and `char'a'`, `types/string/read`, `types/string/indices`
and `types/string/build`; with top-level dependency order,
`types/float/ops` and `types/int/limits`; with `~` and `>>>`,
`types/int/shifty`. Most others need imports, named arguments,
statements in strings, or builtin types such as `Map`, `Deque` and
`DenseBitVector`; `types/list/sorting` and
`interfaces/property-members` also need object literals typed by
inference. (Counts are on the Temper repo's `main`; on its
`connected-types` branch, as of the previous chunk, the suite had 67
tests, of which 41 passed.)

### Left for this milestone

- The rest of classes: inherited fields, `super`, and
  the checks listed under Classes above.
- `import`, with a library export table and a fact for a module's
  exports.
- `case` patterns in `when`, `do … while`, statements in strings,
  named arguments, destructuring beyond `let` in blocks;
  `Map` and the rest of the builtin types' methods.
- `Yield` and `Await`, for generators and `async`.
