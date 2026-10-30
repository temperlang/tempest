//! The Python backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! (`temper-funtests`), and on the shared programs
//! (`crates/temper-be-testkit/programs/`) that it covers so far:
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

use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};
use temper_be_py::{OutFile, PY, translate};
use temper_be_testkit::{self as kit, Case, Goldens};

/// The shared programs that Python's translation covers so far.
const PROGRAMS: [&str; 1] = ["string-order"];

fn cases() -> Vec<Case> {
    kit::cases("py", &PROGRAMS)
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&kit::program(case), &PY).unwrap_or_else(|e| panic!("{}: {e}", case.key))
}

#[test]
fn goldens() {
    kit::with_stack(|| {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
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
            goldens.check(&format!("{path}.py"), &files[2].text);
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
            let files = generate(&case);
            let path = &case.key;
            let root = out.join(path);
            let _ = fs::remove_dir_all(&root);
            kit::write_files(
                &root,
                files.iter().map(|f| (f.path.as_str(), f.text.as_str())),
            );
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
            kit::check_log(&mut failures, path, &run, &case.expected);
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
        kit::assert_none(&failures);
    });
}
