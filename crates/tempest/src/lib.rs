//! What the `tempest` command does, as a library too: the one list of
//! backends this build has ([`BACKENDS`]), and one way from a settled
//! library to each backend's files ([`checked`], [`with_std`],
//! [`translate`]), which `tempest build` ([`build`]), the commands for one
//! backend, and the playground's wasm (`temper-wasm`) all take.
//! [`test_run`] runs built libraries' tests with each language's own
//! tools, for `tempest test -b`. [`lsp`] is `tempest lsp`, a language
//! server over [`temper_analysis`].
//!
//! Each backend is a cargo feature (`cpp`, `csharp`, `java`, `js`, `lua`,
//! `py`, and `rust`), all on by default.

pub mod build;
// Native only: the wasm answers the playground's queries itself.
#[cfg(any(unix, windows))]
pub mod lsp;
// Not in the wasm, which runs no tools.
#[cfg(any(unix, windows))]
pub mod test_run;

use std::path::Path;
use temper_frontend::settle::Settled;
use temper_frontend::to_tmpl::to_tmpl;
use temper_tmpl::naming::{Interface, assign};
use temper_tmpl::{Backend, Profile, Program, validate};

/// The backends this build has, alphabetically.
pub const BACKENDS: &[&Backend] = &[
    #[cfg(feature = "cpp")]
    &temper_be_cpp::BACKEND,
    #[cfg(feature = "csharp")]
    &temper_be_csharp::BACKEND,
    #[cfg(feature = "java")]
    &temper_be_java::BACKEND,
    #[cfg(feature = "js")]
    &temper_be_js::BACKEND,
    #[cfg(feature = "lua")]
    &temper_be_lua::BACKEND,
    #[cfg(feature = "py")]
    &temper_be_py::BACKEND,
    #[cfg(feature = "rust")]
    &temper_be_rust::BACKEND,
];

/// Every backend's name, whether or not this build has it, so a name left
/// out can say so.
pub const NAMES: [&str; 7] = ["cpp", "csharp", "java", "js", "lua", "py", "rust"];

/// What `build` translates for without `-b`: Kotlin's defaults, but for
/// `data`, which tempest doesn't have.
pub const DEFAULT: [&str; 6] = ["csharp", "java", "js", "lua", "py", "rust"];

/// A backend this build has, by name.
pub fn backend(name: &str) -> Option<&'static Backend> {
    BACKENDS.iter().copied().find(|b| b.name == name)
}

/// A settled library as tmpl, named `name`, and checked; or why not: what
/// tmpl doesn't cover yet, or the validator's problems, each `tmpl: …` on
/// a line.
pub fn checked(settled: &Settled, name: &str) -> Result<Program, String> {
    let program = to_tmpl(settled, name).map_err(|e| e.to_string())?;
    let problems = validate(&program, &Profile::full());
    if !problems.is_empty() {
        let problems: Vec<String> = problems.iter().map(|p| format!("tmpl: {p}")).collect();
        return Err(problems.join("\n"));
    }
    Ok(program)
}

/// Whether a program uses std.
pub fn uses_std(p: &Program) -> bool {
    (p.uses.iter()).any(|u| p.strings.get(p.decls.get(u.decl).name) == "std")
}

/// What other libraries use of a library, as a backend names it.
pub fn interface(p: &Program, backend: &Backend) -> Interface {
    Interface::of(p, &assign(p, backend.rules))
}

/// A library built alone, with std's interface for a backend where it
/// uses std, so it spells the names std's own naming gives
/// (notes/tmpl/open-questions.md, 46); `std` gives std as tmpl, asked for
/// only then.
pub fn with_std<'s>(
    mut p: Program,
    backend: &Backend,
    std: impl FnOnce() -> Result<&'s Program, String>,
) -> Result<Program, String> {
    if uses_std(&p) {
        p.interfaces = vec![interface(std()?, backend)];
    }
    Ok(p)
}

/// A program's files from a backend, each a path and its text, or what
/// isn't covered yet. With `maps`, a prefix that leads from the
/// library's output directory to its sources, a backend that writes
/// source maps writes them.
pub fn translate(
    p: &Program,
    backend: &Backend,
    maps: Option<&str>,
) -> Result<Vec<(String, String)>, String> {
    let files = (backend.translate)(p, maps)?;
    Ok(files.into_iter().map(|f| (f.path, f.text)).collect())
}

/// The relative path, with `/` after each part, from a package's
/// directory to the directory of a library's first file, for a source
/// map's `sources`; or that directory's absolute path where there's none.
pub fn sources_prefix(package: &Path, first: &str) -> String {
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
