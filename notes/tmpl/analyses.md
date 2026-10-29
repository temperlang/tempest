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
