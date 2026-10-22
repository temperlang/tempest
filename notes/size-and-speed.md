# Size and speed

Part of the [rewrite plan](README.md).

## 12. Size: today vs. target

Today, from the source survey (Kotlin, main code, not tests; approximate):

| Area | Today (hand-written) | Plus generated |
|---|---|---|
| Lexer, parser, CST/AST building | ~23k | |
| Frontend, fundamentals, builtins, interpreter, names | ~92k | |
| Backend shared layer (TmpL translation etc.) | ~18k | ~12k (TmpL classes) |
| Seven backends | ~52k | ~42k (per-language ASTs) |
| CLI, build/watch, LSP, logging, files, common | ~38k | |
| Code generator for the grammars (kcodegen) | ~7k | |
| **Total** | **~230–240k** | **~54k** |

Plus roughly 70k lines of Kotlin unit tests.

Target in Rust:

| Area | Target | Why smaller |
|---|---|---|
| Literate, lexer, parser into the tree | 4–6k | One table-driven parser straight into the tree; no separate CST, AST or grammar DSL. (The literate part is done: ~650 lines.) |
| Tree, settle loop, names, macros, types, interpreter, core functions | 20–30k | One representation instead of several; no stage machinery or re-walks; one type system instead of two plus bridges |
| tmpl, shared lowerings and naming | 4–7k | Small, specified IR; no frontend types; lowering done once for all backends |
| Seven backends | 21–35k | Printers via a shared document builder; no generated ASTs |
| CLI, build/watch, LSP, logging | 8–12k | Arenas per build make caching and cleanup simple; no JVM concerns |
| **Total** | **~60–90k** | **About 3× smaller (4× counting generated code)** |

The biggest uncertainty is the long tail of type-system and macro semantics
that the 69 functional tests don't pin down. If it pushes, the frontend could
grow toward the top of its range, but I don't expect past ~100k total.

## 13. Speed and memory

What we measured on Prismora (~900 lines):

- `temper watch` rebuild plus Python tests: 2–5 s. A one-shot
  `temper test -b py` takes ~28 s, much of it JVM startup and warmup.
- Watch process: ~400 MB RSS, and ~40 MB live heap after GC once the leak
  is fixed.
- One build holds ~80k edge objects plus as many nodes, positions, values and
  type-inference records, and hundreds of thousands of map entries.

Where Rust gets its speed, roughly in order of impact:

1. **Far fewer allocations.** A build is a few growing arrays instead of
   hundreds of thousands of small objects. This is the biggest win: less
   allocator work, no garbage collection, and data that stays in cache.
2. **No JVM start or warmup.** Milliseconds instead of seconds to start, which
   matters most for one-shot `build`/`test` and the language server.
3. **No repeated whole-tree passes.** The settle loop only revisits what was
   waiting on something that changed.
4. **Interned names and types**, so comparisons are integer comparisons and
   there's one type representation, not two plus conversions.
5. **Parallel builds** of independent libraries.

Estimates for the compiler itself (source to generated code, not running the
target language's tests):

- **Time:** 10–50× faster warm rebuilds; 50–100×+ for one-shot runs. Aim for
  under 100–200 ms for Prismora-sized libraries.
- **Memory:** tens of MB peak for libraries this size, against ~400 MB RSS.
- **Binary:** one static binary of roughly 10–20 MB, with no JRE.

These are targets to measure against. First, time Kotlin Temper's phases on
Prismora and the functional tests, so both sides use real numbers. And
`temper test` is often dominated by the target toolchains (Python, Maven,
cargo, dotnet), which a faster compiler doesn't change.
