//! Temper's std library, built in, so that the binaries and the wasm build
//! import `std/…` modules with nothing shipped beside them. The sources
//! are tempest's vendored copy, `vendor/temper/std`; a test checks that
//! [`FILES`] lists every file there.
//!
//! Built-in std files' paths start with [`DIR`], as
//! `-std-/json/json.temper.md`, which no library's own files are likely
//! to share. [`Sources`](crate::sources::Sources) serves them there, or
//! mounts a directory there instead, for testing against another copy.

/// The directory that built-in std modules' paths start with.
pub const DIR: &str = "-std-";

/// Each std source, by path from std's root, with its text.
pub const FILES: &[(&str, &str)] = &[
    (
        "config.temper.md",
        include_str!("../../../vendor/temper/std/config.temper.md"),
    ),
    (
        "json/json.temper.md",
        include_str!("../../../vendor/temper/std/json/json.temper.md"),
    ),
    (
        "net/net.temper.md",
        include_str!("../../../vendor/temper/std/net/net.temper.md"),
    ),
    (
        "regex/regex.temper.md",
        include_str!("../../../vendor/temper/std/regex/regex.temper.md"),
    ),
    (
        "temporal/temporal.temper.md",
        include_str!("../../../vendor/temper/std/temporal/temporal.temper.md"),
    ),
    (
        "testing/testing.temper.md",
        include_str!("../../../vendor/temper/std/testing/testing.temper.md"),
    ),
];

/// std's modules that build as a library of its own, in the order they
/// run (notes/tmpl/modules.md, "std as a library"). `temporal` waits for
/// `Date` as a connected type, and `net` for host code that serves it.
pub const MODULES: [&str; 3] = ["testing", "regex", "json"];

/// std as a library of its own, its modules named in `modules` (as
/// [`MODULES`] names them) loaded from `sources`, with what they import,
/// ready to settle and translate as the library `std`.
pub fn library(sources: &crate::sources::Sources, modules: &[&str]) -> crate::library::Library {
    let mut library = crate::library::Library::new();
    for module in modules {
        let path = format!("{DIR}/{module}/{module}.temper.md");
        if let Ok(text) = sources.read(&path) {
            library.add_file(&path, &text, crate::project::source_kind(&path));
        }
    }
    crate::project::load_imports(sources, &mut library);
    library
}

/// std's [`MODULES`] as one tmpl program, the library `std`, at the
/// version its config gives; or what isn't covered yet. Its `@connected`
/// members keep their Temper bodies: each backend connects those its own
/// table has and translates the rest (notes/tmpl/open-questions.md, 48).
pub fn program(
    sources: &crate::sources::Sources,
) -> Result<temper_tmpl::Program, crate::to_tmpl::NotYet> {
    program_of(sources, &MODULES)
}

/// [`program`] with only some of std's modules (and what they import),
/// for a backend that translates only those so far; its declarations'
/// names are those [`program`]'s naming gives, since naming is by module
/// and class.
pub fn program_of(
    sources: &crate::sources::Sources,
    modules: &[&str],
) -> Result<temper_tmpl::Program, crate::to_tmpl::NotYet> {
    let settled = crate::settle::settle(library(sources, modules));
    let mut program = crate::to_tmpl::to_tmpl(&settled, "std")?;
    program.library.version = crate::to_tmpl::STD_VERSION.to_string();
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Every file under `vendor/temper/std`, by path from there.
    fn vendored(dir: &Path, prefix: &str, paths: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            if entry.file_type().unwrap().is_dir() {
                vendored(&entry.path(), &format!("{prefix}{name}/"), paths);
            } else {
                paths.push(format!("{prefix}{name}"));
            }
        }
    }

    #[test]
    fn lists_every_vendored_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/std");
        let mut paths = Vec::new();
        vendored(&root, "", &mut paths);
        paths.sort();
        let listed: Vec<&str> = FILES.iter().map(|&(path, _)| path).collect();
        assert_eq!(paths, listed);
    }
}
