//! The Python backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! (`temper-funtests`), and on the shared programs
//! (`crates/temper-be-testkit/programs/`) that it covers so far:
//! `string-order`, which every backend runs; `modules`,
//! `imports-library`, and `testing`, for modules, imports, and tests;
//! for classes and interfaces, `classes` and the testkit's class tests
//! (`CLASSES`); and for nullable types, `orelse`, `bubble()`, and
//! `panic()`, `nullables` and the testkit's `NULLABLES`; and for
//! generics and core's lists and maps, `generics` and the testkit's
//! `GENERICS`; for function values, `functions` and the testkit's
//! `FUNCTIONS`; for parameters' defaults, `defaults` and the testkit's
//! `DEFAULTS`; for core's other types and members, the testkit's
//! `CORE_TYPES`; and for a library's own connected functions and
//! classes, with host code in `__connected__.py`, the testkit's
//! `CONNECTED`, each as is and under `--no-connected`; and for divisions
//! that can't fail and maps, the testkit's `DIVISIONS`; and for labeled
//! loops' jumps, classes declared in functions, type aliases, explicit
//! type arguments, and upcasting `as`, the testkit's `LOOPS_AND_CLASSES`.
//!
//! - `goldens`: each test's generated Python, laid out as Kotlin's, with
//!   its root module matching the golden in `tests/goldens/`, named for
//!   the test's path (`algos-fibonacci.py`), and its other files, if
//!   any, theirs, named for the path and the file's place
//!   (`modules.helpers.deep.py`, `testing.tests.test_testing.py`). To
//!   accept new or changed output, run with `UPDATE_SNAPSHOTS=1` and
//!   review the diff.
//! - `runs`: each test's generated Python runs, against the vendored
//!   `temper_core` runtime (`vendor/temper/be-py/temper-core`), and logs
//!   what the test expects of Kotlin's `py` backend; and type checkers
//!   find no problems in it, in strict mode: mypy and pyright, each where
//!   installed (on `PATH`, or named by `MYPY` or `PYRIGHT`), except for
//!   a library with host code, which isn't written for them. Skipped, with
//!   a message, without Python 3.11 or later (`python3`, or `PYTHON`);
//!   with `TEMPEST_REQUIRE_CHECKERS` set, as in CI, a missing Python or
//!   checker fails it instead. A library that imports another
//!   (`imports-library`) runs with it beside it. Tests aren't run: they
//!   need std's `testing`, which tmpl doesn't translate yet; `python -m
//!   py_compile` checks each tests file.

use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};
use temper_be_py::{OutFile, PY, translate};
use temper_be_testkit::{self as kit, Case, Goldens};

/// The shared programs that Python's translation covers so far.
const PROGRAMS: [&str; 9] = [
    "classes",
    "nullables",
    "generics",
    "functions",
    "defaults",
    "string-order",
    "modules",
    "imports-library",
    "testing",
];

fn cases() -> Vec<Case> {
    kit::cases_with(
        "py",
        &PROGRAMS,
        &[
            &kit::CLASSES[..],
            &kit::NULLABLES[..],
            &kit::GENERICS[..],
            &kit::FUNCTIONS[..],
            &kit::DEFAULTS[..],
            &kit::CORE_TYPES[..],
            &kit::CONNECTED[..],
            &kit::DIVISIONS[..],
            &kit::LOOPS_AND_CLASSES[..],
        ]
        .concat(),
    )
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&kit::program_for(case, &PY.rules), &PY)
        .unwrap_or_else(|e| panic!("{}: {e}", case.key))
}

#[test]
fn goldens() {
    kit::with_stack(|| {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
        for case in cases() {
            let files = generate(&case);
            let path = &case.key;
            let library = &case.library;
            let package = temper_be_py::names::library_package(library);
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(
                paths[..2],
                [
                    format!("{library}/pyproject.toml"),
                    format!("{library}/{package}/__init__.py"),
                ],
                "{path}"
            );
            let root_module = format!("{library}/{package}/{package}.py");
            assert!(paths.contains(&root_module.as_str()), "{path}");
            let single = format!("from .{package} import *\n");
            if files.len() == 3 {
                assert_eq!(files[1].text, single, "{path}");
            }
            for file in &files[1..] {
                let rest = &file.path[library.len() + 1..];
                let name = if rest == format!("{package}/{package}.py") {
                    format!("{path}.py")
                } else if file.text == single {
                    continue;
                } else {
                    let rest = rest.strip_prefix(&format!("{package}/")).unwrap_or(rest);
                    format!("{path}.{}", rest.replace('/', "."))
                };
                goldens.check(&name, &file.text);
            }
        }
        goldens.finish();
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

fn runtime() -> PathBuf {
    kit::vendored("be-py/temper-core")
}

#[test]
fn runs() {
    let Some(python) = python() else {
        kit::missing("no Python 3.11 or later (set PYTHON to one)");
        return;
    };
    let mypy = kit::tool("MYPY", "mypy");
    let pyright = kit::tool("PYRIGHT", "pyright");
    for (name, found) in [("mypy", &mypy), ("pyright", &pyright)] {
        if found.is_none() {
            kit::missing(&format!(
                "type check by {name}: no {name} (on PATH, or named by its variable)"
            ));
        }
    }
    kit::with_stack(move || {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-py");
        let mut failures = Vec::new();
        for case in cases() {
            let path = &case.key;
            let root = out.join(path);
            let _ = fs::remove_dir_all(&root);
            let mut projects = vec![runtime()];
            for case in kit::libraries(&case, "py").iter().chain([&case]) {
                let files = generate(case);
                kit::write_files(
                    &root,
                    files.iter().map(|f| (f.path.as_str(), f.text.as_str())),
                );
                projects.push(root.join(&case.library));
            }
            let library = &case.library;
            let package = temper_be_py::names::library_package(library);
            let project = root.join(library);
            let path_var = env::join_paths(&projects).unwrap();
            let tests = project.join("tests");
            if tests.exists() {
                let mut files = Vec::new();
                kit::files_with(&tests, "py", &mut files);
                let compile = Command::new(&python)
                    .args(["-B", "-m", "py_compile"])
                    .args(&files)
                    .output()
                    .unwrap();
                if !compile.status.success() {
                    failures.push(format!(
                        "{path}: tests don't compile:\n{}",
                        String::from_utf8_lossy(&compile.stderr)
                    ));
                }
            }
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
            kit::check_log(&mut failures, path, &run, &case.expected);
            let package_dir = project.join(&package);
            // Host code is the library's own, which its module's file
            // starts with, and the functional tests' isn't written for
            // strict checkers (its functions take no `self`), nor are calls
            // of it, so a library with host code isn't checked.
            let checked = case.host_files.is_empty();
            if let Some(mypy) = mypy.as_ref().filter(|_| checked) {
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
                    .env("MYPYPATH", &path_var)
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
            if let Some(pyright) = pyright.as_ref().filter(|_| checked) {
                let config = root.join("pyrightconfig.json");
                let paths: Vec<String> = (projects.iter())
                    .map(|p| format!("\"{}\"", p.to_string_lossy().replace('\\', "/")))
                    .collect();
                let paths = paths.join(", ");
                fs::write(
                    &config,
                    format!(
                        "{{\"typeCheckingMode\": \"strict\", \"pythonVersion\": \"3.11\", \
                         \"extraPaths\": [{paths}], \"include\": [\"{library}/{package}\"]}}\n"
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
        kit::assert_none(&failures);
    });
}
