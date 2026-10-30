//! The vendored runtime, `temper-core`, built in, for `temper.out` beside
//! generated libraries, as Kotlin Temper copies it there. A test in
//! `tempest` checks that [`FILES`] lists every file of
//! `vendor/temper/be-lua/temper-core` that Kotlin copies: not its
//! dotfiles, nor its own tests and tools (JS's `package-lock.json`,
//! Python's `mypyc/`, and Java's `src/test/`); and Lua's rockspec is
//! named for the runtime's version, as Kotlin names it.

/// Each file, by path from the runtime's directory, with its text.
pub const FILES: &[(&str, &str)] = &[
    (
        "init.lua",
        include_str!("../../../vendor/temper/be-lua/temper-core/init.lua"),
    ),
    (
        "intnew.lua",
        include_str!("../../../vendor/temper/be-lua/temper-core/intnew.lua"),
    ),
    (
        "intold.lua",
        include_str!("../../../vendor/temper/be-lua/temper-core/intold.lua"),
    ),
    (
        "powersort/README.md",
        include_str!("../../../vendor/temper/be-lua/temper-core/powersort/README.md"),
    ),
    (
        "powersort/init-internal.lua",
        include_str!("../../../vendor/temper/be-lua/temper-core/powersort/init-internal.lua"),
    ),
    (
        "powersort/init.lua",
        include_str!("../../../vendor/temper/be-lua/temper-core/powersort/init.lua"),
    ),
    (
        "regex/runtime.lua",
        include_str!("../../../vendor/temper/be-lua/temper-core/regex/runtime.lua"),
    ),
    (
        "temper-core-0.6.0-1.rockspec",
        include_str!("../../../vendor/temper/be-lua/temper-core/temper-core-dev-1.rockspec"),
    ),
];
