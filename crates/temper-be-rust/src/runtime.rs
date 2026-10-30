//! The vendored runtime, `temper-core`, built in, for `temper.out` beside
//! generated libraries, as Kotlin Temper copies it there. A test in
//! `tempest` checks that [`FILES`] lists every file of
//! `vendor/temper/be-rust/temper-core` that Kotlin copies: not its
//! dotfiles, nor its own tests and tools (JS's `package-lock.json`,
//! Python's `mypyc/`, and Java's `src/test/`); and Lua's rockspec is
//! named for the runtime's version, as Kotlin names it.

/// Each file, by path from the runtime's directory, with its text.
pub const FILES: &[(&str, &str)] = &[
    (
        "Cargo.lock",
        include_str!("../../../vendor/temper/be-rust/temper-core/Cargo.lock"),
    ),
    (
        "Cargo.toml",
        include_str!("../../../vendor/temper/be-rust/temper-core/Cargo.toml"),
    ),
    (
        "src/float64.rs",
        include_str!("../../../vendor/temper/be-rust/temper-core/src/float64.rs"),
    ),
    (
        "src/generator.rs",
        include_str!("../../../vendor/temper/be-rust/temper-core/src/generator.rs"),
    ),
    (
        "src/lib.rs",
        include_str!("../../../vendor/temper/be-rust/temper-core/src/lib.rs"),
    ),
    (
        "src/listed.rs",
        include_str!("../../../vendor/temper/be-rust/temper-core/src/listed.rs"),
    ),
    (
        "src/mapped.rs",
        include_str!("../../../vendor/temper/be-rust/temper-core/src/mapped.rs"),
    ),
    (
        "src/promise.rs",
        include_str!("../../../vendor/temper/be-rust/temper-core/src/promise.rs"),
    ),
    (
        "src/string.rs",
        include_str!("../../../vendor/temper/be-rust/temper-core/src/string.rs"),
    ),
];
