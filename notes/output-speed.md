# Speed of generated code

How to make the code Temper generates faster. Measured mostly on Kelpto's
benchmarks (2026-11-02), where the same Temper source ran 1.7× to 100×
slower than each language's usual Markdown parsers.

There are three kinds of change, from cheapest to hardest:

- **Compatible:** the output is faster, with the same behavior and public
  API. Tempest can do these on its own.
- **Small additions:** to std or the backends, compatible but new API.
  They need the team's agreement.
- **Design changes:** to the language or a backend's model. They need
  the team, and usually wait.

Each idea has a status: **done** in tempest, **planned**, or an **idea**.

## Compatible

1. **Rust: lock only what changes.** Kotlin's Rust puts every object with
   a `var` field behind `Arc<RwLock>`, and every `ListBuilder` too. That
   made Kelpto's Rust 4× slower than its C++ from the same source.
   - **Done:** tempest locks only `var` fields, each on its own, and an
     object with no `var` fields has no lock ([api-compat.md](api-compat.md)).
   - **Planned:** `Mutex` rather than `RwLock`, which is simpler and faster
     when uncontended.
   - **Idea:** a `ListBuilder` that never leaves its function is a plain
     `Vec`.
2. **Plain arithmetic where it can't overflow.** Python and Lua route
   `Int` operations through helpers that emulate 32-bit wraparound
   (`int_add`: 755,000 calls, about 10% of a Python parse of the
   CommonMark spec). Most of these are indexes and lengths, which value
   ranges can show can't overflow ([typing.md](typing.md), "What flow
   facts are used for"). **Planned.**
3. **No exceptions for failures handled on the spot.** Where bubbling is
   an exception (Python, Java, JavaScript, C#), a failure that's common
   and handled right where it happens, like `orelse` after a search that
   finds nothing, can be a null check instead of a throw. **Idea.**
4. **Native string indexing everywhere.** Core already iterates code
   points through `StringIndex` (`get`, `next`, `slice`) on every backend.
   Check that each backend's `StringIndex` operations are native and O(1),
   so libraries never copy text to an array for speed. Kelpto's
   `SourceText` copying was its own choice, and is being fixed there.
   **Idea.**
5. **Less recursion where it costs.** Python's recursion limit stopped
   Kelpto at 1,000 nested block quotes. Libraries can avoid it; tempest
   could also warn about deep self-recursion in hot code. **Idea.**

6. **JS: unfrozen arrays where nothing can see them.** Slicing a frozen
   array is about 80× slower in V8 than slicing a plain one (Kelpto,
   2026-11-02). Freezing only what escapes, or slicing by index into a
   preallocated array, avoids it. Also, `listBuilderAddAll` spreads its
   items into one `push(...)`, which overflows the stack past about
   120,000 items; add in chunks instead. **Idea.**
7. **Rust: fewer lock round trips.** Kotlin's Rust takes a lock and
   clones `Arc` fields on every field access. Beyond item 1, one guard
   per run of accesses, and atomics for scalar `var`s, would help.
   Libraries work around it by reading fields into locals in hot loops.
   **Idea.**

## Small additions

8. **Non-throwing variants of common operations:** for example
   `list.getOr(i, fallback)`, parsing that returns null, and `Regex.found`
   (a real library needed it). They pair with item 3. **Idea.**

## Design changes

9. **Rust: `Rc` instead of `Arc`.** Most objects never cross threads or
   workers. If the isolates and actors plan ([tmpl/mutability.md](tmpl/mutability.md))
   made it clear what may cross those boundaries, everything else could
   use `Rc` and `RefCell`. **Idea; needs the isolates plan.**
10. **Small immutable classes as values:** structs in C#, `Copy` in Rust,
   and plain values in C++, to cut allocation and garbage collection. In
   C# and Lua, collection on Kelpto's 2 MB document took up to twice its
   usual share of the time. **Idea; bigger, later.**

## Measuring

Kelpto's `scripts/bench.py` runs the same benchmarks on all seven
languages, and Chromarite has its own. A scorecard row comparing the
output of Kotlin Temper and tempest on the same source belongs in the
switch-over plan ([switch-over.md](switch-over.md), step 4).
