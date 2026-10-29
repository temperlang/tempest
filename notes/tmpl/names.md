# tmpl: declarations, names, and linking

Part of the [tmpl design](README.md).

## Level 2: Declarations, names, and linking

The `DeclTable` holds every declared thing, `DeclId` dense from 0. Each
entry:

- **Kind:** module, class, interface, function, method, constructor,
  getter, setter, field, module `let`, local, parameter, type parameter,
  label, or test.
- **Name:** the source name, or for compiler-made declarations a role
  hint (`result`, `index`, `done`, or "copy of" another declaration).
  With kind and visibility, this is the `NameRequest` (design.md section
  8).
- **Owner:** the enclosing declaration; owners form a tree rooted at
  modules.
- **Facts read once** (design.md, "Declaration facts in one place"):
  visibility, exported, static, abstract, `var`, connected (and how
  it resolves, [below](#level-3-connected-declarations)), `@keep`,
  `@fun`, `@operator` marks, `@auto("==")`, optional (for
  parameters with defaults), generator, `async`, may bubble, `@imu`
  and `@partialImu` (on types and type parameters), and compiler-made.
- **Type,** span, and docs.

**External declarations** stand for what this library uses from
another, core included: same entry, no body, linked by its qname
([below](#level-3-qnames); Proposed 2026-10-29, replacing a separate
link key). Temper has no overloading, so a qname is unique; the
declaration's kind and type are kept too, so a reader checks them
against the dependency's exports and reports a mismatch rather than
mis-linking. Within a library, modules share one
table and refer to each other's declarations by `DeclId`; the import
list says which ([modules.md](modules.md)).

**Pretty names everywhere, inside functions too** (Decided 2026-10-29;
design.md section 8: no numbers except to resolve a real conflict).
Users have complained even about suffixed locals, so tmpl makes this
achievable:

- **Every local, parameter, label, and temporary is a declaration with
  a name:** its source name, or for one a pass introduces, a hint
  saying what it holds. The validator rejects a compiler-made
  declaration without a hint, so no pass can fall back to `tmp`.
- **Hints come with candidates,** in preference order, from shared
  helpers in `naming::hints`, so passes name what they add the same
  way: the parameter a value feeds (`x` for `new Point(x = …)`), the
  property or local it copies (`items`), the call it holds without
  `get` or `to` (`parsed`, `int32`), its type (`point`), or a role
  (`result`, `index`, `done`, `state`).
- **Scopes and shadowing are explicit:** every declaration's owner is
  its enclosing block, function, class, or module, and every reference
  is a `DeclId`, so two source locals named `x` are told apart without
  renaming them in tmpl.
- **The naming pass renames only on a real conflict,** as the target
  sees it: `NamingRules` describe its scoping as data (Python's and
  JS `var`'s function scope, Python's class bodies not enclosing
  methods, C#'s ban on shadowing an outer local, Rust's free
  shadowing). Order: exported names first, never suffixed (a clash
  between exports is an error reported once, not renamed), then source
  names in source order, then compiler-made names. On a conflict it
  tries the hint's next candidate, then, in function-scoped targets,
  sharing the name with a local whose lifetime doesn't overlap and
  isn't captured (liveness, [analyses.md](analyses.md)), and only then the shortest suffix
  ([question 23](open-questions.md)).

The naming pass returns a `NameMap` from `DeclId` to string; tmpl
itself stores no output names.

## Level 3: Declarations

Proposed. The `DeclTable` holds one record per `DeclId`, in columns
like the nodes:

```rust
pub struct Decl {
    pub kind: DeclKind,          // u8; `Removed` for a pass's tombstone
    pub name: StrId,             // the source name, or the first hint candidate
    pub alts: StrListId,         // further candidates; compiler-made only
    pub owner: DeclId,           // its function, class, or module; NONE for a module
    pub ty: TypeId,              // a function's is its signature
    pub facts: Facts,            // bit set: Level 2's facts read once
    pub visibility: Visibility,
    pub qname: StrId,            // an override of the derived qname, or NONE
    pub connected: bool,         // marked @connected (or core's equivalent)
    pub span: Span,
    pub name_span: Span,
}
```

A local's owner is its function, not its block: the block scope comes
from the tree (the `Let`'s enclosing `Block`), so it isn't stored
twice. A loop's label is a declaration of kind label, owned by its
function.

## Level 3: Qnames

**Decided (2026-10-29):** tmpl has a place for qualified names
(qnames). Kotlin Temper derives one for each declaration, and its
backends map core's and std's qnames to custom translation. tmpl's may
follow Kotlin's format, so backend tables and compatibility carry
over.

Proposed:

- **Kotlin's format, exactly** (its `QName.kt`): the library's name,
  the module's directory relative to the library root, then one part
  per enclosing declaration, each kinded by punctuation: `x` (a top
  level or property), `x=` (a local), `f()` (a function or method),
  `(x)` (a parameter), `<T>` (a type parameter), `type C`,
  `constructor()`, `get x()`, and `set x()`. So
  `my-lib/foo/bar.type C.f().(x)`. `#n` follows a qname that would
  otherwise be ambiguous in its module (two locals `x` in one
  function), in declaration order; `\` escapes meta-characters. Core's
  library is written `core` and std's `std`, as in Kotlin:
  `core.type Int32.toString()`, `std/temporal.type Date`.
- **Why exactly Kotlin's:** backend tables port verbatim (Kotlin's
  Python table has about 160 entries keyed by qname); a library's
  qnames identify its declarations across versions, for api-compat and
  `temper.keep`; and Kotlin's per-library qname metadata stays
  comparable, for a bridge.
- **Every declaration has one, except compiler-made ones**
  (temporaries and compiler-made labels), as in Kotlin. A declaration
  the compiler makes for the surface, such as an `@auto("==")` `eq` or
  a default constructor, gets the qname Kotlin gives it.
- **Derived, with overrides.** `qname(&Program, DeclId)` builds it from
  the library's name, the module's path, the owner chain, the source
  names, and the declarations' kinds. A local's owner is its function,
  so a local in a nested block is `f().y=`, as in Kotlin. Stored is
  only an override, `NONE` almost always: the part after the module's
  path, where derivation would differ (a disambiguation index, or
  Kotlin's kind differing from tmpl's, as `f()` for a module `let`
  holding a function literal). The frontend computes qnames by Kotlin's
  rules (`AttachQNameMetadata.kt`) and stores those that differ.
- **Qnames are link keys:** an external declaration links to its
  dependency's export by qname.
- **Validator:** qnames are unique within a library, and every
  declaration but compiler-made ones has one (rules Q1 and Q2,
  [nodes.md](nodes.md#level-3-the-validators-rules)).

## Level 3: Connected declarations

**Decided (2026-10-29):**

- **One concept from outside:** core's, std's, and user code's
  `@connected` declarations are all "connected". Inside, each is
  resolved either by its translated name or, for core and std, by
  qname lookup to custom translation or lowering.
- **Customization is general, by naming:** a backend customizes only
  operators and connected declarations, never a hardcoded table of
  core functions.
- **User space resolves by name.** Kotlin Temper expects the user's
  host code to define the name the translation would otherwise give
  the declaration; it uses no qname or key. Only core's and std's
  connected declarations are mapped by qname.
- **Std may move to user space's mechanism** someday; core likely
  keeps specialized tree rewriting.

Proposed:

- **A fact on the declaration,** `connected: bool`, from `@connected`
  (core.temper marks its members the same way). Decided (2026-10-29):
  how it resolves isn't stored; it's recognized from the declaration's
  library. Core's and std's go through the variant's connection table
  by qname ("by qname" below), and everything else by name ("by name"
  below). One shared helper, `resolves_by_qname(decl)`, makes that
  call, so moving std over later changes only the helper and std's host
  code.
- **A connected declaration may have a body,** its pure Temper
  fallback (Kotlin: a translation that returns nothing uses the Temper
  implementation). Without one, it needs a translation on every
  backend.
- **By qname (core and std):** the variant's connection table, sorted by qname with
  Kotlin's strings as keys, maps a declaration to data or a hook:

  ```rust
  pub enum Connection {
      Template(&'static str),  // inline, with argument slots: "len({0})"
      Runtime(&'static str),   // a function in the variant's runtime library
      Type(&'static str),      // a connected type's host type
      Translate(fn(&mut Translator, &Call) -> Out), // custom output
      Lower(fn(&mut Rewriter, NodeId) -> NodeId),   // custom tmpl
  }
  ```

  `Lower` hooks run in one pass, first in the fixed order
  ([profiles.md](profiles.md)), so what they produce is lowered further
  like any tmpl; shared ones live in `temper-tmpl` for any variant to
  name. A declaration with no entry uses its body; with neither, it's
  an error naming the qname. Tests check each table: every key names a
  connected declaration of core or std, and every bodiless one has an
  entry.
- **By name (user code):** the naming pass names the declaration as any other,
  and the translator calls host code by that name, in the module's
  host file (Kotlin's convention: `__connected__.py`, `_connected.rs`,
  and so on), after applying defaults, since defaults apply in the
  callee. With a body, the body is the pure Temper fallback, as in the
  connected-fallback design proposed for Kotlin Temper: the declaration
  may list the backends that have host code
  (`@connected(["cpp", "rust"])`); listed backends call host code, and
  unlisted ones, and the interpreter, use the body. With no list, every
  backend needs host code, as today (Decided 2026-10-29, [question
  41](open-questions.md)). In user code, only top-level functions so
  far. Host code depends on those names, so connected and `@keep`
  declarations claim theirs first, with exports, and are never
  suffixed (the naming pass, below). In the first variants, these
  names are Kotlin's, checked against the functional tests' host files.
- **Operators aren't connected:** each variant spells every `Op`
  ([nodes.md](nodes.md#level-3-ops-and-core-as-declarations)) in a
  table indexed by `Op`, complete by construction.

## Level 3: The naming pass and each backend's `NamingRules`

Proposed, apart from Level 2's Decided points. `naming::assign(&Program,
&NamingRules) -> NameMap` gives every local declaration a string; the
`NameMap` is a `Vec<StrId>` by `DeclId`. External declarations take
their own library's names, and connected declarations resolved by
qname their variant's table's ([above](#level-3-connected-declarations)).

### The pass

1. **Scopes as the target sees them.** Walk each module and put every
   declaration in the scope its target gives it: module, class body,
   function, or block. Python and JS `var` put a block's locals in the
   function's scope; Python's class bodies don't enclose methods.
2. **Claim in order:** first each module's exported, connected
   (user code's), `@keep`, and other module-level names, then, scope by scope from the outside in, source names in
   source order (parameters first), then compiler-made names in
   `DeclId` order.
3. **Candidates:** the source name, or a compiler-made declaration's
   hint candidates; each put in the target's case for its kind and
   visibility, then escaped if reserved.
4. **Conflicts.** A candidate conflicts with a declaration that holds
   the same name when they're in the same scope as the target sees it;
   or the other is in an enclosing scope and is referenced inside this
   one's scope, so shadowing would change what that reference means;
   or the target forbids shadowing (C#). It also conflicts with
   reserved words and with names the translator prints in that scope
   (runtime helpers, and names from the variant's operator and
   connection tables). In Python, a
   local's scope is its whole function, since Python decides locality
   per function: a temporary can't be named for a module function
   that the function calls anywhere.
5. **First free candidate wins.** If none is free, a local in a
   function-scoped target may **share** the name of a local of the same
   function whose live range doesn't overlap its own, if neither is
   captured. Otherwise the first candidate takes the **shortest
   suffix** that's free: `x2`, `x3`, ….
6. **Exports are never suffixed:** two exports with one output name are
   one error, reported once. The same holds for user code's connected
   and `@keep` declarations, since host code calls them by name
   ([above](#level-3-connected-declarations)).

The result depends only on the program and the rules, so names are
stable across builds. `NameMap::suffixed()` lists the declarations
that got a suffix, for the golden count
([testing.md](testing.md#level-3-golden-tests)).

### `NamingRules`

```rust
pub struct NamingRules {
    pub reserved: &'static [&'static str], // keywords, and names the translator prints
    pub case: fn(DeclKind, Visibility) -> Case, // Keep, Camel, Snake, Pascal, ScreamingSnake
    pub private_prefix: &'static str,      // "_" for Python's internal top levels
    pub escape: Escape,                    // Suffix("_"), or Prefix("r#") in Rust
    pub suffix_separator: &'static str,    // "" gives `x2`
    pub identifier: fn(char, bool) -> bool, // allowed characters, first or not
    pub block_scoped_locals: bool,         // false in Python
    pub class_body_encloses_methods: bool, // false in Python
    pub may_shadow_outer_local: bool,      // false in C#
    pub may_rebind_in_scope: bool,         // Rust's `let` shadowing
    pub one_namespace: bool,               // types and values share names (Python, JS)
}
```

A character a target doesn't allow becomes `_` plus its hex code, and
that name is then treated as any other candidate.

**Python's first variant,** Kotlin-compatible (Decided 2026-10-29),
uses the conventions api-compat.md records: snake_case values,
functions, methods, and properties; types keep their case; reserved
words get a trailing `_` (`from_`); and internal top levels get a
leading `_`. Its reserved list is Python's keywords plus the builtins
and runtime names its translator prints. Kotlin compatibility covers
the public surface only: parameters are positional-only, so their
names, like locals', are free to be pretty.

**Example,** Python:

```temper
let f(a: Int): Int {
  if (a > 0) { let x = a * 2; console.log(x.toString()); }
  else { let x = 0 - a; console.log(x.toString()); }
  let x = a + 1;
  { let x = 3; console.log(x.toString()); }
  x
}
```

The first two `x`s share `x`: one scope in Python, but lifetimes that
don't overlap. The third `x` shares it too. The fourth is alive while
the third is, so it's `x2`, the only suffix, and the one the golden
count would show.
