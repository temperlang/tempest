//! The vendored runtime, `temper-core`, built in, for `temper.out` beside
//! generated libraries, as Kotlin Temper copies it there. A test in
//! `tempest` checks that [`FILES`] lists every file of
//! `vendor/temper/be-java/temper-core` that Kotlin copies: not its
//! dotfiles, nor its own tests and tools (JS's `package-lock.json`,
//! Python's `mypyc/`, and Java's `src/test/`); and Lua's rockspec is
//! named for the runtime's version, as Kotlin names it.

/// Each file, by path from the runtime's directory, with its text.
pub const FILES: &[(&str, &str)] = &[
    (
        "pom.xml",
        include_str!("../../../vendor/temper/be-java/temper-core/pom.xml"),
    ),
    (
        "src/main/java/temper/core/Core.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/Core.java"
        ),
    ),
    (
        "src/main/java/temper/core/Generator.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/Generator.java"
        ),
    ),
    (
        "src/main/java/temper/core/NonNull.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/NonNull.java"
        ),
    ),
    (
        "src/main/java/temper/core/Nullable.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/Nullable.java"
        ),
    ),
    (
        "src/main/java/temper/core/Stub.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/Stub.java"
        ),
    ),
    (
        "src/main/java/temper/core/Util.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/Util.java"
        ),
    ),
    (
        "src/main/java/temper/core/net/Core.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/net/Core.java"
        ),
    ),
    (
        "src/main/java/temper/core/net/NetResponse.java",
        include_str!(
            "../../../vendor/temper/be-java/temper-core/src/main/java/temper/core/net/NetResponse.java"
        ),
    ),
];
