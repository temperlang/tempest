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
  install.
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
- **C#:** Roslyn's `csc` directly, or one shared project, rather than a
  build per package.
- **C++:** the compiler directly, a precompiled header for the runtime,
  and outputs cached by content hash.
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
