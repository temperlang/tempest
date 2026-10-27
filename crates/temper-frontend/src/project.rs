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
//! so diagnostics show them relative to where the search started. Files
//! and directories are read through [`Sources`], which also says where
//! std's modules come from.

use crate::library::{Library, normalize};
use crate::sources::{Entry, Sources};
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
pub fn work_root(sources: &Sources, dir: &str) -> String {
    let dir = slashed(dir);
    let Ok(absolute) = std::path::absolute(&dir) else {
        return dir;
    };
    // `absolute` keeps `..`, which `ancestors` would count as a level.
    let absolute = normalize(&slashed(&absolute.to_string_lossy()));
    // Each ancestor, `up` levels up, as a path from `dir`, with its entries.
    let ancestor = |up: usize| normalize(&format!("{dir}{}", "/..".repeat(up)));
    let levels: Vec<Vec<Entry>> = (0..Path::new(&absolute).ancestors().count())
        .map(|up| sources.list(&ancestor(up)).unwrap_or_default())
        .collect();
    let up = (levels.iter())
        .position(|entries| entries.iter().any(|e| e.name == "temper.out" && e.is_dir))
        .or_else(|| {
            (levels.iter()).rposition(|entries| entries.iter().any(|e| e.name == CONFIG_FILE))
        });
    ancestor(up.unwrap_or(0))
}

/// The libraries under a work root, in path order.
pub fn find_libraries(sources: &Sources, root: &str) -> Vec<LibrarySources> {
    let mut libraries = Vec::new();
    scan(sources, &normalize(&slashed(root)), None, &mut libraries);
    libraries
}

/// Adds the sources under `dir` to the library whose root is the index
/// `library` in `libraries`, or to a new one if `dir` has a config file.
fn scan(sources: &Sources, dir: &str, library: Option<usize>, libraries: &mut Vec<LibrarySources>) {
    let entries = sources.list(dir).unwrap_or_default();
    let library = if (entries.iter()).any(|e| e.name == CONFIG_FILE && !e.is_dir) {
        libraries.push(LibrarySources {
            root: dir.to_string(),
            files: Vec::new(),
        });
        Some(libraries.len() - 1)
    } else {
        library
    };
    for Entry { name, is_dir } in entries {
        let path = if dir == "." {
            name.clone()
        } else {
            format!("{dir}/{name}")
        };
        if is_dir {
            if !(name.starts_with('.') || name == "temper.out" || name == "temper.keep") {
                scan(sources, &path, library, libraries);
            }
        } else if let Some(i) = library {
            let config = name == CONFIG_FILE && libraries[i].root == dir;
            if is_source(&name) && !config {
                libraries[i].files.push(path);
            }
        }
    }
}

/// Reads a library's sources and the modules they import. The error
/// names a file that can't be read.
pub fn load_library(sources: &Sources, library: &LibrarySources) -> Result<Library, String> {
    let mut loaded = Library::new();
    for path in &library.files {
        let text = sources.read(path).map_err(|e| format!("{path}: {e}"))?;
        loaded.add_file(path, &text, source_kind(path));
    }
    load_imports(sources, &mut loaded);
    Ok(loaded)
}

/// Adds the files of each module that an `import` names, and of those
/// they import in turn. A directory that can't be read is left out, so
/// the import fails when settled.
pub fn load_imports(sources: &Sources, library: &mut Library) {
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
            // The module's files, sorted, leaving out those that can't
            // be read.
            let entries = sources.list(&dir).unwrap_or_default();
            for entry in entries.iter().filter(|e| !e.is_dir && is_source(&e.name)) {
                let path = format!("{dir}/{}", entry.name);
                if let Ok(text) = sources.read(&path) {
                    library.add_file(&path, &text, source_kind(&path));
                }
            }
            tried.push(dir);
        }
    }
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
