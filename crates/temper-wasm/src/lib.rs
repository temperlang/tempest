//! A wasm entry point for Temper, for a browser playground and for measuring
//! how big Temper is as wasm (`compat/measure_size.py`).
//!
//! For now it parses source and returns the parse tree and the grammar
//! step's tree as text (`parse`), or settles and runs it, with the std
//! modules it imports, built in, and returns its diagnostics and what it
//! logs (`run`), or translates it to Python, Rust, JavaScript, Java, C#,
//! C++, or Lua (`generate`), returning the files as JSON. JavaScript calls `alloc` for
//! a buffer, writes UTF-8 source into it, calls `parse`, `run`, or
//! `generate`, and reads the result from `output_ptr` and `output_len`:
//!
//! ```js
//! const bytes = new TextEncoder().encode(source);
//! const ptr = exports.alloc(bytes.length);
//! new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
//! exports.parse(ptr, bytes.length, markdown ? 1 : 0);
//! const out = new Uint8Array(exports.memory.buffer, exports.output_ptr(), exports.output_len());
//! console.log(new TextDecoder().decode(out));
//! ```
//!
//! Each backend is a cargo feature (`py`, `rust`, `js`, `java`, `csharp`,
//! `cpp`, and `lua`), all on by default, as in `tempest`.

use std::cell::RefCell;
use temper_frontend::project::load_imports;
use temper_frontend::sources::Sources;
use temper_frontend::{interp, library::Library, print::write_diagnostics, settle::settle};
use temper_syntax::ast::build;
use temper_syntax::{
    lex::SourceKind, parse::parse as parse_source, print::write_ast, print::write_tree,
};

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// A buffer of `len` bytes for the next input, replacing the last one.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    INPUT.with_borrow_mut(|input| {
        *input = vec![0; len];
        input.as_mut_ptr()
    })
}

/// Parses the input buffer as UTF-8 Temper source, or literate markdown if
/// `markdown` is nonzero, and keeps the parse tree and then the grammar
/// step's tree as text for `output_ptr`, with a blank line between.
/// Invalid UTF-8 is replaced rather than rejected.
#[unsafe(no_mangle)]
pub extern "C" fn parse(ptr: *const u8, len: usize, markdown: u32) {
    with_source(ptr, len, markdown, |source, kind, out| {
        let parsed = parse_source(source, kind);
        write_tree(&parsed, 100, out);
        out.push('\n');
        write_ast(&build(&parsed), 100, out);
    });
}

/// Settles the input buffer as a library, with the std modules it
/// imports, and runs it, keeping its diagnostics, then what it logs, then
/// how it failed, if it did. The
/// interpreter's recursion needs more than wasm's default 1 MiB stack for
/// deeply nested code.
#[unsafe(no_mangle)]
pub extern "C" fn run(ptr: *const u8, len: usize, markdown: u32) {
    with_source(ptr, len, markdown, |source, kind, out| {
        let mut library = Library::new();
        library.add_file("main.temper", source, kind);
        load_imports(&Sources::memory(&[]), &mut library);
        let settled = settle(library);
        write_diagnostics(&settled, out);
        let run = interp::run(&settled, Some(interp::FUEL));
        for line in run.output {
            out.push_str(&line);
            out.push('\n');
        }
        if let Some(failure) = run.failure {
            out.push_str(&failure);
            out.push('\n');
        }
    });
}

/// The backends `generate` knows, by number: `0` for Python, `1` for
/// Rust, `2` for JavaScript, `3` for Java, `4` for C#, `5` for C++, and
/// `6` for Lua.
pub const BACKENDS: [&str; 7] = ["py", "rust", "js", "java", "csharp", "cpp", "lua"];

/// Translates the input buffer, as a library named `main` from a file
/// `main.temper` (or `main.temper.md`), with the std modules it imports,
/// by a backend of [`BACKENDS`], as `tempest py`, `rust`, `js`, `java`,
/// `csharp`, `cpp`, or `lua` do, and keeps the generated files as JSON, `{"files": [{"path": ..., "text":
/// ...}, ...]}`, laid out as Kotlin's `temper.out/LANG` under it, or
/// `{"error": ...}` with what went wrong. JavaScript's module comes with
/// its source map, which names the source `../main.temper` (or `.md`).
#[unsafe(no_mangle)]
pub extern "C" fn generate(ptr: *const u8, len: usize, markdown: u32, backend: u32) {
    with_source(ptr, len, markdown, |source, kind, out| {
        let json = match translate(source, kind, backend) {
            Ok(files) => files_json(&files),
            Err(error) => format!("{{\"error\":{}}}", json_string(&error)),
        };
        out.push_str(&json);
    });
}

/// Generated files, each a path and text.
pub type Files = Vec<(String, String)>;

/// What `generate` does, before the JSON.
#[cfg_attr(
    not(any(
        feature = "py",
        feature = "rust",
        feature = "js",
        feature = "java",
        feature = "csharp",
        feature = "cpp",
        feature = "lua"
    )),
    allow(unused_variables)
)]
pub fn translate(source: &str, kind: SourceKind, backend: u32) -> Result<Files, String> {
    let Some(&name) = BACKENDS.get(backend as usize) else {
        return Err(format!("no backend {backend}"));
    };
    #[cfg(any(
        feature = "py",
        feature = "rust",
        feature = "js",
        feature = "java",
        feature = "csharp",
        feature = "cpp",
        feature = "lua"
    ))]
    return backends::translate(source, kind, name);
    #[cfg(not(any(
        feature = "py",
        feature = "rust",
        feature = "js",
        feature = "java",
        feature = "csharp",
        feature = "cpp",
        feature = "lua"
    )))]
    Err(format!("built without the {name} backend"))
}

#[cfg(any(
    feature = "py",
    feature = "rust",
    feature = "js",
    feature = "java",
    feature = "csharp",
    feature = "cpp",
    feature = "lua"
))]
mod backends {
    use super::Files;
    use temper_frontend::project::load_imports;
    use temper_frontend::sources::Sources;
    use temper_frontend::to_tmpl::to_tmpl;
    use temper_frontend::{library::Library, print::write_diagnostics, settle::settle};
    use temper_syntax::lex::SourceKind;
    use temper_tmpl::{Profile, validate};

    /// The program with std's interface for a backend whose naming
    /// follows `rules`, if it uses std, so it spells the names std's own
    /// naming gives (notes/tmpl/open-questions.md, 46).
    #[allow(dead_code)]
    fn with_std(
        program: &temper_tmpl::Program,
        rules: &temper_tmpl::naming::NamingRules,
    ) -> Result<temper_tmpl::Program, String> {
        let mut program = program.clone();
        let uses_std = (program.uses.iter())
            .any(|u| program.strings.get(program.decls.get(u.decl).name) == "std");
        if uses_std {
            let std = temper_frontend::stdlib::program(&Sources::memory(&[]))
                .map_err(|e| format!("std: {e}"))?;
            let names = temper_tmpl::naming::assign(&std, rules);
            program.interfaces = vec![temper_tmpl::naming::Interface::of(&std, &names)];
        }
        Ok(program)
    }

    /// Settles, translates to tmpl, and generates code with a backend
    /// that's built in.
    pub fn translate(source: &str, kind: SourceKind, backend: &str) -> Result<Files, String> {
        let file = match kind {
            SourceKind::Markdown => "main.temper.md",
            _ => "main.temper",
        };
        let mut library = Library::new();
        library.add_file(file, source, kind);
        load_imports(&Sources::memory(&[]), &mut library);
        let settled = settle(library);
        let program = to_tmpl(&settled, "main").map_err(|e| {
            let mut diagnostics = String::new();
            write_diagnostics(&settled, &mut diagnostics);
            format!("{diagnostics}{e}")
        })?;
        let problems = validate(&program, &Profile::full());
        if !problems.is_empty() {
            let problems: Vec<String> = problems.iter().map(|p| format!("tmpl: {p}")).collect();
            return Err(problems.join("\n"));
        }
        match backend {
            #[cfg(feature = "py")]
            "py" => temper_be_py::translate(
                &with_std(&program, &temper_be_py::PY.rules)?,
                &temper_be_py::PY,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "rust")]
            "rust" => temper_be_rust::translate(
                &with_std(&program, &temper_be_rust::RUST.rules)?,
                &temper_be_rust::RUST,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "js")]
            "js" => temper_be_js::translate_with_source_maps(
                &with_std(&program, &temper_be_js::JS.rules)?,
                &temper_be_js::JS,
                "../",
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "java")]
            "java" => temper_be_java::translate(
                &with_std(&program, &temper_be_java::JAVA.rules)?,
                &temper_be_java::JAVA,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "csharp")]
            "csharp" => temper_be_csharp::translate(
                &with_std(&program, &temper_be_csharp::CSHARP.rules)?,
                &temper_be_csharp::CSHARP,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "cpp")]
            "cpp" => temper_be_cpp::translate(
                &with_std(&program, &temper_be_cpp::CPP.rules)?,
                &temper_be_cpp::CPP,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "lua")]
            "lua" => temper_be_lua::translate(
                &with_std(&program, &temper_be_lua::LUA.rules)?,
                &temper_be_lua::LUA,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            _ => Err(format!("built without the {backend} backend")),
        }
    }
}

/// Files as `generate`'s JSON.
pub fn files_json(files: &[(String, String)]) -> String {
    let files: Vec<String> = (files.iter())
        .map(|(path, text)| {
            format!(
                "{{\"path\":{},\"text\":{}}}",
                json_string(path),
                json_string(text)
            )
        })
        .collect();
    format!("{{\"files\":[{}]}}", files.join(","))
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Reads the input buffer as source and fills the output.
fn with_source(
    ptr: *const u8,
    len: usize,
    markdown: u32,
    f: impl FnOnce(&str, SourceKind, &mut String),
) {
    let kind = if markdown != 0 {
        SourceKind::Markdown
    } else {
        SourceKind::Standalone
    };
    INPUT.with_borrow(|input| {
        // Only our own buffer from `alloc` is accepted.
        let bytes = if ptr == input.as_ptr() && len <= input.len() {
            &input[..len]
        } else {
            &[][..]
        };
        let source = String::from_utf8_lossy(bytes);
        OUTPUT.with_borrow_mut(|out| {
            out.clear();
            f(&source, kind, out);
        });
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn output_ptr() -> *const u8 {
    OUTPUT.with_borrow(|out| out.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn output_len() -> usize {
    OUTPUT.with_borrow(|out| out.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs source as JavaScript would, natively, on a thread with room
    /// for the interpreter's recursion, giving the output.
    fn run_source(source: &'static str) -> String {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let ptr = alloc(source.len());
                unsafe { std::ptr::copy_nonoverlapping(source.as_ptr(), ptr, source.len()) };
                run(ptr, source.len(), 0);
                let out = unsafe { std::slice::from_raw_parts(output_ptr(), output_len()) };
                String::from_utf8(out.to_vec()).unwrap()
            })
            .unwrap()
            .join()
            .unwrap()
    }

    /// The functional tests tmpl's translation covers, as the backends'
    /// own tests list them, whose goldens are the native output.
    use temper_be_testkit::COVERED;

    /// Generates code as JavaScript would, natively, on a thread with
    /// room for the settle loop's recursion.
    fn generate_source(source: String, backend: u32) -> String {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let ptr = alloc(source.len());
                unsafe { std::ptr::copy_nonoverlapping(source.as_ptr(), ptr, source.len()) };
                generate(ptr, source.len(), 1, backend);
                let out = unsafe { std::slice::from_raw_parts(output_ptr(), output_len()) };
                String::from_utf8(out.to_vec()).unwrap()
            })
            .unwrap()
            .join()
            .unwrap()
    }

    /// Each built-in backend's output for subset A matches its native
    /// golden (in `crates/temper-be-*/tests/goldens/`), and `generate`
    /// gives it as JSON.
    #[test]
    fn generates_as_natively() {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for path in COVERED {
            let test = temper_funtests::test(path).unwrap();
            let [source] = &test.sources()[..] else {
                panic!("{path}: not one file");
            };
            for (backend, &name) in BACKENDS.iter().enumerate() {
                let json = generate_source(source.text.clone(), backend as u32);
                let built_in = (name == "py" && cfg!(feature = "py"))
                    || (name == "rust" && cfg!(feature = "rust"))
                    || (name == "js" && cfg!(feature = "js"))
                    || (name == "java" && cfg!(feature = "java"))
                    || (name == "csharp" && cfg!(feature = "csharp"))
                    || (name == "cpp" && cfg!(feature = "cpp"))
                    || (name == "lua" && cfg!(feature = "lua"));
                if !built_in {
                    let error = format!("built without the {name} backend");
                    assert_eq!(json, format!("{{\"error\":{}}}", json_string(&error)));
                    continue;
                }
                let text = source.text.clone();
                let files = std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn(move || translate(&text, SourceKind::Markdown, backend as u32))
                    .unwrap()
                    .join()
                    .unwrap()
                    .unwrap_or_else(|e| panic!("{path} {name}: {e}"));
                assert_eq!(json, files_json(&files), "{path} {name}");
                let (module, extension) = match name {
                    "py" => ("main/main/main.py", "py"),
                    "rust" => ("main/src/mod.rs", "rs"),
                    "java" => ("main/src/main/java/main/MainGlobal.java", "java"),
                    "csharp" => ("main/src/MainGlobal.cs", "cs"),
                    "cpp" => ("main/init.cpp", "cpp"),
                    "lua" => ("main/init.lua", "lua"),
                    _ => ("main/main.internal.js", "js"),
                };
                let text = &files.iter().find(|(p, _)| p == module).unwrap().1;
                let text = text
                    .strip_suffix("//# sourceMappingURL=main.internal.js.map\n")
                    .unwrap_or(text);
                let golden = (crates.join(format!("temper-be-{name}/tests/goldens")))
                    .join(format!("{}.{extension}", path.replace('/', "-")));
                let mut expected = std::fs::read_to_string(&golden).unwrap();
                if name == "java" {
                    // Java's class names its package, class, and logger
                    // for the library, here `main`.
                    let package = test.name().replace('-', "_");
                    let class: String = (package.split('_'))
                        .map(|w| w[..1].to_uppercase() + &w[1..])
                        .collect();
                    expected = expected
                        .replace(&format!("package {package};"), "package main;")
                        .replace(&format!("{class}Global"), "MainGlobal")
                        .replace(&format!("getLogger(\"{package}\")"), "getLogger(\"main\")");
                }
                if name == "csharp" {
                    // C#'s class names its namespace, class, and console
                    // for the library, here `Main`.
                    let root: String = (test.name().split('-'))
                        .map(|w| w[..1].to_uppercase() + &w[1..])
                        .collect();
                    expected = expected
                        .replace(&format!("namespace {root};"), "namespace Main;")
                        .replace(&format!("{root}Global"), "MainGlobal")
                        .replace(
                            &format!("CreateConsole(\"{root}\")"),
                            "CreateConsole(\"Main\")",
                        );
                }
                if name == "cpp" {
                    // C++'s golden holds the header and the module, in
                    // the library's namespace and directory, here `main`.
                    let header = &files.iter().find(|(p, _)| p == "main/init.hpp").unwrap().1;
                    let library = test.name();
                    let namespace = library.replace('-', "_");
                    expected = expected
                        .replace(&format!("namespace {namespace} "), "namespace main ")
                        .replace(&format!("namespace {namespace}\n"), "namespace main\n")
                        .replace(&format!(" {namespace}::"), " main::")
                        .replace(&format!("({namespace}::"), "(main::")
                        .replace(&format!("<{library}/init.hpp>"), "<main/init.hpp>");
                    let both = format!("// init.hpp\n{header}\n// init.cpp\n{text}");
                    assert_eq!(both, expected, "{path} {name}");
                    continue;
                }
                assert_eq!(text, expected, "{path} {name}");
                if name == "js" {
                    let (_, map) = (files.iter())
                        .find(|(p, _)| p == "main/main.internal.js.map")
                        .unwrap();
                    assert!(map.contains("\"sources\":[\"../main.temper.md\"]"), "{map}");
                }
            }
        }
        assert!(generate_source(String::new(), 7).contains("no backend 7"));
    }

    #[test]
    fn runs_with_built_in_std() {
        let source = "let { Date } = import(\"std/temporal\");\n\
                      console.log((new Date(2024, 3, 5) orelse panic()).month.toString());";
        assert_eq!(run_source(source), "3\n");
    }
}
