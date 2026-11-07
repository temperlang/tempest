//! The `tempest` command, a thin layer over the other crates, which do
//! the work as libraries: argument parsing and output.
//!
//!     tempest build [-w DIR] [-b BACKEND]... [--no-connected] [--verbose] [--std DIR]
//!     tempest test [-b interp|BACKEND] [--verbose] [--std DIR] [DIR]
//!     tempest lsp [--std DIR]
//!     tempest py [--out DIR] [--std DIR] FILE...
//!     tempest rust [--out DIR] [--std DIR] FILE...
//!     tempest js [--out DIR] [--std DIR] FILE...
//!     tempest java [--out DIR] [--std DIR] FILE...
//!     tempest csharp [--out DIR] [--std DIR] FILE...
//!     tempest cpp [--out DIR] [--std DIR] FILE...
//!     tempest lua [--out DIR] [--std DIR] FILE...
//!
//! `test` runs the tests of the libraries under DIR's work root on the
//! interpreter, as Kotlin Temper's `temper test` does
//! ([`temper_frontend::library_tests`]), printing only failures and a
//! summary, or with `--verbose` (`-v`), every test's result and the
//! libraries' output. DIR defaults to the current
//! directory. `-b interp` names the interpreter, as Kotlin's does. With
//! another backend, `-b py` say, it builds the libraries for that backend
//! as `build` does, saying what each needs that isn't covered yet, then
//! runs the tests of those that built with the language's own tools
//! ([`temper_driver::test_run`]), reporting as on the interpreter; tests of a library
//! that didn't build count as not run. Std modules are built in ([`temper_frontend::stdlib`]), or
//! come from `--std`'s directory, to test against another copy.
//!
//! `build` translates every library under the work root (DIR with `-w`,
//! or else found from the current directory as `test` finds it) for each
//! backend that `-b` names, or Kotlin's defaults that tempest has
//! (`csharp`, `java`, `js`, `lua`, `py`, and `rust`), writing
//! `temper.out/BACKEND/LIBRARY/…` laid out as Kotlin Temper's `temper
//! build` does, with each backend's runtime beside the libraries
//! ([`temper_driver::build`]). What it doesn't cover yet, it says, per library, and fails.
//!
//! **Errors** don't stop any command, as in Kotlin Temper: every error is
//! reported, and each fails only where it runs (tmpl's `Error`), so
//! `build` and the commands for one backend still write every backend's
//! code, and `test` still runs the tests, on the interpreter and on each
//! backend, where the code with errors fails only the tests that reach
//! it. Each exits with 1 when there were errors, whatever else happened,
//! as Kotlin's `temper build` and `temper test` do.
//! Each backend uses a library's host code for its `@connected` functions
//! and classes, or their Temper implementations where they don't list it,
//! as Kotlin's does; `--no-connected` uses Temper implementations wherever
//! there are any ([`temper_tmpl::host`]).
//!
//! `py`, `rust`, `js`, `java`, `csharp`, `cpp`, and `lua` build FILEs as
//! one library, named after the first file (`fibonacci` for
//! `fibonacci.temper.md`), translate it to tmpl and then with that
//! backend's first, Kotlin-compatible variant (its crate's `BACKEND`), as
//! `build` does, and print each generated file after a line marking its
//! path in the language's line comment (`# ==> PATH`), or with `--out`,
//! write them under DIR, laid out as Kotlin Temper's `temper.out/LANG`,
//! without the runtime, which `build` copies beside them. Written, JS's
//! module has a source map beside it, as Kotlin's has, naming the
//! sources by their paths relative to the package.
//!
//! `lsp` serves an editor over stdio as a language server ([`temper_lsp`]):
//! diagnostics as you type, hover, go to definition, completion, find
//! references, rename, document symbols, and signature help, over the
//! library around each open file.
//!
//! Each backend is a cargo feature, all on by default ([`temper_driver`]); one
//! left out says so and fails.
//!
//! Exits with 0 on success, 1 on failure, and 2 on a usage error.

use std::cell::OnceCell;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::OnceLock;
use temper_driver::{Registry, build, test_run};
use temper_frontend::library::Library;
use temper_frontend::library_tests::{Tally, test_dir};
use temper_frontend::project::{load_imports, source_kind, work_root};
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_tmpl::Backend;

const USAGE: &str =
    "usage: tempest build [-w DIR] [-b BACKEND]... [--no-connected] [--verbose] [--std DIR]
       tempest test [-b interp|BACKEND] [--verbose] [--std DIR] [DIR]
       tempest lsp [--std DIR]
       tempest py [--out DIR] [--std DIR] FILE...
       tempest rust [--out DIR] [--std DIR] FILE...
       tempest js [--out DIR] [--std DIR] FILE...
       tempest java [--out DIR] [--std DIR] FILE...
       tempest csharp [--out DIR] [--std DIR] FILE...
       tempest cpp [--out DIR] [--std DIR] FILE...
       tempest lua [--out DIR] [--std DIR] FILE...";

fn main() -> ExitCode {
    // Calls and lowering recurse; give them room (see `interp`).
    let thread = std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let args: Vec<String> = std::env::args().skip(1).collect();
        ExitCode::from(run(&args))
    });
    thread
        .and_then(|t| t.join().map_err(|_| io::Error::other("panicked")))
        .unwrap_or(ExitCode::FAILURE)
}

/// Runs a command, `args` not including the program's name, and returns
/// the exit code.
fn run(args: &[String]) -> u8 {
    match args.first().map(String::as_str) {
        Some("test") => test(&args[1..]),
        Some("build") => build_command(&args[1..]),
        Some("lsp") => lsp(&args[1..]),
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            0
        }
        Some(name) => match named(name) {
            Ok(backend) => generate(&args[1..], backend),
            Err(1) => 1,
            Err(_) => usage(),
        },
        None => usage(),
    }
}

/// `tempest test [-b interp|BACKEND] [--verbose] [--std DIR] [DIR]`.
fn test(args: &[String]) -> u8 {
    let (mut std_dir, mut dir, mut verbose) = (None, None, false);
    let mut backend = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--verbose" | "-v" => verbose = true,
            "--std" => match args.next() {
                Some(d) => std_dir = Some(d.clone()),
                None => return usage(),
            },
            // The interpreter, as Kotlin's `-b interp`, or a backend.
            "-b" | "--backend" => match args.next().map(String::as_str) {
                Some("interp") => backend = None,
                Some(name) => match named(name) {
                    Ok(b) => backend = Some(b),
                    Err(1) => return 1,
                    Err(_) => return usage(),
                },
                None => return usage(),
            },
            _ if arg.starts_with('-') || dir.is_some() => return usage(),
            _ => dir = Some(arg.clone()),
        }
    }
    let sources = match std_dir {
        Some(std_dir) => Sources::with_std_dir(&std_dir),
        None => Sources::standard(),
    };
    let dir = dir.as_deref().unwrap_or(".");
    if let Some(backend) = backend {
        return test_on(&sources, dir, backend, verbose);
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    let passed = test_dir(&sources, dir, verbose, &mut out, &mut io::stderr());
    let _ = out.flush();
    if passed { 0 } else { 1 }
}

/// `tempest test -b BACKEND`: builds the libraries under DIR's work root
/// for the backend, as `build` does, saying what each needs that isn't
/// covered yet, then runs the tests of those that built with the
/// language's own tools ([`test_run`]), printing failures (or with
/// `--verbose`, every result) and the summary, as `-b interp` does.
fn test_on(sources: &Sources, dir: &str, backend: &'static Backend, verbose: bool) -> u8 {
    let root = work_root(sources, dir);
    let mut out = io::BufWriter::new(io::stdout().lock());
    let built = build::build(
        sources,
        &root,
        &[backend],
        build::Options::default(),
        &mut out,
        &mut io::stderr(),
    );
    let Some(built) = built else {
        let _ = out.flush();
        return 1;
    };
    // Libraries with errors still built, and their tests run, as on the
    // interpreter and in Kotlin's `temper test`, but the run fails.
    let failed: Vec<&str> = (built.results.iter())
        .filter(|(_, _, r)| r.is_err())
        .map(|(l, _, _)| l.as_str())
        .collect();
    let tested: Vec<test_run::Tested> = (built.tests.iter())
        .filter(|(name, _)| !failed.contains(&name.as_str()))
        .map(|(name, tests)| test_run::Tested {
            name: name.clone(),
            tests: *tests,
        })
        .collect();
    let dir = Path::new(&root).join("temper.out").join(backend.name);
    let mut tally = Tally::default();
    let mut ok = failed.is_empty() && built.errors.is_empty();
    // Tests of libraries that didn't build didn't run.
    for (name, tests) in &built.tests {
        if failed.contains(&name.as_str()) {
            tally.defined += tests;
            tally.not_run += tests;
        }
    }
    for (name, result) in test_run::run(backend, &dir, &tested) {
        match result {
            Ok(tests) => {
                let failing = tests.results.iter().any(|(_, f)| f.is_some());
                if verbose || failing {
                    let _ = writeln!(out, "Testing {name} on {}", backend.name);
                    for (test, failure) in &tests.results {
                        let _ = match failure {
                            None if verbose => writeln!(out, "  ok      {test}"),
                            None => Ok(()),
                            // Its first line, as a stack trace may follow.
                            Some(cause) => {
                                let cause = cause.lines().next().unwrap_or("");
                                writeln!(out, "  failed  {test} - {cause}")
                            }
                        };
                    }
                }
                tally.add(&tests);
            }
            Err(why) => {
                ok = false;
                let _ = writeln!(out, "{name}: {}: {why}", backend.name);
                let defined = (tested.iter())
                    .find(|t| t.name == name)
                    .map_or(0, |t| t.tests);
                tally.defined += defined;
                tally.not_run += defined;
            }
        }
    }
    let _ = writeln!(out, "{}", tally.summary());
    let _ = out.flush();
    if ok && tally.all_passed() { 0 } else { 1 }
}

/// `tempest lsp [--std DIR]`: a language server on stdin and stdout.
fn lsp(args: &[String]) -> u8 {
    let std_dir = match args {
        [] => None,
        [flag, dir] if flag == "--std" => Some(dir.clone()),
        _ => return usage(),
    };
    temper_lsp::serve(io::stdin(), io::stdout().lock(), std_dir)
}

/// `tempest build [-w DIR] [-b BACKEND]... [--no-connected] [--verbose]
/// [--std DIR]`.
fn build_command(args: &[String]) -> u8 {
    let (mut std_dir, mut root, mut verbose) = (None, None, false);
    let mut backends: Vec<&Backend> = Vec::new();
    let mut no_connected = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--verbose" | "-v" => verbose = true,
            "--no-connected" => no_connected = true,
            "--std" | "-w" | "--workroot" | "-b" | "--backend" => {
                let Some(value) = args.next() else {
                    return usage();
                };
                match arg.as_str() {
                    "--std" => std_dir = Some(value.clone()),
                    "-w" | "--workroot" => root = Some(value.clone()),
                    _ => match named(value) {
                        Ok(b) if backends.iter().any(|c| c.name == b.name) => {}
                        Ok(b) => backends.push(b),
                        Err(1) => return 1,
                        Err(code) => {
                            eprintln!("tempest: no backend {value}");
                            return code;
                        }
                    },
                }
            }
            _ => return usage(),
        }
    }
    if backends.is_empty() {
        backends = registry().defaults();
    }
    let sources = match std_dir {
        Some(std_dir) => Sources::with_std_dir(&std_dir),
        None => Sources::standard(),
    };
    let root = root.unwrap_or_else(|| work_root(&sources, "."));
    let mut out = io::BufWriter::new(io::stdout().lock());
    let options = build::Options {
        verbose,
        no_connected,
    };
    let built = build::build(
        &sources,
        &root,
        &backends,
        options,
        &mut out,
        &mut io::stderr(),
    );
    let _ = out.flush();
    match built {
        Some(built) if built.ok() => 0,
        _ => 1,
    }
}

/// The backends this run has, built in.
fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::built_in)
}

/// A backend this build has, by name, or else the exit code: 1 for one
/// it was built without, saying so, or 2 for a name that isn't one.
fn named(name: &str) -> Result<&'static Backend, u8> {
    match registry().get(name) {
        Some(backend) => Ok(backend),
        None if registry().lacks(name) => {
            eprintln!("tempest: built without the {name} backend");
            Err(1)
        }
        None => Err(2),
    }
}

/// `tempest py|rust|js|java|csharp|cpp|lua [--out DIR] [--std DIR] FILE...`.
fn generate(args: &[String], backend: &Backend) -> u8 {
    let (mut std_dir, mut out_dir, mut files) = (None, None, Vec::new());
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--std" => match args.next() {
                Some(d) => std_dir = Some(d.clone()),
                None => return usage(),
            },
            "--out" => match args.next() {
                Some(d) => out_dir = Some(d.clone()),
                None => return usage(),
            },
            _ if arg.starts_with('-') => return usage(),
            _ => files.push(arg.clone()),
        }
    }
    let Some(first) = files.first() else {
        return usage();
    };
    let sources = match std_dir {
        Some(std_dir) => Sources::with_std_dir(&std_dir),
        None => Sources::standard(),
    };
    let mut library = Library::new();
    for path in &files {
        let text = match sources.read(path) {
            Ok(text) => text.into_owned(),
            Err(err) => {
                eprintln!("{path}: {err}");
                return 1;
            }
        };
        library.add_file(path, &text, source_kind(path));
    }
    load_imports(&sources, &mut library);
    let settled = settle(library);
    // Every diagnostic, then the code anyway, each error failing where it
    // runs, as `build` does; errors make the exit status 1.
    let mut diagnostics = String::new();
    temper_frontend::library_tests::write_located_diagnostics(&settled, &mut diagnostics);
    eprint!("{diagnostics}");
    let errors = (settled.library.diagnostics.iter())
        .filter(|d| d.level == temper_frontend::library::Level::Error)
        .count();
    let file = first.rsplit(['/', '\\']).next().unwrap_or(first);
    let name = (file.strip_suffix(".temper.md"))
        .or_else(|| file.strip_suffix(".temper"))
        .unwrap_or(file);
    // std, settled only if the library uses it.
    let std = OnceCell::new();
    let std = || {
        (std.get_or_init(|| temper_frontend::stdlib::program(&sources))
            .as_ref())
        .map_err(|e| format!("std: {e}"))
    };
    let program = temper_driver::checked(&settled, name)
        .and_then(|p| temper_driver::with_std(p, backend, std));
    let program = match program {
        Ok(program) => program,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    // The sources' directory, for source maps.
    let source_dir = Path::new(first)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| ".".into());
    // Written files get source maps, which lead back to the sources.
    let maps = (out_dir.as_deref()).map(|d| {
        temper_driver::sources_prefix(
            &Path::new(d).join(&program.library.name),
            &format!("{source_dir}/x"),
        )
    });
    let generated = match temper_driver::translate(&program, backend, maps.as_deref()) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    match out_dir {
        Some(dir) => {
            for (file, text) in &generated {
                let path = Path::new(&dir).join(file);
                let written = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(&path, text));
                if let Err(err) = written {
                    eprintln!("{}: {err}", path.display());
                    return 1;
                }
            }
        }
        None => {
            let mut out = io::BufWriter::new(io::stdout().lock());
            for (file, text) in &generated {
                let _ = writeln!(out, "{} ==> {file}", backend.comment);
                let _ = out.write_all(text.as_bytes());
            }
            let _ = out.flush();
        }
    }
    if errors > 0 {
        eprintln!("{}", build::error_count(errors));
        return 1;
    }
    0
}

fn usage() -> u8 {
    eprintln!("{USAGE}");
    2
}
