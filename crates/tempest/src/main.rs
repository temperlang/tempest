//! The `tempest` command, a thin layer over the other crates, which do
//! the work as libraries.
//!
//!     tempest test [--verbose] [--std DIR] [DIR]
//!     tempest py [--out DIR] [--std DIR] FILE...
//!     tempest rust [--out DIR] [--std DIR] FILE...
//!     tempest js [--out DIR] [--std DIR] FILE...
//!     tempest java [--out DIR] [--std DIR] FILE...
//!     tempest csharp [--out DIR] [--std DIR] FILE...
//!     tempest cpp [--out DIR] [--std DIR] FILE...
//!
//! `test` runs the tests of the libraries under DIR's work root on the
//! interpreter, as Kotlin Temper's `temper test` does
//! ([`temper_frontend::library_tests`]), printing only failures and a
//! summary, or with `--verbose` (`-v`), every test's result and the
//! libraries' output. DIR defaults to the current
//! directory. Std modules are built in ([`temper_frontend::stdlib`]), or
//! come from `--std`'s directory, to test against another copy.
//!
//! `py` builds FILEs as one library, named after the first file
//! (`fibonacci` for `fibonacci.temper.md`), translates it to tmpl
//! ([`temper_frontend::to_tmpl`]) and then to Python with the first,
//! Kotlin-compatible variant ([`temper_be_py::PY`]), and prints each
//! generated file after a `# ==> PATH` line, or with `--out`, writes them
//! under DIR, laid out as Kotlin Temper's `temper.out/py`. What it doesn't
//! cover yet, it says, and fails.
//!
//! `rust` does the same with the Rust backend's first, Kotlin-compatible
//! variant ([`temper_be_rust::RUST`]), printing each file after a
//! `// ==> PATH` line, or writing a crate under DIR laid out as Kotlin
//! Temper's `temper.out/rust`, which depends on `temper-core` beside it
//! (Kotlin copies it there; this doesn't yet).
//!
//! `js` does the same with the JS backend's first, Kotlin-compatible
//! variant ([`temper_be_js::JS`]), printing each file after a `// ==> PATH`
//! line, or writing an npm package under DIR laid out as Kotlin Temper's
//! `temper.out/js`, which depends on `@temperlang/core` (Kotlin copies it
//! beside the package; this doesn't yet). Written, the module has a source
//! map beside it, as Kotlin's has, which names the sources by their paths
//! relative to the package.
//!
//! `java` does the same with the Java backend's first, Kotlin-compatible
//! variant ([`temper_be_java::JAVA`]), printing each file after a
//! `// ==> PATH` line, or writing a Maven package under DIR laid out as
//! Kotlin Temper's `temper.out/java`, which depends on `temper-core`
//! (Kotlin copies it beside the package; this doesn't yet).
//!
//! `csharp` does the same with the C# backend's first, Kotlin-compatible
//! variant ([`temper_be_csharp::CSHARP`]), writing a library's projects
//! laid out as Kotlin Temper's `temper.out/csharp`, which refer to
//! `TemperLang.Core`'s project beside them (Kotlin copies it there; this
//! doesn't yet).
//!
//! `cpp` does the same with the C++ backend's first, Kotlin-compatible
//! variant ([`temper_be_cpp::CPP`]), writing a library's `init.hpp`,
//! `init.cpp`, and `main.cpp` laid out as Kotlin Temper's `temper.out/cpp`,
//! which include `<temper-core/core.hpp>` from the directory above them
//! (Kotlin copies it there; this doesn't yet).
//!
//! Each backend is a cargo feature (`py`, `rust`, `js`, `java`, `csharp`,
//! and `cpp`), all on by default; one left out says so and fails.
//!
//! Exits with 0 on success, 1 on failure, and 2 on a usage error.

use std::io::{self, Write};
use std::process::ExitCode;
use temper_frontend::library_tests::test_dir;
use temper_frontend::sources::Sources;
#[cfg(any(
    feature = "py",
    feature = "rust",
    feature = "js",
    feature = "java",
    feature = "csharp",
    feature = "cpp"
))]
use {
    std::path::Path,
    temper_frontend::{library::Library, project::load_imports, settle::settle, to_tmpl::to_tmpl},
    temper_syntax::lex::SourceKind,
    temper_tmpl::{Profile, validate},
};

const USAGE: &str = "usage: tempest test [--verbose] [--std DIR] [DIR]
       tempest py [--out DIR] [--std DIR] FILE...
       tempest rust [--out DIR] [--std DIR] FILE...
       tempest js [--out DIR] [--std DIR] FILE...
       tempest java [--out DIR] [--std DIR] FILE...
       tempest csharp [--out DIR] [--std DIR] FILE...
       tempest cpp [--out DIR] [--std DIR] FILE...";

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
        #[cfg(feature = "py")]
        Some("py") => generate(&args[1..], Backend::Py),
        #[cfg(feature = "rust")]
        Some("rust") => generate(&args[1..], Backend::Rust),
        #[cfg(feature = "js")]
        Some("js") => generate(&args[1..], Backend::Js),
        #[cfg(feature = "java")]
        Some("java") => generate(&args[1..], Backend::Java),
        #[cfg(feature = "csharp")]
        Some("csharp") => generate(&args[1..], Backend::CSharp),
        #[cfg(feature = "cpp")]
        Some("cpp") => generate(&args[1..], Backend::Cpp),
        #[allow(unreachable_patterns)] // Unless a backend is left out.
        Some(backend @ ("py" | "rust" | "js" | "java" | "csharp" | "cpp")) => {
            eprintln!("tempest: built without the {backend} backend");
            1
        }
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            0
        }
        _ => usage(),
    }
}

/// `tempest test [--verbose] [--std DIR] [DIR]`.
fn test(args: &[String]) -> u8 {
    let (mut std_dir, mut dir, mut verbose) = (None, None, false);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--verbose" | "-v" => verbose = true,
            "--std" => match args.next() {
                Some(d) => std_dir = Some(d.clone()),
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
    let mut out = io::BufWriter::new(io::stdout().lock());
    let passed = test_dir(&sources, dir, verbose, &mut out, &mut io::stderr());
    let _ = out.flush();
    if passed { 0 } else { 1 }
}

/// A backend `generate` translates to.
#[cfg(any(
    feature = "py",
    feature = "rust",
    feature = "js",
    feature = "java",
    feature = "csharp",
    feature = "cpp"
))]
#[derive(Clone, Copy)]
enum Backend {
    #[cfg(feature = "py")]
    Py,
    #[cfg(feature = "rust")]
    Rust,
    #[cfg(feature = "js")]
    Js,
    #[cfg(feature = "java")]
    Java,
    #[cfg(feature = "csharp")]
    CSharp,
    #[cfg(feature = "cpp")]
    Cpp,
}

/// `tempest py|rust|js|java|csharp|cpp [--out DIR] [--std DIR] FILE...`.
#[cfg(any(
    feature = "py",
    feature = "rust",
    feature = "js",
    feature = "java",
    feature = "csharp",
    feature = "cpp"
))]
fn generate(args: &[String], backend: Backend) -> u8 {
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
        let kind = if path.ends_with(".md") {
            SourceKind::Markdown
        } else {
            SourceKind::Standalone
        };
        library.add_file(path, &text, kind);
    }
    load_imports(&sources, &mut library);
    let settled = settle(library);
    let file = first.rsplit(['/', '\\']).next().unwrap_or(first);
    let name = (file.strip_suffix(".temper.md"))
        .or_else(|| file.strip_suffix(".temper"))
        .unwrap_or(file);
    let program = match to_tmpl(&settled, name) {
        Ok(program) => program,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let problems = validate(&program, &Profile::full());
    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("tmpl: {problem}");
        }
        return 1;
    }
    // Each file's path and text.
    let generated: Result<Vec<(String, String)>, String> = match backend {
        #[cfg(feature = "py")]
        Backend::Py => temper_be_py::translate(&program, &temper_be_py::PY)
            .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
        #[cfg(feature = "rust")]
        Backend::Rust => temper_be_rust::translate(&program, &temper_be_rust::RUST)
            .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
        #[cfg(feature = "js")]
        Backend::Js => match &out_dir {
            // Written files get source maps, which lead back to the
            // sources' directory.
            Some(dir) => {
                let prefix = sources_prefix(&Path::new(dir).join(name), first);
                temper_be_js::translate_with_source_maps(&program, &temper_be_js::JS, &prefix)
            }
            None => temper_be_js::translate(&program, &temper_be_js::JS),
        }
        .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
        .map_err(|e| e.to_string()),
        #[cfg(feature = "java")]
        Backend::Java => temper_be_java::translate(&program, &temper_be_java::JAVA)
            .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
        #[cfg(feature = "csharp")]
        Backend::CSharp => temper_be_csharp::translate(&program, &temper_be_csharp::CSHARP)
            .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
        #[cfg(feature = "cpp")]
        Backend::Cpp => temper_be_cpp::translate(&program, &temper_be_cpp::CPP)
            .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
    };
    let generated = match generated {
        Ok(files) => files,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let marker = match backend {
        #[cfg(feature = "py")]
        Backend::Py => "#",
        #[cfg(any(
            feature = "rust",
            feature = "js",
            feature = "java",
            feature = "csharp",
            feature = "cpp"
        ))]
        _ => "//",
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
                let _ = writeln!(out, "{marker} ==> {file}");
                let _ = out.write_all(text.as_bytes());
            }
            let _ = out.flush();
        }
    }
    0
}

/// The relative path, with `/` after each part, from a package's
/// directory to the directory of a library's first file, for a source
/// map's `sources`; or that directory's absolute path where there's none.
#[cfg(feature = "js")]
fn sources_prefix(package: &Path, first: &str) -> String {
    let absolute = |p: &Path| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let normal = |p: &Path| {
        let mut parts = Vec::new();
        for c in absolute(p).components() {
            match c {
                std::path::Component::ParentDir => {
                    parts.pop();
                }
                std::path::Component::CurDir => {}
                c => parts.push(c.as_os_str().to_string_lossy().into_owned()),
            }
        }
        parts
    };
    let source = Path::new(first).parent().unwrap_or(Path::new(""));
    let (from, to) = (normal(package), normal(source));
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut prefix = if common == 0 {
        String::new()
    } else {
        "../".repeat(from.len() - common)
    };
    for part in &to[common..] {
        prefix.push_str(part.trim_end_matches(['/', '\\']));
        prefix.push('/');
    }
    prefix
}

fn usage() -> u8 {
    eprintln!("{USAGE}");
    2
}
