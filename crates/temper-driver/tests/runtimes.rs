//! Each backend's built-in runtime (`temper_be_*::runtime::FILES`), which
//! `tempest build` writes beside the libraries, is its crate's
//! `runtime/temper-core`, copied as Kotlin copies it: every file but its
//! dotfiles and its own tests and tools, with the same text.

use std::fs;
use std::path::{Path, PathBuf};

/// What Kotlin leaves out of a runtime it copies.
fn left_out(backend: &str, path: &str) -> bool {
    // Dotfiles, and what's in dot folders, as a checker's `.mypy_cache`.
    path.split('/').any(|part| part.starts_with('.'))
        || (backend, path) == ("js", "package-lock.json")
        || (backend == "py" && path.starts_with("mypyc/"))
        || (backend == "java" && path.starts_with("src/test/"))
}

fn source_files(backend: &str, dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            source_files(backend, &path, root, out);
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if !left_out(backend, &rel) {
            out.push((rel, fs::read_to_string(&path).unwrap()));
        }
    }
}

#[allow(dead_code)] // Unless every backend is left out.
fn check(backend: &str, files: &[(&str, &str)]) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../temper-be-{backend}/runtime/temper-core"));
    let mut expected = Vec::new();
    source_files(backend, &root, &root, &mut expected);
    expected.sort();
    let mut actual: Vec<(String, String)> = (files.iter())
        .map(|(p, t)| {
            // Lua's rockspec, named for the runtime's version, as Kotlin's.
            let p = p.replace("temper-core-0.6.0-1.rockspec", "temper-core-dev-1.rockspec");
            (p, t.to_string())
        })
        .collect();
    actual.sort();
    let names = |v: &[(String, String)]| v.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>();
    assert_eq!(names(&actual), names(&expected), "{backend}");
    assert!(actual == expected, "{backend}: a file's text differs");
}

#[test]
fn runtimes_are_the_crates_own() {
    #[cfg(feature = "cpp")]
    check("cpp", temper_be_cpp::runtime::FILES);
    #[cfg(feature = "csharp")]
    check("csharp", temper_be_csharp::runtime::FILES);
    #[cfg(feature = "java")]
    check("java", temper_be_java::runtime::FILES);
    #[cfg(feature = "js")]
    check("js", temper_be_js::runtime::FILES);
    #[cfg(feature = "lua")]
    check("lua", temper_be_lua::runtime::FILES);
    #[cfg(feature = "py")]
    check("py", temper_be_py::runtime::FILES);
    #[cfg(feature = "rust")]
    check("rust", temper_be_rust::runtime::FILES);
}
