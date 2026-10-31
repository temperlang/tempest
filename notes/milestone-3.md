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

Here, `Yield` and `Await` stay ordinary nodes through typing, and the
frontend hands every backend the same tmpl with them still abstract,
since some backends have their own generators and async. A shared
state-machine pass runs on tmpl afterwards, only where a backend needs it. It needs
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

### The interpreter: flat frames (decided 2026-10-24)

The interpreter doesn't use the state-machine pass, so that pass has one
data model, tmpl. Instead, the recursive tree walk is replaced (not kept
alongside) by a flat form per function: blocks and jumps, with locals in
numbered slots and source positions for errors. Lowering is its own
module, apart from execution. One `Frame` type and one `run_frame` loop
run it. An ordinary call runs its frame nested on the Rust stack, as now;
a generator's or `async` block's frame is owned by its generator value,
and suspending returns from the loop, keeping the position, to re-enter
on `next()`. That's enough because `yield` and `await` appear only
directly in the generator's own body, never in a closure it passes on.

`yield` inside a `for … of` body (2026-10-24): the lowering makes such a
body its own function, called by `forEach`, so a `yield` there isn't in
the generator's own frame. It ought to work eventually, likely by
treating `for … of` differently (lowered inline as a loop over the
iterable, say). If Kotlin Temper doesn't support it, defer it here too,
but keep the design open to it. It doesn't (checked in `temper repl`,
2026-10-25): the body runs up to the `yield`, which then fails, with or
without a `break` that would have it inlined. So it's an error here for
now ("Not supported yet: yield in a for … of body"); an idea for both,
iterables providing generators, is in [kotlin-differences.md](kotlin-differences.md#changes-we-recommend-to-temper).

Not fully resumable (an explicit stack for every call): that would need
every builtin that calls back into Temper (`sorted`, `map`, …) to be
resumable too. Step budgets and cancellation only need to abort (compile-
time evaluation is pure, so it reruns), deep recursion gets a depth
limit, and a debugger can hook the loop. The flat form leaves the option
open if a real need comes up.

Steps, each its own chunk: (1) the flat form with no new features, all
funtests still passing, and size-and-speed comparing it with the tree
walk, profiling first if it's much slower; (2) generators
(`control-flow/actor-run`, Kotlin's `convert-coro` stage cases as a
guide); (3) `async`, `await` and a run queue, as in Kotlin Temper's
interpreter (`control-flow/async`, `async-waiters`), then std `net`
stand-ins for the `netresponse` pair.

**Step 1 is done** (2026-10-25). `interp/lower.rs` lowers a function's
body on its first call (cached for the run), and also a class's
initializers and supertypes and each module's top level, to ops for a
small stack machine; `interp/frame.rs` has `Frame` (its code, slots and
stack, cells, captures and position) and `run_frame`. Most locals are
slots; only variables a nested function or class uses are shared cells,
made fresh each time their block is entered, as the tree walk's frame
per block was, so each pass of a loop has its own (a C-style `for`'s
variable stays one binding, as before). Module-level declarations are
globals. `break`, `continue` and `return` are jumps within a function;
out of a `for … of` body they still end its call, and the frame they
reach jumps to a landing. `orelse` handlers are a table of op ranges.
Messages and the 63 funtests are unchanged, and it runs 2.4–3× faster
(size-and-speed.md). Fuel now counts ops run rather than nodes, and a
class's supertypes are evaluated once, when first needed. Settling
doesn't call the interpreter yet; when compile-time evaluation does,
lowering will read the tree as it is then, and a function rewritten
after it was lowered will need its cached code dropped.

**Step 2 is done** (2026-10-25): generators, and `control-flow/actor-run`
(64 funtests). A `Fn` keeps its supertypes as children (`extends
GeneratorFn`), and `yield` becomes a `Yield` node naming its function.
Calling a generator's function makes a `Generator` value owning the
call's frame, which runs only through the parameters, so arguments are
checked and defaults computed then, as in Kotlin. A `Yield` op returns
from `run_frame` with the frame kept, and `next()` runs it on from there;
the frame is taken out while it runs, so a `next()` from within gets a
`DoneResult`, as Kotlin's `_done` gives. It's a `SafeGenerator`, with
`nextSafe()`, when the declared return type doesn't fail, else a
`Generator`, whose `next()` passes on a failure from the body and ends
it. `yield` with no value yields null; `orelse` around a `yield` works
as anywhere else, since handlers are found by position. `ValueResult`,
`DoneResult`, `doneResult()` and `is` checks for these types are
builtins for now, as core.temper isn't loaded. Differences from Kotlin:
`return` in a generator ends it (Kotlin's fails typing); named functions,
`fn` literals and methods may be generators, not only block lambdas;
and `close()` ends the generator, as core.temper's docs say, where
Kotlin's does nothing yet. `yield` outside a generator fails when run,
with Kotlin's message, or is an error outside any function. For step 3,
an `async` block's frame can be owned the same way, with an `Await` op
returning from `run_frame` as `Yield` does.

**Step 3 is done** (2026-10-26): `async`, `await` and promises, and
`control-flow/async` and `async-waiters` (66 funtests). `await` becomes
an `Await` node naming its function, as `yield` does. `async { … }`
calls its block, making a generator, and queues it; `interp/promises.rs`
has the queue and `Promise`. After each module's top level (and a test
run), queued blocks run in order, each until it ends or reaches an
`await` of a promise not resolved yet: the `Await` op then returns from
`run_frame` with the frame kept and the promise left on its stack, the
block is added to the promise's waiters, and it runs again from that op
once `complete` or `breakPromise` queues every waiter, in the order they
began to wait. An `await` of a resolved promise goes straight on, and a
broken one fails, for `orelse`. The first resolution wins. As in Kotlin,
the queue runs even if the top level failed, and a failing block doesn't
stop the others; the run reports the first failure, where Kotlin's
interpreter loses it ([kotlin-differences.md](kotlin-differences.md#interpreter-core-and-std)). `PromiseBuilder` and
`Promise` are builtins, as core.temper isn't loaded. Differences from
Kotlin: an `await` that would pause under a plain `next()` panics, since
only the queue resumes blocks (Kotlin returns its internal "not yet");
top-level `await` is an error (Kotlin allows it only in its REPL); and
promises left unresolved at the end aren't reported (Kotlin warns).
`yield` in an `async` block pauses it for good, as in Kotlin, and
`await` in a `for … of` body is unsupported, as `yield` is.

**std `net`** (2026-10-26): `types/netresponse` and
`netresponse-not-found` (68 funtests). `@connected let f(…): T;`, a
bodiless function, becomes a `Let` marked `MEMBER_CONNECTED` whose type
is its signature, `fn (A): T`; it runs only where the interpreter
implements it (`interp.rs`, `connected_function`), so far std `net`'s
`sendRequest`. That makes a promise and queues a job behind the `async`
blocks already queued, as Kotlin's Rust backend runs it as an `async`
task; when the queue reaches the job, it sends the request and waits for
the response, then completes the promise with a native `NetResponse`
(`interp/net.rs`), whose getters `status`, `contentType` and
`bodyContent` (an already completed promise) are its connected ones, or
breaks it if there's no response. A broken promise may now carry a
message, which an uncaught `await` reports ("Failed: io: Connection
refused"). The client is `web_client.rs`, which knows nothing of the
interpreter: `ureq`, as Kotlin's Rust backend uses, behind the cargo
feature `net`, on by default; without it, as in `temper-wasm`, it fails
with "web client not supported here". An HTTP error status is a
response, as in the backends. `run-temper --test-server-port N` declares
`testServerPort`, and `check_functional.py` runs a server answering as
Kotlin's `TestWebServer` does (`{"method": "POST"}` as JSON for any path,
404 and "not here" for paths starting `/missing`). Kotlin's interpreter
fails both tests (issue 214), so this follows the backends. std's `post`
ignores its MIME type ([kotlin-differences.md](kotlin-differences.md#interpreter-core-and-std)), so no `Content-Type` is
sent yet. Not yet: user-space connected functions (`functions/connected`
now fails at run time with "No connected implementation of sum"), and
on wasm, a path to the browser's `fetch`.

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

Done (2026-10-27): all 69 functional tests run in-process with Kotlin's
output, and the rest of the "Done when" list holds. What's left below
that needs types is milestone 4's ([milestone-4.md](milestone-4.md)).

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
declaration still to come. As in Kotlin Temper (checked in `temper
repl`), a block's local is visible only after its declaration, so `var
v = v` reads an outer `v`, and a use before it reads an outer one or is
undeclared; local functions and classes cover the whole block, so they
may call each other. (Module top levels are whole-scope, ordered as
below.) A statement's own macro name resolves against what's declared
so far; if a later function in that block would have shadowed it, that's
an error (`DeclaredAfterUse`) rather than a retracted fact. Debug builds
check that no step read a declaration from a node that a later step
rewrote.

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
Fields may be `x: T = v` as well as `let`. `interface F(params): R;`,
with no body, is Kotlin's `interface F { apply(params): R }`, which
`@fun` makes a function type; the interpreter calls the function value
itself, and the decorator is ignored for now (fun interfaces should become
named function types; see design.md, "Function types and fun interfaces"). `new`, `as` and `is` are builtin
functions. `obj.x += 1` and `obj.x++` read `obj` once, through a temporary. A bare non-`var` field may be
assigned only in its class's constructor, if it has no initializer; that's
an error otherwise, reported, and the assignment isn't performed: its
value stands in for it (Kotlin's typer reports it, with the message we
use, "Member x defined in C incompatible with usage"), for `this.x = …`
too (not yet for `this.x += 1`, which goes through a temporary).

Inheritance (2026-10-26): a class's scope also holds what it inherits,
declared once its supertypes' scopes are complete (a `ClassScope` fact;
a cycle of supertypes gets forced through), so a bare name may mean an
inherited member of `this`, as Kotlin allows. Own members come first,
then supertypes' breadth-first; private and static members and
constructors aren't inherited. Interfaces can't have backed properties,
so objects never hold inherited fields. Classes extend only interfaces,
and `super.` is reserved syntax, a parse error ("Expected a Word (not
"super") here") as in Kotlin, which hasn't decided what super calls
should mean; so there are no super calls or super constructors.

Checks, with Kotlin's messages: "Cannot extend concrete type(s) A";
"Constructor parameters are not allowed on interface types"; "Type K
must implement f from J.  Maybe add `public f(): Int`" for each abstract
method, getter or setter that nothing in the hierarchy implements, where
a field counts as a getter and a `var` field as a setter too (the
suggestion is the source text, so `Int` where Kotlin writes `Int32`);
"Type M has method f but its visibility is private which is narrower
than the method it overrides in L"; "Members of class Q require explicit
visibility: [.x, get.g(...), static.s]" (a class's members, not an
interface's; a `@noProperty` parameter counts as private); "Cannot
instantiate abstract type J"; and "`this` may only appear inside a type
definition". Ours: an interface's backed property is an error; Kotlin
accepts it, then fails at run time. Kotlin also misses a setter whose
getter is implemented; we report it (both in [kotlin-differences.md](kotlin-differences.md#language-and-types)).

Private members are usable anywhere in their class, on any instance, not
just `this`, as a Kotlin Temper PR makes them. Kotlin's typer checks
that ("Member x defined in C not publicly accessible"); with no types
yet, the interpreter does, and panics: each piece of lowered code knows
the classes it's inside. Abstract members' parameters were dropped
when lowering until milestone 4 kept them, as a bodiless `Fn`. Not yet:
signature checks (`IncompatibleSignature`, which needs
types), type parameters on overridable methods, sealed hierarchies, and
assigning an abstract property (`CannotSetAbstractProperty`).

**Types:** none here; milestone 4 adds them as facts in the loop
([milestone-4.md](milestone-4.md)). The interpreter stays dynamically
typed. Type parameters (`class Box<T>`, `let
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
can't capture them; every named builtin has one now. Branches are
gathered as Kotlin's `buildBranches` does, with its messages ("Invalid
case condition", "Missing case value", "Other cases are invalid after
else"). A `case` pattern, `case f(let y) ->`, is an error node in place
of its test ("Not supported yet: case patterns, which need types"), so
the `when` still works until that test runs: Kotlin keeps a pattern's
tokens as a `postponedCase` call for a macro of the subject's type to
decompose, which isn't written there either, and fails with type errors
([kotlin-differences.md](kotlin-differences.md#language-and-types)). `T?` is a call of a hidden `?`
builtin, `T` to the interpreter. `is` and `as` now check builtin types
(`Int`, `String`, `List`…) instead of always passing.

**Defaults and null:** a parameter's default applies when its argument
is missing or null, in the callee, as Kotlin Temper does
(`functions/defaulting`), header parameters' defaults included.

**Object literals** (`classes.rs`; only object literals take names,
and the `\a` markers don't outlive this step; see design.md, "Object
literals, not named arguments"): `{ a: 1, b }` arrives as `new void(\a
1, \b desugarPun())`, and `{ class: C, a: 1 }` as `new C(\a 1)`. As
Kotlin's `ConvertObjectLiteralNew` does, by name and not by type
inference, `void` becomes the class (since milestone 4, in typing's
round, by the type wanted too: typing.md, "Object literals and target
typing"), among those visible from the literal (imported ones too), whose constructor takes every given name:
its constructor's parameters, or its header's; body fields don't count.
None is "No signature matches", several "Multiple types have matching
constructors: A, B"; the nearest symbol for a class wins. A pun `b` is
the name `b`. The call then becomes a plain `new C(…)` with its
arguments placed, null (so the default) for skipped ones, and "No value
passed for x" for a skipped one without a default. Values run in the
order written (`functions/named-args` checks that): when that differs
from the parameters' order, through temporaries, `do { let t1 = b; let
t2 = a; new C(t2, t1) }`, literals excepted.

**Trailing blocks:** a function literal as the last argument goes to the
last `fn`-typed parameter, counting optional ones only if the other
arguments cover the required ones, as Kotlin's
`applicationOrderForActuals` does; so `calc(2, opts) { … }` fits
`calc(n, transform: fn …, options = …)`. Kotlin places it in its typer
(`ReorderArgs`); ours was at run time, for closures only, not
methods, and has moved to typing, for methods and constructors too
(milestone 4, 2026-10-29). Fun-interface-typed parameters don't take
trailing blocks yet.

`@auto("==")` on a class makes `==` compare its public fields (Kotlin's
`AddAutoMembers`; sealed hierarchies not yet). `"${obj}"` calls
`obj.toString()`. `Empty` and `empty()` exist.

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
far.

**Statements in strings** (2026-10-26), the `:` lines of a `"""`
string, arrive as `stringExpr(tag, isTagged, \funString, fn { body })`,
the body's text and holes as `\safeStringPart "a"` and `\interpolate b`
statements among its code. As Kotlin's `StringExprMacro` does, without a
tag it's `do { let t = new StringBuilder(); body; t.toString() }`, where
in the body and the blocks inside it (not nested strings with statements)
each run of text is `t.append("a")` and each hole `t.append(cat(b))`.
A tag that names a class is an accumulator type, with or without
statements: `do { let t = new C(); body; t.accumulated }`, with
`t.appendSafe(text)`, escapes undecoded, and `t.append(b)`. Kotlin
evaluates the tag to tell a type from a function; we follow names
instead (`let` names for a class, as imports make, are followed), so a
tag computed some other way is taken as a function. Statements with a
function tag are "not supported yet", where Kotlin crashes on a `TODO`
([kotlin-differences.md](kotlin-differences.md#language-and-types)).

**Destructuring** `let { a, b as c, d is T } = x` lowers as Kotlin's
`flattenMultiInit`, to `let t = x; let a = t.a; let c = t.b; let d: T
= t.d`, spliced into the enclosing block or module (no temporary when
`x` is a name). Decorated, `@var let { a, b } = x`, each declaration,
the temporary too, gets a copy of the decorators, as `flattenMultiInit`
followed by `flattenMultiDeclarations` makes it. As in Kotlin's
`walkDestructuring`, a second `as` is "Extra rename not allowed" and the
first is kept, and `...` outside an import is "Wildcard destructure
allowed only for import" and the rest still declares. `var { a } = x`
isn't destructuring but `var` called with a block, assigned to:
"Malformed assignment", as Kotlin says among other errors. Nested
patterns are "Declaration is malformed"; Kotlin has a TODO for them and
ignores them.

**`do … while`** (2026-10-26) is `Loop(body, next: If(c, {}, Break))`,
as Kotlin's `DoTransform` makes a loop that checks at the end: the body
runs first, `continue` goes to the check, and a label names the loop. A
`do … while` used as a value is void; "Void expressions cannot be used
as values", as Kotlin reports, came with types (milestone-4.md).

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
and it's in [kotlin-differences.md](kotlin-differences.md#changes-we-recommend-to-temper). Across a module's files, the order is global:
the interpreter runs each module's `top_levels`, and each file's root is
reordered to match for printing. Kotlin also puts imports first; ours go
before their readers anyway (see Modules). Uses are counted after name resolution, so a
local that shadows a top-level name doesn't count, as in Kotlin, where
sorting follows `resolveNames`.

**Modules and imports** (`imports.rs`): a module is a directory's
files (`library::Module`), each with its own scope under the builtins.
`export` marks a top-level declaration's symbol exported. `let { a, b as
c } = import("./dir")` waits for that module's scope to complete (the
fact it waits on), then becomes `let a = a'; let c = b'`, with `a'` and
`b'` already resolved to the exported symbols; a name that isn't
exported is Kotlin's "%s does not export symbol %s", and a missing
module, or a cycle of imports (forced when stuck), "Import of %s
failed". Modules run each after those it imports; their declarations
are the interpreter's globals. `run-temper` loads directories that imports name
(`Library::missing_imports`). `let { ... } = import(…)` imports every
export, and `...` beside named ones the rest. `std/temporal` names a
std module under `-std-/`, built in (`stdlib`, from `vendor/temper/std`)
or a directory that `run-temper --std DIR` mounts there (`Sources`);
`check_functional.py` points it at Kotlin Temper's
`frontend/src/commonMain/resources/std`, used as is, and std modules
import each other relatively (`../json`). Not yet: imports outside the
top level (an error), and a `var`'s later changes, which an importer
doesn't see, since it copies the value; backends will want an import
construct instead of the copy.

**Static extensions** (`extensions.rs`): `@staticExtension(Int,
"jsonAdapter") let f()` makes `Int.jsonAdapter()` call `f` where `f` is
visible, under its own name or an import's, as Kotlin requires; the call
becomes `f()` when settling. A class's own static member of that name
wins. Instance extensions (`@extension`) aren't supported yet.

**`@connected` members** are marked (`MEMBER_CONNECTED`). A bodiless
one runs in the interpreter only if it has its own implementation, as
Kotlin's `standardLibraryConnecteds` do; so far std's `Date.today()`, in
UTC, and `regex`'s (below). Bodiless connected top-level functions run
the same way, so far only std `net`'s `sendRequest` (above); others,
like `functions/connected`'s, need the test's host-language code.
This is specialized for core and std; user-space connected code, outside
them, is separate and not done yet, and std should move to it eventually
(design.md, "Longer term").

**Tests** (`testing.rs`): `test("name") { … }` is a top-level `let t# =
new Pair(name, fn (test#: Test) { … })`, a block's own parameter kept and
typed `Test`; a module's first test also adds `let { Test, runTestCases
as r# } = import("std/testing")` to its file, as Kotlin's `TestMacro`
does. `assert(c) { "why" }` is `t.assert(c, fn …)` on the nearest
enclosing parameter named `test` or typed `Test`; without a message it's
`"expected c"`, from the source text where Kotlin prints its tree, or for
`a == b`, `"expected a == (b's value) not (a's value)"`. Each module's
tests run, if asked, after the program (`interp::run_tests`, `run-temper
--test`), through std's `runTestCases`, which reports JUnit XML.
`check_functional.py` runs `runAsTest` tests that way and compares the
failures with `expectedFailures`, as Kotlin's `assertTestingTestFromJunit`
does. Not yet: Kotlin's `Void throws Bubble` return type for tests, and
`Test` imported twice if a module also imports it.

**Regexes** (`regex.rs`, `interp/regexes.rs`): `/a+/` and `rgx"a${s}"`
become `rgx(list(parts), list(values))` calls. Settling parses the
pattern as temper-regex-parser does (Kotlin's `rgx` macro uses it) into
std's `regex` model as plain data, and rewrites the call to build std's
objects from `std/regex`'s exports, `new Sequence([…]).compiled()`; it
waits for that module's scope, and loads it as `test` does
`std/testing`. Interpolated strings become `new CodePoints(s)`, joined
with literal text beside them, at run time, where Kotlin wants them
constant. std's connected `Regex` methods compile those objects, not
their formatted text, for a small backtracking matcher over code points,
leftmost-first as the JVM's and JS's engines are; `find`, `replace` and
`split` follow Kotlin's `RegexExec.kt` and `Regex` (a char past an empty
match). Where backends differ, it follows std's docs: `.` skips only
`\n`, and `^` and `$` match only the text's ends. A [`regex` crate]
would add a dependency and size for features std's model doesn't have,
and its own dialect and rules to reconcile; the matcher is ~150 lines.
Both macros name std modules by path (`std/testing`, `std/regex`), so
they keep working when std moves into this tree. Interpreter-run
connecteds stay std's alone, as Kotlin's are; user-space connected code
is separate.

Later, maybe (user, 2026-10-24): Temper has its own regex parser and
engine written in Temper, which we might switch to; and Kotlin Temper's
Rust backend uses the `regex` crate, adapting std's regex model to it,
which tempest's Rust backend could do too. In Kotlin Temper, `rgx` and
`raw` string literals use similar machinery, so keep them close here.
Aim for roughly the right semantics: Temper's own regex semantics aren't
fully consistent or settled yet.

[`regex` crate]: https://crates.io/crates/regex

**`Map`, `MapBuilder` and `Pair`** (`interp/maps.rs`) are written in the
interpreter, with `Mapped`'s methods from `core.temper`. Entries keep
their keys' first insertion order, keys are `Int` or `String`
(`MapKey`), and `m[k]` and `remove` fail for a missing key. `doPure { …
}` calls its block, `void` is the `Void` value, `raw"a\b"` leaves escapes
undecoded, and `fn` and `throws` types evaluate to placeholders, so std's
top-level type aliases run. `@auto("==")` on a supertype, as on a sealed
interface, applies to its subclasses (Kotlin also requires the hierarchy
to be sealed; we don't check that yet).

**Several declarations in one,** `@var let a = 1, b;` (`var a = 1, b`,
also in a `for` loop's start), become one statement each, every one with
a copy of the decorators, as Kotlin's `flattenMultiDeclarations` does.
Kotlin also moves complex decorator arguments into temporaries; ours are
copied, as decorators other than `var` and `export` are ignored so far.

**`Deque` and `DenseBitVector`** are written in the interpreter, as in
`core.temper`: `Deque` has `add`, `isEmpty` and `removeFirst`, which
panics when empty; a `DenseBitVector`'s bits past the end read as false,
and setting one there grows it. `v[i]` is `v.get(i)`.

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

First picked for using only `let`, `var`, functions and closures,
arithmetic, `if`, loops, `console.log` and simple classes; more since.
69 of 69 pass (`compat/check_functional.py`): `algos/hello-world`,
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
`types/int/shifty`; with modules and `import`, `imports/functions`,
`imports/types` and `imports/values`; with `Deque`, `DenseBitVector`
and several declarations in one, `types/deque`, `types/dense-bit-vector`
and `algos/myers-diff`; with `@fun interface`, `x: T = v` fields and
locals visible after their declaration, `functions/as-values` and
`functions/locals`; with object literals, trailing blocks, `@auto("==")` and `Empty`, `classes/object-literals`,
`functions/named-args` and `types/list/sorting`; with `Map`, `Pair`,
`@auto("==")` from a supertype, `std` imports, static extensions and
`Date.today()`, `types/map`, `interfaces/property-members`, `types/date`
and `types/json-syntax-tree`; with `test` and `assert`,
`testing/asserts`; with regex literals and a matcher, `regex/match` and
`regex/zero-advance`; with generators, `control-flow/actor-run`; with
`async`, `await` and promises, `control-flow/async` and
`control-flow/async-waiters`; with std `net` and a web client,
`types/netresponse` and `types/netresponse-not-found` (which Kotlin's
interpreter doesn't run). The last, `functions/connected`, passes on the
Temper repo's `connected-fallback` branch (2026-10-26), where connected
functions have Temper bodies that the interpreter runs, and its `log`
blocks name the runs they apply to (`interp`, backend ids, `~` to
exclude), as `check_functional.py` now reads them; on `main`, where it
needs the test's host-language code, it still fails, so 68 there.
(On the Temper repo's `connected-types` branch, as of an earlier chunk,
the suite had 67 tests, of which 41 passed.)

### Left over

Instance extensions, the builtin types' remaining methods and std `net`
stay open; the rest needs types, in milestone 4.


- The rest of classes' checks: those listed as not yet under
  Inheritance above, most of which wait for types.
- Trailing-block placement for fun-interface parameters (at compile
  time, for methods too, done in milestone 4); instance extensions; the rest of the builtin
  types' methods. (Object literals are done: they become placed
  constructor calls, with no `\a` markers after that step. General named
  arguments aren't planned.)
- For types (milestone 4), from the language constructs done 2026-10-26:
  `case` patterns, which need the subject's type and aren't implemented
  in Kotlin either; "Void expressions cannot be used as values" for a
  `do … while` used as one (done 2026-10-28); and string tags that
  evaluate to a class other than through names.
- std `net`'s web client, beyond what's done (Status, above): the
  interpreter should end up as capable as any backend (user,
  2026-10-26), and the client is built in for now, to move to connected
  code once there's a plan for user-space connected code, which std
  should use too; then `net` is one more connected library with an
  interpreter implementation. Package management (likely wanted someday,
  maybe in core Temper) would also use it. Wasm: "web client not
  supported here" for now; later our own path to the browser's `fetch`,
  maybe without `web-sys`.

Order chosen (2026-10-24): `std` imports and `...` first, since they're
small and the std files mostly settled already; then what `temporal` and
`json` turned out to need (`doPure`, `void`, `Date.today()`, `Map`,
`MapBuilder`, `Pair`, static extensions), which also unblocked
`types/map` and, with `@auto("==")` from a supertype,
`interfaces/property-members`. Regexes and tests come next, being larger
(a matcher, and a test runner with metadata); generators and `async`
after them. Of those two, tests went first (2026-10-24): smaller, and
independent of the matcher question; then regexes, with a matcher of our
own rather than a crate (see Regexes above). That leaves generators and
`async`, and bodiless connected functions, for the last 6. Generators
went first (2026-10-25), on the flat frames, leaving `async` for 4 of
the last 5; `async` followed (2026-10-26), then std `net`'s pair. `functions/connected` was deferred, as Kotlin Temper's interpreter didn't
run it either; it passes on Kotlin's `connected-fallback` branch (above).
Then the remaining language constructs
(2026-10-26): `do … while`, statements in strings, decorated
destructuring and Kotlin's destructuring messages, and `case` patterns
as far as they go without types.

Target typing waits for static typing in milestone 4 (see design.md,
"Object literals, not named arguments").
