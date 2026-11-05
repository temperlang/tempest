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
//! type arguments, and upcasting `as`, the testkit's `LOOPS_AND_CLASSES`; and for
//! tests run with std's `testing`, the testkit's `TESTS`; and for std's
//! other modules, the testkit's `WITH_STD`.
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
//!   (`imports-library`) runs with it beside it. `python -m py_compile`
//!   checks each tests file; a test that runs as tests
//!   (`testing/asserts`) runs, with std's `temper-std` beside it, by
//!   `python -m temper_core.testing result.xml`, as Kotlin's `RunPython`,
//!   its JUnit XML's failures compared with its `expectedFailures`.
//!   Every library module generated, and std's, sets `__all__` to its
//!   exports and binds no other module-level name without a leading `_`
//!   (`tests/module_names.py`). With `TEMPEST_PY_BULK` set, the cases
//!   that log run in one interpreter (`tests/bulk_runner.py`), and any
//!   that doesn't log what it expects runs again alone, whose result
//!   counts (notes/test-speed.md, "Python's functional tests in bulk").

use std::path::{Path, PathBuf};
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
            &kit::TYPING[..],
            &kit::ANY_VALUES[..],
            &kit::TESTS[..],
            &kit::WITH_STD[..],
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

/// A library's modules among its generated files: the `.py` files in its
/// package but the package's own `__init__.py` (which star-imports the
/// root module's exports and names its modules) and copies of host files.
fn library_modules<'f>(
    library: &str,
    files: &'f [OutFile],
    host_files: &[(String, String)],
) -> impl Iterator<Item = &'f OutFile> {
    let package = temper_be_py::names::library_package(library);
    let init = format!("{library}/{package}/__init__.py");
    let prefix = format!("{library}/{package}/");
    let host: Vec<String> = host_files.iter().map(|(_, t)| t.clone()).collect();
    files.iter().filter(move |f| {
        f.path.starts_with(&prefix)
            && f.path.ends_with(".py")
            && f.path != init
            && !host.contains(&f.text)
    })
}

/// Runs `module_names.py` over generated modules: each sets `__all__` to
/// its exports, and every other module-level name it binds starts with
/// `_`.
fn check_module_names(python: &str, modules: &[PathBuf], failures: &mut Vec<String>) {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/module_names.py");
    let check = Command::new(python)
        .arg("-B")
        .arg(&script)
        .args(modules)
        .output()
        .unwrap();
    if !check.status.success() {
        failures.push(format!(
            "module-level names:\n{}{}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        ));
    }
}

/// A case that logs, run in a process of its own: its package imported
/// with the runtime's logging to stdout, then its async work awaited.
fn run_alone(
    python: &str,
    root: &Path,
    projects: &[PathBuf],
    package: &str,
) -> std::process::Output {
    let script = format!(
        "from temper_core import init_simple_logging, await_safe_to_exit\n\
         init_simple_logging()\n\
         import {package}\n\
         await_safe_to_exit()\n"
    );
    // -B: write no bytecode into the vendored runtime.
    Command::new(python)
        .args(["-B", "-c", &script])
        .env("PYTHONPATH", env::join_paths(projects).unwrap())
        .current_dir(root)
        .output()
        .unwrap()
}

/// A case that logs, waiting for a bulk run.
struct Logged {
    key: String,
    root: PathBuf,
    projects: Vec<PathBuf>,
    package: String,
    expected: Vec<String>,
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Runs the cases that log in one interpreter (`tests/bulk_runner.py`),
/// each after a marker line, and checks each one's lines. A case that
/// doesn't log what it expects, or that a crash leaves without its
/// marker, is run again alone (`run_alone`), whose result counts, so a
/// failure's message is as precise as without the bulk run.
fn run_batch(python: &str, batch: &[Logged], failures: &mut Vec<String>) {
    if batch.is_empty() {
        return;
    }
    let runtime = runtime();
    let cases: Vec<String> = (batch.iter())
        .map(|c| {
            let path: Vec<String> = (c.projects.iter())
                .map(|p| json_string(&p.to_string_lossy()))
                .collect();
            format!(
                "{{\"key\": {}, \"cwd\": {}, \"path\": [{}], \"package\": {}}}",
                json_string(&c.key),
                json_string(&c.root.to_string_lossy()),
                path.join(", "),
                json_string(&c.package)
            )
        })
        .collect();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/bulk_runner.py");
    let mut child = Command::new(python)
        .arg("-B")
        .arg(&script)
        .env("PYTHONPATH", &runtime)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        write!(stdin, "[{}]", cases.join(", ")).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Each case's lines, by key; `None` after an exception.
    let mut logged: std::collections::HashMap<&str, Option<Vec<&str>>> = Default::default();
    let mut current: Option<&str> = None;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix('\x1e') {
            if rest == "!" {
                if let Some(key) = current {
                    logged.insert(key, None);
                }
                current = None;
            } else {
                logged.insert(rest, Some(Vec::new()));
                current = Some(rest);
            }
        } else if let Some(Some(lines)) = current.and_then(|k| logged.get_mut(k)) {
            lines.push(line);
        }
    }
    // The last case's lines are complete only if the runner finished.
    if !out.status.success() {
        if let Some(key) = current {
            logged.insert(key, None);
        }
    }
    let mut alone = Vec::new();
    for case in batch {
        let passed =
            matches!(logged.get(case.key.as_str()), Some(Some(lines)) if *lines == case.expected);
        if !passed {
            alone.push(case.key.as_str());
            let run = run_alone(python, &case.root, &case.projects, &case.package);
            kit::check_log(failures, &case.key, &run, &case.expected);
        }
    }
    eprintln!(
        "bulk: {} cases in one interpreter; {} run again alone: {alone:?}",
        batch.len(),
        alone.len()
    );
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
        // With `TEMPEST_PY_BULK` set, the cases that log run in one
        // interpreter after the others (`run_batch`).
        let bulk = env::var("TEMPEST_PY_BULK").is_ok_and(|v| !v.is_empty());
        let mut batch = Vec::new();
        // Every library module generated, for `check_module_names`.
        let mut modules = Vec::new();
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
                let found = library_modules(&case.library, &files, &case.host_files);
                modules.extend(found.map(|f| root.join(&f.path)));
            }
            // A run as tests needs std's `testing`, and an import of std's
            // modules them: std's output beside it.
            if case.needs_std() {
                let files = translate(&kit::std_program(), &PY).unwrap();
                kit::write_files(
                    &root,
                    files.iter().map(|f| (f.path.as_str(), f.text.as_str())),
                );
                projects.push(root.join("temper-std"));
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
            if case.tests.is_some() {
                // As Kotlin's `RunPython`: `temper_core.testing` runs the
                // project's tests and writes their JUnit XML.
                let run = Command::new(&python)
                    .args(["-B", "-m", "temper_core.testing", "result.xml"])
                    .env("PYTHONPATH", &path_var)
                    .current_dir(&project)
                    .output()
                    .unwrap();
                let actual = kit::junit_file_failures(&project.join("result.xml"));
                kit::check_tests(&mut failures, &case, actual, &run, |n| n.to_string());
            } else if bulk {
                batch.push(Logged {
                    key: path.clone(),
                    root: root.clone(),
                    projects: projects.clone(),
                    package: package.clone(),
                    expected: case.expected.clone(),
                });
            } else {
                let run = run_alone(&python, &root, &projects, &package);
                kit::check_log(&mut failures, path, &run, &case.expected);
            }
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
        run_batch(&python, &batch, &mut failures);
        // std's modules, once.
        let std_root = out.join("std");
        let _ = fs::remove_dir_all(&std_root);
        let files = translate(&kit::std_program(), &PY).unwrap();
        kit::write_files(
            &std_root,
            files.iter().map(|f| (f.path.as_str(), f.text.as_str())),
        );
        let found = library_modules("temper-std", &files, &[]);
        modules.extend(found.map(|f| std_root.join(&f.path)));
        check_module_names(&python, &modules, &mut failures);
        kit::assert_none(&failures);
    });
}

/// std's modules that build as a library of its own
/// (`temper_be_testkit::std_program`) translate to the `temper-std`
/// package, matching the goldens `std.*.py`; `runs` runs tests with
/// them.
#[test]
fn std_goldens() {
    kit::with_stack(|| {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
        let files = translate(&kit::std_program(), &PY).unwrap_or_else(|e| panic!("std: {e}"));
        for file in &files {
            let rest = file.path.strip_prefix("temper-std/").unwrap_or(&file.path);
            goldens.check(&format!("std.{}", rest.replace('/', ".")), &file.text);
        }
        goldens.finish();
    });
}
