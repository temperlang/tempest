//! Temper's std library, built in, so that the binaries and the wasm build
//! import `std/…` modules with nothing shipped beside them. The sources
//! are tempest's vendored copy, `vendor/temper/std`; a test checks that
//! [`FILES`] lists every file there.
//!
//! Built-in std files' paths start with [`DIR`], as
//! `-std-/json/json.temper.md`, which no library's own files are likely
//! to share. A [`Library::std_dir`](crate::library::Library::std_dir)
//! loads std from a directory instead, for testing against another copy.

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

/// The built-in files of the module in `dir`, as `-std-/json`, with their
/// paths and texts, or `None` if `dir` isn't under [`DIR`].
pub fn module_files(dir: &str) -> Option<Vec<(String, &'static str)>> {
    let rest = dir.strip_prefix(DIR)?;
    let module = match rest.strip_prefix('/') {
        Some(module) => module,
        None if rest.is_empty() => "",
        None => return None,
    };
    let files = FILES
        .iter()
        .filter(|(path, _)| path.rsplit_once('/').map_or("", |(d, _)| d) == module)
        .map(|&(path, text)| (format!("{DIR}/{path}"), text))
        .collect();
    Some(files)
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

    #[test]
    fn module_files_by_dir() {
        let paths = |dir| module_files(dir).map(|f| f.into_iter().map(|(p, _)| p).collect());
        assert_eq!(
            paths("-std-/json"),
            Some(vec!["-std-/json/json.temper.md".to_string()])
        );
        assert_eq!(
            paths("-std-"),
            Some(vec!["-std-/config.temper.md".to_string()])
        );
        assert_eq!(paths("-std-/nope"), Some(vec![]));
        assert_eq!(paths("-std-x/json"), None);
        assert_eq!(paths("lib/json"), None);
    }
}
