//! This crate's runtime, `runtime/temper-core`, built in, for
//! `temper.out` beside generated libraries, as Kotlin Temper copies it
//! there. A test in `temper-driver` checks that [`FILES`] lists every file of
//! it that Kotlin copies: not its
//! dotfiles, nor its own tests and tools (JS's `package-lock.json`,
//! Python's `mypyc/`, and Java's `src/test/`); and Lua's rockspec is
//! named for the runtime's version, as Kotlin names it.

/// Each file, by path from the runtime's directory, with its text.
pub const FILES: &[(&str, &str)] = &[
    ("Async.cs", include_str!("../runtime/temper-core/Async.cs")),
    ("Core.cs", include_str!("../runtime/temper-core/Core.cs")),
    (
        "Float64.cs",
        include_str!("../runtime/temper-core/Float64.cs"),
    ),
    (
        "Generator.cs",
        include_str!("../runtime/temper-core/Generator.cs"),
    ),
    (
        "Listed.cs",
        include_str!("../runtime/temper-core/Listed.cs"),
    ),
    (
        "Logging.cs",
        include_str!("../runtime/temper-core/Logging.cs"),
    ),
    (
        "Optional.cs",
        include_str!("../runtime/temper-core/Optional.cs"),
    ),
    (
        "OrderedDictionary.cs",
        include_str!("../runtime/temper-core/OrderedDictionary.cs"),
    ),
    (
        "StringUtil.cs",
        include_str!("../runtime/temper-core/StringUtil.cs"),
    ),
    (
        "TemperLang.Core.csproj",
        include_str!("../runtime/temper-core/TemperLang.Core.csproj"),
    ),
];
