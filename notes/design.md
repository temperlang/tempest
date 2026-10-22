# Design

Part of the [rewrite plan](README.md).

## 3. The tree and its memory layout

Kotlin Temper allocates at least four objects per tree child (the node, an
edge object, a copied position, a value or name), plus back-pointers, hash
maps keyed by nodes, and two type representations. The new tree uses a few
big arrays instead:

```rust
struct Tree {
    kinds: Vec<NodeKind>,   // one byte-ish enum per node
    spans: Vec<Span>,       // u32 start, u32 end into the source
    first_child: Vec<u32>,  // children are a contiguous range in `children`
    child_count: Vec<u32>,
    children: Vec<NodeId>,  // NodeId is a u32
    data: Vec<NodeData>,    // small payload: symbol id, literal id, etc.
    types: Vec<TypeId>,     // filled in as types settle; TypeId::UNKNOWN until then
}
```

- **Ids, not pointers.** Nodes, symbols, types and literals are `u32` ids
  into arrays. No reference counting, no back-pointers, no cycles to manage.
- **Interning.** Names and string literals are interned once per build, so
  comparing names is comparing integers. Types are interned too, so equal
  types have equal ids.
- **Rewrites append.** A macro expansion builds its replacement nodes at the
  end of the arrays and points the parent at them. The old nodes simply become
  unreachable; they're freed with the arena. No per-node frees.
- **One arena per library build.** Dropping a build frees everything at once,
  so there's no way to leak earlier builds the way Kotlin's watch mode did.
- **Side tables** hold per-node facts that only some nodes need (resolved
  binding, macro expansion record, diagnostics), as `Vec`s or maps keyed by
  `NodeId`, rather than fields on every node.

Why this is fast: walking children reads contiguous memory, most data fits in
cache, and building a tree costs a handful of `Vec` growths instead of
hundreds of thousands of small allocations. Prismora's histogram showed ~80k
edge objects alone for one build of ~900 lines.

## 4. Parsing

- **Literate split:** done (`temper-literate`). It matches Kotlin exactly on
  all 129 `.temper.md` files.
- **Lexer:** hand-written, producing tokens with byte spans and a kind. It
  keeps comments and prose (for docs and the language server).
- **Parser:** one table-driven operator-precedence parser that builds tree
  nodes directly. Temper's flexibility comes from treating almost everything
  as an operator or call: calls, blocks, generics, `.`, type annotations. The
  same operator set and precedences, plus the "call join" rule that lets a
  word after `}` continue a call, give `if (a) { } else { }`, `foo(x) { }` and
  `a.b { } c { }` with no special cases per keyword. Keywords like `if` and
  `while` are ordinary names bound to builtin macros.
- Parse errors produce error nodes in place, so the rest of the file still
  parses.

## 5. The settle loop

This replaces Kotlin Temper's stages (lex, parse, import, disambiguate,
syntax macro, define, type, function macro, export, generate) with one loop.

### Work items

Each node that isn't settled yet has a pending work item, which records what
it's waiting for:

```rust
enum Waiting {
    Nothing,                 // ready to try now
    Binding(SymbolId),       // name not declared yet (maybe a macro will declare it)
    Type(NodeId),            // needs another node's type first
    Value(NodeId),           // needs a compile-time value (constant, macro argument)
    Module(ModuleId),        // import from a library that hasn't exported yet
}
```

The loop keeps a ready queue. When a fact becomes known (a binding is
declared, a node's type is fixed, a value is computed, a module exports), the
items waiting on exactly that fact move to the ready queue. Nothing else is
revisited, which is the key difference from passes over whole modules.

### Steps

Processing a ready item tries one step:

- **Resolve a name** to its declaration, or wait on the symbol.
- **Expand a macro call:** once the callee resolves to a macro, and the macro
  says which of its inputs it needs (some need only syntax, some need types or
  values), run it. It rewrites its own call node; new nodes get their own work
  items.
- **Evaluate at compile time:** constants, macro bodies written in Temper, and
  `const`-like lets, with a small interpreter over the tree.
- **Infer a type:** each expression produces constraints when its inputs are
  known. Small cases resolve immediately; generic calls are solved per call
  site once their arguments' types are known.
- **Inline** where Temper's semantics call for it (known function values,
  small helpers), when the callee has settled.

### Termination and order

- Facts are mostly **monotonic**: a binding, once found, doesn't change, and a
  type, once fixed, doesn't change. Macro expansions only replace their own
  call node. That makes termination easy to reason about.
- A **step budget** per build catches runaway macro expansion. Hitting it is
  an error that names the cycle.
- The queue is processed in **source order** within each priority, so output
  and error messages are deterministic.
- When the ready queue empties with items still waiting, the loop is
  **settled**. Each remaining item becomes an error (section 6), which may
  unblock others (an error type is still a type), so the loop runs once more.
  It then hands off to tmpl.

### Imports across libraries

Each library builds in its own arena. A library exports a compact summary
(names, types, macro definitions it exports) once its exported items settle.
Importers wait on that summary, not on the whole tree. Independent libraries
can build in parallel.

### Example

```temper
let x = f(1);
let f(n: Int): Int { n + 1 }
```

1. `let x` is a macro call; it declares `x` with an unknown type and queues its
   initializer. `let f` declares `f`.
2. `f(1)` resolves `f` (declared in step 1) and waits on `f`'s signature,
   which settles when `let f` processes its annotations.
3. The call's type becomes `Int`, and so does `x`'s. Nothing is waiting, so
   the loop ends. No stage ordering was needed for the forward reference.

## 6. Errors

- Every diagnostic has a span and message, and is collected, never thrown.
- An unresolvable node becomes an **error node**. Its type is the expected
  type when one is known, otherwise `Never`, so surrounding code keeps
  checking without a cascade of follow-on errors.
- Backends translate an error node into a runtime failure carrying the
  message (`throw`, `panic!`, `raise`, …). The rest of the library still
  works, matching Temper's philosophy.

## 7. tmpl, the backend IR

A small, explicitly specified, fully settled IR:

- Modules with imports and exports; type declarations (classes, interfaces)
  with members; functions; statements (`let`, assignment, `if`, `while`,
  `return`, `throw`, `try`, labeled `break`/`continue`); expressions (calls,
  operators, member access, literals, lambdas, casts, null checks).
- Types are tmpl's own: nominal types with arguments, function types,
  nullable types, and "may fail" (Temper's bubble), never frontend objects.
- Names are tmpl's own (section 8).
- It depends on nothing in the frontend, so backends build and test against
  tmpl alone, and a tmpl dump is a readable debugging format.

Shared lowerings run on tmpl before the backends, chosen per backend:

- **Failure handling:** exceptions, result values, or error codes.
- **Coroutines:** native async/generators, or state machines.
- **Control flow:** labeled breaks for languages without them, expressions
  that must become statements, and so on.

This keeps that logic out of every individual backend.

## 8. Naming

People reading generated code dislike the numbered names (`values__139`,
`_fn_820`, `space_138`). The goal is generated code, including function
bodies, that reads like a person wrote it: **no numbers anywhere except to
resolve a real conflict**.

### Rules

1. **Public names** are source names in the language's convention
   (camelCase, snake_case, PascalCase), escaped for reserved words. Never
   numbered.
2. **Locals, parameters and private members** use their source names. Only an
   actual conflict in the same scope, as the target language sees it, adds a
   suffix, and then the shortest one that works: `x`, then `x2`, `x3`.
   Shadowing that the target language allows is not a conflict.
3. **Compiler-made names** (temporaries, hoisted lambdas, loop labels) are
   named for what they hold, not numbered: the variable they come from
   (`items` for a copy of `items`), the call they hold (`parsed` for
   `parse(...)`), or a short role (`result`, `index`, `done`). Same collision
   rule.
4. **Fewer temporaries in the first place.** Lowerings keep expressions as
   expressions wherever the target language allows, and introduce a temporary
   only when evaluation order or a statement-only construct requires it.

### Infrastructure, so backends get this for free

Naming happens in one shared tmpl pass, not in each backend:

- **Every name in tmpl is a `NameRequest`**: a base name (from source, or a
  role hint for compiler-made names), a kind (local, parameter, field, method,
  type, module-level), and a visibility. Backends never invent name strings.
- **tmpl knows its scopes**: function, block, class body, module, plus the
  target language's extra scoping rules (Python's class bodies don't enclose
  methods; Lua and JS `var` hoisting; C# disallowing a local that shadows an
  outer local). Each backend describes these as data, not code.
- **One resolver** walks the scopes, claims public names first, then source
  names in source order, then compiler-made names, and adds suffixes only on
  conflict. Same input, same names, so output is stable across builds.
- **Each backend supplies a small `NamingRules`**: reserved words, case
  conventions per kind, character escaping, and its scoping quirks. That's
  typically a few dozen lines.
- **Backends print resolved names** with a lookup. A backend that needs a new
  helper variable asks for a `NameRequest` with a role hint and gets a pretty,
  conflict-free name.

C++'s `CppNames.kt` already does "bare name first, suffix only on conflict"
for some names; this generalizes it to everything, everywhere.

## 9. Backends (translators)

Today each backend has a generated per-language AST (~42k generated lines
across seven backends) plus a translator, a support network and post-passes.
Those output ASTs earn their keep in two ways, and the new design keeps both:
**correct output structure** (precedence and parentheses, statements vs.
expressions, blocks and indentation, valid syntax) and **source maps** from
generated code back to Temper source.

The new approach gets both from a small shared layer instead of a full
generated AST per language:

- **A shared output tree.** A compact tree of output syntax shared by all
  backends: statements, expressions, declarations, blocks, comments, plus a
  few escape hatches for language-specific forms. It lives in an arena like
  everything else.
- **Every output node carries the span** of the Temper source it came from,
  copied from tmpl, which copied it from the settled tree.
- **Correctness from per-language tables, not per-language types.** Each
  backend gives operator precedence and associativity, which expressions need
  a statement form, comment and string syntax, and block style (braces,
  indentation, `end`). The shared printer uses them to add parentheses only
  where needed, to refuse invalid nesting (an assertion in debug builds), and
  to lay out blocks.
- **A shared document builder** (Wadler-style: text, line breaks, indentation,
  groups) does the actual layout. It tracks output line and column as it
  prints, and records a mapping whenever it enters a node with a span. That
  produces standard v3 source maps (`.map` files, as today) for every backend,
  at no extra work per backend.
- **Language-specific checks stay possible.** A backend can walk the output
  tree before printing, for things like Lua's local-variable limits or Java's
  checked exceptions, without having its own AST classes.

So a backend is: lowering choices (which shared tmpl lowerings it wants), a
table of how builtins and connected methods map to its runtime library, its
naming rules, its syntax tables, and a translator from tmpl to output-tree
nodes. Most of the hard decisions (control-flow lowering, failure handling,
naming) were already made by shared code, so a backend is mostly "how does
this construct look in this language".
- **The runtime support libraries** (`temper-core` for each language) are
  copied from the Kotlin repo unchanged, which keeps runtime APIs identical.
- **Idiomatic output:** real loops and conditionals, source names, no needless
  temporaries, comments and doc comments carried over.

I expect each backend at 3–5k lines instead of today's ~6–10k hand-written plus
~3–11k generated, but this is the least certain estimate. Some languages
(C++, Rust, Lua) have real quirks, and "simple" depends on how much the shared
lowerings can absorb.

## 11. CLI, watch and language server

- **CLI:** the same commands (`build`, `test`, `run`, `watch`, `repl`, `doc`,
  `init`, `serve`), flags and output formats, so scripts keep working.
- **Watch:** a file-system watcher triggers a rebuild of the changed library
  and libraries that import it, each in a fresh arena. Unchanged libraries
  keep their exported summaries. Memory stays flat by construction.
- **Language server:** built on the same tree and side tables. Completions,
  hover and go-to-definition come from bindings and types already computed by
  the settle loop.
