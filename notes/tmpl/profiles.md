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

**Pass order,** fixed: named function types; properties to methods;
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
%console.log(%Int32.toString(%Int32.add(count#2(), (value { %console.log("counted"); 1 }): Int32)));
```

becomes:

```text
let ~count|int32#9: Int32 = count#2();
%console.log("counted");
%console.log(%Int32.toString(%Int32.add(~count#9, 1)));
```

In Python, `count` is a module function the enclosing function calls,
so the temporary takes its second candidate and is named `int32`.
