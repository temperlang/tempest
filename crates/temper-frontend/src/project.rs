//! Temper libraries on disk, found as Kotlin Temper's `temper test` finds
//! them, and loaded into a [`Library`] with the modules they import.
//!
//! - The work root, from a starting directory: the nearest directory, it
//!   or an ancestor, holding a `temper.out` directory, or else the highest
//!   holding a `config.temper.md`, or else the starting directory
//!   ([`work_root`]).
//! - Under it, each directory with a `config.temper.md` is a library's
//!   root, and its `.temper` and `.temper.md` files, in it and below it
//!   but not under another library's root, are the library's sources
//!   ([`find_libraries`]). Each directory is a module. The config file
//!   itself configures the library and isn't one of its modules.
//! - Directories named `temper.out` or `temper.keep`, or starting with
//!   `.`, are skipped. Kotlin also skips what `.gitignore` lists, which we
//!   don't read; and it builds files outside any library as one more,
//!   which we leave out.
//!
//! Paths are kept as given, joined with `/`, which every platform reads,
//! so diagnostics show them relative to where the search started.

use crate::library::{Library, normalize};
use crate::stdlib;
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use temper_syntax::lex::SourceKind;

/// The file whose directory is a library's root.
pub const CONFIG_FILE: &str = "config.temper.md";

/// A library's root directory and its source files, sorted, without its
/// config file.
#[derive(Debug, PartialEq)]
pub struct LibrarySources {
    pub root: String,
    pub files: Vec<String>,
}

/// The work root for a starting directory (see the module docs), as a
/// path relative to `dir` when `dir` is relative.
pub fn work_root(dir: &str) -> String {
    let dir = slashed(dir);
    let Ok(absolute) = std::path::absolute(&dir) else {
        return dir;
    };
    // `absolute` keeps `..`, which `ancestors` would count as a level.
    let absolute = normalize(&slashed(&absolute.to_string_lossy()));
    let ancestors: Vec<&Path> = Path::new(&absolute).ancestors().collect();
    let has = |p: &Path, name: &str| p.join(name).exists();
    let up = ancestors
        .iter()
        .position(|p| p.join("temper.out").is_dir())
        .or_else(|| ancestors.iter().rposition(|p| has(p, CONFIG_FILE)));
    match up {
        Some(up) => normalize(&format!("{dir}{}", "/..".repeat(up))),
        None => normalize(&dir),
    }
}

/// The libraries under a work root, in path order.
pub fn find_libraries(root: &str) -> Vec<LibrarySources> {
    let mut libraries = Vec::new();
    scan(&normalize(&slashed(root)), None, &mut libraries);
    libraries
}

/// Adds the sources under `dir` to the library whose root is the index
/// `library` in `libraries`, or to a new one if `dir` has a config file.
fn scan(dir: &str, library: Option<usize>, libraries: &mut Vec<LibrarySources>) {
    let mut entries: Vec<(String, bool)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().into_string().ok()?;
            Some((name, e.file_type().ok()?.is_dir()))
        })
        .collect();
    entries.sort();
    let library = if entries.iter().any(|(n, d)| n == CONFIG_FILE && !d) {
        libraries.push(LibrarySources {
            root: dir.to_string(),
            files: Vec::new(),
        });
        Some(libraries.len() - 1)
    } else {
        library
    };
    for (name, is_dir) in entries {
        let path = if dir == "." {
            name.clone()
        } else {
            format!("{dir}/{name}")
        };
        if is_dir {
            if !(name.starts_with('.') || name == "temper.out" || name == "temper.keep") {
                scan(&path, library, libraries);
            }
        } else if let Some(i) = library {
            let config = name == CONFIG_FILE && libraries[i].root == dir;
            if is_source(&name) && !config {
                libraries[i].files.push(path);
            }
        }
    }
}

/// Reads a library's sources and the modules they import, with std
/// modules from `std_dir`, or else built in. The error names a file that
/// can't be read.
pub fn load_library(sources: &LibrarySources, std_dir: Option<&str>) -> Result<Library, String> {
    let mut library = Library::new();
    library.std_dir = std_dir.map(|d| normalize(&slashed(d)));
    for path in &sources.files {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        library.add_file(path, &text, source_kind(path));
    }
    load_imports(&mut library);
    Ok(library)
}

/// Adds the files of each module that an `import` names, and of those
/// they import in turn. A directory that can't be read is left out, so
/// the import fails when settled.
pub fn load_imports(library: &mut Library) {
    let mut tried = Vec::new();
    loop {
        let missing: Vec<String> = library
            .missing_imports()
            .into_iter()
            .filter(|d| !tried.contains(d))
            .collect();
        if missing.is_empty() {
            return;
        }
        for dir in missing {
            for (path, text) in module_files(library, &dir) {
                library.add_file(&path, &text, source_kind(&path));
            }
            tried.push(dir);
        }
    }
}

/// The source files of the module in `dir`, sorted, with their texts:
/// built in for std's without a [`Library::std_dir`], else read from
/// disk, leaving out those that can't be read.
fn module_files(library: &Library, dir: &str) -> Vec<(String, Cow<'static, str>)> {
    if library.std_dir.is_none() {
        if let Some(files) = stdlib::module_files(dir) {
            return files
                .into_iter()
                .map(|(path, text)| (path, Cow::Borrowed(text)))
                .collect();
        }
    }
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| is_source(name))
        .collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let path = format!("{dir}/{name}");
            let text = fs::read_to_string(&path).ok()?;
            Some((path, Cow::Owned(text)))
        })
        .collect()
}

/// Whether a file name is a Temper source's.
pub fn is_source(name: &str) -> bool {
    name.ends_with(".temper") || name.ends_with(".temper.md")
}

/// How to read a source: literate markdown, or plain Temper.
pub fn source_kind(path: &str) -> SourceKind {
    if path.ends_with(".md") {
        SourceKind::Markdown
    } else {
        SourceKind::Standalone
    }
}

/// A path with `/` between its parts, on Windows too.
pub fn slashed(path: &str) -> String {
    if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_string()
    }
}
