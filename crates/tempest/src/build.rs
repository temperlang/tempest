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
//!   whose names come from std's own naming, as std's modules that build
//!   as a library ([`temper_frontend::stdlib::program`]) give them; std's
//!   output is written beside the libraries that use it, as Kotlin's.
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
//! - **Host code:** a library's files that aren't Temper go with its
//!   program ([`temper_tmpl::host`]), for each backend to take those with
//!   its extensions; `--no-connected` ([`Options::no_connected`]) has
//!   every backend use Temper implementations instead, wherever there are
//!   any.
//! - **Errors** don't stop a library: as Kotlin's `temper build` does, it
//!   reports each, then translates and writes the library anyway, each
//!   error failing where it runs, as `bubble()` does (tmpl's `Error`), so
//!   its good parts work in every language. [`Built::errors`] counts
//!   them, and `tempest build` exits with 1 when there are any.
//! - **What isn't covered yet** fails that library, with the first reason
//!   on a line of its own (`LIBRARY: BACKEND: why`), and the build goes on
//!   with the others. With `--verbose`, every gap in tmpl's translation
//!   comes first, one per line (`LIBRARY: not yet in tmpl: …`), from
//!   [`temper_frontend::to_tmpl::gaps`].

use crate::{checked, interface, sources_prefix, translate, uses_std};
use std::io::Write;
use std::path::Path;
use temper_frontend::library::Level;
use temper_frontend::library_tests::write_located_diagnostics;
use temper_frontend::project::{
    CONFIG_FILE, LibrarySources, duplicate_names, find_libraries, library_of, load_library,
};
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::gaps;
use temper_tmpl::host::HostFile;
use temper_tmpl::naming::Interface;
use temper_tmpl::{Backend, Program};

/// How to build.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Says what was written.
    pub verbose: bool,
    /// Uses connected functions' and classes' Temper implementations
    /// wherever there are any, rather than host code, as Kotlin's
    /// `--no-connected`.
    pub no_connected: bool,
}

/// What a build did: each library's result per backend.
pub struct Built {
    /// Each library, backend, and why it failed, if it did.
    pub results: Vec<(String, &'static str, Result<(), String>)>,
    /// Each library, with how many tests it has: those that didn't reach
    /// tmpl too, which count as not run, as long as it settled.
    pub tests: Vec<(String, usize)>,
    /// Each library with errors, and how many, whether or not it built.
    pub errors: Vec<(String, usize)>,
}

impl Built {
    /// Whether every library built for every backend, without errors.
    pub fn ok(&self) -> bool {
        self.errors.is_empty() && self.results.iter().all(|(_, _, r)| r.is_ok())
    }
}

/// Builds the libraries under a work root for the backends given,
/// writing to `ROOT/temper.out`, and diagnostics and failures to `err`,
/// and with `verbose`, what was written to `out`. `None` if there's no
/// library, or two share a name.
pub fn build(
    sources: &Sources,
    root: &str,
    backends: &[&Backend],
    options: Options,
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
    let mut programs = Vec::new();
    let mut tests = Vec::new();
    let mut errors = Vec::new();
    for library in &libraries {
        let name = &library.config.name;
        match program(sources, library, &libraries, options, &mut errors, err) {
            Ok(program) => {
                let count = program.modules.iter().map(|m| m.tests.len()).sum();
                tests.push((name.clone(), count));
                programs.push((library, program));
            }
            Err((why, count)) => {
                tests.push((name.clone(), count));
                for &backend in backends {
                    results.push((name.clone(), backend.name, Err(why.clone())));
                }
            }
        }
    }
    // Each library after those it uses, so each backend names it with
    // their interfaces (notes/tmpl/open-questions.md, 46 and 47).
    let order = dependency_order(&programs);
    // std, as a library of its own, named by each backend before the
    // libraries that use it.
    let std = match programs.iter().any(|(_, p)| uses_std(p)) {
        true => match temper_frontend::stdlib::program(sources) {
            Ok(std) => Some(std),
            Err(why) => {
                let _ = writeln!(err, "std: {why}");
                None
            }
        },
        false => None,
    };
    for &backend in backends {
        let mut interfaces: Vec<Interface> = Vec::new();
        if let Some(std) = &std {
            interfaces.push(interface(std, backend));
        }
        // std's output beside the libraries, which their tests and
        // imports of std's modules need, as `temper.out/BACKEND/std`, as
        // Kotlin's.
        if let Some(std) = &std {
            let dir = temper_out.join(backend.name);
            let result = translate(std, backend, None).and_then(|files| {
                let files: Vec<(String, String)> = (files.into_iter())
                    .map(|(path, text)| {
                        let rest = path.split_once('/').map_or(path.as_str(), |(_, r)| r);
                        (format!("std/{rest}"), text)
                    })
                    .collect();
                write_library(&dir, "std", &files)
            });
            if let Err(why) = result {
                results.push(("std".into(), backend.name, Err(why)));
            }
        }
        for &i in &order {
            let (library, program) = &mut programs[i];
            let name = &library.config.name;
            program.interfaces = interfaces.clone();
            interfaces.push(interface(program, backend));
            let dir = temper_out.join(backend.name);
            // Source maps lead back to the sources' directory.
            let maps = sources_prefix(
                &dir.join(&program.library.name),
                &format!("{}/x", library.root),
            );
            let result = translate(program, backend, Some(&maps))
                .and_then(|files| write_library(&dir, name, &files));
            if result.is_ok() && options.verbose {
                let _ = writeln!(out, "Wrote {}", dir.join(name).display());
            }
            results.push((name.clone(), backend.name, result));
        }
    }
    for &backend in backends {
        let dir = temper_out.join(backend.name);
        if let Err(why) = write_runtime(&dir, backend.runtime) {
            results.push(("temper-core".into(), backend.name, Err(why)));
        }
    }
    for (library, backend, result) in &results {
        if let Err(why) = result {
            let _ = writeln!(err, "{library}: {backend}: {why}");
        }
    }
    for (library, count) in &errors {
        let _ = writeln!(err, "{library}: {}", error_count(*count));
    }
    Some(Built {
        results,
        tests,
        errors,
    })
}

/// "1 error" or "N errors".
pub fn error_count(count: usize) -> String {
    match count {
        1 => "1 error".into(),
        n => format!("{n} errors"),
    }
}

/// The programs' indexes, each after those of the libraries it uses,
/// else in the order found.
fn dependency_order(programs: &[(&LibrarySources, Program)]) -> Vec<usize> {
    let mut order: Vec<usize> = Vec::new();
    while order.len() < programs.len() {
        let before = order.len();
        for (i, (_, program)) in programs.iter().enumerate() {
            if order.contains(&i) {
                continue;
            }
            let ready = program.dependencies().iter().all(|(name, _)| {
                (programs.iter().enumerate())
                    .all(|(j, (l, _))| l.config.name != *name || order.contains(&j))
            });
            if ready {
                order.push(i);
            }
        }
        if order.len() == before {
            // A cycle, which imports rule out: the rest as found.
            let rest: Vec<usize> = (0..programs.len()).filter(|i| !order.contains(i)).collect();
            order.extend(rest);
        }
    }
    order
}

/// A library's program in tmpl, with its config's version, or why there
/// isn't one: what tmpl doesn't cover yet. Its diagnostics go to `err`,
/// and a count of its errors, if any, to `errors`; its errors become
/// tmpl's `Error`s, which fail where they run.
fn program(
    sources: &Sources,
    library: &LibrarySources,
    all: &[LibrarySources],
    options: Options,
    errors: &mut Vec<(String, usize)>,
    err: &mut dyn Write,
) -> Result<Program, (String, usize)> {
    let settled = settle(load_library(sources, library, all).map_err(|e| (e, 0))?);
    // Its tests, which count where it fails, as not run.
    let count = settled.modules.iter().map(|m| m.test_cases.len()).sum();
    let mut diagnostics = String::new();
    write_located_diagnostics(&settled, &mut diagnostics);
    let _ = err.write_all(diagnostics.as_bytes());
    let count_errors = (settled.library.diagnostics.iter())
        .filter(|d| d.level == Level::Error)
        .count();
    if count_errors > 0 {
        errors.push((library.config.name.clone(), count_errors));
    }
    // The libraries it imports, named when it fails, which may be why.
    let used: Vec<&str> = (settled.library.modules.iter())
        .filter_map(|m| library_of(all, &m.dir))
        .filter(|l| l.root != library.root)
        .map(|l| l.config.name.as_str())
        .collect();
    let translated = checked(&settled, &library.config.name);
    // With `--verbose`, every gap, not only the first.
    if translated.is_err() && options.verbose {
        for gap in gaps(&settled, &library.config.name) {
            let _ = writeln!(err, "{}: {gap}", library.config.name);
        }
    }
    let mut program = translated.map_err(|e| {
        let why = if used.is_empty() {
            e
        } else {
            format!("{e} (it imports {})", used.join(", "))
        };
        (why, count)
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
    // Its host code, by paths relative to its root.
    program.host.no_connected = options.no_connected;
    for path in &library.host_files {
        let text = (sources.read(path)).map_err(|e| (format!("{path}: {e}"), count))?;
        let rel = (path.strip_prefix(&library.root))
            .unwrap_or(path)
            .trim_start_matches('/');
        program.host.files.push(HostFile {
            path: rel.to_string(),
            text: text.into_owned(),
        });
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
