# tmpl: analyses

Part of the [tmpl design](README.md).

## Level 2: Analyses shared by passes

Each is a function from `&Program` to a side table keyed by node or
declaration id. Programs are immutable, so a result is valid for the
program it came from; stored facts are rechecked by recomputing them.

- **Captures:** which declarations each function literal captures,
  and which it or its owner assigns after capture.
- **Purity:** whether an expression may have side effects or bubble,
  for evaluation-order temporaries.
- **Bubbling:** may-bubble calls and their handlers.
- **Call graph and reaches-module-state:** for init guards and init
  order.
- **Definite assignment:** for `Let`s without initializers (Java, Rust,
  and C# reject reads they can't prove assigned).
- **Mutability** ([mutability.md](mutability.md)).
- **Value ranges:** where `Int32` arithmetic can't overflow, for JS,
  Lua, and Python helpers.
- **Liveness and last use:** for letting locals with lifetimes that
  don't overlap share a name, and later for Rust's clones ([question
  3](open-questions.md)).
- **Naming hints** (`naming::hints`): candidate names for any
  expression a pass hoists or copies ([names.md](names.md)), so every pass and backend
  names what it adds well, and function bodies read as hand-written.
- **Type queries:** subtyping, substitution, member lookup, operator
  methods, override chains, and which interface method bodies a class
  inherits (for Go and Rust trait defaults).

## Level 3: Analyses for subset A

Proposed. Each is a plain function returning a side table; subset A
needs these:

- **The operator table** (`builtins::KEYS`, data rather than an
  analysis): each operator's name (`Int32.add`), its signature, whether
  it may bubble (`Int32.div` and `Int32.mod` on zero), and whether it's
  pure. As built, subset A's table also has core's members that
  typing's signatures cover today
  (`crates/temper-frontend/src/typing/signatures.rs`) and
  `console.log`, 113 keys in all; the next chunk moves those to core's
  declarations, leaving 65 operators
  ([nodes.md](nodes.md#level-3-ops-and-core-as-declarations)). A
  test checks each backend's table against it.
- **Order-sensitive operands,** for hoisting: an operand to the left
  of hoisted statements stays in place only if it's a literal, or a
  read of a local that those statements don't assign; anything else
  (a call, a module `var`'s read when the statements call anything) is
  hoisted into a temporary first. In subset A no call can assign a
  local, since there are no captures.
- **May-bubble:** which calls and functions may bubble, for F3 and F4
  and for stored marks.
- **Module state:** which module `let`s each function reads and
  writes, directly or through calls; for F5, [question 19](open-questions.md)'s warning,
  Python's `global`, and later init guards.
- **Definite assignment:** for `Let`s without initializers (R5), and
  for Java, Rust, and C# later.
- **Liveness:** each local's live range, as a span of statement
  positions in its function, for names shared by locals whose
  lifetimes don't overlap ([names.md](names.md)).
- **Naming hints** (`naming::hints::for_expr`): candidates for an
  expression a pass stores, in order: the parameter it feeds (`x` for
  `new Point(x = …)`); the local or field it copies; its callee's name
  with a leading `get` or `to` dropped (`toInt32` gives `int32`,
  `getName` gives `name`); its type's name in lower camel case
  (`point`, `string`); then a role (`result`, `value`). Duplicates are
  dropped, so the list is short.
