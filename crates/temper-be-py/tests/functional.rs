//! The Python backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! ([`temper_funtests`]), and on the Rust backend's programs
//! (`crates/temper-be-rust/tests/programs/`) that it covers so far:
//! `string-order`, which every backend runs.
//!
//! - `goldens`: each test's generated Python, laid out as Kotlin's, with
//!   its module matching the golden in `tests/goldens/`, named for the
//!   test's path (`algos-fibonacci.py`). To accept new or changed output,
//!   run with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `runs`: each test's generated Python runs, against the vendored
//!   `temper_core` runtime (`vendor/temper/be-py/temper-core`), and logs
//!   what the test expects of Kotlin's `py` backend; and type checkers
//!   find no problems in it, in strict mode: mypy and pyright, each where
//!   installed (on `PATH`, or named by `MYPY` or `PYRIGHT`). Skipped, with
//!   a message, without Python 3.11 or later (`python3`, or `PYTHON`);
//!   with `TEMPEST_REQUIRE_CHECKERS` set, as in CI, a missing Python or
//!   checker fails it instead.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use temper_be_py::{OutFile, PY, translate};
use temper_frontend::library::Library;
use temper_frontend::project::load_imports;
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::to_tmpl;
use temper_syntax::lex::SourceKind;
use temper_tmpl::{Profile, Program, validate};

/// The functional tests tmpl's translation covers, by their paths.
const COVERED: [&str; 7] = [
    "algos/fibonacci",
    "algos/hello-world",
    "control-flow/if-return",
    "control-flow/loop-reenterable",
    "names/nonascii",
    "types/int/basics",
    "types/string/is-empty",
];

/// The Rust backend's programs, in its `tests/programs/`, that Python's
/// translation covers so far.
const PROGRAMS: [&str; 1] = ["string-order"];

/// A program to translate: its library's name, its sources, and the log
/// it expects.
struct Case {
    /// Its golden's name: the test's path with `-` for `/`.
    key: String,
    library: String,
    sources: Vec<(String, String)>,
    expected: Vec<String>,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for path in COVERED {
        let test = temper_funtests::test(path).unwrap_or_else(|| panic!("no test {path}"));
        cases.push(Case {
            key: path.replace('/', "-"),
            library: test.name().to_string(),
            sources: (test.sources().into_iter())
                .map(|s| (s.path, s.text))
                .collect(),
            expected: test.expected_log("py"),
        });
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../temper-be-rust/tests/programs");
    for file in PROGRAMS {
        let path = dir.join(format!("{file}.temper.md"));
        let text = fs::read_to_string(&path).unwrap();
        cases.push(Case {
            key: file.to_string(),
            library: file.to_string(),
            expected: temper_funtests::expected_log(&text, "py"),
            sources: vec![(path.to_string_lossy().into_owned(), text)],
        });
    }
    cases
}

/// A case's library, settled and translated to tmpl.
fn program(case: &Case) -> Program {
    let key = &case.key;
    let mut library = Library::new();
    for (path, text) in &case.sources {
        let kind = if path.ends_with(".md") {
            SourceKind::Markdown
        } else {
            SourceKind::Standalone
        };
        library.add_file(path, text, kind);
    }
    load_imports(&Sources::standard(), &mut library);
    let program = to_tmpl(&settle(library), &case.library).unwrap_or_else(|e| panic!("{key}: {e}"));
    let problems = validate(&program, &Profile::full());
    assert!(problems.is_empty(), "{key}: {problems:#?}");
    program
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&program(case), &PY).unwrap_or_else(|e| panic!("{}: {e}", case.key))
}

/// Runs a test on a thread with room for the settle loop's recursion.
fn with_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn goldens() {
    with_stack(|| {
        let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
        let goldens = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens");
        let mut differ = Vec::new();
        for case in cases() {
            let files = generate(&case);
            let path = &case.key;
            let library = &case.library;
            let package = library.replace('-', "_");
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(
                paths,
                [
                    format!("{library}/pyproject.toml"),
                    format!("{library}/{package}/__init__.py"),
                    format!("{library}/{package}/{package}.py"),
                ],
                "{path}"
            );
            assert_eq!(files[1].text, format!("from .{package} import *\n"));
            let module = &files[2].text;
            let golden = goldens.join(format!("{path}.py"));
            let expected = fs::read_to_string(&golden).unwrap_or_default();
            if *module != expected {
                if update {
                    fs::create_dir_all(&goldens).unwrap();
                    fs::write(&golden, module).unwrap();
                } else {
                    differ.push(format!("{}:\n{module}", golden.display()));
                }
            }
        }
        assert!(
            differ.is_empty(),
            "goldens differ (UPDATE_SNAPSHOTS=1 accepts new output):\n{}",
            differ.join("\n")
        );
    });
}

/// Python 3.11 or later, if there is one.
fn python() -> Option<String> {
    let candidates = match env::var("PYTHON") {
        Ok(p) => vec![p],
        Err(_) => vec!["python3".to_string(), "python".to_string()],
    };
    candidates.into_iter().find(|p| {
        Command::new(p)
            .args([
                "-c",
                "import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)",
            ])
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

/// A tool named by an environment variable, or found on `PATH`.
fn tool(var: &str, name: &str) -> Option<String> {
    let candidate = env::var(var).unwrap_or_else(|_| name.to_string());
    Command::new(&candidate)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
        .then_some(candidate)
}

fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/be-py/temper-core")
}

fn write_files(root: &Path, files: &[OutFile]) {
    for file in files {
        let path = root.join(&file.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &file.text).unwrap();
    }
}

#[test]
fn runs() {
    let required = env::var_os("TEMPEST_REQUIRE_CHECKERS").is_some();
    let Some(python) = python() else {
        assert!(!required, "no Python 3.11 or later");
        eprintln!("skipped: no Python 3.11 or later (set PYTHON to one)");
        return;
    };
    let mypy = tool("MYPY", "mypy");
    let pyright = tool("PYRIGHT", "pyright");
    for (name, found) in [("mypy", &mypy), ("pyright", &pyright)] {
        if found.is_none() {
            assert!(!required, "no {name}");
            eprintln!("type check skipped: no {name} (on PATH, or named by its variable)");
        }
    }
    with_stack(move || {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-py");
        let mut failures = Vec::new();
        for case in cases() {
            let files = generate(&case);
            let path = &case.key;
            let root = out.join(path);
            let _ = fs::remove_dir_all(&root);
            write_files(&root, &files);
            let library = &case.library;
            let package = library.replace('-', "_");
            let project = root.join(library);
            let path_var = env::join_paths([runtime(), project.clone()]).unwrap();
            let script = format!(
                "from temper_core import init_simple_logging, await_safe_to_exit\n\
                 init_simple_logging()\n\
                 import {package}\n\
                 await_safe_to_exit()\n"
            );
            // -B: write no bytecode into the vendored runtime.
            let run = Command::new(&python)
                .args(["-B", "-c", &script])
                .env("PYTHONPATH", &path_var)
                .current_dir(&root)
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&run.stdout);
            let actual: Vec<&str> = stdout.lines().collect();
            let expected = &case.expected;
            if !run.status.success() || actual != *expected {
                failures.push(format!(
                    "{path}: expected {expected:?}, got {actual:?}\n{}",
                    String::from_utf8_lossy(&run.stderr)
                ));
            }
            let package_dir = project.join(&package);
            if let Some(mypy) = &mypy {
                let check = Command::new(mypy)
                    .args([
                        "--strict",
                        "--python-version",
                        "3.11",
                        "--follow-imports=silent",
                    ])
                    .arg("--cache-dir")
                    .arg(root.join(".mypy_cache"))
                    .arg(&package_dir)
                    .env("MYPYPATH", runtime())
                    .current_dir(&root)
                    .output()
                    .unwrap();
                if !check.status.success() {
                    failures.push(format!(
                        "{path}: mypy --strict:\n{}",
                        String::from_utf8_lossy(&check.stdout)
                    ));
                }
            }
            if let Some(pyright) = &pyright {
                let config = root.join("pyrightconfig.json");
                let runtime = runtime().to_string_lossy().replace('\\', "/");
                fs::write(
                    &config,
                    format!(
                        "{{\"typeCheckingMode\": \"strict\", \"pythonVersion\": \"3.11\", \
                         \"extraPaths\": [\"{runtime}\"], \"include\": [\"{library}/{package}\"]}}\n"
                    ),
                )
                .unwrap();
                let check = Command::new(pyright)
                    .arg("-p")
                    .arg(&config)
                    .current_dir(&root)
                    .output()
                    .unwrap();
                if !check.status.success() {
                    failures.push(format!(
                        "{path}: pyright (strict):\n{}",
                        String::from_utf8_lossy(&check.stdout)
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}
