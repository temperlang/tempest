# tmpl: open questions

Part of the [tmpl design](README.md).

## Open questions, with recommendations

Answered on 2026-10-29 and folded into the topic files: profile shape, canonical
text syntax and type elision, `for … of`, surface facts stored, where
immutability facts come from (revised: `@imu` and `@partialImu` plus
analysis), and init order; also backend variants and pretty names
everywhere. Still open from Level 1:

1. **Schema mechanism:** `macro_rules!` or a `build.rs` generator?
   Recommend `macro_rules!` until it hurts.
2. **Captures in the tree or computed?** Recommend in the tree, checked
   by recomputation, so passes and text show them.
3. **Ownership facts** for Rust, Swift, and C (clones, last uses):
   stored or computed? Recommend computed ([analyses.md](analyses.md)) until a backend proves
   otherwise.
4. **Bubbling payloads:** reserve a value on failure now? Recommend
   reserving the field, unused until Temper decides.
5. **Builtin keys:** one namespace for builtins, operators, and
   connected members, listed in the crate so backends' tables are
   checked for completeness? Recommend yes.
6. **Statements inside expressions** at the high level? Recommend yes,
   with a statement-izing pass; Rust keeps them.
7. **Spans:** file id and byte range, with line and column computed
   only for source maps? Recommend yes.
8. **Generics for C-like targets:** erasure or monomorphizing? Defer;
   keep full generics in the model.
9. **The tmpl interpreter** for compile-time evaluation of cached
   dependencies? Recommend deciding with the cache, after milestone 6.
10. **Surface choices in profiles:** one struct with body and surface
    sections? Recommend yes, so a backend's whole stance is in one
    place.
11. **What may cross an isolate?** Defer until an isolation model
    exists.

New from Level 2:

12. **Identity's spelling in Temper.** Temper has none today: `===` and
    `!==` lex as operators, but nothing gives them a meaning, and the
    docs don't mention identity. Recommend a core function,
    `identical(a, b)` (Dart's name), allowed only on class and
    interface types, with `===` left unused. It's explicit, greppable,
    and one typo away from nothing, where `===` is one character from
    `==` and means something else in JS. A language change for the
    team; tmpl's `Identical` node doesn't depend on the spelling.
13. **`==` without an operator method:** on a generic `T`, `AnyValue`,
    a function value, or an interface without one. Recommend an error
    in all these, never a silent identity check; a bound such as `T
    extends Equatable` calls the bound's method.
14. **Host equality protocols:** should a class's `@operator("==")`
    method also implement Python's `__eq__`, Rust's `PartialEq`, and
    C++'s `operator==`? Recommend yes for those three; for Java and C#,
    keep a plain `eq` method until Temper has hashing, since `equals`
    without a matching `hashCode` breaks hash maps.
15. **Null-safe `==` between nullable values:** one wrapper node,
    expanded by a pass, or expanded by the frontend as Kotlin's
    `EqMacro` does? Recommend the wrapper (`EqNullable`): Python and
    Rust compare nullable values natively, and the expansion is long.
16. **Comparisons:** `<` on builtin types as per-type keys (`Int64.lt`)
    or as `<=>` then `< 0`, as Kotlin now desugars? Recommend per-type
    keys, since every primary target has them natively; classes' `<`
    stays `compareTo(…) < 0`, which a backend may resugar.
17. **Fixed-type nodes in text:** may text also leave out types that a
    node's kind fixes (`Boolean` for `&&`, null checks, and
    `Identical`)? Recommend yes, as a fourth local rule; it's as
    trivial as a literal's.
18. **References in text:** `name#id` everywhere, or `#id` after the
    definition? Recommend `name#id` everywhere, the name checked by the
    reader, so text reads and diffs well.
19. **Init order through calls:** an initializer that reads a later
    module value directly is an error; through a function it calls?
    Recommend a warning from the reaches-module-state analysis, since
    it can't be exact.
20. **Rust's `async`:** native `async fn` over Temper's `Promise`, or
    the state machine as for generators? Recommend native, as
    milestone-3.md's `Future` plan suggests; generators get the state
    machine.
21. **Where `@imu` is checked:** tempest's frontend doesn't check it
    yet. Recommend the frontend reports violations in Kotlin's words (a
    small milestone 4 item, near slice 8, which needs the same facts),
    and the tmpl validator rechecks them as an invariant.
22. **The first Python variant:** Kotlin-compatible or idiomatic?
    Recommend Kotlin-compatible first, so milestone 5's API diff
    against Kotlin is empty by design and existing users can switch;
    the idiomatic variant follows as profile, naming, and surface data,
    with its own API baseline.
23. **Natural alternatives before numbers:** in what order? Recommend
    the hint's own candidates, then a name shared with a local whose
    lifetime doesn't overlap (where the target's scoping allows), then
    the shortest suffix (`x2`), and a golden-test count of suffixed
    names across the functional tests, std, Chromarite, and Ordable,
    kept near zero.
