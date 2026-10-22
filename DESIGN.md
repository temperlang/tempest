# temper-rs design

A skunkworks Temper implementation in Rust. The goal is Temper's philosophy,
intent and behavior, not its implementation:

- It must pass Temper's functional tests and generate backend code with the
  same public APIs, so libraries and their users can't tell the difference.
- It should be much smaller and easier to understand, faster, use less memory,
  and build to a small single binary. Smaller should come from being simpler,
  not terser: fewer concepts and representations, plainly written code.

Where this design and Kotlin Temper differ in internals, that's on purpose.

## Principles

1. **One representation.** Source becomes one tree, which is refined in place
   until it settles. There are no stages that switch representations, so
   there's no CST → AST → staged tree → woven tree chain.
2. **A settle loop instead of stages.** Each pass over pending work does
   whatever is now possible: resolve a name, infer a type, expand a macro
   whose definition and needed inputs are known, inline a known value. The
   loop ends when nothing changes, or when a step budget runs out
   (non-convergence is an error, not a hang).
3. **Report errors and keep going.** Anything still unresolved when the loop
   settles becomes an error *node* with a best-effort type, and code
   generation continues. Generated code fails at runtime only where the error
   is, as Temper does today.
4. **Data locality.** Nodes live in a per-build arena, referenced by `u32`
   indexes, with interned names and compact spans. A build's memory goes away
   all at once with its arena, so cross-build leaks can't happen by accident.
5. **A clean backend layer.** Backends see only a separate, settled IR
   ("tmpl") with its own names and types, never frontend structures. Naming
   for readable output (source names, suffixed only on real collisions) is a
   tmpl pass shared by every backend.
6. **Reuse what's already the contract.** Each backend's runtime support
   library (`temper-core` for Python, JS, Java, C#, Rust, Lua, C++), the
   Temper-written std library, and the core prelude are copied as they are.
   That keeps runtime APIs identical with no reimplementing.

## Pipeline

```text
.temper / .temper.md
  → literate split (prose vs code blocks)        temper-literate
  → tokens                                        temper-lex
  → operator-precedence parse straight into tree  temper-parse
  → settle loop over the tree                     temper-settle
      names · macro calls · types · inlining · compile-time evaluation
  → tmpl (settled, backend-facing)                temper-tmpl
  → backend code + runtime libraries              temper-be-<lang>
```

### Parsing

Temper's grammar flexibility comes from treating nearly everything as an
operator or a call, so blocks chain as macro or function calls
(`if (a) { } else { }`, `foo(x) { }`, `a.b { } c { }`). The parser is a
table-driven operator-precedence parser with the same operator set,
precedences and "call join" rule (a word after `}` continues the call), and it
builds tree nodes directly. Prose, comments and exact spans are kept, for
docs, the language server and formatting.

### The settle loop

Work is demand-driven, not a sweep over every node each round:

- Each pending item (an unresolved name, a call whose callee isn't known yet,
  an expression awaiting a type) records what it's waiting on.
- When a fact becomes known (a binding, a type, a macro's definition, a
  value), only the items waiting on it are retried.
- Macro expansion is just one kind of step: once a callee is known to be a
  macro and the inputs it inspects are available, it rewrites its call node
  in place, and the new nodes join the queue.
- Compile-time evaluation (constants, macro bodies written in Temper) runs on
  the same tree, with an interpreter.
- Ordering is deterministic, so results and error messages are stable.

Open questions to settle early, with small experiments:

- Which facts must be monotonic (only ever refined) for termination, and how
  to detect oscillation when a rewrite could undo another.
- How type inference fits as incremental steps: per-expression constraints
  solved as their inputs arrive, versus solving a whole function at once when
  its body has settled.
- When an error can "unblock" waiting items (by settling to an error type)
  without hiding the real cause.

### Errors

Diagnostics are collected with spans and never stop the build. An error node
behaves like a value of the expected type where one is known, otherwise like
"never". Backends translate it to a runtime failure with the message.

### tmpl

A small, explicitly specified IR: modules, imports, types, functions,
statements and expressions, with backend-neutral types, plus a strategy
switch per backend for things like coroutines and failure handling. It
depends on nothing in the frontend, so backends can be built and tested
alone, and a tmpl dump is a stable debugging format.

### Backends

Each backend turns tmpl into its language through a small pretty-printing
document builder, aiming for idiomatic code: real loops and conditionals,
source variable names, no needless temporaries. Public API shape (names,
parameters, overloads, types) must match Kotlin Temper's output, which is
checked mechanically (see Compatibility).

## Compatibility

- **Behavior:** Temper's functional test suite (69 markdown tests with
  expected `log` output) is the acceptance test, run on every backend.
- **Generated APIs:** for each backend, extract the public API surface from
  Kotlin Temper's and this implementation's output for the same libraries
  (functional tests, std, Prismora), and diff them.
- **Oracles for the front end:** small Java programs call into Kotlin
  Temper's jars to produce reference results, such as literate code-block
  ranges, so each part can be checked against real behavior file by file.
  See `compat/`.

## Milestones

1. Literate split matching Kotlin on every `.temper.md` file. Done: all 1,727
   blocks in 129 files match (`compat/check-literate.sh`).
2. Lexer and parser that read every Temper source file without errors.
3. Settle loop with names, core macros (`let`, `fn`, `if`, `while`, `class`)
   and the interpreter: run the simplest functional tests in-process.
4. Types in the loop.
5. tmpl plus one backend (Python or JS) passing the functional tests.
6. Remaining backends, the API diff tool, then watch and the language server.

Milestone 3 is the real test of the design: if the settle loop handles
macros, names and evaluation cleanly there, the rest is mostly volume.
