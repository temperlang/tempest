//! Tests of the `tempest` binary's command line. What `test` reports is
//! tested in temper-frontend's `tests/library_tests.rs`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Runs `tempest`, giving its exit code, stdout and stderr.
fn tempest(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_tempest"))
        .args(args)
        .output()
        .unwrap();
    let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
    (
        output.status.code().unwrap(),
        text(output.stdout),
        text(output.stderr),
    )
}

#[test]
fn usage() {
    assert_eq!(tempest(&["build"]).0, 2);
    assert_eq!(tempest(&[]).0, 2);
    assert_eq!(tempest(&["test", "a", "b"]).0, 2);
    assert_eq!(tempest(&["test", "--std"]).0, 2);
    assert_eq!(tempest(&["test", "--bench"]).0, 2);
    assert_eq!(tempest(&["py"]).0, 2);
    assert_eq!(tempest(&["py", "--out"]).0, 2);
    assert_eq!(tempest(&["rust"]).0, 2);
    assert_eq!(tempest(&["rust", "--out"]).0, 2);
    let (code, out, _) = tempest(&["--help"]);
    assert_eq!(
        (code, out.as_str()),
        (
            0,
            "usage: tempest test [--verbose] [--std DIR] [DIR]\n       \
             tempest py [--out DIR] [--std DIR] FILE...\n       \
             tempest rust [--out DIR] [--std DIR] FILE...\n"
        )
    );
}

/// A fresh directory for a test, with a library of these files beside its
/// config file, removed when the test is done.
fn with_library(test: &str, files: &[(&str, &str)], f: impl FnOnce(&str)) {
    let dir = std::env::temp_dir().join(format!("tempest-cli-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.temper.md"),
        "    export let name = \"a\";\n",
    )
    .unwrap();
    for (name, text) in files {
        fs::write(dir.join(name), text).unwrap();
    }
    f(&dir.to_string_lossy());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_a_library() {
    let files = [("a.temper", "test(\"a\") { assert(1 + 1 == 2) }\n")];
    with_library("plain", &files, |dir| {
        let std_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/std");
        let std_dir = std_dir.to_string_lossy();
        // Only the summary, as `temper test` prints when all pass.
        let (code, out, err) = tempest(&["test", "--std", &std_dir, dir]);
        assert_eq!(err, "");
        assert_eq!(out, "Tests passed: 1 of 1\n");
        assert_eq!(code, 0);
        // With --verbose, every test's result too.
        let (code, out, _) = tempest(&["test", "-v", "--std", &std_dir, dir]);
        assert!(
            out.ends_with("  ok      a\nTests passed: 1 of 1\n"),
            "{out}"
        );
        assert_eq!(code, 0);
    });
}

/// Std is built in, and `--std` replaces it.
#[test]
fn built_in_std() {
    let files = [(
        "a.temper",
        "let { Date } = import(\"std/temporal\");\n\
         test(\"a\") { assert((new Date(2024, 3, 5) orelse panic()).year == 2024) }\n",
    )];
    with_library("std", &files, |dir| {
        let (code, out, err) = tempest(&["test", dir]);
        assert_eq!(err, "");
        assert_eq!(out, "Tests passed: 1 of 1\n");
        assert_eq!(code, 0);
        let (code, _, err) = tempest(&["test", "--std", dir, dir]);
        assert!(err.contains("Import of std/temporal failed"), "{err}");
        assert_eq!(code, 1);
    });
}

#[test]
fn py_prints_python() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let (code, out, err) = tempest(&["py", &fib.to_string_lossy()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("# ==> fibonacci/pyproject.toml\n"), "{out}");
    assert!(
        out.contains("# ==> fibonacci/fibonacci/fibonacci.py\n"),
        "{out}"
    );
    assert!(out.contains("def _fib(i: int, /) -> int:\n"), "{out}");
}

#[test]
fn rust_prints_rust() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let (code, out, err) = tempest(&["rust", &fib.to_string_lossy()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("// ==> fibonacci/Cargo.toml\n"), "{out}");
    assert!(out.contains("// ==> fibonacci/src/mod.rs\n"), "{out}");
    assert!(
        out.contains("pub(crate) fn fib(mut i: i32) -> i32 {\n"),
        "{out}"
    );
}

#[test]
fn rust_writes_a_crate() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let out = std::env::temp_dir().join(format!("tempest-cli-{}-rust", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let (code, stdout, err) = tempest(&[
        "rust",
        "--out",
        &out.to_string_lossy(),
        &fib.to_string_lossy(),
    ]);
    assert_eq!((code, stdout.as_str()), (0, ""), "{err}");
    for file in [
        "Cargo.toml",
        "src/lib.rs",
        "src/main.rs",
        "src/mod.rs",
        "src/support/mod.rs",
    ] {
        assert!(out.join("fibonacci").join(file).is_file(), "{file}");
    }
    let _ = fs::remove_dir_all(&out);
}
