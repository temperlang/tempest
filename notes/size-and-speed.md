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

What we measured on Prismora (now Chromarite), at ~900 lines:

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
  under 100–200 ms for Chromarite-sized libraries.
- **Memory:** tens of MB peak for libraries this size, against ~400 MB RSS.
- **Binary:** one static binary of roughly 10–20 MB, with no JRE.

These are targets to measure against. First, time Kotlin Temper's phases on
Chromarite and the functional tests, so both sides use real numbers. And
`temper test` is often dominated by the target toolchains (Python, Maven,
cargo, dotnet), which a faster compiler doesn't change.

### When to optimize

Defer optimizations unless putting them off would force a major
architectural change later, such as data layout, how tokens or trees are
represented, or what the settle loop indexes. Measure every stage as it
lands, note ideas, and tune later.

### What will make it fast

Arenas of plain structs indexed by `u32` already work like a bump allocator,
and small nodes with interned names and types should use a fraction of
Kotlin's memory. One-shot builds lose JVM startup and warmup outright. But
the lexer shows the JIT is good at tight loops (Rust is only 2–5× warm
Kotlin there), so watch mode's big wins must come from doing less work:
waking only items waiting on a changed fact (indexed, never rescanned),
union-find over interned types, rebuilding only what changed, building
libraries in parallel, a reasonably fast compile-time interpreter for
macros, and no quadratic string building in backends. Benchmark each stage
as it lands, and profile with `perf`.

### Measured so far

Per stage, both sides read all inputs first and then time passes over them,
Kotlin after warmup, so these compare the work itself, without file reads or
JVM startup. Inputs: all Temper files in the Temper and Prismora (now Chromarite) repos,
814–815 files, ~700 KB. Ranges are medians across alternating runs (`compat/bench_*.py`);
this machine is noisy, so the low ends are the better guide.

| Stage | Commit | Rust | Kotlin, warm | Kotlin / Rust | Date |
|---|---|---|---|---|---|
| Lexing | `1ef64d8` | 28–30 ms | 43–49 ms | 1.5–1.7× | 2026-10-22 |
| Lexing | after `1ef64d8`, ASCII fast path | 21–30 ms | 39–80 ms | ~2× | 2026-10-22 |
| Lexing | after `1ef64d8`, tokens as kind and span, no `String` each | 8.2–12 ms | 37–52 ms | 3.7–4.7× | 2026-10-22 |
| Lexing and parsing | after `bf5ebfb`, operator lookup by table | 56–61 ms | 158–174 ms | 2.6–2.9× | 2026-10-22 |
| Lexing, parsing and grammar step | after `cd8a59b` | 96–128 ms | 541–621 ms | 4.5–6.0× | 2026-10-22 |
| Same, plus the rest of `ParseStage` (`--module`) | after `cd8a59b` | 103–144 ms | 630–802 ms | 4.6–6.1× | 2026-10-22 |
| Same, plus settling each file as a library | after `cd8a59b` | 126–136 ms | — | — | 2026-10-22 |

Rows give the commit measured, or the commit a change was measured on top
of. For the token change, the old and new lexer alternated directly took
18–22 ms and 8.2–9.3 ms. Of the ~57 ms for parsing, lexing is ~9 ms, the token
adapters ~12 ms and the parser ~39 ms (`compat/bench_parse.py`).

Settling adds 34–38 ms over the module step on the same 818 files
(`compat/bench_settle.py`, which alternates the two), including copying
each file's tree into a library arena. Most files stop early at classes
and imports, which aren't in the loop yet, so this will grow. There's no
Kotlin column: its stages don't map onto ours. Ideas for later: build the
grammar tree straight into the arena instead of copying; scopes find names
by scanning a `Vec`, fine for blocks but worth a map for big modules; and
finding stuck items scans all items, which only happens when the loop is
stuck. Running the passing functional tests is instant (each well under
10 ms as a process).

The module step (call joins and metadata) costs little: alternating
`ast-tree --bench` with and without `--module` gave 123 and 125 ms, within
the noise (818 files; the machine was busier than for the row above).

The grammar step (816 files by then) adds ~35–40 ms to that, about as much
as the parser, and is untuned: a straight port of Kotlin's combinators, trying `Expr`'s 27
alternatives in order with an event list that's truncated on backtracking.
Ideas: dispatch on the next part's operator instead of trying alternatives
in turn (most fail on their first lookahead, but each call costs); keep
values out of events until lift, so backtracking drops no strings; compute
doc comment contents lazily; and find the previous token for comment and
bag lookups from a table rather than scanning back.

The lexer and parser aren't tuned much either. Ideas, roughly by expected
gain: the adapters as one streaming pass instead of five passes copying
token vectors; pooled child vectors for the parser's stack; each token's
operator matches found once, shared by the adapters and the parser; no copy
of the source in the lexer and adapters; and a direct lookup for markdown
paragraph blocks instead of a scan.

### Interpreter

`compat/bench_interp.py` runs the loops in `benchmarks/interp/` at two
sizes, so the difference gives the cost per iteration without startup or
compiling. Ours runs as `run-temper` processes; Kotlin's in one JVM per
benchmark (`compat/oracle/InterpBench.java`), which builds and runs the
program round after round as `temper repl` would, without the REPL's step
quota, and drops the first 8 rounds while the JIT warms up (loop costs
fall from ~15–20 µs to a steady 6–9 µs over the first 6 or so). Runs
alternate between the sides; ranges are of per-run medians, over two
invocations of 3 runs each (2026-10-22, after list builders and `for …
of`):

| Benchmark | Per iteration | Rust | Kotlin | Kotlin / Rust |
|---|---|---|---|---|
| `loop` | `count += 1` | 0.63–0.87 µs | 6.2–9.4 µs | 7–15× |
| `calls` | one call of a small function | 0.98–1.5 µs | 10–22 µs | 7.5–15× |
| `closures` | one call of a closure updating a captured `var` | 1.2–1.5 µs | 11–16 µs | 8–13× |
| `fib` | `fib(12)`, about 465 calls | 380–440 µs | 2,900–4,000 µs | 7–10× |

After `when`, `?.`, `??` and Float64 (on `85b5cbb` plus that work,
2026-10-23, 3 runs per side): `loop` 0.59–0.68 µs against 4.8–9.0 µs
(8–14×), `calls` 0.96–0.99 µs against 7.8–11 µs (8–11×), `closures`
0.99–1.1 µs against 8.3–13 µs (8–14×), `fib` 340–390 µs against
2,640–3,230 µs (7–8×). Peak memory 15 MiB against 395–492 MiB.

After strings, `StringBuilder`, tags and destructuring `let` (`704dca9`),
and top-level dependency order, `~` and `>>>` on top (2026-10-23, 3 runs
per side): `loop` 0.76–0.95 µs against 7.8–8.6 µs (9–11×), `calls`
1.1–1.4 µs against 14–27 µs (10–26×), `closures` 1.1–1.4 µs against
14–24 µs (12–18×), `fib` 422–476 µs against 3,741–4,580 µs (8–10×). Both
sides ran about a quarter slower than the run before, so this looks like
machine drift rather than a change; the ratios held. Peak memory 15 MiB
against 387–492 MiB.

After modules and `import` (`f735e3d` plus that work, 2026-10-24, two
invocations of 3 runs per side, the first much noisier): `loop`
0.64–2.0 µs against 8.8–17 µs (8–21×), `calls` 0.78–2.7 µs against
12–29 µs (4–20×), `closures` 0.82–1.9 µs against 12–29 µs (11–16×),
`fib` 391–842 µs against 3,328–6,949 µs (6–13×). The quieter invocation
alone matches earlier runs (`fib` 391–412 µs, `loop` 0.64–1.2 µs), so
no change; the interpreter only gained a frame per module. Peak memory
15 MiB against 378–481 MiB.

After `Deque`, `DenseBitVector` and several declarations in one
(`0827f8a` plus that work, 2026-10-24, 3 runs per side): `loop`
0.66–0.72 µs against 7.0–7.3 µs (10–11×), `calls` 0.88–1.1 µs against
11–14 µs (13×), `closures` 1.1 µs against 11–14 µs (10–13×), `fib`
388–399 µs against 3,273–3,833 µs (8–10×): no change. A second
invocation overlapped a multi-backend `temper test` from another session
and was far noisier (`fib` 404–1,300 µs, ratios 4–20×), so it's left
out. Peak memory 15 MiB against 377–491 MiB.

After `@fun interface`, `x: T = v` fields and locals visible only after
their declaration (`6a4f4dc` plus that work, 2026-10-24, two quiet
invocations of 3 runs per side; a first one beside another session's
`temper test` ran about twice as slow and is left out): `loop`
0.52–0.60 µs against 4.3–5.9 µs (7–10×), `calls` 0.73–1.0 µs against
6.5–8.3 µs (7–9×), `closures` 0.77–1.0 µs against 7.6–11 µs (9–10×),
`fib` 293–359 µs against 2,371–2,814 µs (7–9×): no change, as the work
was all at settle time. Peak memory 15 MiB against 379–481 MiB.

After object literals, trailing blocks, `@auto("==")` and `Empty`
(`3f2c331` plus that work, 2026-10-24, two invocations of 3 runs per
side): `loop` 0.63–1.0 µs against 7.4–9.9 µs (10–13×), `calls`
0.96–1.3 µs against 11–20 µs (12–18×), `closures` 1.0–1.3 µs against
11–17 µs (10–15×), `fib` 371–438 µs against 3,305–4,443 µs (8–11×).
Alternating the old and new `run-temper` directly, 9 runs each at large
`N`, gave the same times within noise (`fib` 647–816 against 682–835 ms,
`closures` 1,070–1,119 against 1,071–1,141 ms): the call path's new
checks, for a trailing block and for objects in `cat` and `==`, cost
nothing visible. Peak memory 15 MiB against 397–488 MiB.

After `std` imports, `Map`, `Pair` and static extensions (`5ab2a49`
plus that work, 2026-10-24, two invocations of 3 runs per side): `loop`
0.54–0.61 µs against 4.4–5.1 µs (7–9×), `calls` 0.82–1.0 µs against
6.7–8.7 µs (7–9×), `closures` 0.86–1.0 µs against 7.4–9.1 µs (8–9×),
`fib` 319–367 µs against 2,197–2,726 µs (7–8×): no change, as the call
path only gained arms for new kinds of values. Peak memory 15 MiB
against 383–494 MiB. A whole `types/json-syntax-tree` run, settling
std's `json` (~1,400 lines) with the test and running it, takes
10.1–12.0 ms as a process (9 runs), with an 11 MiB peak.

After `test`, `assert`, regex literals and a regex matcher (`f3beacc`
plus that work, 2026-10-24, two invocations of 3 runs per side): `loop`
0.53–0.71 µs against 4.3–6.8 µs (8–11×), `calls` 0.79–1.2 µs against
6.9–9.2 µs (7–9×), `closures` 0.86–1.2 µs against 7.1–10 µs (8–9×),
`fib` 319–439 µs against 2,118–2,892 µs (6–8×): no change, as the call
path is untouched. Peak memory 15 MiB against 379–457 MiB. Whole runs
as a process (9 runs each, three invocations): `regex/match`, settling
std's `regex` (~840 lines) and compiling and matching 15 regexes,
9.7–14 ms; `testing/asserts` with its tests, 4.5–4.9 ms; peaks
11.3–11.5 MiB. `types/json-syntax-tree` took 13–19 ms this time, but
alternating with the previous commit's binary, 15 runs each, gave the
same times (13.2–16.4 against 13.2–19.9 ms), so that's machine drift.

Flat frames (on `561b4d9`, 2026-10-25): the tree walk replaced by code
lowered per function and run by one loop (milestone-3.md). Alternating
the old and new `run-temper` directly, two invocations of 3 rounds of 3
runs each, ranges of per-round medians: `loop` 0.18–0.23 µs against
0.53–0.59 µs for the tree walk (2.5–3×), `calls` 0.32–0.36 against
0.85–0.92 µs (2.5–2.8×), `closures` 0.33–0.37 against 0.90–0.96 µs
(2.6–2.8×), `fib` 137–146 against 336–345 µs (2.4×). Kotlin wasn't
rerun; against its numbers above, that's roughly 15–30×. Whole runs,
mostly settling, didn't change: `types/json-syntax-tree` 9.7–10.5
against 10.0–11.2 ms, `regex/match` 7.0–7.8 against 7.4–8.8 ms,
`testing/asserts` 3.2–3.4 against 3.4–4.2 ms, `algos/myers-diff`
2.4–2.6 against 2.5–3.1 ms, with 14.2–14.3 MiB peaks for both. The gain
is from no `Result` passed up through a recursive `eval` for each node,
locals by slot rather than by walking linked frames to their scope's,
no frame for each block entered, and globals by index. Not tuned yet: a
call still allocates its frame's values and its arguments, and a method
call boxes a `Val::Method`; an operand stack shared by frames,
unboxed method calls and fused ops (a local and a constant) are the
obvious next steps.

Generators (on `25e32ca`, 2026-10-25), alternating the old and new
`run-temper` the same way, three invocations (3 or 5 rounds of 3 runs):
no change beyond noise. `loop` 0.18–0.25 µs against 0.20–0.25 µs for
`25e32ca`, `calls` 0.32–0.37 against 0.31–0.39 µs, `closures` 0.31–0.41
against 0.32–0.37 µs, `fib` 143–150 against 134–153 µs. The call path
gained only a check of the callee's code for a generator. Whole runs'
low ends matched (`types/json-syntax-tree` 10.1 against 10.1 ms,
`regex/match` 7.2 against 7.2 ms, `testing/asserts` 3.2 against 3.3 ms,
`algos/myers-diff` 2.5 against 2.4 ms), though the new binary's high
ends were noisier in two invocations (up to 14.7 against 11.2 ms for
json); 14.2–14.3 MiB peaks for both.

`async`, `await` and promises (on `13f4889`, 2026-10-26), alternating
the old and new `run-temper` the same way, three invocations of 3 rounds
of 3 runs: no change beyond noise, though the machine ran about a
quarter slower than for the rows above, old binary included. `loop`
0.29–0.33 µs against 0.26–0.54 µs for `13f4889`, `calls` 0.44–0.51
against 0.43–0.54 µs, `closures` 0.41–0.49 against 0.41–0.49 µs, `fib`
179–218 against 175–219 µs; whole runs `types/json-syntax-tree`
12.8–17.0 against 13.4–18.0 ms, `regex/match` 8.8–11.4 against
9.0–13.0 ms, `testing/asserts` 4.1–5.7 against 4.2–6.9 ms,
`algos/myers-diff` 3.2–4.7 against 3.2–5.6 ms; 14.2–14.3 MiB peaks for
both. The hot path gained only the `Await` op's arm. `control-flow/async`
and `async-waiters` run as a process in 2.0–2.2 ms (medians of 9; 12.2
MiB peak).

A whole small run is 8–19 ms for ours (process startup, settling and
1,000–10,000 iterations) and 24–67 ms for Kotlin's in its warm JVM
(compiling and 10–1,000 iterations; JVM startup excluded). Peak memory is
15 MiB for ours and 350–440 MiB for the JVM. Earlier `temper repl`
numbers (6–75 µs per iteration) mostly measured an unwarmed JIT.

So the tree walk was roughly 7–15× faster than Kotlin's warm, but still
slow for an interpreter (roughly 600 ns–1 µs a call or loop turn, where
20–50 ns is typical): it was dynamically typed and untuned. Flat frames
brought that to 180–370 ns; it's still worth profiling once classes and
types settle, since compile-time evaluation runs on it too.

### Binary size and memory

Small is a goal in itself. Low memory matters everywhere: in CI, on small
machines, beside an editor and an LSP, and in watch mode. Binary size
matters too, most of all for wasm, where it's download time for a browser
playground; natively it's mostly startup and install size.
`compat/measure_size.py` measures both. Sizes use the `small` profile
(optimize for size, LTO, `panic = "abort"`, stripped; see `Cargo.toml`).
Peak memory is the resident set of one `--bench 1` run: read all 815 Temper
files (~700 KB) and lex and parse them all once.

| Commit | What | Binary (small) | Peak memory | Kotlin (JVM) peak | Date |
|---|---|---|---|---|---|
| after `bf5ebfb` | lexer and parser (`parse-tree`) | 482 KiB | 14.0 MiB | 198 MiB | 2026-10-22 |
| after `cd8a59b` | lexer, parser and grammar step (`ast-tree`) | 638 KiB | 14.8 MiB | 236 MiB | 2026-10-22 |
| after `cd8a59b` | the same with the module step (`ast-tree`) | 646 KiB | 15.0 MiB | 242 MiB | 2026-10-22 |
| after `cd8a59b` | and the settle loop and interpreter (`run-temper`) | 734 KiB | 15.0 MiB | — | 2026-10-22 |
| after `8945bcb` | classes, type parameters, lists, `for … of`, NFKC (`run-temper`) | 818 KiB | 14.9 MiB | — | 2026-10-22 |
| after `85b5cbb` | `when`, `?.`, `??`, Float64 math and formatting (`run-temper`) | 838 KiB | 15.0 MiB | — | 2026-10-23 |
| after `04789c6` | string indices, `String` methods, `StringBuilder`, tags, destructuring `let` (`run-temper`) | 854 KiB | 14.9 MiB | — | 2026-10-23 |
| after `704dca9` | top-level dependency order, `~`, `>>>` (`run-temper`) | 862 KiB | 14.9 MiB | — | 2026-10-23 |
| after `f735e3d` | modules, `export` and `import` (`run-temper`) | 890 KiB | 15.1 MiB | — | 2026-10-24 |
| after `0827f8a` | `Deque`, `DenseBitVector`, several declarations in one (`run-temper`) | 894 KiB | 15.0 MiB | — | 2026-10-24 |
| after `6a4f4dc` | `@fun interface`, typed field initializers, locals visible after declaration (`run-temper`) | 894 KiB | 15.1 MiB | — | 2026-10-24 |
| after `3f2c331` | object literals, trailing blocks, `@auto("==")`, `Empty` (`run-temper`) | 918 KiB | 15.1 MiB | — | 2026-10-24 |
| after `5ab2a49` | `std` imports, `...`, `Map`, `MapBuilder`, `Pair`, static extensions (`run-temper`) | 942 KiB | 15.0 MiB | — | 2026-10-24 |
| after `f3beacc` | `test`, `assert`, regex literals and matcher (`run-temper`) | 998 KiB | 15.5 MiB | — | 2026-10-24 |
| after `561b4d9` | flat frames instead of the tree walk (`run-temper`) | 1046 KiB | 13.3 MiB | — | 2026-10-25 |
| after `25e32ca` | generators (`run-temper`) | 1054 KiB | 13.3 MiB | — | 2026-10-25 |
| after `13f4889` | `async`, `await`, promises (`run-temper`; 822 files) | 1062 KiB | 13.4 MiB | — | 2026-10-26 |
| after `684e37c` | inherited members, class checks, private members on any instance (`run-temper`; 824 files) | 1087 KiB | 14.9–15.4 MiB | — | 2026-10-26 |

| Commit | What | wasm (small) | gzipped | Date |
|---|---|---|---|---|
| after `bf5ebfb` | `temper-wasm`: parse and print the tree | 148 KiB | 61 KiB | 2026-10-22 |
| after `cd8a59b` | `temper-wasm`: also the grammar step and its tree | 337 KiB | 132 KiB | 2026-10-22 |
| after `cd8a59b` | `temper-wasm`: also the module step, and `run` (settle and interpret) | 434 KiB | 164 KiB | 2026-10-22 |
| after `8945bcb` | `temper-wasm`: also classes, type parameters, lists, `for … of`, NFKC | 518 KiB | 194 KiB | 2026-10-22 |
| after `85b5cbb` | `temper-wasm`: also `when`, `?.`, `??`, Float64 math and formatting | 557 KiB | 208 KiB | 2026-10-23 |
| after `04789c6` | `temper-wasm`: also strings, `StringBuilder`, tags, destructuring `let` | 574 KiB | 215 KiB | 2026-10-23 |
| after `704dca9` | `temper-wasm`: also top-level dependency order, `~`, `>>>` | 586 KiB | 218 KiB | 2026-10-23 |
| after `f735e3d` | `temper-wasm`: also modules, `export` and `import` | 597 KiB | 222 KiB | 2026-10-24 |
| after `0827f8a` | `temper-wasm`: also `Deque`, `DenseBitVector`, several declarations in one | 601 KiB | 223 KiB | 2026-10-24 |
| after `6a4f4dc` | `temper-wasm`: also `@fun interface`, typed field initializers, positional locals | 603 KiB | 223 KiB | 2026-10-24 |
| after `3f2c331` | `temper-wasm`: also object literals, trailing blocks, `@auto("==")`, `Empty` | 621 KiB | 229 KiB | 2026-10-24 |
| after `5ab2a49` | `temper-wasm`: also `std` imports, `...`, `Map`, `Pair`, static extensions | 647 KiB | 238 KiB | 2026-10-24 |
| after `f3beacc` | `temper-wasm`: also `test`, `assert`, regex literals and matcher | 702 KiB | 256 KiB | 2026-10-24 |
| after `561b4d9` | `temper-wasm`: also flat frames instead of the tree walk | 758 KiB | 273 KiB | 2026-10-25 |
| after `25e32ca` | `temper-wasm`: also generators | 766 KiB | 276 KiB | 2026-10-25 |
| after `13f4889` | `temper-wasm`: also `async`, `await`, promises | 773 KiB | 278 KiB | 2026-10-26 |
| after `684e37c` | `temper-wasm`: also inherited members and class checks | 795 KiB | 285 KiB | 2026-10-26 |

The release-profile native binary is 701 KiB. Peak memory includes holding
all the sources at once. `crates/temper-wasm` is a tiny entry point (source
in, tree text out) that's the seed of a browser playground; it's what keeps
the wasm measurement honest, since a binary that reads files gets its whole
parser optimized away on wasm.

The grammar step more than doubles the wasm module. Measured by leaving
parts out: the grammar, its lift and value reading are about 124 KiB, float
parsing from std about 18 KiB, and printing its tree about 47 KiB (float
formatting among it). The grammar is ~100 productions written as Rust
closures, which inline into a lot of code; making `attempt` non-generic
didn't help. The size lever, for later: productions as data, a table of
combinators run by one small matcher, much as Kotlin's are objects. Its
peak memory is the same as the parser's, since `--bench` drops each file's
trees before the next. The settle loop and interpreter add about 90 KiB to
both the native binary and the wasm module. `run-temper`'s peak memory
settles one file at a time, so it's the same as the parser's. Classes, type
parameters, lists, `for … of` and NFKC checks then added about 84 KiB
(`run-temper` 734 → 818 KiB small, 1326 KiB release; wasm 434 → 518 KiB,
30 KiB more gzipped), of which the NFKC tables are roughly 16 KiB of data.
Peak memory didn't move (14.9 MiB, against 193–217 MiB for Kotlin on the
same lexing and parsing). `when`, `?.`, `??` and Float64 then added
20 KiB natively and 39 KiB to wasm (14 KiB gzipped), likely mostly the math
functions the Float64 methods pull in from Rust's libm; memory stayed at
15 MiB. Strings, `StringBuilder`, tags and destructuring `let` then
added 16 KiB natively (1389 KiB release) and 17 KiB to wasm (7 KiB
gzipped); memory stayed at 14.9 MiB, against 199–250 MiB for Kotlin.
Top-level dependency order, `~` and `>>>` then added 8 KiB natively
(1411 KiB release) and 12 KiB to wasm (3 KiB gzipped); memory stayed at
14.9 MiB, against 194–243 MiB for Kotlin.
Modules, `export` and `import` then added 28 KiB natively (1455 KiB
release; `run-temper` also loads imported directories) and 11 KiB to
wasm (4 KiB gzipped); memory 15.1 MiB against 200–248 MiB for Kotlin,
now over 802 files (628 KiB), as the Temper repo has changed.
`Deque`, `DenseBitVector` and several declarations in one then added
4 KiB natively (1469 KiB release) and 4 KiB to wasm (1 KiB gzipped);
memory 15.0 MiB against 196–274 MiB for Kotlin.
`@fun interface`, typed field initializers and locals visible after
their declaration then added under 1 KiB natively (1470 KiB release)
and 2 KiB to wasm (under 1 KiB gzipped); memory 15.1 MiB against
199–240 MiB for Kotlin.
Object literals, trailing blocks, `@auto("==")` and `Empty` then added
24 KiB natively (1500 KiB release) and 18 KiB to wasm (6 KiB gzipped),
more than their few hundred lines suggest; memory 15.1 MiB against
194–236 MiB for Kotlin.
`std` imports, `...`, `Map`, `MapBuilder`, `Pair`, static extensions
and `Date.today()` then added 24 KiB natively (1553 KiB release) and
26 KiB to wasm (9 KiB gzipped); memory 15.0 MiB against 208–237 MiB
for Kotlin.
`test`, `assert`, regex literals, the regex parser and matcher and
std's connected `Regex` methods (~1,100 lines) then added 56 KiB
natively (1662 KiB release) and 55 KiB to wasm (18 KiB gzipped), more
per line than usual: the matcher's closures through `dyn FnMut` and the
parser's error formatting are candidates to look at. Peak memory read
15.5 MiB, but so did the unchanged lexer and parser's (against
192–235 MiB for Kotlin), so that's the environment, not this change.
Flat frames then added 48 KiB natively (1778 KiB release, 116 KiB more)
and 56 KiB to wasm (17 KiB gzipped), for ~1,080 more lines (the
interpreter's 1,454 lines, outside its builtins' modules, became 2,531:
lowering ~1,040, the run loop ~250 and the rest ~1,240). In the release binary's symbols, the interpreter's code
went from 128 to 160 KiB (the run loop 14 KiB, lowering expressions
7 KiB, the layout pass 5 KiB) and hash tables' from 30 to 37 KiB, for
the layout's maps and sets. Peak memory read 13.3 MiB for both the
old and the new binary (802 files), so the drop from the row before is
the environment.
Generators then added 8 KiB natively (1811 KiB release, 33 KiB more)
and 8 KiB to wasm (3 KiB gzipped), for ~210 more lines. Peak memory
read 13.3 MiB for both `25e32ca`'s and the new small binary, alternated
three times (`compat/measure_size.py` read 15.6 MiB, run beside its
JVM oracles).
`async`, `await` and promises then added 8 KiB natively (1825 KiB
release, 14 KiB more) and 7 KiB to wasm (3 KiB gzipped), for ~180 more
lines. Peak memory read 13.2–13.4 MiB for `13f4889`'s small binary and
13.4 MiB for the new one, alternated three times, now over 822 files
(770 KiB), since the inputs now include Chromarite and
Ordable; rows before this one read fewer files.
Inherited members, the class checks and private members on any instance
then added 24 KiB natively (1877 KiB release, 52 KiB more) and 22 KiB to
wasm (8 KiB gzipped), for ~600 more lines, many of them building
messages. Alternating `684e37c`'s binaries and the new ones six times
(824 files): peak memory read
14.9–15.4 MiB for both, higher than the row before for both, so the
environment; settling all files took 157–175 ms before and 158–177 ms
after; the interpreter loops were the same within noise (`loop`
0.20–0.23 against 0.19–0.24 µs, `calls` 0.31–0.36 against 0.31–0.41 µs,
`fib` 141–148 against 140–148 µs), and a loop of method calls through an
interface, reading a private field of another instance, took 1.66–1.80
µs before and 1.78–1.99 µs after, maybe 5% slower, though a method call
no longer allocates a `Val::Method`.
Not yet measured: the full compiler, which will
add types, backends and the std library.
