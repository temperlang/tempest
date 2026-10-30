//! The vendored runtime, `temper-core`, built in, for `temper.out` beside
//! generated libraries, as Kotlin Temper copies it there. A test in
//! `tempest` checks that [`FILES`] lists every file of
//! `vendor/temper/be-py/temper-core` that Kotlin copies: not its
//! dotfiles, nor its own tests and tools (JS's `package-lock.json`,
//! Python's `mypyc/`, and Java's `src/test/`); and Lua's rockspec is
//! named for the runtime's version, as Kotlin names it.

/// Each file, by path from the runtime's directory, with its text.
pub const FILES: &[(&str, &str)] = &[
    (
        "README-temper-core.md",
        include_str!("../../../vendor/temper/be-py/temper-core/README-temper-core.md"),
    ),
    (
        "pyproject.toml",
        include_str!("../../../vendor/temper/be-py/temper-core/pyproject.toml"),
    ),
    (
        "temper_core/__init__.py",
        include_str!("../../../vendor/temper/be-py/temper-core/temper_core/__init__.py"),
    ),
    (
        "temper_core/py.typed",
        include_str!("../../../vendor/temper/be-py/temper-core/temper_core/py.typed"),
    ),
    (
        "temper_core/regex.py",
        include_str!("../../../vendor/temper/be-py/temper-core/temper_core/regex.py"),
    ),
    (
        "temper_core/testing.py",
        include_str!("../../../vendor/temper/be-py/temper-core/temper_core/testing.py"),
    ),
];
