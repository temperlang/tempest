# Source

Copied from Kotlin Temper by `compat/sync_temper.py` on 2026-11-04.

- Branch: `connected-type-fallback`
- Commit: `a4cd9a9f26f1841e12edf9690740d069fd25922c`
- Uncommitted changes in the copied folders: no

Folders:

- `functional-tests/` from `functional-test-suite/src/commonMain/resources/`
- `std/` from `frontend/src/commonMain/resources/std/`
- `core/` from `frontend/src/commonMain/resources/core/`
- `be-py/temper-core/` from `be-py/src/commonMain/resources/lang/temper/be/py/temper-core/`
- `be-rust/temper-core/` from `be-rust/src/commonMain/resources/lang/temper/be/rust/temper-core/`
- `be-js/temper-core/` from `be-js/src/commonMain/resources/lang/temper/be/js/temper-core/`
- `be-java/temper-core/` from `be-java/src/commonMain/resources/lang/temper/be/java/temper-core/`
- `be-csharp/temper-core/` from `be-csharp/src/commonMain/resources/lang/temper/be/csharp/temper-core/`
- `be-csharp/library-template/` from `be-csharp/src/commonMain/resources/lang/temper/be/csharp/library-template/`
- `be-cpp/temper-core/` from `be-cpp/src/commonMain/resources/lang/temper/be/cpp/core/`
- `be-lua/temper-core/` from `be-lua/src/commonMain/resources/lang/temper/be/lua/temper-core/`
- `be-lua/luaunit/` from `be-lua/src/commonMain/resources/lang/temper/be/lua/luaunit/`
- `be-rust/std/regex/` from `be-rust/src/commonMain/resources/lang/temper/be/rust/std/regex/`
- `be-java/temper-std/regex/` from `be-java/src/commonMain/resources/lang/temper/be/java/temper-std/src/main/java/temper/std/regex/`
- `be-csharp/std/Regex/` from `be-csharp/src/commonMain/resources/lang/temper/be/csharp/std/Regex/`
- `be-csharp/std/Temporal/` from `be-csharp/src/commonMain/resources/lang/temper/be/csharp/std/Temporal/`

Patches, tempest's fixes to the copy (behavior kept), from
`vendor/temper-patches/`, applied after copying:

- `0001-js-runtime-jsdoc-types.patch`: Fix the JSDoc types of eight be-js runtime helpers, so strict tsc takes generated calls without casts. Upstream: not yet; logged for a Kotlin Temper fix.
- `0002-js-splice-jsdoc-types.patch`: Fix the JSDoc types of be-js's listBuilderSplice, so strict tsc takes generated calls. Upstream: not yet; logged for a Kotlin Temper fix.
- `0003-rust-promise-future.patch`: Make be-rust's Promise a std Future, so Rust code can .await Temper promises on any executor. Upstream: not yet; to offer Kotlin Temper.
- `0004-js-break-promise-jsdoc.patch`: Make the reason of be-js's breakPromise optional in its JSDoc, so strict tsc takes generated calls. Upstream: not yet; logged for a Kotlin Temper fix.
- `0005-cpp-async-drain-co-await.patch`: Drain be-cpp's async queue when code outside any async block settles a promise, and let C++20 coroutines co_await Temper promises. Upstream: not yet; logged for a Kotlin Temper fix (Kotlin's C++ fails async-broken-early and await-in-loop, note 520 in its functional-test-matrix.md); co_await to offer Kotlin Temper.
- `0006-lua-promises-async.patch`: Add promises and async blocks to be-lua's runtime, which Kotlin's Lua calls but lacks. Upstream: not yet; to offer Kotlin Temper (its issue 144).
- `0007-py-one-turn-at-a-time.patch`: Run be-py's async steps one at a time on one runner, settle promises by queueing waiters, and make promises awaitable from asyncio. Upstream: partly Temper PR #532 (open), followed as closely as fits; the runner hook, asyncio adapter, TemperFuture, and deadlock check to offer Kotlin Temper.
- `0008-java-one-turn-at-a-time.patch`: Run be-java's async steps one at a time on one runner, add Core.awakeUpon and the top-level turn, and make Generator Iterable. Upstream: partly Temper PR #532 (open), followed as closely as fits; setAsyncRunner, the deadlock check, and Iterable to offer Kotlin Temper.
- `0009-csharp-one-turn-at-a-time.patch`: Run be-csharp's async steps one at a time on one thread, and hold top-level code in a turn. Upstream: not yet; to offer Kotlin Temper (its issue 516, which PR #532 leaves for C#).
- `0010-rust-awake-upon-and-iterator.patch`: Queue be-rust's resumes on the runner (AsyncRunner::awake_upon), guard turns, and make SafeGenerator an Iterator. Upstream: not yet; to offer Kotlin Temper (its issue 519).
- `0011-lua-queued-waiters-and-generators.patch`: Queue be-lua's promise waiters behind one drain loop, and make generators callable tables that for-in steps and a failed step leaves done. Upstream: not yet; to offer Kotlin Temper with patch 0006 (its issue 144).
- `0012-cpp-generator-range-for.patch`: Let a range-for step be-cpp's generators (C++14), and leave a generator done after a failed step. Upstream: not yet; to offer Kotlin Temper.

Temper and tempest are both The Temper Project's, under the same licenses
(see COPYRIGHT), except LuaUnit (`be-lua/luaunit/`), under the BSD
license in its LICENSE.txt. Refresh with `uv run compat/sync_temper.py`.
