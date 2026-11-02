//! The `tempest` command, a thin layer over the other crates, which do
//! the work as libraries.
//!
//!     tempest build [-w DIR] [-b BACKEND]... [--no-connected] [--verbose] [--std DIR]
//!     tempest test [-b interp|BACKEND] [--verbose] [--std DIR] [DIR]
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
//! as `build` does, saying what each needs that tmpl doesn't translate
//! yet; running their tests waits for std's `testing` in tmpl. Std modules are built in ([`temper_frontend::stdlib`]), or
//! come from `--std`'s directory, to test against another copy.
//!
//! `build` translates every library under the work root (DIR with `-w`,
//! or else found from the current directory as `test` finds it) for each
//! backend that `-b` names, or Kotlin's defaults that tempest has
//! (`csharp`, `java`, `js`, `lua`, `py`, and `rust`), writing
//! `temper.out/BACKEND/LIBRARY/…` laid out as Kotlin Temper's `temper
//! build` does, with each backend's runtime beside the libraries
//! ([`build`]). What it doesn't cover yet, it says, per library, and fails.
//! Each backend uses a library's host code for its `@connected` functions
//! and classes, or their Temper implementations where they don't list it,
//! as Kotlin's does; `--no-connected` uses Temper implementations wherever
//! there are any ([`temper_tmpl::host`]).
//!
//! `py` builds FILEs as one library, named after the first file
//! (`fibonacci` for `fibonacci.temper.md`), translates it to tmpl
//! ([`temper_frontend::to_tmpl`]) and then to Python with the first,
//! Kotlin-compatible variant ([`temper_be_py::PY`]), and prints each
//! generated file after a `# ==> PATH` line, or with `--out`, writes them
//! under DIR, laid out as Kotlin Temper's `temper.out/py` (without the
//! runtime, which `build` copies). What it doesn't cover yet, it says, and
//! fails.
//!
//! `rust` does the same with the Rust backend's first, Kotlin-compatible
//! variant ([`temper_be_rust::RUST`]), printing each file after a
//! `// ==> PATH` line, or writing a crate under DIR laid out as Kotlin
//! Temper's `temper.out/rust`, which depends on `temper-core` beside it
//! (Kotlin copies it there; `build` does).
//!
//! `js` does the same with the JS backend's first, Kotlin-compatible
//! variant ([`temper_be_js::JS`]), printing each file after a `// ==> PATH`
//! line, or writing an npm package under DIR laid out as Kotlin Temper's
//! `temper.out/js`, which depends on `@temperlang/core` (Kotlin copies it
//! beside the package; `build` does). Written, the module has a source
//! map beside it, as Kotlin's has, which names the sources by their paths
//! relative to the package.
//!
//! `java` does the same with the Java backend's first, Kotlin-compatible
//! variant ([`temper_be_java::JAVA`]), printing each file after a
//! `// ==> PATH` line, or writing a Maven package under DIR laid out as
//! Kotlin Temper's `temper.out/java`, which depends on `temper-core`
//! (Kotlin copies it beside the package; `build` does).
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
//! (Kotlin copies it there; `build` does).
//!
//! `lua` does the same with the Lua backend's first, Kotlin-compatible
//! variant ([`temper_be_lua::LUA`]), printing each file after a
//! `-- ==> PATH` line, or writing a library's `init.lua` and rockspec laid
//! out as Kotlin Temper's `temper.out/lua`, which requires `temper-core`
//! from beside it, with Kotlin's `LUA_PATH` of `?.lua;?/init.lua` (Kotlin
//! copies it there; `build` does).
//!
//! Each backend is a cargo feature (`py`, `rust`, `js`, `java`, `csharp`,
//! `cpp`, and `lua`), all on by default; one left out says so and fails.
//!
//! Exits with 0 on success, 1 on failure, and 2 on a usage error.

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use temper_frontend::library::Library;
use temper_frontend::library_tests::test_dir;
use temper_frontend::project::{load_imports, source_kind, work_root};
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::to_tmpl;
use temper_tmpl::{Profile, Program, validate};

mod build;

const USAGE: &str =
    "usage: tempest build [-w DIR] [-b BACKEND]... [--no-connected] [--verbose] [--std DIR]
       tempest test [-b interp|BACKEND] [--verbose] [--std DIR] [DIR]
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
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            0
        }
        Some(name) => match Backend::named(name) {
            Some(backend) if backend.built_in() => generate(&args[1..], backend),
            Some(_) => {
                eprintln!("tempest: built without the {name} backend");
                1
            }
            None => usage(),
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
                Some(name) => match Backend::named(name) {
                    Some(b) if b.built_in() => backend = Some(b),
                    Some(_) => {
                        eprintln!("tempest: built without the {name} backend");
                        return 1;
                    }
                    None => return usage(),
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
        return test_on(&sources, dir, backend);
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    let passed = test_dir(&sources, dir, verbose, &mut out, &mut io::stderr());
    let _ = out.flush();
    if passed { 0 } else { 1 }
}

/// `tempest test -b BACKEND`: builds the libraries under DIR's work root
/// for the backend, as `build` does, saying what each needs that tmpl
/// doesn't translate yet. Running their tests waits for std's `testing`
/// module, whose `Test` each test takes, which tmpl doesn't translate yet
/// (notes/test-speed.md, "`tempest test -b`").
fn test_on(sources: &Sources, dir: &str, backend: Backend) -> u8 {
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
    let _ = out.flush();
    let Some(built) = built else { return 1 };
    let failed = built.results.iter().filter(|(_, _, r)| r.is_err()).count();
    if failed == 0 {
        eprintln!(
            "tempest: running tests on the {} backend needs std's testing, \
             which tmpl doesn't translate yet",
            backend.name()
        );
    } else {
        eprintln!(
            "tempest: {failed} libraries didn't build for {}",
            backend.name()
        );
    }
    1
}

/// `tempest build [-w DIR] [-b BACKEND]... [--no-connected] [--verbose]
/// [--std DIR]`.
fn build_command(args: &[String]) -> u8 {
    let (mut std_dir, mut root, mut verbose, mut backends) = (None, None, false, Vec::new());
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
                    _ => match Backend::named(value) {
                        Some(b) if b.built_in() => {
                            if !backends.contains(&b) {
                                backends.push(b);
                            }
                        }
                        Some(_) => {
                            eprintln!("tempest: built without the {value} backend");
                            return 1;
                        }
                        None => {
                            eprintln!("tempest: no backend {value}");
                            return 2;
                        }
                    },
                }
            }
            _ => return usage(),
        }
    }
    if backends.is_empty() {
        backends = Backend::DEFAULT
            .iter()
            .copied()
            .filter(|b| b.built_in())
            .collect();
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
        Some(built) if built.results.iter().all(|(_, _, r)| r.is_ok()) => 0,
        _ => 1,
    }
}

/// A backend, whether or not this build has it (a cargo feature).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Cpp,
    CSharp,
    Java,
    Js,
    Lua,
    Py,
    Rust,
}

impl Backend {
    /// Every backend, by name, alphabetically.
    const ALL: [Backend; 7] = [
        Backend::Cpp,
        Backend::CSharp,
        Backend::Java,
        Backend::Js,
        Backend::Lua,
        Backend::Py,
        Backend::Rust,
    ];

    /// What `build` translates for without `-b`: Kotlin's defaults, but
    /// for `data`, which tempest doesn't have.
    const DEFAULT: [Backend; 6] = [
        Backend::CSharp,
        Backend::Java,
        Backend::Js,
        Backend::Lua,
        Backend::Py,
        Backend::Rust,
    ];

    fn named(name: &str) -> Option<Backend> {
        Backend::ALL.into_iter().find(|b| b.name() == name)
    }

    /// Its name on the command line and in `temper.out`, as Kotlin's.
    pub fn name(self) -> &'static str {
        match self {
            Backend::Cpp => "cpp",
            Backend::CSharp => "csharp",
            Backend::Java => "java",
            Backend::Js => "js",
            Backend::Lua => "lua",
            Backend::Py => "py",
            Backend::Rust => "rust",
        }
    }

    /// Whether this build has it.
    fn built_in(self) -> bool {
        match self {
            Backend::Cpp => cfg!(feature = "cpp"),
            Backend::CSharp => cfg!(feature = "csharp"),
            Backend::Java => cfg!(feature = "java"),
            Backend::Js => cfg!(feature = "js"),
            Backend::Lua => cfg!(feature = "lua"),
            Backend::Py => cfg!(feature = "py"),
            Backend::Rust => cfg!(feature = "rust"),
        }
    }

    /// The line comment that marks each printed file.
    fn marker(self) -> &'static str {
        match self {
            Backend::Py => "#",
            Backend::Lua => "--",
            _ => "//",
        }
    }

    /// Its runtime's files, as `temper.out` has them beside the
    /// libraries; none if this build doesn't have it.
    pub fn runtime(self) -> &'static [(&'static str, &'static str)] {
        match self {
            #[cfg(feature = "cpp")]
            Backend::Cpp => temper_be_cpp::runtime::FILES,
            #[cfg(feature = "csharp")]
            Backend::CSharp => temper_be_csharp::runtime::FILES,
            #[cfg(feature = "java")]
            Backend::Java => temper_be_java::runtime::FILES,
            #[cfg(feature = "js")]
            Backend::Js => temper_be_js::runtime::FILES,
            #[cfg(feature = "lua")]
            Backend::Lua => temper_be_lua::runtime::FILES,
            #[cfg(feature = "py")]
            Backend::Py => temper_be_py::runtime::FILES,
            #[cfg(feature = "rust")]
            Backend::Rust => temper_be_rust::runtime::FILES,
            #[allow(unreachable_patterns)] // Unless a backend is left out.
            _ => &[],
        }
    }

    /// A program's files, each a path and its text. Written ones, under
    /// `dir`, from sources in `root`, have source maps where the backend
    /// makes them (JS), naming the sources by their paths from there.
    pub fn translate(
        self,
        program: &Program,
        written: Option<(&Path, &str)>,
    ) -> Result<Vec<(String, String)>, String> {
        // Used by no arm when every backend is left out.
        let _ = (program, written);
        match self {
            #[cfg(feature = "py")]
            Backend::Py => temper_be_py::translate(program, &temper_be_py::PY)
                .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
                .map_err(|e| e.to_string()),
            #[cfg(feature = "rust")]
            Backend::Rust => temper_be_rust::translate(program, &temper_be_rust::RUST)
                .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
                .map_err(|e| e.to_string()),
            #[cfg(feature = "js")]
            Backend::Js => match written {
                // Written files get source maps, which lead back to the
                // sources' directory.
                Some((dir, root)) => {
                    let package = dir.join(&program.library.name);
                    let prefix = sources_prefix(&package, &format!("{root}/x"));
                    temper_be_js::translate_with_source_maps(program, &temper_be_js::JS, &prefix)
                }
                None => temper_be_js::translate(program, &temper_be_js::JS),
            }
            .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "java")]
            Backend::Java => temper_be_java::translate(program, &temper_be_java::JAVA)
                .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
                .map_err(|e| e.to_string()),
            #[cfg(feature = "csharp")]
            Backend::CSharp => temper_be_csharp::translate(program, &temper_be_csharp::CSHARP)
                .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
                .map_err(|e| e.to_string()),
            #[cfg(feature = "cpp")]
            Backend::Cpp => temper_be_cpp::translate(program, &temper_be_cpp::CPP)
                .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
                .map_err(|e| e.to_string()),
            #[cfg(feature = "lua")]
            Backend::Lua => temper_be_lua::translate(program, &temper_be_lua::LUA)
                .map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
                .map_err(|e| e.to_string()),
            #[allow(unreachable_patterns)] // Unless a backend is left out.
            _ => Err(format!("built without the {} backend", self.name())),
        }
    }
}

/// `tempest py|rust|js|java|csharp|cpp|lua [--out DIR] [--std DIR] FILE...`.
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
        library.add_file(path, &text, source_kind(path));
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
    // The sources' directory, for source maps.
    let source_dir = Path::new(first)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| ".".into());
    let written = out_dir
        .as_deref()
        .map(|d| (Path::new(d), source_dir.as_str()));
    let generated = match backend.translate(&program, written) {
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
                let _ = writeln!(out, "{} ==> {file}", backend.marker());
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
#[cfg_attr(not(feature = "js"), allow(dead_code))]
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
