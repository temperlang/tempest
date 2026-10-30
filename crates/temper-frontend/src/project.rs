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
//! - A library's name is what its config exports as `name` (`export let
//!   name = "chromarite";`), or else its config's first line as a slug
//!   (`# Hi There!` gives `hi-there`), or else its root's path under the
//!   work root, with `-` between directories, or else `unnamed`, as
//!   Kotlin's `libraryNameFor` decides ([`Config`]). Two libraries of one
//!   name are an error, as in Kotlin.
//! - An import's path may start with a library's name, as
//!   `import("other-lib/sub")`, naming that library's `sub` directory,
//!   as Kotlin resolves it; the library's modules then build with the
//!   importer's ([`Library::libraries`]).
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
/// config file, and what its config says.
#[derive(Debug, PartialEq)]
pub struct LibrarySources {
    pub root: String,
    pub files: Vec<String>,
    pub config: Config,
}

/// What a library's config file says, as far as tempest reads it: the
/// string values it exports, `export let KEY = "VALUE";` a line, in
/// order, and the library's name. Kotlin runs the config as Temper; this
/// reads only such lines, which is all that the libraries we know of
/// write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    /// The library's name (see the module docs).
    pub name: String,
    pub exports: Vec<(String, String)>,
}

impl Config {
    /// Reads a config file's text, for a library whose root is `path`
    /// under the work root (as `a/b`, or empty for the work root).
    pub fn read(text: &str, path: &str) -> Config {
        let mut exports = Vec::new();
        for line in text.lines() {
            let Some(rest) = line.trim().strip_prefix("export let ") else {
                continue;
            };
            let Some((key, value)) = rest.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_end_matches(';').trim_end();
            let Some(value) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) else {
                continue;
            };
            let simple = |s: &str| !s.contains(['"', '\\']);
            if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && simple(value) {
                exports.push((key.to_string(), value.to_string()));
            }
        }
        let configured = (exports.iter())
            .find(|(k, v)| k == "name" && !v.trim().is_empty())
            .map(|(_, v)| v.clone());
        let from_path = || {
            let joined = (path.split('/'))
                .filter(|p| !p.is_empty() && *p != ".")
                .collect::<Vec<_>>()
                .join("-");
            dashed(&joined)
        };
        let name = configured
            .or_else(|| text.lines().next().and_then(|l| dashed(&slug(l))))
            .or_else(from_path)
            .unwrap_or_else(|| "unnamed".to_string());
        Config { name, exports }
    }

    /// A string the config exports, if it does.
    pub fn get(&self, key: &str) -> Option<&str> {
        (self.exports.iter())
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The library's version, or Kotlin's default, `0.0.1`.
    pub fn version_or_default(&self) -> &str {
        self.get("version").unwrap_or("0.0.1")
    }
}

/// A heading's text as GitHub slugs it: lowercase, without punctuation,
/// spaces as `-` (`# Hi There!` gives `hi-there`).
fn slug(line: &str) -> String {
    let text = line.trim_start_matches('#').trim();
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c == ' ' {
            out.push('-');
        }
    }
    out
}

/// A name, if it's a dashed identifier as Kotlin's `DashedIdentifier`
/// allows: ASCII letters, digits, and `-` or `_` between them, starting
/// with a letter.
fn dashed(name: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    let ok = name.starts_with(|c: char| c.is_ascii_lowercase())
        && !name.ends_with(['-', '_'])
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    ok.then_some(name)
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

/// The libraries under a work root, in path order, each with its config
/// read.
pub fn find_libraries(sources: &Sources, root: &str) -> Vec<LibrarySources> {
    let root = normalize(&slashed(root));
    let mut libraries = Vec::new();
    scan(sources, &root, None, &mut libraries);
    for library in &mut libraries {
        let config = format!("{}/{CONFIG_FILE}", library.root);
        let text = sources.read(&config).map(|t| t.into_owned());
        let path = (library.root.strip_prefix(&root))
            .unwrap_or(&library.root)
            .trim_start_matches('/');
        library.config = Config::read(&text.unwrap_or_default(), path);
    }
    libraries
}

/// The library a directory belongs to, of those in `libraries`: the one
/// with the deepest root that holds it.
pub fn library_of<'a>(libraries: &'a [LibrarySources], dir: &str) -> Option<&'a LibrarySources> {
    let holds = |root: &str| {
        root == "." && !dir.starts_with("..") && !dir.starts_with('/')
            || dir == root
            || dir.strip_prefix(root).is_some_and(|r| r.starts_with('/'))
    };
    (libraries.iter())
        .filter(|l| holds(&l.root))
        .max_by_key(|l| l.root.len())
}

/// The names that more than one library has, each once, in path order.
pub fn duplicate_names(libraries: &[LibrarySources]) -> Vec<&str> {
    let mut duplicates = Vec::new();
    for (i, library) in libraries.iter().enumerate() {
        let name = library.config.name.as_str();
        if libraries[..i].iter().any(|l| l.config.name == name) && !duplicates.contains(&name) {
            duplicates.push(name);
        }
    }
    duplicates
}

/// Adds the sources under `dir` to the library whose root is the index
/// `library` in `libraries`, or to a new one if `dir` has a config file.
fn scan(sources: &Sources, dir: &str, library: Option<usize>, libraries: &mut Vec<LibrarySources>) {
    let entries = sources.list(dir).unwrap_or_default();
    let library = if (entries.iter()).any(|e| e.name == CONFIG_FILE && !e.is_dir) {
        libraries.push(LibrarySources {
            root: dir.to_string(),
            files: Vec::new(),
            config: Config::default(),
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

/// Reads a library's sources and the modules they import, from std, and
/// from the libraries in `all` (those of its work root), which imports
/// name by their names. The error names a file that can't be read.
pub fn load_library(
    sources: &Sources,
    library: &LibrarySources,
    all: &[LibrarySources],
) -> Result<Library, String> {
    let mut loaded = Library::new();
    loaded.libraries = (all.iter())
        .map(|l| (l.config.name.clone(), l.root.clone()))
        .collect();
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
