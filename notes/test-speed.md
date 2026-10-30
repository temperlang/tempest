# Test speed: `tempest test` on every backend

Part of the [rewrite plan](README.md). A proposal (2026-10-30), not a
decision.

`tempest test` runs a library's `test` blocks on the interpreter today,
which is fast but checks no backend. Kotlin's `temper test -b LANG`
builds each backend's output as a real package (Maven, `dotnet`, CMake,
cargo, pip, npm) and runs it with that ecosystem's tools. Leaning on
package descriptions is reliable and reuses the tooling, but some
backends are slow that way. Testing on any backend should be as fast as
we can make it, without losing that reliability.

## Two paths, kept in agreement

1. **The package path:** what Kotlin does, and what users get. Generated
   output always includes the package descriptions. CI runs this path.
2. **The fast path:** the same generated sources, built and run by
   calling each language's tools directly, skipping package managers
   and per-package build setup. It's the local default.

A check that the two agree keeps the fast path honest: the same passes
and failures on the functional tests and library tests, in CI at least,
maybe nightly. Where they can't agree (dependency resolution, native
libraries), that library falls back to the package path.

## The fast path by backend

First guesses, to be checked against measurements:

- **Python and Lua:** run from the generated folders with the runtime on
  `PYTHONPATH` or `LUA_PATH`, all tests in one process, with no venv or
  install. The Lua backend's harness does this (below, "Lua's fast path,
  as tried").
- **JS:** link packages into one `node_modules` and run everything in one
  Node process. The functional-test harness already copies the vendored
  runtime this way instead of running `npm install`.
- **Rust:** a persistent target directory, `temper-core` built once per
  machine, one test binary per library, and a dev profile tuned for build
  speed. The functional-test harness already shares one offline cargo
  workspace and target directory.
- **Java:** `javac` against a prebuilt runtime, without Maven, and a
  small test runner of our own. The Java backend's functional-test
  harness does this already (below, "Java's fast path, as tried").
- **C#:** Roslyn's `csc` directly, rather than a build per package.
  The C# backend's functional-test harness does this already (below,
  "C#'s fast path, as tried").
- **C++:** the compiler directly, a precompiled header for the runtime,
  and outputs cached by content hash. The C++ backend's harness does the
  first two (below, "C++'s fast path, as tried").
- **Go:** `go test`, already fast with its build and test caches.

## Java's fast path, as tried (2026-10-30)

The Java backend's run test (`crates/temper-be-java/tests/functional.rs`)
uses no Maven, and what it found, unmeasured, by eye:

- **The runtime needs nothing but the JDK.** `temper-core`'s main
  sources compile with `javac --release 8` alone, in about a second,
  without its `pom.xml`; JUnit is only for its own test. So "prebuilt"
  can be a directory of classes, compiled once and kept, keyed by a hash
  of the sources and the JDK's version, with no jar.
- **One `javac` for everything generated.** Each library is its own
  package, so all of them, and a test harness, compile in one run, a
  second or two for 10 small libraries; JVM startup, paid once, is most
  of it. A later step could keep one compiler warm (the
  `javax.tools.JavaCompiler` API in a long-lived JVM).
- **Running is cheap.** A small program's `java -cp … Main` takes about
  0.05 to 0.1 seconds; `-XX:TieredStopAtLevel=1 -XX:+UseSerialGC` suit
  short runs. Many libraries' tests could share one JVM, through a
  runner of our own that loads each library's classes in turn.
- **What the package path adds:** dependency resolution (here only
  `temper-core`, and JUnit for tests), the jar, and Maven's own startup,
  which is the slow part. `pom.xml` is still generated, as Kotlin's, so
  the package path stays available.

## C#'s fast path, as tried (2026-10-30)

The C# backend's run test (`crates/temper-be-csharp/tests/functional.rs`)
uses no MSBuild and no NuGet restore, and what it found, by eye on the
.NET 6.0.402 SDK:

- **`csc` runs alone.** `dotnet exec SDK/Roslyn/bincore/csc.dll`, with
  `-noconfig -nostdlib` and .NET 6's reference assemblies
  (`packs/Microsoft.NETCore.App.Ref/6.0.x/ref/net6.0/*.dll`) as `-r:`s,
  is all a library needs; the SDK's .NET analyzers load with
  `-analyzer:` and a global config (`dotnet_analyzer_diagnostic.severity
  = warning` is `AnalysisMode` `All`). What MSBuild adds that matters:
  `DefineConstants` (`TRACE` keeps the runtime's `Trace.WriteLine`
  calls, which logging falls back to; without it nothing logs) and
  `NoWarn` 1701 and 1702.
- **The runtime is one more `csc`,** about half a second, kept under
  `CARGO_TARGET_TMPDIR`, keyed by a hash of its sources, the options,
  and the SDK's version. Its one dependency, `Microsoft.Extensions.
  Logging` 8.0.0 and what that needs, are read straight from NuGet's
  cache (`~/.nuget/packages/ID/8.0.0/lib/net6.0/`), never restored.
- **One `csc` for every library:** each library has its own namespace,
  so all of them compile into one assembly in one run, about a second
  (1.06 to 1.16 seconds, three runs) for 12 small libraries with the
  analyzers; Roslyn's startup is most
  of it. Each generated `Program.cs` declares `Program` in the global
  namespace, so one is compiled as itself and the rest, renamed, into a
  runner that runs a library's static constructor by name, a few tenths
  of a second more. The whole test takes about 3 seconds warm.
- **Running is cheap:** `dotnet Runner.dll NAME` takes about 0.04 to
  0.05 seconds, given a `runtimeconfig.json` (with `rollForward` `Major`,
  so a newer runtime will do) and the dependencies copied beside it. Many
  libraries' tests could share one process, as Java's could share one
  JVM.
- **No build servers:** `csc.dll` run directly starts no
  `VBCSCompiler` and no MSBuild node, so nothing lingers; `dotnet
  build` would need `-p:UseSharedCompilation=false -nodeReuse:false`
  (.NET 6's SDK has no `--disable-build-servers`), and `dotnet
  build-server shutdown` after.
- **What the package path adds:** restore (offline, `--source
  ~/.nuget/packages` works, since NuGet's cache is a folder feed), a
  build of `TemperLang.Core` for both of its frameworks, and MSBuild's
  own startup: about 7 seconds cold for one library, by hand. The
  `.csproj`s are still generated, as Kotlin's, so that path stays
  available.

## C++'s fast path, as tried (2026-10-30)

The C++ backend's run test (`crates/temper-be-cpp/tests/functional.rs`)
uses no CMake, and what it found, by eye (not measured) with g++ 13.3:

- **The runtime is header-only,** so "built once" means a precompiled
  header: a one-line `runtime.hpp` including `<temper-core/core.hpp>`
  (a header can't be precompiled as the main file with `#pragma once`
  in it), compiled with `-x c++-header` into a `.gch` of about 58 MB in
  about 1.4 seconds, and used through `-include runtime.hpp`, so it's
  the first thing each file sees. The generated header's own `#include
  <temper-core/core.hpp>` is then skipped, as `#pragma once` carries
  over. One per set of options (plain, AddressSanitizer with
  UndefinedBehaviorSanitizer, ThreadSanitizer), since a `.gch` serves
  only the options it was built with; `-Winvalid-pch` makes a mismatch
  an error rather than a silent slow path. Kept under
  `CARGO_TARGET_TMPDIR`, keyed by a hash of the headers, the options,
  and `g++ --version`.
- **A file takes about 0.2 seconds with the header, 0.55 without;**
  under the sanitizers, about twice that. Parsing the runtime is most
  of a file's time, so the header pays off past a couple of files.
- **Each `.cpp` on its own,** four at a time, as a user's build would,
  rather than one file including every library's: a library's missing
  include or a clash between two would hide in one big file. Each
  library has its own namespace, so one runner links every library's
  `init.cpp` and runs one by name, as C#'s runner does; each
  `main.cpp` is compiled, and one linked as Kotlin's program.
- **The whole test takes about 10 seconds warm** and 15 cold, for 12
  libraries built three ways (plain, ASan with UBSan, and TSan for the
  threads), most of it the sanitizers' compiles and links.
- **ThreadSanitizer** in GCC 13 fails to start ("unexpected memory
  mapping") about half the time under Linux 6's address randomization;
  a non-PIE binary helps, `setarch -R` or `sysctl vm.mmap_rnd_bits=28`
  fixes it, and where neither is allowed (a sandbox), the test retries.
- **What the package path adds:** none, as Kotlin writes no build files;
  its `temper build` runs `g++ -I.. -std=c++14` on a library's `.cpp`
  files and its dependencies', about a second for `module-state` by
  hand, and a CMake project like Kotlin's tours' (an
  `add_library` of `init.cpp`, an executable of `main.cpp`) configured
  and built in about 1.3 seconds, cold, by hand.

## Lua's fast path, as tried (2026-10-30)

The Lua backend's run test (`crates/temper-be-lua/tests/functional.rs`)
uses no LuaRocks, and what it found, timed by hand with Lua 5.4.6 (three
runs each):

- **Nothing to build.** The runtime is Lua source, copied beside the
  libraries (where Kotlin's `temper.out/lua` has it), and loads in about
  6 milliseconds; Lua compiles each chunk as it's required.
- **One process for every library:** a 7-line runner requires each
  library named on its command line, after a line naming it, catching
  any error as a line too, so the test splits one output by library.
  Kotlin's `LUA_PATH` (`?.lua;?/init.lua`), from the directory holding
  the libraries, is all `require` needs. 14 small libraries take 6 to 7
  milliseconds in one process, against 47 to 73 for a process each
  (`lua LIB/init.lua`, as Kotlin runs one, which the test also does
  once). Requiring a library runs its top level, so tests would be a
  call per test after the require, as Kotlin's luaunit runner makes
  them.
- **One process, not one state per library:** libraries share the
  process's globals, so one that wrote a global could affect the next;
  generated code writes none (LuaLS's global diagnostics check), and
  each library's state is its own chunk's locals. Where isolation
  matters, a runner could load each into a fresh environment
  (`load(…, "t", setmetatable({}, { __index = _G }))`) at no real cost.
- **The checks are cheap too:** `luac -p` on every generated file takes
  about 6 milliseconds. LuaLS, a language server started for one
  `--check`, will be the slow part (not timed: it isn't installed
  here), which only CI pays for now.
- **What the package path adds:** LuaRocks and a rock per library, which
  Kotlin's rockspecs don't yet install usably
  ([kotlin-differences.md](kotlin-differences.md), "Lua rockspecs that
  don't install"); its `temper test` runs each library's tests in a
  process of its own, with luaunit.

## `tempest test -b`, planned (2026-10-30)

Begun (2026-10-30): tmpl translates `test` blocks (`Test` nodes, each
taking std's `Test`), and each backend writes them in its test form,
but running them waits for std's `testing` module in tmpl, whose
`Test` is a class with closures and lists (subset B), and a test's
`assert` is a call of its method, so most tests stop there too.
`tempest test -b BACKEND` builds the libraries for that backend into
`temper.out`, as `tempest build` does, saying per library what tmpl
doesn't translate yet, and then that running the tests needs std's
`testing`. The plan, once std translates, reuses the backends'
harnesses, which `crates/temper-be-testkit` now shares:

- **Build once with `tempest build`'s layout,** into `temper.out`, so
  the package path stays available and the fast path runs the same
  files.
- **Per backend, the harness's fast path:** Python and Lua run every
  library's tests in one process from `temper.out/BACKEND`, with the
  runtime on `PYTHONPATH` or `LUA_PATH`; JS one Node process, the
  runtime linked into one `node_modules`; Java one `javac` against the
  runtime's classes, cached by hash, and one JVM with a runner of ours;
  C# one `csc` and one `dotnet` runner; C++ `g++` through the runtime's
  precompiled header, or the generated CMake projects; Rust one cargo
  workspace with a kept target directory.
- **Results as JUnit,** as `temper test` reads them, through the same
  summary (`Tests passed: N of M`), so `-b interp` and `-b py` print
  alike.
- **The package path in CI,** checked against the fast path on the same
  libraries, as above.

## For every backend

- **Content-hash caching:** skip generating, building, and testing
  libraries that haven't changed, as Go caches test results. The hashes
  can be the [library cache](library-cache.md)'s.
- **Runtimes built once,** keyed by version and hash, never per run.
- **Backends in parallel,** within a memory budget.
- **The interpreter as tier 0,** the default for quick edit-and-test
  loops.

## Measure first

Time Kotlin's `temper test` per backend on Chromarite, cold and warm,
splitting setup (package tools, dependency resolution) from compiling
and running. The guess is that Maven, `dotnet`, and CMake setup
dominate. Measure only when the machine is otherwise idle, and never
hold up other work for it.

Fast-path ideas that work out are worth proposing for Kotlin's
`temper test` too.
