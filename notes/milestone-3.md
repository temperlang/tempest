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
parameter, method, getter, setter, constructor) and visibility. The body
is read by shape, not expanded as code, so the class's scope is complete
as soon as it's made: members may use each other in any order, after a
synthetic `this` symbol. A bare member name means that member of `this`,
dispatched by name at run time, as `this.name` is. No context record or
parent links were needed: `this` finds its class by walking scopes. The
interpreter's objects hold their own class's fields; a member runs in a
frame for its declaring class's scope holding just `this`. `new`, `as`
and `is` are builtin functions. Not yet: static members, type
parameters, inherited fields, abstract members' parameters, `super`
calls, checks that a class implements its interfaces, and visibility or
`let`-field assignment checks outside constructors.

**Types:** none yet. The interpreter is dynamically typed, and type
expressions are only resolved. Type parameters are reported as not
supported.

### Functional tests

Picked for using only `let`, `var`, functions and closures, arithmetic,
`if`, loops, `console.log` and simple classes, with no lists, generics or
imports. 23 of 69 pass (`compat/check_functional.py`): `algos/hello-world`,
`algos/fibonacci`, `control-flow/if-return`, `control-flow/loop-reenterable`,
`control-flow/loops` (labels, `break`, `continue`), `functions/simple-locals`
(closures, block arguments), `names/nonascii`, `types/int/basics`,
`types/string/is-empty` (`if` as an expression), `semantics/constness`,
`semantics/type-checked-locals` (`do`, `orelse`, `bubble`; Kotlin also
reports type errors there, which we don't yet) and `semantics/broken`
(expected to fail at run time); with classes, `algos/hello-world-object`,
`algos/hello-from-class-to-top`, `casts/as-expr`, `classes/direct-getter`,
`classes/inherited-getter`, `classes/object-literals/extra-class`,
`classes/private-method`, `classes/property-order`, `interfaces/empty`,
`interfaces/pure-virtual` and `semantics/mutually-referencing-types`.
Most others need lists and generics, imports, `when`, `?.` and `??`,
static members, or `Float64` formatting.

### Left for this milestone

- The rest of classes: static members, type parameters, inherited
  fields, `super`, and the checks listed under Classes above.
- `import`, with a library export table and a fact for a module's
  exports.
- `when`, `?.`, `??`, `for … of`, `do … while`, tagged strings, named
  arguments, lists.
- `Yield` and `Await`, for generators and `async`.
