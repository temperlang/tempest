//! A wasm entry point for Temper, for the web playground
//! (`site/playground/`) and for measuring how big Temper is as wasm
//! (`compat/measure_size.py`).
//!
//! `compile` is the playground's: it settles source once and gives, as
//! JSON, its diagnostics with their spans, what it logs and its tests'
//! results from the interpreter, and each backend's files; `backends`
//! lists the backends built in, and `std_library` gives std's own
//! output from one, for running generated tests. The older calls each do
//! one thing: `parse` returns the parse tree and the grammar step's tree
//! as text, `run` settles and runs source, with the std modules it
//! imports, built in, returning its diagnostics and what it logs, and
//! `generate` translates it with one backend, returning the files as
//! JSON. JavaScript calls `alloc` for a buffer, writes UTF-8 source into
//! it, calls one of them, and reads the result from `output_ptr` and
//! `output_len`:
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
use std::fmt::Write;
use temper_frontend::library_tests::{Tally, run_settled_tests};
use temper_frontend::project::load_imports;
use temper_frontend::settle::{Settled, settle};
use temper_frontend::sources::Sources;
use temper_frontend::{interp, library::Library, print::write_diagnostics};
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

/// `compile`'s `options` bit for literate markdown source.
pub const MARKDOWN: u32 = 1;
/// `compile`'s `options` bit to run the library in the interpreter.
pub const RUN: u32 = 2;
/// `compile`'s `options` bit to also run its tests, if it runs.
pub const TESTS: u32 = 4;

/// The backends this build has, in [`BACKENDS`]' order, as JSON,
/// `["py", ...]`, for `compile`'s `backends` mask, whose bit `1 << i` is
/// the `i`th of them.
#[unsafe(no_mangle)]
pub extern "C" fn backends() {
    let names: Vec<String> = built_in().map(json_string).collect();
    OUTPUT.with_borrow_mut(|out| *out = format!("[{}]", names.join(",")));
}

/// Each of [`BACKENDS`], and whether this build has it.
const BUILT_IN: [(&str, bool); 7] = [
    ("py", cfg!(feature = "py")),
    ("rust", cfg!(feature = "rust")),
    ("js", cfg!(feature = "js")),
    ("java", cfg!(feature = "java")),
    ("csharp", cfg!(feature = "csharp")),
    ("cpp", cfg!(feature = "cpp")),
    ("lua", cfg!(feature = "lua")),
];

/// The backends this build has, in [`BACKENDS`]' order.
pub fn built_in() -> impl Iterator<Item = &'static str> {
    (BUILT_IN.into_iter()).filter_map(|(name, on)| on.then_some(name))
}

/// Settles the input buffer once, as `generate` does, and keeps all a
/// playground shows as one JSON object: its diagnostics, what it logs
/// if `options` has [`RUN`] (and its tests' report, with [`TESTS`]),
/// and what each backend in the `backends` mask (bits by [`backends`])
/// generates:
///
/// ```json
/// {"diagnostics": [{"level": "error", "message": "...", "file": "main.temper",
///                   "from": 4, "to": 9, "line": 1, "column": 5}],
///  "run": {"output": ["..."], "failure": null,
///          "tests": [{"name": "...", "failure": null}], "defined": 1,
///          "summary": "Tests passed: 1 of 1"},
///  "backends": {"py": {"files": [{"path": "...", "text": "..."}]},
///               "rust": {"error": "..."}}}
/// ```
///
/// `from` and `to` count UTF-16 code units in the main file, as
/// JavaScript strings and editors do, and `line` and `column` count
/// from 1; diagnostics in other files (std's) have no position.
#[unsafe(no_mangle)]
pub extern "C" fn compile(ptr: *const u8, len: usize, options: u32, backends: u32) {
    with_source(ptr, len, options & MARKDOWN, |source, kind, out| {
        compile_into(source, kind, options, backends, out);
    });
}

/// What `compile` does, into `out`.
pub fn compile_into(source: &str, kind: SourceKind, options: u32, mask: u32, out: &mut String) {
    let file = match kind {
        SourceKind::Markdown => "main.temper.md",
        _ => "main.temper",
    };
    let mut library = Library::new();
    library.add_file(file, source, kind);
    load_imports(&Sources::memory(&[]), &mut library);
    let settled = settle(library);
    out.push_str("{\"diagnostics\":[");
    diagnostics_json(&settled, source, out);
    out.push(']');
    if options & RUN != 0 {
        let (output, failure, tests) = if options & TESTS != 0 {
            let tests = run_settled_tests(&settled, Some(interp::FUEL));
            let mut tally = Tally::default();
            tally.add(&tests);
            (
                tests.output.clone(),
                tests.failure.clone(),
                Some((tests, tally)),
            )
        } else {
            let run = interp::run(&settled, Some(interp::FUEL));
            (run.output, run.failure, None)
        };
        let lines: Vec<String> = output.iter().map(|l| json_string(l)).collect();
        let failure = failure.as_deref().map_or("null".into(), json_string);
        let _ = write!(
            out,
            ",\"run\":{{\"output\":[{}],\"failure\":{failure}",
            lines.join(",")
        );
        if let Some((tests, tally)) = tests {
            let results: Vec<String> = (tests.results.iter())
                .map(|(name, failure)| {
                    let failure = failure.as_deref().map_or("null".into(), json_string);
                    format!("{{\"name\":{},\"failure\":{failure}}}", json_string(name))
                })
                .collect();
            let _ = write!(
                out,
                ",\"tests\":[{}],\"defined\":{},\"summary\":{}",
                results.join(","),
                tests.defined,
                json_string(&tally.summary())
            );
        }
        out.push('}');
    }
    out.push_str(",\"backends\":{");
    let chosen: Vec<&str> = (built_in().enumerate())
        .filter(|&(i, _)| i < 32 && mask & (1 << i) != 0)
        .map(|(_, name)| name)
        .collect();
    #[cfg(feature = "tmpl")]
    if !chosen.is_empty() {
        let program = backends::program(&settled);
        for (i, name) in chosen.into_iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let generated = program.clone().and_then(|p| backends::generate(&p, name));
            let json = match generated {
                Ok(files) => files_json(&files),
                Err(error) => format!("{{\"error\":{}}}", json_string(&error)),
            };
            let _ = write!(out, "{}:{json}", json_string(name));
        }
    }
    #[cfg(not(feature = "tmpl"))]
    let _ = chosen;
    out.push_str("}}");
}

/// What the backend that's `backend`th of [`backends`] generates for
/// std, as `generate`'s JSON, translated once and kept, for running a
/// library's tests with std's `testing` in its own language.
#[unsafe(no_mangle)]
pub extern "C" fn std_library(backend: u32) {
    let json = match built_in().nth(backend as usize) {
        #[cfg(feature = "tmpl")]
        Some(name) => match backends::std_files(name) {
            Ok(files) => files_json(&files),
            Err(error) => format!("{{\"error\":{}}}", json_string(&error)),
        },
        _ => format!(
            "{{\"error\":{}}}",
            json_string(&format!("no backend {backend}"))
        ),
    };
    OUTPUT.with_borrow_mut(|out| *out = json);
}

/// A settled library's diagnostics as JSON objects, comma-separated.
fn diagnostics_json(settled: &Settled, source: &str, out: &mut String) {
    let library = &settled.library;
    for (i, d) in library.diagnostics.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let level = format!("{:?}", d.level).to_lowercase();
        let (file, start) = library.locate(d.span.start);
        let path = library.files.get(file.0 as usize).map_or("", |f| &f.path);
        let _ = write!(
            out,
            "{{\"level\":{},\"message\":{},\"file\":{}",
            json_string(&level),
            json_string(&d.message.text()),
            json_string(path)
        );
        if file.0 == 0 {
            let end = start + (d.span.end - d.span.start);
            let at = Position::of(source, start as usize);
            let to = Position::of(source, end as usize);
            let _ = write!(
                out,
                ",\"from\":{},\"to\":{},\"line\":{},\"column\":{}",
                at.utf16, to.utf16, at.line, at.column
            );
        }
        out.push('}');
    }
}

/// Where a byte offset is, as JavaScript counts.
struct Position {
    /// UTF-16 code units before it.
    utf16: usize,
    /// Its line, from 1.
    line: usize,
    /// Its column in UTF-16 code units, from 1.
    column: usize,
}

impl Position {
    /// The position of a byte offset in `source`, clamped to its end and
    /// moved back to a character boundary.
    fn of(source: &str, offset: usize) -> Position {
        let mut offset = offset.min(source.len());
        while !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let before = &source[..offset];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        Position {
            utf16: before.encode_utf16().count(),
            line: before.matches('\n').count() + 1,
            column: before[line_start..].encode_utf16().count() + 1,
        }
    }
}

/// Generated files, each a path and text.
pub type Files = Vec<(String, String)>;

/// What `generate` does, before the JSON.
#[cfg_attr(not(feature = "tmpl"), allow(unused_variables))]
pub fn translate(source: &str, kind: SourceKind, backend: u32) -> Result<Files, String> {
    let Some(&name) = BACKENDS.get(backend as usize) else {
        return Err(format!("no backend {backend}"));
    };
    #[cfg(feature = "tmpl")]
    return backends::translate(source, kind, name);
    #[cfg(not(feature = "tmpl"))]
    Err(format!("built without the {name} backend"))
}

#[cfg(feature = "tmpl")]
mod backends {
    use super::Files;
    use temper_frontend::project::load_imports;
    use temper_frontend::settle::{Settled, settle};
    use temper_frontend::sources::Sources;
    use temper_frontend::to_tmpl::to_tmpl;
    use temper_frontend::{library::Library, print::write_diagnostics};
    use temper_syntax::lex::SourceKind;
    use temper_tmpl::{Profile, validate};

    thread_local! {
        /// std as tmpl, settled once and kept, since it never changes
        /// and settling it takes most of a translation's time.
        static STD: std::cell::OnceCell<Result<temper_tmpl::Program, String>> =
            const { std::cell::OnceCell::new() };
    }

    /// std as tmpl, settled the first time it's asked for.
    fn with_std_program<R>(
        f: impl FnOnce(&temper_tmpl::Program) -> Result<R, String>,
    ) -> Result<R, String> {
        STD.with(|std| {
            let std = std.get_or_init(|| {
                temper_frontend::stdlib::program(&Sources::memory(&[]))
                    .map_err(|e| format!("std: {e}"))
            });
            f(std.as_ref().map_err(Clone::clone)?)
        })
    }

    /// std's own files from a backend that's built in.
    pub fn std_files(backend: &str) -> Result<Files, String> {
        with_std_program(|std| generate(std, backend))
    }

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
            program.interfaces = with_std_program(|std| {
                let names = temper_tmpl::naming::assign(std, rules);
                Ok(vec![temper_tmpl::naming::Interface::of(std, &names)])
            })?;
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
        generate(&program(&settle(library))?, backend)
    }

    /// A settled library as tmpl, checked, or what went wrong.
    pub fn program(settled: &Settled) -> Result<temper_tmpl::Program, String> {
        let program = to_tmpl(settled, "main").map_err(|e| {
            let mut diagnostics = String::new();
            write_diagnostics(settled, &mut diagnostics);
            format!("{diagnostics}{e}")
        })?;
        let problems = validate(&program, &Profile::full());
        if !problems.is_empty() {
            let problems: Vec<String> = problems.iter().map(|p| format!("tmpl: {p}")).collect();
            return Err(problems.join("\n"));
        }
        Ok(program)
    }

    /// Generates code from tmpl with a backend that's built in.
    pub fn generate(program: &temper_tmpl::Program, backend: &str) -> Result<Files, String> {
        match backend {
            #[cfg(feature = "py")]
            "py" => temper_be_py::translate(
                &with_std(program, &temper_be_py::PY.rules)?,
                &temper_be_py::PY,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "rust")]
            "rust" => temper_be_rust::translate(
                &with_std(program, &temper_be_rust::RUST.rules)?,
                &temper_be_rust::RUST,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "js")]
            "js" => temper_be_js::translate_with_source_maps(
                &with_std(program, &temper_be_js::JS.rules)?,
                &temper_be_js::JS,
                "../",
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "java")]
            "java" => temper_be_java::translate(
                &with_std(program, &temper_be_java::JAVA.rules)?,
                &temper_be_java::JAVA,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "csharp")]
            "csharp" => temper_be_csharp::translate(
                &with_std(program, &temper_be_csharp::CSHARP.rules)?,
                &temper_be_csharp::CSHARP,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "cpp")]
            "cpp" => temper_be_cpp::translate(
                &with_std(program, &temper_be_cpp::CPP.rules)?,
                &temper_be_cpp::CPP,
            )
            .map(|fs| fs.into_iter().map(|f| (f.path, f.text)).collect())
            .map_err(|e| e.to_string()),
            #[cfg(feature = "lua")]
            "lua" => temper_be_lua::translate(
                &with_std(program, &temper_be_lua::LUA.rules)?,
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

    /// `compile`'s JSON for source, natively, with room to recurse.
    fn compile_source(source: &'static str, options: u32, mask: u32) -> String {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let mut out = String::new();
                compile_into(source, SourceKind::Standalone, options, mask, &mut out);
                out
            })
            .unwrap()
            .join()
            .unwrap()
    }

    /// Diagnostics count UTF-16 code units, as JavaScript does.
    #[test]
    fn compiles_with_positions() {
        let json = compile_source("let \u{e9} = 1;\nconsole.log(y);", 0, 0);
        assert_eq!(
            json,
            "{\"diagnostics\":[{\"level\":\"error\",\"message\":\"y has not been declared\",\
             \"file\":\"main.temper\",\"from\":23,\"to\":24,\"line\":2,\"column\":13}],\
             \"backends\":{}}"
        );
    }

    /// One compile runs the tests and generates each backend asked for.
    #[cfg(feature = "py")]
    #[test]
    fn compiles_runs_and_tests() {
        let source = "export let twice(n: Int): Int { n * 2 }\n\
                      test(\"doubles\") { assert(twice(2) == 4); }\n\
                      test(\"fails\") { assert(twice(2) == 5) { \"no\" } }\n\
                      console.log(\"hi\");";
        let py = 1 << built_in().position(|b| b == "py").unwrap();
        let json = compile_source(source, RUN | TESTS, py);
        assert!(json.starts_with("{\"diagnostics\":[],\"run\":{\"output\":[\"hi\"],\"failure\":null,\
             \"tests\":[{\"name\":\"doubles\",\"failure\":null},{\"name\":\"fails\",\"failure\":\"no\"}],\
             \"defined\":2,\"summary\":\"Tests passed: 1 of 2\"},\"backends\":{\"py\":{\"files\":["), "{json}");
        assert!(json.contains("def twice(n: int, /) -> int:"), "{json}");
        assert!(!json.contains("\"rust\":"), "{json}");
        backends();
        let names = OUTPUT.with_borrow(|out| out.clone());
        assert!(names.starts_with("[\"py\""), "{names}");
    }

    /// std's own JavaScript, for running tests with std's `testing`.
    #[cfg(feature = "js")]
    #[test]
    fn generates_std() {
        let js = built_in().position(|b| b == "js").unwrap() as u32;
        let json = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                std_library(js);
                OUTPUT.with_borrow(|out| out.clone())
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(
            json.contains("{\"path\":\"std/testing.js\""),
            "{}",
            &json[..200]
        );
    }

    #[test]
    fn runs_with_built_in_std() {
        let source = "let { Date } = import(\"std/temporal\");\n\
                      console.log((new Date(2024, 3, 5) orelse panic()).month.toString());";
        assert_eq!(run_source(source), "3\n");
    }
}
