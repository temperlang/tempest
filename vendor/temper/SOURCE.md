# Source

Copied from Kotlin Temper by `compat/sync_temper.py` on 2026-11-02.

- Branch: `connected-type-fallback`
- Commit: `1f407c3aac3838c1e4b523fd50a5293d07fb88c6`
- Uncommitted changes in the copied folders: yes
  (the follow-up to 1f407c3a, staged and unstaged, in
  `functional-tests/types/connected/`: `total()` adds 1000 in Temper,
  a `Doubler` class listed for every backend but Lua, with its host
  code, `Doubler.cs`, and `Doubler.java`, and expected-output blocks
  named per run)

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

Temper and tempest are both The Temper Project's, under the same licenses
(see COPYRIGHT). Refresh with `uv run compat/sync_temper.py`.
