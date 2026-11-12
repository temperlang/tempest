# The Lua runtime

What generated Lua needs beside it, begun as Kotlin Temper's (commit
69f1a5b0) and tempest's own since; [the backends' internals](../../../docs/internals/detail/06-backends.md#changes-from-kotlin-tempers-runtimes)
list what tempest changed.

- `temper-core/`: the runtime, which `src/runtime.rs` embeds for
  `tempest build` to write beside the output.
- `luaunit/`: LuaUnit, which generated tests use (BSD license, in its
  `LICENSE.txt`), embedded as `src/runtime.rs`'s `LUAUNIT`.
