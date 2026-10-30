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
    for backend in ["py", "rust", "js", "java", "csharp"] {
        // A backend left out fails the same way whatever the arguments.
        let code = if built_in(backend) { 2 } else { 1 };
        assert_eq!(tempest(&[backend]).0, code);
        assert_eq!(tempest(&[backend, "--out"]).0, code);
    }
    let (code, out, _) = tempest(&["--help"]);
    assert_eq!(
        (code, out.as_str()),
        (
            0,
            "usage: tempest test [--verbose] [--std DIR] [DIR]\n       \
             tempest py [--out DIR] [--std DIR] FILE...\n       \
             tempest rust [--out DIR] [--std DIR] FILE...\n       \
             tempest js [--out DIR] [--std DIR] FILE...\n       \
             tempest java [--out DIR] [--std DIR] FILE...\n       \
             tempest csharp [--out DIR] [--std DIR] FILE...\n"
        )
    );
}

/// Whether this build of `tempest` has a backend (a cargo feature).
fn built_in(backend: &str) -> bool {
    match backend {
        "py" => cfg!(feature = "py"),
        "rust" => cfg!(feature = "rust"),
        "js" => cfg!(feature = "js"),
        "java" => cfg!(feature = "java"),
        "csharp" => cfg!(feature = "csharp"),
        _ => unreachable!(),
    }
}

#[test]
fn backends_left_out() {
    for backend in ["py", "rust", "js", "java", "csharp"]
        .into_iter()
        .filter(|b| !built_in(b))
    {
        let (code, _, err) = tempest(&[backend, "a.temper"]);
        let expected = format!("tempest: built without the {backend} backend\n");
        assert_eq!((code, err.as_str()), (1, expected.as_str()));
    }
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
#[cfg(feature = "py")]
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
#[cfg(feature = "rust")]
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
#[cfg(feature = "rust")]
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

#[test]
#[cfg(feature = "js")]
fn js_prints_js() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let (code, out, err) = tempest(&["js", &fib.to_string_lossy()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("// ==> fibonacci/package.json\n"), "{out}");
    assert!(
        out.contains("// ==> fibonacci/fibonacci.internal.js\n"),
        "{out}"
    );
    assert!(out.contains("function fib(i) {\n"), "{out}");
}

#[test]
#[cfg(feature = "js")]
fn js_writes_a_package() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let out = std::env::temp_dir().join(format!("tempest-cli-{}-js", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let (code, stdout, err) = tempest(&[
        "js",
        "--out",
        &out.to_string_lossy(),
        &fib.to_string_lossy(),
    ]);
    assert_eq!((code, stdout.as_str()), (0, ""), "{err}");
    for file in [
        "package.json",
        "index.js",
        "fibonacci.js",
        "fibonacci.internal.js",
        "fibonacci.internal.js.map",
    ] {
        assert!(out.join("fibonacci").join(file).is_file(), "{file}");
    }
    // The map names the source by a path that leads to it.
    let module = fs::read_to_string(out.join("fibonacci/fibonacci.internal.js")).unwrap();
    assert!(module.ends_with("\n//# sourceMappingURL=fibonacci.internal.js.map\n"));
    let map = fs::read_to_string(out.join("fibonacci/fibonacci.internal.js.map")).unwrap();
    let source = map.split("\"sources\":[\"").nth(1).unwrap();
    let source = &source[..source.find('"').unwrap()];
    assert!(source.ends_with("/fibonacci.temper.md"), "{map}");
    assert_eq!(
        fs::canonicalize(out.join("fibonacci").join(source)).unwrap(),
        fs::canonicalize(&fib).unwrap()
    );
    let _ = fs::remove_dir_all(&out);
}

#[test]
#[cfg(feature = "java")]
fn java_prints_java() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let (code, out, err) = tempest(&["java", &fib.to_string_lossy()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("// ==> fibonacci/pom.xml\n"), "{out}");
    assert!(
        out.contains("// ==> fibonacci/src/main/java/fibonacci/FibonacciGlobal.java\n"),
        "{out}"
    );
    assert!(out.contains("    static int fib(int i) {\n"), "{out}");
}

#[test]
#[cfg(feature = "java")]
fn java_writes_a_package() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let out = std::env::temp_dir().join(format!("tempest-cli-{}-java", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let (code, stdout, err) = tempest(&[
        "java",
        "--out",
        &out.to_string_lossy(),
        &fib.to_string_lossy(),
    ]);
    assert_eq!((code, stdout.as_str()), (0, ""), "{err}");
    for file in [
        "pom.xml",
        "src/main/java/fibonacci/FibonacciGlobal.java",
        "src/main/java/fibonacci/FibonacciMain.java",
    ] {
        assert!(out.join("fibonacci").join(file).is_file(), "{file}");
    }
    let _ = fs::remove_dir_all(&out);
}

#[test]
#[cfg(feature = "csharp")]
fn csharp_prints_csharp() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let (code, out, err) = tempest(&["csharp", &fib.to_string_lossy()]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("// ==> fibonacci/src/Fibonacci.csproj\n"),
        "{out}"
    );
    assert!(
        out.contains("// ==> fibonacci/src/FibonacciGlobal.cs\n"),
        "{out}"
    );
    assert!(
        out.contains("    internal static int Fib(int i)\n"),
        "{out}"
    );
}

#[test]
#[cfg(feature = "csharp")]
fn csharp_writes_projects() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let out = std::env::temp_dir().join(format!("tempest-cli-{}-csharp", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let (code, stdout, err) = tempest(&[
        "csharp",
        "--out",
        &out.to_string_lossy(),
        &fib.to_string_lossy(),
    ]);
    assert_eq!((code, stdout.as_str()), (0, ""), "{err}");
    for file in [
        "src/Fibonacci.csproj",
        "src/FibonacciGlobal.cs",
        "src/Logging.cs",
        "program/FibonacciProgram.csproj",
        "program/Program.cs",
    ] {
        assert!(out.join("fibonacci").join(file).is_file(), "{file}");
    }
    let _ = fs::remove_dir_all(&out);
}
