# tmpl: open questions

Part of the [tmpl design](README.md).

## Open questions, with recommendations

**Decided (2026-10-29), second round:** the user accepted the
recommendations below, with these adjustments:

- **Protocols:** track only the common operators and traits that Kotlin
  Temper already lets code control (`@operator("==")`,
  `@operator("<=>")`) or that Temper's features and usage strongly
  imply (`toString` as `Show`, `for … of` iteration). More will come,
  but don't add `Protocol` variants or table rows ahead of the language
  (questions 32 and 33).
- **Identity** is a placeholder: Temper hasn't defined how code asks for
  it, so `Identical` stays in the model, and its spelling waits for the
  language (question 12).
- **Source maps:** make them work correctly for JS in browsers first
  (standard source maps, UTF-16 columns), and test them in JS engines
  as the proof of concept. Kotlin Temper emits them for every backend;
  other backends may get forms more practical for them later (question
  34).

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
   checked for completeness? Superseded (2026-10-29) by questions 36
   to 43: operators are a closed set, and connected declarations are
   keyed by qname or resolved by name.
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

New from Level 2. The user approved the recommendations marked
Decided below on 2026-10-29:

12. **Identity's spelling in Temper.** Temper has none today: `===` and
    `!==` lex as operators, but nothing gives them a meaning, and the
    docs don't mention identity. Recommend a core function,
    `identical(a, b)` (Dart's name), allowed only on class and
    interface types, with `===` left unused. It's explicit, greppable,
    and one typo away from nothing, where `===` is one character from
    `==` and means something else in JS. A language change for the
    team; tmpl's `Identical` node doesn't depend on the spelling.
    **Decided (2026-10-29):** `Identical` stays as a placeholder; how
    Temper code asks for identity isn't defined yet, so its spelling
    waits for the language.
13. **`==` without an operator method:** on a generic `T`, `AnyValue`,
    a function value, or an interface without one. Recommend an error
    in all these, never a silent identity check; a bound such as `T
    extends Equatable` calls the bound's method. **Decided
    (2026-10-29):** an error, once a scan of std and the libraries
    finds what relies on identity fallback today (milestone 4, slice 7).
14. **Host equality protocols:** should a class's `@operator("==")`
    method also implement Python's `__eq__`, Rust's `PartialEq`, and
    C++'s `operator==`? Recommend yes for those three; for Java and C#,
    keep a plain `eq` method until Temper has hashing, since `equals`
    without a matching `hashCode` breaks hash maps. **Decided
    (2026-10-29)** as recommended ([surface.md](surface.md), Level 3).
15. **Null-safe `==` between nullable values:** one wrapper node,
    expanded by a pass, or expanded by the frontend as Kotlin's
    `EqMacro` does? Recommend the wrapper (`EqNullable`): Python and
    Rust compare nullable values natively, and the expansion is long.
    **Decided (2026-10-29):** `EqNullable`.
16. **Comparisons:** `<` on builtin types as per-type keys (`Int64.lt`)
    or as `<=>` then `< 0`, as Kotlin now desugars? Recommend per-type
    keys, since every primary target has them natively; classes' `<`
    stays `compareTo(…) < 0`, which a backend may resugar.
17. **Fixed-type nodes in text:** may text also leave out types that a
    node's kind fixes (`Boolean` for `&&`, null checks, and
    `Identical`)? Recommend yes, as a fourth local rule; it's as
    trivial as a literal's. **Decided (2026-10-29):** yes.
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
    and the tmpl validator rechecks them as an invariant. **Decided
    (2026-10-29)** as recommended.
22. **The first Python variant:** Kotlin-compatible or idiomatic?
    Recommend Kotlin-compatible first, so milestone 5's API diff
    against Kotlin is empty by design and existing users can switch;
    the idiomatic variant follows as profile, naming, and surface data,
    with its own API baseline. **Decided (2026-10-29),** widened:
    every backend's first variant is Kotlin-compatible, so Chromarite's
    and Ordable's tours keep working.
23. **Natural alternatives before numbers:** in what order? Recommend
    the hint's own candidates, then a name shared with a local whose
    lifetime doesn't overlap (where the target's scoping allows), then
    the shortest suffix (`x2`), and a golden-test count of suffixed
    names across the functional tests, std, Chromarite, and Ordable,
    kept near zero. **Decided (2026-10-29)** as recommended.

Questions 16, 18, 19, and 20 weren't in the list approved on
2026-10-29, so their recommendations stay Proposed until confirmed.
Questions 1 to 11 also stay open.

New from Level 3 (subset A), all Proposed:

24. **Declaration ids through passes:** keep each `DeclId`, append
    new ones, and leave tombstones, or renumber in writing order?
    Recommend keeping them, so lowered text differs from starting text
    only where a pass changed it.
25. **A fifth local type rule:** `IfValue`, `BlockValue`, and later
    `OrElse` take their value's type, which T3 already makes equal.
    Recommend yes; most written `(e): T` would go.
26. **Scalar builtin types as fixed type entries,** not external
    declarations? Recommend yes; generic builtin types stay
    declarations.
27. **`Error`'s type:** the type its place wants, rather than `Never`
    of it? Recommend yes; it avoids a `FromNever` around every error,
    and backends treat it the same.
28. **`GetModuleLet` and `SetModuleLet`** for every module `let`, the
    declaration saying `var`, in place of Level 2's `GetModuleVar`?
    Recommend yes.
29. **Every loop declares a label,** compiler-made if the source has
    none? Recommend yes, so jumps always name their target.
30. **Python's `Void`:** a value natively, as `None`, unlike Level 2's
    profile table? Recommend yes.
31. **A debug form for `toString`'s protocol:** derive `Debug` and
    `__repr__` per backend, or a source-level hook? Recommend deriving
    now, and asking the team about a hook later.
32. **More `@operator` protocols** (`[]` for `get` and `set`,
    iteration, hashing): which, and how they're spelled, is for the
    team; tmpl only needs a `Protocol` variant and table rows.
33. **`toString` as `Show`:** by its well-known name and signature, or
    by an explicit mark? Recommend by name, since Kotlin's backends
    rely on the name already.
34. **Source map columns:** UTF-16 units, converted with stored text
    of non-ASCII lines only? Recommend yes.
35. **Bubbling in module init:** a may-bubble call in an init block,
    with no handler, fails the module's initialization. Check against
    Kotlin Temper's behavior before `to_tmpl` relies on it.

**Decided (2026-10-29), third round,** on qnames and connected
declarations (folded into [names.md](names.md#level-3-qnames),
[nodes.md](nodes.md#level-3-ops-and-core-as-declarations), and
[overview.md](overview.md#core-decisions)):

- tmpl has a place for qnames, which may follow Kotlin's.
- Connected is one concept from outside; inside, custom translation or
  lowering is chosen by qname. Std may later use user space's
  mechanism; core likely keeps specialized tree rewriting.
- Customization is general, by naming, and only for operators and
  connected declarations, never a hardcoded table of core functions.
- User-space connected declarations resolve by name, as Kotlin's do:
  host code defines the name the translation gives. Only core's and
  std's are keyed by qname.

New with that round, all Proposed:

36. **Qname format:** Kotlin's exactly, core written `core` and std
    `std`? Recommend yes, so Kotlin's connection tables port verbatim
    and api-compat can match declarations across Kotlin and tempest
    builds.
37. **Qnames stored or derived:** recommend derived from owners,
    names, and kinds, with a stored override only where Kotlin's
    differs (disambiguation indexes, a `let` holding a function
    literal); text writes only overrides.
38. **What stays built in:** a closed set of 65 operators on builtin
    types, or operators as connected core declarations too? Recommend
    the closed set: Kotlin has no declarations for `+` and the rest,
    and passes and analyses should match operators by enum. Core's
    other 48 members become declarations.
39. **Renaming `Builtin` to `Op`** (and `builtins.rs` to `ops.rs`)?
    Recommend yes, as its own commit before the migration, so the
    migration's diff stays readable.
40. **Recording how a connected declaration resolves:** **Decided
    (2026-10-29):** not stored. `connected` is a flag, and how it
    resolves is recognized from the declaration's library (core and std
    by qname, user code by name) by one shared helper.
41. **A user-code connected declaration with a body:** **Decided
    (2026-10-29):** the same connected fallback as the design proposed
    for Kotlin Temper: an optional list of backends with host code
    (`@connected(["cpp", "rust"])`); listed backends call host code,
    and unlisted backends and the interpreter use the body. With no
    list, every backend needs host code, as today.
42. **`console`:** a compiler-made module `let` initialized by core's
    `getConsole()`, as Kotlin's frontend does, rather than a special
    form? Recommend yes; Kotlin's backends give each module its own
    logger through it.
43. **Pulling into subset A** the pieces core's declarations need:
    external declarations with `uses`, non-generic `Nominal` types,
    `CallMethod`, and `Omitted` with optional parameters? Recommend
    yes, as one chunk, before `to_tmpl`, so `to_tmpl` never emits the
    48 keys.
