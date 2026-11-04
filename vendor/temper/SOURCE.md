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

Temper and tempest are both The Temper Project's, under the same licenses
(see COPYRIGHT), except LuaUnit (`be-lua/luaunit/`), under the BSD
license in its LICENSE.txt. Refresh with `uv run compat/sync_temper.py`.
