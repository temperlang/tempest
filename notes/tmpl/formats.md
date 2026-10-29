# tmpl: formats and versioning

Part of the [tmpl design](README.md).

## Level 1: Formats

**Decided (the direction):** a binary and a human-friendly but complete
text form, each round-tripping losslessly; one schema defining both;
one canonical, complete text form; an optional viewer that elides
derivable detail; binary also serving the library cache.

- **Decided (2026-10-29): canonical text is Temper-like,** with a
  distinct header and an obviously explicit style, so it's never
  mistaken for Temper. It leaves out types that the three local rules
  give back (a literal's type, a name's type from its declaration, and
  a call's result from its callee's signature with explicit type
  arguments); the reader restores them and the validator checks them.
- **The schema is Rust code in the crate** (Proposed), a `macro_rules!`
  table that declares each node kind and its fields once, and generates
  the node types, a field walker, and the text and binary readers and
  writers. No proc macros or new crates (code-style.md).
- **Spans in a trailing section,** optional on reading: golden tests
  strip it, and text without it reads with unknown spans.
- **The viewer** elides ids, conversions, capture lists, and obvious
  types for people. It never needs to read back.
- **Binary** can come later without changing the model, written to a
  spec so another implementation (Kotlin Temper, as a bridge) could
  read it.

Level 2, below, has the structure and an example.

### Where the goals conflict, and the recommendations

The user's priorities: (1) complete and explicit for every backend, (2)
profiles and lowering, (3) text round-trip, (4) binary.

- **Explicit versus readable text (1 against 3):** derivation rules plus
  a trailing spans section keep canonical text complete yet close to
  source; the viewer covers reading.
- **Explicit versus idiomatic output (1 against design.md's naming
  rules):** order is a guarantee and purity a fact; temporaries come
  only from a profile's lowering.
- **One model across levels versus simple passes (2):** node families
  with default structural walks, so a pass touches only its kinds.
- **One starting form versus per-backend choices (1 and 2):** where a
  choice is purely a target's spelling (Java's boxed type parameters,
  Rust's `Option`), it belongs in that backend's translator, not a pass.
- **Binary later versus a stable cache (4):** versioning starts on day
  one, in the text header too ([below](#versioning-and-compatibility)).

### Versioning and compatibility

Proposed:

- **Two versions in every file's header:** the format's (how it's
  encoded) and the model's (what nodes and fields mean). Text and binary
  share the model version.
- **At first, exact match only:** a cache is valid only for the compiler
  that wrote it ([library-cache.md](../library-cache.md)), so the model can
  change freely until tempest's backends settle.
- **Later, for published caches:** new node kinds and fields only with
  defaults; meaning changes bump the major version. Recorded in
  `temper.keep` alongside other backward-compatibility facts.
- **Only the starting tmpl is ever stored.** Lowered forms are internal
  to a build, so passes can change without a version bump.

## Level 2: Format structure

**Text** (Decided 2026-10-29: Temper-like, explicit, with a distinct
header). Its shape:

```text
file    := header uses* module* spans?
header  := "tmpl" FORMAT-VERSION "model" MODEL-VERSION "library" NAME VERSION
uses    := "uses" LIBRARY VERSION "{" external-decl* "}"
module  := "module" NAME#ID "{" import* decl* "init" "{" stmt* "}" test* "}"
```

Its style, so it's never mistaken for Temper:

- Every declaration is written `name#id`; compiler-made ones are
  `~role#id`. References are `name#id` too, the name checked by the
  reader ([question 18](open-questions.md)).
- Declarations always carry their types; expressions carry them only
  where the local rules don't give them back, as `(e): T`.
- Control nodes use Temper's syntax (`if`, `while`, `for`, `&&`,
  `orelse`, `return`); every builtin is a call written `%Key(…)`, so
  `%Int32.add(i#4, 1)`, never `i + 1`.
- A call that may bubble is marked `^`.
- Bodies are explicit: constructors assign their fields, and a
  function's last value is a `return`.
- Spans follow in a trailing `spans { … }` section, keyed by each
  node's position in writing order.

**The viewer** writes the same shape with options: hide ids where a
name is unique, hide conversions and capture lists, show `==` and `+`
for their builtins and operator methods, and show spans as
`file:line:column`. A CLI command (`tempest tmpl FILE --profile py
--view`) prints starting or lowered tmpl.

**Binary:** a header (magic `TMPL`, format and model versions, the
library's identity and its dependencies' versions), then a section
table, so a reader can skip docs or spans, and sections for strings,
types, declarations, external links, nodes stored column-wise (kinds,
types, child ranges, payloads), docs, and spans delta-encoded. Integers
are LEB128 varints and every reference an index. Equal programs give
equal bytes, so caches can compare by hash.

### Worked example

Temper source:

```temper
export class Point(public x: Int, public y: Int) {
  @operator("==")
  public eq(other: Point): Boolean { x == other.x && y == other.y }
}

export let firstIndex(points: List<Point>, text: String): Int {
  let wanted: Point = { x: text.toInt32() orelse 0, y: 0 };
  var i = 0;
  for (let p of points) {
    if (p == wanted) { return i; }
    i += 1;
  }
  -1
}
```

Starting tmpl (canonical text, spans left out):

```text
tmpl 1 model 1 library example 0.1.0
uses temper-core 0.1.0 {
  class List#90<T#91>;
  class String#92 { method toInt32#93(): Int32 throws Bubble = %String.toInt32; }
}

module example#1 {
  export class Point#2 {
    public field x#3: Int32;
    public field y#4: Int32;

    public constructor new#5(x#6: Int32, y#7: Int32) {
      this.x#3 = x#6;
      this.y#4 = y#7;
    }

    @operator("==") public method eq#8(other#9: Point#2): Boolean {
      return %Int32.eq(this.x#3, other#9.x#3) && %Int32.eq(this.y#4, other#9.y#4);
    }
  }

  export fn firstIndex#10(points#11: List#90<Point#2>, text#12: String#92): Int32 {
    let wanted#13: Point#2 = new Point#2((%String.toInt32(text#12)^ orelse 0): Int32, 0);
    var i#14: Int32 = 0;
    for (let p#15: Point#2 of points#11) {
      if (p#15.eq#8(wanted#13)) {
        return i#14;
      }
      i#14 = %Int32.add(i#14, 1);
    }
    return -1;
  }

  init {}
}
```

The object literal is a `New` with placed arguments; `p == wanted` is a
call of `eq#8`; the field reads inside `eq#8` use `Int32`'s own
equality; and `orelse`'s type is written, since no local rule gives it.

**Lowered for Python** (exceptions; no statements inside expressions;
everything else native). Only `firstIndex`'s first statement changes:

```text
    var ~x#16: Int32;
    try {
      ~x#16 = %String.toInt32(text#12)^;
    } catch {
      ~x#16 = 0;
    }
    let wanted#13: Point#2 = new Point#2(~x#16, 0);
```

The temporary's hint comes from the constructor parameter it feeds,
`x`, with `parsed` (from `toInt32`) as its next candidate. Python's
locals are then `points`, `text`, `x`, `wanted`, `i`, and `p`, with no
numbers. Had `firstIndex` already a local `x` alive here, the temporary
would be `parsed`, not `x2`. Evaluation order needs no other
temporary, since `0` is pure. If [question 14](open-questions.md) lands as recommended,
`eq#8` presents as `__eq__` too, and the translator may print `p ==
wanted`; `%Int32.add` prints as `i + 1` only where value ranges rule
out overflow.

**Lowered for Rust** (results; everything else in this example native):

```text
    let wanted#13: Point#2 = new Point#2(result_or(%String.toInt32(text#12), 0), 0);
```

`%String.toInt32` now has the lowered signature, returning
`Result<Int32>`, so the call needs no `^`; `ResultOr` replaces the
`orelse` over one call, and the translator may print `.unwrap_or(0)`.
`firstIndex` doesn't bubble, so its signature is unchanged. No
temporary is added, so Rust's locals are the source's: `points`,
`text`, `wanted`, `i`, and `p`. The module
has no state, so first-use init adds no guard. `Point`'s `x` and `y`
stay fields in bodies; the surface presents them as getters, and the
properties-to-methods pass rewrites only reads from outside `Point`.
