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
hundreds of thousands of small allocations. Prismora's (now Chromarite's) histogram showed ~80k
edge objects alone for one build of ~900 lines.

## 4. Parsing

`temper-syntax` goes from source to a tree in five steps, each checked
against Kotlin Temper (see [testing-and-milestones.md](testing-and-milestones.md)):

- **Literate split** (`literate`): code blocks and paragraphs in
  `.temper.md` files, as commonmark finds them. Code in block quotes is prose
  here, unlike in Kotlin.
- **Lexer** (`lex`): hand-written, tokens of a kind, a byte span and flags;
  their text is the source in the span, or for synthetic tokens (inserted
  closers, paragraph doc comments) fixed by their kind. It keeps comments and
  prose, and knows no keywords beyond Kotlin's: `return`, `throw` and
  `yield` may precede a regex literal.
- **Token adapters** (`adapt`), as Kotlin's `TokenSourceAdapter`: group
  comments; wrap strings in synthetic `( )`, or `{ }` for multi-quoted
  strings with `:` scriptlet lines, whose parts become statements; drop
  space and comments, inserting `;` after a `}` that ends a line and before
  a `{` that starts one; put `callJoin:` between `}` and a following word;
  and turn modifier words into decorators (`public x` is `@public x`,
  `var x` is `@var let x`).
- **Parser** (`parse`): one table-driven operator-precedence parser
  (`operator` has the table), a port of Kotlin's operator stack and its
  `canNest` rules. Temper's flexibility comes from treating almost
  everything as an operator: calls and blocks are infix brackets, and the
  call join gives `if (a) { } else { }`, `foo(x) { }` and `a.b { } c { }` with
  no special cases per keyword. Keywords like `if` are plain names. Words are
  interned (`names`). Operators with too few or too many operands stay in the
  tree, each with an error, so the rest of the file still parses.

The tree (`tree`, section 3's layout) has operator nodes whose children are
operands and the operator's own tokens, like `(Plus 1 + 2)`; a `Leaf` node
holds a run of value tokens, one for `1` or several for `let x`. Every token
the parser sees is in the tree once, in order.

Multi-quoted strings follow rules of our own where Kotlin's handling was
flaky: blocks opened on scriptlet lines carry over to later ones in order
and close when the string ends; a scriptlet line stays code after a `${...}`
in it; each content part is its own statement. The full list is in
testing-and-milestones.md, and examples in `tests/parse/multi-quoted.cases`.

The **grammar step** (`ast`, with the productions in `grammar`) turns the
operator tree into the tree the settle loop works on, as Kotlin's
`astbuild` does:

- **Few node kinds:** `Call` (callee, then arguments), `Block`, `Decl`,
  `Fun`, `Esc` (a quoted tree), names (`LeftName` declared, `RightName`
  used, `BuiltinName` never shadowed), `Value` (numbers, strings, symbols
  like `\word` naming the next argument, builtins like `desugarOperation`),
  and `Error`. Values and names are side tables; each node has an origin
  link to the parse node it came from.
- **No keywords.** `if (a) { b } else { c }` is a call of `if` with `a`, a
  `Fun` for the block, then `\callJoin:`, `\else` and the next `Fun`;
  `let x: T = 1` is a `Decl` with `\type` and `\init` pairs; `class C
  extends A {}` is a call of `class` with `\word`, `C`, `\super`, `A` and a
  `Fun`; `a + b` is `desugarOperation(+, a, b)`; `@A let x, y` is a call of
  `@` around a comma of declarations; `"a${b}"` is a call of `stringExpr`.
  Macros give these meaning later.
- **A port of Kotlin's grammar.** The parse tree is flattened into open,
  token and close parts, and productions written as methods (each with
  Kotlin's definition in its doc comment) match them PEG-style, emitting
  start, leaf, finish and swap events; a failed alternative drops its
  events. A lift step then builds the tree from the events, reading names
  and values from tokens and reporting malformed numbers. Matching Kotlin
  production by production keeps its exact structure and error messages
  (`Expected a TopLevel here` and the like), checked against Kotlin on all
  inputs (`compat/check_ast.py`).
- **Errors are values:** unparsable parts become an `Error` node with a
  diagnostic, and the rest still builds. Operands nested more than 1000
  deep become errors too, where Kotlin runs out of stack (at about 300
  nested calls), so nothing overflows ours, including in wasm.
- **Fast where Kotlin backtracks badly.** Kotlin's grammar takes exponential
  time on some nesting (`((((x))))`, `a[a[a[x]]]`, `@a (@a (x))`), because
  an alternative parses a whole operand before failing on something after
  it. Ours checks that cheaply first, with the same result.
- **Finishing a module** (`module::build_module`), as the rest of Kotlin's
  `ParseStage`: the code's tree is wrapped in a block if it isn't one; the
  `;;;` metadata, built with Kotlin's `Json` production, becomes a `Json`
  value, with `InvalidMetadata` errors (and null) where it isn't JSON, such
  as `1i64`; and call joins are rewritten. For a library, what follows each
  `\callJoin:` and its symbol becomes a function passing it all to its
  argument, so `if (a) { b } else { c }` is
  `if(a, fn { b }, \else, fn (f#0) { f#0(fn { c }) })`, where `f#0` is a
  temporary name (never interned, so it can't clash with a source name);
  for documentation, the `\callJoin:` is just dropped.

### The parse tree and the working copy

There's one node type, in two copies:

- **The parse result** (`Parsed`) is shaped like the source and never
  changed after parsing. It serves formatting, outlines, docs and finding
  nodes by position for the language server. It keeps every lexer token, so
  comments and blank lines are found between node spans rather than stored
  in the tree, and synthetic tokens (inserted semicolons, call joins, string
  parentheses, closers) are flagged so a formatter can skip them.
- **The settle loop works on a copy** (`ast::Ast`, from the grammar step),
  without what only formatting and the language server need. Its nodes keep
  an origin link to the parse node they came from, for error spans, hover
  and go-to-definition.

There's no separate lossless syntax tree type.

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

### Risks to design for

- **Facts only get added.** Termination and determinism depend on it. Macros
  that need types (Kotlin's late "function macros") are the risk: they may
  rewrite only their own call node, and anything that read a fact that could
  still change must wait until they've run. Otherwise facts get retracted
  and the loop can oscillate.
- **Cycles get a deterministic tie-break,** not only an error: for example a
  local recursive function whose type depends on itself breaks at the
  earliest item in source order, so output stays stable.
- **Debugging:** a fixed point can't be dumped between stages the way a
  pipeline can. Build a "why is this node still waiting, and on what chain?"
  query early, and put that chain in errors for things that never settle.

### As built so far

`crates/temper-frontend` has the first slice (details and differences in
[milestone-3.md](milestone-3.md)): a generic scheduler (`engine`) and
kinds of work: reading a block's statements in order to find its
declarations, visiting an expression to resolve names and expand macros,
completing a class's scope with what it inherits, and inferring a
declaration's type, in a second round once expansion has settled
([milestone-4.md](milestone-4.md)). The facts they publish: a scope's
declarations are complete, a class's scope is, a declaration's type is
known. A stuck item is forced to go on without its fact, the earliest first,
rather than left to become an error.

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

### Declaration facts in one place

Kotlin Temper spreads facts about a declaration across stages and shapes:
`@connected` arrives as a decorator that applies after a class body is
read, so the class macro has to look for unapplied decorations to learn
that its bodiless methods are connected rather than pure virtual
(`connected-types` branch, 2026-10-23). Here, a declaration's modifiers
(connected, static, exported, abstract or concrete, visibility) should be
read once, before its members are, into one record on the declaration,
and members should see their owner's record through the context they're
settled in. Then "a bodiless method of a connected class is connected"
is one rule in one place, and backends ask one question ("is this
connected?") to skip a host-provided type and all its members.

Kotlin Temper has to guess at that rule early: the class macro runs
before `@connected` applies (the decoration wraps the class's result), so
it looks for the decoration by value around its own call. Reading
modifiers first avoids that chicken-and-egg ordering entirely.

Default arguments are applied inside the callee, never filled in at call
sites, on purpose. So connected classes can't have methods (constructors
included) with default arguments: host code implements them directly, and
nothing would apply the defaults. Kotlin Temper reports that as an error
(`ConnectedMethodDefaults`, 2026-10-23); connected top-level functions keep
defaults, since Temper generates a wrapper that applies them.

Longer term (user, 2026-10-23): std should use the same user-space connected
types as everyone, with only core keeping custom code generation. One
catch: std sometimes exposes per-backend native types unwrapped (dates,
for example), while user-space connected types wrap them. Per-backend code
or decorators in Temper source stay avoided, though an opt-in for people
who need them could come someday.

Core and std sources belong in tempest's own tree eventually, built into
the single binary in some form (`run-temper --std` pointing at Kotlin
Temper's copy is a stopgap). Std might be split out someday, but for now
it ships with the compiler.

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

### Function types and fun interfaces

Fun interfaces (`@fun interface F(params): R;`) are meant to replace bare
function types: each function type gets a name in the source, and backends
use it as they can. Backends with function types (JS, Python, Rust, C++,
Lua, likely C#) emit plain function types and lose nothing; backends that
need names (Java, and C# if we choose) emit the named interface rather
than a synthetic one. This follows a general rule: Temper makes it easy to
give things unique names, and backends that allow more flexibility use
less of it.

So tmpl should carry the interface's name on function types, and the
interpreter treats a fun interface value as the function itself (as it
does now). Kotlin Temper hasn't finished this plumbing, and bare function
types still exist there, so for now tempest accepts both and stays
compatible with the functional tests; don't wire fun interfaces as mere
sugar for `apply` methods in ways that would make this harder later.

What Kotlin Temper does, as a hint at the non-sugar meaning (2026-10-24):

- **Declaration.** `@fun interface F<T>(x: T): R;` is an abstract
  interface marked `@functionalInterface`, with one public method `apply`
  and no `this` parameter ("arguably this is the function"). Its type
  category is *Functional*, and its signature is `apply`'s, with the
  interface's type arguments substituted (`sigForFunInterfaceType`).
- **Old function types become fun interfaces.** The newer type system
  (`type2`) represents each bare `fn (A): R` as a synthesized fun
  interface `Fn<I…, O>`, one per shape (count of required and optional
  inputs, type formals, whether it can bubble), with contravariant inputs,
  a covariant output and a `Function` supertype. So structurally, a
  function type is just an anonymous fun interface, which is the direction
  the language is going.
- **Calls and values.** Anything whose type is a fun interface is
  callable: the typer finds callees through `apply`'s signature. tmpl
  marks the crossing between function values and fun interface values
  (`FunInterfaceExpression`, `FunInterfaceCallable`), and backends that
  use function types just drop it.
- **Per backend:** `FunctionTypeStrategy` is `ToFunctionType` (no
  interface emitted, uses become plain function types, as in C++'s
  `std::function` from the `apply` signature) or `ToFunctionalInterface`
  (emit the interface with its `apply` method). Every real backend uses
  `ToFunctionType` today, Java included, with a TODO there to rework it to
  use `@fun` interfaces, and Java's `FunInterfaceExpression` is a `TODO()`.
  That's the unfinished plumbing.

For tempest: a fun interface is a nominal type whose meaning is a
function signature. Keep the name through tmpl, treat values as
functions, and let each backend pick a function type or the named
interface.

### Object literals, not named arguments

Temper is dropping named arguments everywhere except object literals
(property bags), because named arguments don't work well in every
language. In C#, for example, callers' named arguments make parameter names
part of the API, and a name we'd have to escape can break backward
compatibility. Where names really help, the idiom is a small helper class
built with an object literal.

- **What exists:** `{ a: 1, b }` and `{ class: C, a: 1 }` construct a
  class. They call its constructor, maybe `@factory` static methods later.
  Nothing else takes arguments by name, and tempest needn't support named
  arguments otherwise. If a classic Temper test uses them beyond object
  literals, change the test there. As of 2026-10-24 none does:
  `functions/named-args` was already rewritten to use object literals.
- **No first-class symbols.** It's not settled that Temper keeps
  first-class symbols at all, so don't build on them. The parser's `\a`
  markers may stay a parse-time detail of the object-literal conversion.
  After that, the tree and tmpl should have a constructor call with its
  arguments already placed (and a property-name map only if a backend
  needs one), never symbol values.
- **Which class:** target typing first, once static typing is in
  (milestone 4); until then, find by names only. Where the context says what type
  is expected, the literal constructs that class: a declared variable,
  field or return type, a parameter of a known callee, or a list element
  type. Kotlin Temper doesn't do this yet; tempest should, with its own
  tests (classic Temper should follow). Without an expected type, or as a
  fallback, Kotlin's `ConvertObjectLiteralNew` rule applies: the one class
  in scope whose constructor takes every given name. If the expected type
  is an interface, it narrows that search to classes implementing it.
  The user would like to drop find-by-matching-names entirely, but that
  needs a team decision, so keep it for now behind target typing.
- **Target typing beyond object literals:** the same expected types should
  type list literals (`let xs: List<Point> = []`, and object literals as
  their elements) and function literals (parameter and return types from
  an expected function type or fun interface, as Kotlin already does for
  callees' parameters, such as `sorted { a, b => … }`), wherever the
  context gives one.
- **Backends:** languages with good named arguments or object literals use
  them (JS and Python options, maybe C#, with the escaping caveat above).
  Languages without them get a builder when a constructor has more than
  one property: Java's nested `Builder` class, and Rust's `…Builder` /
  `…Options` structs, as Kotlin's `classBuilder` and
  `processForClassBuilder` do. That's decided in shared code, once.

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
- **Watch:** maybe not needed. If a full library build takes tens of
  milliseconds, rebuilding from scratch on each run is simpler than keeping a
  process alive, and `watch` can stay a thin loop that reruns `build` or
  `test`. If it's needed, a file-system watcher rebuilds the changed library
  and libraries that import it, each in a fresh arena, keeping unchanged
  libraries' exported summaries, so memory stays flat by construction.
  Decide once the full compiler is measured.
- **Language server:** built on the same tree and side tables. Completions,
  hover and go-to-definition come from bindings and types already computed by
  the settle loop.
