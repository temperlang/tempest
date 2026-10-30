//! `tempest build`: every library under a work root, translated by each
//! backend asked for, written to `temper.out` as Kotlin Temper's `temper
//! build` lays it out.
//!
//! - **Libraries** are found as `temper build` finds them
//!   ([`temper_frontend::project`]): each directory with a
//!   `config.temper.md`, named by its config, and two of one name are an
//!   error. A library may import another by its name
//!   (`import("other-lib/sub")`); its modules then settle with the
//!   importer's, and what it uses of them becomes a `uses` section of its
//!   tmpl, with the other library's version, which its package files
//!   name as a dependency. So does what it uses of std, as `temper-std`,
//!   whose output tempest doesn't write yet: tmpl doesn't translate std's
//!   modules (classes and the rest of subset B).
//! - **Output:** `temper.out/BACKEND/LIBRARY/…`, each backend's layout as
//!   Kotlin's, with the package files it writes (`pyproject.toml`,
//!   `Cargo.toml`, `package.json`, `pom.xml`, the `.csproj`s, and the
//!   rockspec), and the backend's runtime beside the libraries, as
//!   `temper.out/BACKEND/temper-core`, as Kotlin copies it. A library's
//!   directory is emptied first, so no stale file survives a rename.
//!   C++ also gets a `CMakeLists.txt` per library, which Kotlin doesn't
//!   write.
//! - **Backends:** those `-b` names, or else Kotlin's defaults that
//!   tempest has (`csharp`, `java`, `js`, `lua`, `py`, and `rust`; Kotlin's
//!   `data` isn't one of tempest's, and `cpp`, as in Kotlin, is only
//!   built when asked for).
//! - **What isn't covered yet** fails that library, with the first reason
//!   on a line of its own (`LIBRARY: BACKEND: why`), and the build goes on
//!   with the others.

use crate::Backend;
use std::io::Write;
use std::path::Path;
use temper_frontend::library::Level;
use temper_frontend::library_tests::write_located_diagnostics;
use temper_frontend::project::{
    CONFIG_FILE, LibrarySources, duplicate_names, find_libraries, library_of, load_library,
};
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::to_tmpl;
use temper_tmpl::{Profile, Program, validate};

/// What a build did: each library's result per backend.
pub struct Built {
    /// Each library, backend, and why it failed, if it did.
    pub results: Vec<(String, &'static str, Result<(), String>)>,
}

/// Builds the libraries under a work root for the backends given,
/// writing to `ROOT/temper.out`, and diagnostics and failures to `err`,
/// and with `verbose`, what was written to `out`. `None` if there's no
/// library, or two share a name.
pub fn build(
    sources: &Sources,
    root: &str,
    backends: &[Backend],
    verbose: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Option<Built> {
    let libraries = find_libraries(sources, root);
    if libraries.is_empty() {
        let _ = writeln!(err, "No Temper library ({CONFIG_FILE}) under {root}");
        return None;
    }
    let duplicates = duplicate_names(&libraries);
    for name in &duplicates {
        let _ = writeln!(err, "Two libraries are named {name} under {root}");
    }
    if !duplicates.is_empty() {
        return None;
    }
    let temper_out = Path::new(root).join("temper.out");
    let mut results = Vec::new();
    for library in &libraries {
        let name = &library.config.name;
        let program = match program(sources, library, &libraries, err) {
            Ok(program) => program,
            Err(why) => {
                for &backend in backends {
                    results.push((name.clone(), backend.name(), Err(why.clone())));
                }
                continue;
            }
        };
        for &backend in backends {
            let dir = temper_out.join(backend.name());
            let result = backend
                .translate(&program, Some((&dir, &library.root)))
                .and_then(|files| write_library(&dir, name, &files));
            if result.is_ok() && verbose {
                let _ = writeln!(out, "Wrote {}", dir.join(name).display());
            }
            results.push((name.clone(), backend.name(), result));
        }
    }
    for &backend in backends {
        let dir = temper_out.join(backend.name());
        if let Err(why) = write_runtime(&dir, backend.runtime()) {
            results.push(("temper-core".into(), backend.name(), Err(why)));
        }
    }
    for (library, backend, result) in &results {
        if let Err(why) = result {
            let _ = writeln!(err, "{library}: {backend}: {why}");
        }
    }
    Some(Built { results })
}

/// A library's program in tmpl, with its config's version, or why there
/// isn't one: errors, which go to `err`, or what tmpl doesn't cover yet.
fn program(
    sources: &Sources,
    library: &LibrarySources,
    all: &[LibrarySources],
    err: &mut dyn Write,
) -> Result<Program, String> {
    let settled = settle(load_library(sources, library, all)?);
    let mut diagnostics = String::new();
    write_located_diagnostics(&settled, &mut diagnostics);
    let _ = err.write_all(diagnostics.as_bytes());
    let errors = (settled.library.diagnostics.iter())
        .filter(|d| d.level == Level::Error)
        .count();
    if errors > 0 {
        return Err(format!("{errors} errors"));
    }
    // The libraries it imports, named when it fails, which may be why.
    let used: Vec<&str> = (settled.library.modules.iter())
        .filter_map(|m| library_of(all, &m.dir))
        .filter(|l| l.root != library.root)
        .map(|l| l.config.name.as_str())
        .collect();
    let mut program = to_tmpl(&settled, &library.config.name).map_err(|e| {
        if used.is_empty() {
            e.to_string()
        } else {
            format!("{e} (it imports {})", used.join(", "))
        }
    })?;
    if let Some(version) = library.config.get("version") {
        program.library.version = version.to_string();
    }
    // The libraries it uses take their configs' versions, for package
    // files' dependencies.
    for uses in &mut program.uses {
        let name = program.decls.get(uses.decl).name;
        let name = program.strings.get(name);
        let config = all.iter().find(|l| l.config.name == name);
        if let Some(version) = config.and_then(|l| l.config.get("version")) {
            uses.version = version.to_string();
        }
    }
    let problems = validate(&program, &Profile::full());
    if let Some(problem) = problems.first() {
        return Err(format!("tmpl: {problem}"));
    }
    Ok(program)
}

/// Writes a library's files under a backend's directory, after emptying
/// the library's own directory there.
fn write_library(dir: &Path, library: &str, files: &[(String, String)]) -> Result<(), String> {
    let own = dir.join(library);
    if own.exists() {
        std::fs::remove_dir_all(&own).map_err(|e| format!("{}: {e}", own.display()))?;
    }
    for (file, text) in files {
        let path = dir.join(file);
        path.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, text))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// Writes a backend's runtime as `temper-core` in its directory, replacing
/// what was there, as Kotlin does.
fn write_runtime(dir: &Path, files: &[(&str, &str)]) -> Result<(), String> {
    let core = dir.join("temper-core");
    if core.exists() {
        std::fs::remove_dir_all(&core).map_err(|e| format!("{}: {e}", core.display()))?;
    }
    let pairs: Vec<(String, String)> = (files.iter())
        .map(|(p, t)| (format!("temper-core/{p}"), t.to_string()))
        .collect();
    write_library(dir, "temper-core", &pairs)
}
