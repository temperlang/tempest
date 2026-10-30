# Tempest

An experimental, from-scratch implementation of [Temper](https://temperlang.dev/)
in Rust (edition 2024), aiming for the same language, behavior and generated
APIs with a much smaller, simpler and faster compiler. See
[DESIGN.md](DESIGN.md).

Status: early. `temper-syntax` splits literate `.temper.md` files into prose
and code, lexes Temper source into tokens, parses tokens into operator
trees, and turns those into calls, blocks and declarations (the grammar
step), as Kotlin Temper does for every `.temper` and `.temper.md` file in
the Temper, Chromarite and Ordable repos, except where we deliberately
differ (listed in
[notes/testing-and-milestones.md](notes/testing-and-milestones.md)).
`temper-frontend` has the first slice of the settle loop: names, the core
macros (`let`, `var`, `fn`, `if`, `while`, `for`, `do`, `orelse`, labels,
`break`, `continue`, `return`, `for … of`, `when`, `??`, `?.`,
destructuring `let`), simple classes and interfaces, type parameters
(kept in the tree; the interpreter ignores them), lists and list
builders, `Float64` methods and formatting, strings, string indices,
`StringBuilder` and `char'a'`, top-level declarations in dependency
order, imports and std, classes with inheritance, generators and
`async`, and a flat-frame interpreter, running all 69 of Kotlin Temper's
functional tests with matching output. Next: static types.

## Principles

Details are in [notes/](notes/README.md); in short:

- Match Kotlin Temper's behavior, error messages and generated public APIs,
  not its implementation. Check against it with the oracles in `compat/` and
  by running Kotlin Temper itself, including `temper repl`.
- Keep it simple: one tree, a settle loop instead of stages, and errors as
  values that still reach code generation.
- Code without explicit lifetime annotations where avoidable: ids into arenas
  instead of references, borrows on the stack, copies when cheap
  ([notes/code-style.md](notes/code-style.md)).
- Dependencies only when worth their weight, widely used, and fast to compile.
- Generated code: no numbered names except on real conflicts, and source maps
  for every backend.
- Scripts in Python, run with `uv run`, for Python 3.11+ (what Temper's Python
  backend targets).
- Timing on the dev machine is noisy, so compare variants back to back,
  repeated.

## Layout

- `crates/temper-syntax`: source to trees. Modules `literate` (code blocks in
  `.temper.md`), `lex` (tokens), `adapt` (token adapters: semicolon
  insertion, string wrapping and the like), `operator` (the operator table),
  `parse` (operator-precedence parser), `ast` and `grammar` (the grammar
  step: calls, blocks, declarations), `module` (the rest of Kotlin's
  `ParseStage`: call joins rewritten, `;;;` metadata as JSON), `unpack` (numbers, strings and doc
  comments from tokens), `tree`, `names` (interning), `nfkc` (checks that
  identifiers are in Unicode NFKC) and `print` (text forms of trees), with the `literate-blocks`, `lex-tokens`, `parse-tree` and
  `ast-tree` binaries used by the compat checks.
- `crates/temper-frontend`: libraries to settled trees. Modules `library`
  (one arena per library build), `engine` (the settle loop's scheduler),
  `settle` (its steps), `scope` (scopes and symbols), `macros`, `classes`, `core` (the
  core constructs' layouts), `builtins`, `interp` (the interpreter) and
  `print`, `project` (libraries on disk, found as Kotlin's `temper test`
  finds them) and `library_tests` (running their tests), with the
  `run-temper` binary that settles and runs files.
- `crates/temper-tmpl`: tmpl, the IR between the frontend and the backends
  ([notes/tmpl/](notes/tmpl/README.md)), with no dependencies. So far,
  for subset A (functions, locals, `if`, loops, calls, builtins, and
  module init): the model and its tables, the `schema` of node kinds, a
  `Builder`, the canonical `text` writer and reader, the validator, the
  `naming` pass, and shared `analysis`.
- `crates/temper-be-py`: the Python backend, tmpl in and Python out,
  depending only on `temper-tmpl`. Its first variant, `py`, matches
  Kotlin Temper's Python output in layout, runtime (`temper_core`), and
  public names, with prettier bodies. Modules `names` (naming rules),
  `ops` (operators), `connected` (core's connected declarations, by
  qname), `translate`, and `tree` (output tree and printer).
- `crates/temper-be-rust`: the Rust backend, tmpl in and Rust out,
  depending only on `temper-tmpl`, with the same modules. Its first
  variant, `rust`, matches Kotlin Temper's Rust output in crate layout,
  runtime (`temper-core`, vendored in `vendor/temper/be-rust/`), string
  representation (`Arc<String>`), and public names and signatures, with
  prettier bodies laid out as rustfmt lays them out, and module state
  safe across threads.
- `crates/temper-be-js`: the JavaScript backend, tmpl in and JS out,
  depending only on `temper-tmpl`, with the same modules. Its first
  variant, `js`, matches Kotlin Temper's JS output in package and module
  layout (ES modules), runtime (`@temperlang/core`, vendored in
  `vendor/temper/be-js/`), and public names, with prettier bodies and
  JSDoc types that TypeScript's checker accepts in strict mode.
- `crates/temper-be-java`: the Java backend, tmpl in and Java out,
  depending only on `temper-tmpl`, with the same modules. Its first
  variant, `java`, matches Kotlin Temper's Java output in Maven layout
  (`pom.xml`, and a package per library with its `NAMEGlobal` and
  `NAMEMain` classes), runtime (`temper-core`, vendored in
  `vendor/temper/be-java/`), Java 17, and public names, with prettier
  bodies that `javac` compiles with every lint an error.
- `crates/temper-be-csharp`: the C# backend, tmpl in and C# out,
  depending only on `temper-tmpl`, with the same modules. Its first
  variant, `csharp`, matches Kotlin Temper's C# output in project layout
  (a `src` project per library with its `NAMEGlobal` static class and
  `Logging.cs`, and a `program` project), runtime (`TemperLang.Core`,
  vendored in `vendor/temper/be-csharp/`), .NET 6, and public names,
  with prettier bodies that never format by the current culture, which
  `csc` compiles with nullable reference types and every warning and
  .NET analyzer an error.
- `crates/temper-be-cpp`: the C++ backend, tmpl in and C++ out,
  depending only on `temper-tmpl`, with the same modules. Its first
  variant, `cpp`, matches Kotlin Temper's C++ output in layout (a
  library's `init.hpp`, `init.cpp`, and `main.cpp`), namespace, init
  function, runtime (`temper-core`, header-only, vendored in
  `vendor/temper/be-cpp/`), C++14, and public names, with prettier
  bodies that are safe where Kotlin's aren't (init once across threads,
  module `var`s under a lock, Temper's order of evaluation), which `g++`
  compiles with every warning an error and which run clean under
  AddressSanitizer, UndefinedBehaviorSanitizer, and ThreadSanitizer.
- `crates/temper-be-lua`: the Lua backend, tmpl in and Lua out,
  depending only on `temper-tmpl`, with the same modules. Its first
  variant, `lua`, matches Kotlin Temper's Lua output in layout (a
  library's `init.lua`, returning its exports, and its rockspec),
  runtime (`temper-core`, vendored in `vendor/temper/be-lua/`), and
  public names, with prettier bodies, typed by LuaCATS annotations for
  LuaLS, that keep exported `var`s current and stay within Lua's limit
  on locals.
- `crates/temper-funtests`: Kotlin Temper's functional tests, from
  `vendor/temper/`, for other crates' tests: sources, std imports, and
  expected output per backend.
- `crates/temper-be-testkit`: what the backends' tests share, as a
  dev-dependency: the programs they translate (the functional tests tmpl
  covers, and shared programs in its `programs/`), tmpl from Temper,
  goldens, finding tools or skipping without them, and runtimes cached
  between runs.
- `crates/tempest`: the `tempest` command, a thin binary over the other
  crates; so far `tempest build`, `tempest test`, `tempest py`,
  `tempest rust`, `tempest js`, `tempest java`, `tempest csharp`,
  `tempest cpp`, and `tempest lua`.
- `crates/temper-wasm`: a small wasm entry point, the seed of a browser
  playground, which also keeps the wasm size measurement honest: it
  parses, runs, or generates Python, Rust, JavaScript (with its source
  map), Java, C#, C++, or Lua, each backend a cargo feature.
- `compat/`: oracles that run Kotlin Temper's own code for reference results,
  and scripts that compare them with this implementation, fuzz both, and
  time both.
- `notes/`: the design and plan.
- `NOTICE`: the Unicode license notice for the Unicode data in
  `temper-syntax`.

## Testing

    cargo test

runs each crate's own tests, which don't need Kotlin Temper. For the lexer,
`crates/temper-syntax/tests/*.cases` hold inputs with their expected tokens,
and `tests/lex.rs` also lexes randomly edited copies of them and checks
properties that hold for any input. The parser's are in `tests/parse/*.cases`
(expected trees) and `tests/parse.rs`, the grammar step's in
`tests/ast/*.cases` and `tests/ast.rs`, and the module step's in
`tests/module/*.cases` and `tests/module.rs`. The settle loop's are in
`crates/temper-frontend/tests/settle/*.cases` (settled trees,
diagnostics and what running logs) and `tests/settle.rs`. tmpl's are in
`crates/temper-tmpl/tests/text/*.cases` (hand-written tmpl, with its
canonical text, validator problems, or read error) and `tests/text.rs`,
with a round trip over random programs in `tests/round_trip.rs`, qnames
in `tests/qname.rs`, the naming pass in `tests/naming.rs`, and a fixture
of core's members checked against `core.temper` in
`tests/core_fixture.rs`. The Python backend's are goldens of its output
for the functional tests tmpl covers (`crates/temper-be-py/tests/goldens/`),
and a run of each against the vendored `temper_core` runtime, which must
log the expected output and pass pyright, and mypy if installed, in
strict mode (each skipped, with a message, where missing). The Rust
backend's are the same for Rust (`crates/temper-be-rust/tests/goldens/`,
also for the shared programs in `crates/temper-be-testkit/programs/`,
which cover module state and what the functional tests don't yet, and
which every backend runs), with the goldens as
rustfmt lays them out, and a run of every generated crate, in one cargo
workspace built offline against the vendored `temper-core`, which must
log the expected output and pass clippy with warnings as errors (skipped,
with a message, without cargo, clippy, or `temper-core`'s one
dependency, `indexmap`, in cargo's cache, which `cargo fetch` in
`vendor/temper/be-rust/temper-core` puts there). The JS backend's are the
same for JS (`crates/temper-be-js/tests/goldens/`, also for the shared
programs), and a run of every generated package under Node
against the vendored `@temperlang/core`, which must log the expected
output and pass `tsc --checkJs --strict` against declarations generated
from the runtime (skipped, with a message, without Node or `tsc`; `TSC`
can name one, such as a project's `node_modules/.bin/tsc`). The Java
backend's are the same for Java (`crates/temper-be-java/tests/goldens/`,
also for the shared programs), and, without Maven, `javac`
compiling the vendored `temper-core` once (kept under cargo's target
directory) and then every generated package with every lint an error,
and `java` running each, which must log the expected output (skipped,
with a message, without `javac` or `java`; `JAVAC` and `JAVA` can name
them). The C# backend's are the same for C#
(`crates/temper-be-csharp/tests/goldens/`), and, without MSBuild or
NuGet, Roslyn's `csc` from the .NET SDK compiling the vendored
`TemperLang.Core` once and then every generated library, with nullable
reference types and every warning and .NET analyzer but one an error,
and `dotnet` running each in a culture whose minus sign isn't `-`
(skipped, with a message, without `dotnet` or the runtime's one
dependency in NuGet's cache; `DOTNET` can name it). The C++ backend's
are the same for C++ (`crates/temper-be-cpp/tests/goldens/`, each the
header and the module), and, without CMake, `g++` compiling every
generated `.cpp` against the vendored `temper-core`, through a
precompiled header of it built once, C++14 with every warning an error,
and running each library's init, then the same under AddressSanitizer
and UndefinedBehaviorSanitizer, and `module-state` from 8 threads under
ThreadSanitizer too (skipped, with a message, without `g++`; `CXX` can
name it). The Lua backend's are the same for Lua
(`crates/temper-be-lua/tests/goldens/`), and, without LuaRocks, one
`lua` process requiring every generated library beside the vendored
`temper-core`, each of which must log the expected output, `luac -p`
parsing them, and LuaLS (`lua-language-server --check`) checking them
against their annotations (skipped, with a message, without `lua` or
LuaLS; `LUA`, `LUAC`, and `LUALS` can name them). Every backend runs
`string-order`, one of the shared programs, which checks that
strings order by code point, as on the interpreter, and `modules`,
`imports-library`, and `testing`, for several modules, a library
importing another, and tests' forms.
`UPDATE_SNAPSHOTS=1 cargo test` accepts new expected output for review.

A Temper library's tests run on the interpreter as `temper test` runs
them, finding the libraries (each directory with a `config.temper.md`)
under DIR's work root, printing each test's result and Kotlin's summary,
`Tests passed: 56 of 56`, and failing if any test fails or doesn't run:

    cargo run --release -p tempest -- test [-b interp|BACKEND] [--std DIR] [DIR]

DIR defaults to the current directory. `-b interp` names the interpreter,
as Kotlin's `temper test -b interp` does. With a backend's name, as `-b
py`, it builds the libraries for that backend, saying what tmpl doesn't
translate yet in each; running the tests there waits for std's `testing`
module in tmpl (notes/test-speed.md). Libraries build into
`temper.out` as `temper build` lays it out:

    cargo run --release -p tempest -- build [-w DIR] [-b BACKEND]... [--std DIR]

which writes `temper.out/BACKEND/LIBRARY/…` under the work root (DIR with
`-w`, or else found from the current directory), each backend's runtime
beside the libraries as `temper-core`, as Kotlin copies it, and for C++
a `CMakeLists.txt` per library, which Kotlin doesn't write. Without `-b`,
it builds Kotlin's default backends that tempest has (`csharp`, `java`,
`js`, `lua`, `py`, and `rust`). A library is named by its config's `name`
(or its first line, or its path), and imports another by that name
(`import("other-lib/sub")`), which its package files then name as a
dependency. A library may have several modules, which import each
other, and tests, which go where each backend's test runner looks for
them. A library that tmpl doesn't cover yet fails with the reason, and
the others still build.

Temper's std library is built into
the binaries and the wasm build, from `vendor/temper/std`, so nothing
ships beside them; `--std DIR` loads it from a directory instead, to test
against another copy. The compiler reads files only through
temper-frontend's `Sources`, layers of the disk, the built-in std, and
in-memory files, so the wasm build and tests need no file system. Chromarite and Ordable pass all their tests this
way.

Comparing with Kotlin Temper, which needs an installed Kotlin Temper at
`../temper/cli/build/install`:

    uv run compat/check_literate.py   # markdown code blocks
    uv run compat/check_lex.py        # tokens, for all Temper files and cases
    uv run compat/check_parse.py      # trees, for all Temper files and cases
    uv run compat/check_ast.py        # grammar step trees and diagnostics
    uv run compat/check_ast.py --module   # also call joins and metadata JSON
    uv run compat/fuzz_lex.py         # tokens, for randomly edited inputs
    uv run compat/fuzz_parse.py       # trees, for randomly edited inputs
    uv run compat/fuzz_ast.py         # grammar step, for randomly edited inputs
                                      #   (--module: also call joins, metadata)
    uv run compat/bench_lex.py        # lexer speed, ours vs Kotlin's
    uv run compat/bench_parse.py      # lexer and parser speed
    uv run compat/bench_ast.py        # lexer, parser and grammar step speed
    uv run compat/bench_settle.py     # settle loop speed (ours only)
    uv run compat/bench_interp.py     # interpreter speed and memory, ours vs
                                      #   Kotlin's (in a warmed-up JVM)
    uv run compat/measure_size.py     # binary and wasm size, peak memory
                                      #   (--record: append to
                                      #   notes/size-and-speed.csv)
    uv run compat/size_speed_table.py # latest measurements, as Markdown
    uv run compat/gen_nfkc.py         # regenerate NFKC tables from the JDK

Running Kotlin Temper's functional tests in-process, from the copy of
them (with std and core) in `vendor/temper/`, so no Temper checkout is
needed; `--temper DIR` uses a checkout's instead:

    uv run compat/check_functional.py [--verbose] [--temper DIR] [TEST ...]
    uv run compat/sync_temper.py [TEMPER_DIR]   # refresh vendor/temper/

Counting the expressions typing leaves of unknown type, with the calls
it leaves unresolved, over the functional tests, std, and the libraries
under each LIBRARY directory given (`--verbose` lists each one):

    uv run compat/count_untyped.py [--verbose] [--temper DIR] [LIBRARY ...]

Translating a library into tmpl, the backend IR, and printing its
canonical text, or what it uses that the translation doesn't cover yet;
and counting the functional tests that translate cleanly, with why the
rest don't:

    target/release/run-temper --tmpl FILE...
    uv run compat/count_tmpl.py [--verbose] [--temper DIR]

Translating a library into Python, Rust, JS, Java, C#, C++, or Lua, printing
the generated files or writing them under DIR, laid out as Kotlin
Temper's `temper.out/py`, `temper.out/rust`, `temper.out/js`,
`temper.out/java`, `temper.out/csharp`, `temper.out/cpp`, or
`temper.out/lua` (a Rust
crate depends on `temper-core` beside it, a JS package on
`@temperlang/core`, a Java package on `temper-core`, a C# project on
`TemperLang.Core`'s project beside it, and C++ and Lua on `temper-core`
in the directory above, which Kotlin copies there and tempest doesn't yet;
written JS comes with a source map, which Node's `--enable-source-maps`
follows back to the Temper source):

    cargo run --release -p tempest -- py [--out DIR] [--std DIR] FILE...
    cargo run --release -p tempest -- rust [--out DIR] [--std DIR] FILE...
    cargo run --release -p tempest -- js [--out DIR] [--std DIR] FILE...
    cargo run --release -p tempest -- java [--out DIR] [--std DIR] FILE...
    cargo run --release -p tempest -- csharp [--out DIR] [--std DIR] FILE...
    cargo run --release -p tempest -- cpp [--out DIR] [--std DIR] FILE...
    cargo run --release -p tempest -- lua [--out DIR] [--std DIR] FILE...

Each backend is a cargo feature of `tempest`, all on by default, so
`--no-default-features --features net,js`, say, builds a smaller
`tempest` with only the JS backend (size-and-speed.md, "Backends' share").

CI (`.github/workflows/ci.yml`) runs formatting, clippy with warnings as
errors, the tests with and without default features (the `net` web
client, and `tempest`'s backends, whose own crates' tests run either way) (with the latest
mypy and pyright installed for the Python backend's checks,
`temper-core`'s crates fetched for the Rust backend's, Node 24 and the
latest TypeScript for the JS backend's, Temurin's JDK 21 for the Java
backend's, the .NET 6 SDK, with the C# runtime's one dependency
restored, for the C# backend's, and the runner's `g++`, with address
randomization narrowed for ThreadSanitizer, for the C++ backend's, and
Lua 5.4.6 and a pinned LuaLS for the Lua backend's), the
wasm build,
the functional tests, and `tempest test` on fresh checkouts of Chromarite
and Ordable. `.github/workflows/binaries.yml` builds `tempest`
for Linux, macOS, and Windows, and the small `temper-wasm`, by hand or on
a `v*` tag, which also attaches them to a release.
Both pin Rust 1.85.0, the workspace's `rust-version` and the first with
edition 2024, so every crate also works as a library on that toolchain.
Raising it is a deliberate change of its own.

The checks compare every Temper file in the Temper, Chromarite and
Ordable repos; `--verbose` shows diffs. Inputs where we deliberately differ from Kotlin are
listed in `compat/deviations.py` and reported as expected.

Written with Claude Code and Opus 5.5.
