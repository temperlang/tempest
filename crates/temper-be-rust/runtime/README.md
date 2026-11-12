# The Rust runtime

What generated Rust needs beside it, begun as Kotlin Temper's (commit
69f1a5b0) and tempest's own since; [the backends' internals](../../../docs/internals/detail/06-backends.md#changes-from-kotlin-tempers-runtimes)
list what tempest changed.

- `temper-core/`: the runtime, which `src/runtime.rs` embeds for
  `tempest build` to write beside the output.
- `std/regex/`: std's `regex` support, embedded by `src/lib.rs`.

`temper-core` is a crate of its own, with its own `Cargo.lock`, kept out
of the workspace (the root `Cargo.toml`'s `exclude`); CI runs its tests
with `--manifest-path`. It and std are published as crates of their own,
[`temper-core`](https://crates.io/crates/temper-core) and
[`temper-std`](https://crates.io/crates/temper-std) on crates.io,
rather than packaged inside `temper-be-rust`.
