//! This crate's runtime, `runtime/temper-core`, built in, for
//! `temper.out` beside generated libraries, as Kotlin Temper copies it
//! there. A test in `temper-driver` checks that [`FILES`] lists every file of
//! it that Kotlin copies: not its
//! dotfiles, nor its own tests and tools (JS's `package-lock.json`,
//! Python's `mypyc/`, and Java's `src/test/`); and Lua's rockspec is
//! named for the runtime's version, as Kotlin names it.

/// Each file, by path from the runtime's directory, with its text.
pub const FILES: &[(&str, &str)] = &[
    (
        "any_value.hpp",
        include_str!("../runtime/temper-core/any_value.hpp"),
    ),
    (
        "base_types.hpp",
        include_str!("../runtime/temper-core/base_types.hpp"),
    ),
    (
        "boolean.hpp",
        include_str!("../runtime/temper-core/boolean.hpp"),
    ),
    (
        "casting.hpp",
        include_str!("../runtime/temper-core/casting.hpp"),
    ),
    (
        "compare.hpp",
        include_str!("../runtime/temper-core/compare.hpp"),
    ),
    (
        "console.hpp",
        include_str!("../runtime/temper-core/console.hpp"),
    ),
    ("core.hpp", include_str!("../runtime/temper-core/core.hpp")),
    ("date.hpp", include_str!("../runtime/temper-core/date.hpp")),
    (
        "dense_bit_vector.hpp",
        include_str!("../runtime/temper-core/dense_bit_vector.hpp"),
    ),
    (
        "deque.hpp",
        include_str!("../runtime/temper-core/deque.hpp"),
    ),
    (
        "float64.hpp",
        include_str!("../runtime/temper-core/float64.hpp"),
    ),
    (
        "generator.hpp",
        include_str!("../runtime/temper-core/generator.hpp"),
    ),
    ("int.hpp", include_str!("../runtime/temper-core/int.hpp")),
    (
        "int64.hpp",
        include_str!("../runtime/temper-core/int64.hpp"),
    ),
    ("list.hpp", include_str!("../runtime/temper-core/list.hpp")),
    (
        "list_builder.hpp",
        include_str!("../runtime/temper-core/list_builder.hpp"),
    ),
    ("map.hpp", include_str!("../runtime/temper-core/map.hpp")),
    (
        "map_builder.hpp",
        include_str!("../runtime/temper-core/map_builder.hpp"),
    ),
    (
        "mapped.hpp",
        include_str!("../runtime/temper-core/mapped.hpp"),
    ),
    (
        "module_init.hpp",
        include_str!("../runtime/temper-core/module_init.hpp"),
    ),
    ("net.hpp", include_str!("../runtime/temper-core/net.hpp")),
    (
        "nullable_param.hpp",
        include_str!("../runtime/temper-core/nullable_param.hpp"),
    ),
    ("pair.hpp", include_str!("../runtime/temper-core/pair.hpp")),
    (
        "promise.hpp",
        include_str!("../runtime/temper-core/promise.hpp"),
    ),
    (
        "regex.hpp",
        include_str!("../runtime/temper-core/regex.hpp"),
    ),
    ("str.hpp", include_str!("../runtime/temper-core/str.hpp")),
    (
        "string.hpp",
        include_str!("../runtime/temper-core/string.hpp"),
    ),
    (
        "string_builder.hpp",
        include_str!("../runtime/temper-core/string_builder.hpp"),
    ),
    (
        "temper_bubble.hpp",
        include_str!("../runtime/temper-core/temper_bubble.hpp"),
    ),
    (
        "test_main.hpp",
        include_str!("../runtime/temper-core/test_main.hpp"),
    ),
];
