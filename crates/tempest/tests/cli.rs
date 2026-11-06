//! Tests of the `tempest` binary's command line. What `test` reports is
//! tested in temper-frontend's `tests/library_tests.rs`.

use std::fs;
use std::path::{Path, PathBuf};
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
    assert_eq!(tempest(&["build", "-b"]).0, 2);
    assert_eq!(tempest(&["build", "-b", "cobol"]).0, 2);
    assert_eq!(tempest(&["build", "dir"]).0, 2);
    assert_eq!(tempest(&["test", "-b", "cobol"]).0, 2);
    assert_eq!(tempest(&["frobnicate"]).0, 2);
    assert_eq!(tempest(&[]).0, 2);
    assert_eq!(tempest(&["test", "a", "b"]).0, 2);
    assert_eq!(tempest(&["test", "--std"]).0, 2);
    assert_eq!(tempest(&["test", "--bench"]).0, 2);
    for backend in ["py", "rust", "js", "java", "csharp", "cpp", "lua"] {
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
            "usage: tempest build [-w DIR] [-b BACKEND]... [--no-connected] [--verbose] [--std DIR]\n       \
             tempest test [-b interp|BACKEND] [--verbose] [--std DIR] [DIR]\n       \
             tempest py [--out DIR] [--std DIR] FILE...\n       \
             tempest rust [--out DIR] [--std DIR] FILE...\n       \
             tempest js [--out DIR] [--std DIR] FILE...\n       \
             tempest java [--out DIR] [--std DIR] FILE...\n       \
             tempest csharp [--out DIR] [--std DIR] FILE...\n       \
             tempest cpp [--out DIR] [--std DIR] FILE...\n       \
             tempest lua [--out DIR] [--std DIR] FILE...\n"
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
        "cpp" => cfg!(feature = "cpp"),
        "lua" => cfg!(feature = "lua"),
        _ => unreachable!(),
    }
}

#[test]
fn backends_left_out() {
    for backend in ["py", "rust", "js", "java", "csharp", "cpp", "lua"]
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
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
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
        // `-b interp`, as Kotlin's `temper test` takes it, is the same.
        let (code, out, _) = tempest(&["test", "-b", "interp", "--std", &std_dir, dir]);
        assert_eq!(out, "Tests passed: 1 of 1\n");
        assert_eq!(code, 0);
        // On a backend, it builds for it and runs the tests with the
        // language's own tools, here Python's, if there's a `python3`.
        let (code, out, err) = tempest(&["test", "-b", "py", dir]);
        if !cfg!(feature = "py") {
            assert!(
                err.contains("tempest: built without the py backend"),
                "{err}"
            );
            assert_eq!(code, 1);
        } else if Command::new("python3").arg("--version").output().is_ok() {
            assert_eq!((code, out.as_str()), (0, "Tests passed: 1 of 1\n"), "{err}");
            assert!(Path::new(dir).join("temper.out/py/std").is_dir());
        }
        // A failing test fails the run, saying why.
        fs::write(
            Path::new(dir).join("b.temper"),
            "test(\"b\") { assert(1 + 1 == 3) }\n",
        )
        .unwrap();
        if cfg!(feature = "py") && Command::new("python3").arg("--version").output().is_ok() {
            let (code, out, _) = tempest(&["test", "-b", "py", dir]);
            assert!(
                out.contains("  failed  b - expected 1 + 1 == (3) not (2)"),
                "{out}"
            );
            assert!(out.ends_with("Tests passed: 1 of 2\n"), "{out}");
            assert_eq!(code, 1);
        }
        fs::remove_file(Path::new(dir).join("b.temper")).unwrap();
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

#[test]
#[cfg(feature = "cpp")]
fn cpp_prints_cpp() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let (code, out, err) = tempest(&["cpp", &fib.to_string_lossy()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("// ==> fibonacci/init.hpp\n"), "{out}");
    assert!(out.contains("// ==> fibonacci/init.cpp\n"), "{out}");
    assert!(out.contains("static int32_t fib(int32_t i) {\n"), "{out}");
}

#[test]
#[cfg(feature = "cpp")]
fn cpp_writes_files() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let out = std::env::temp_dir().join(format!("tempest-cli-{}-cpp", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let (code, stdout, err) = tempest(&[
        "cpp",
        "--out",
        &out.to_string_lossy(),
        &fib.to_string_lossy(),
    ]);
    assert_eq!((code, stdout.as_str()), (0, ""), "{err}");
    for file in ["init.hpp", "init.cpp", "main.cpp"] {
        assert!(out.join("fibonacci").join(file).is_file(), "{file}");
    }
    let _ = fs::remove_dir_all(&out);
}

#[test]
#[cfg(feature = "lua")]
fn lua_prints_lua() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let (code, out, err) = tempest(&["lua", &fib.to_string_lossy()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("-- ==> fibonacci/init.lua\n"), "{out}");
    assert!(
        out.contains("-- ==> fibonacci/fibonacci-dev-1.rockspec\n"),
        "{out}"
    );
    assert!(out.contains("local function fib(i)\n"), "{out}");
}

#[test]
#[cfg(feature = "lua")]
fn lua_writes_files() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let out = std::env::temp_dir().join(format!("tempest-cli-{}-lua", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let (code, stdout, err) = tempest(&[
        "lua",
        "--out",
        &out.to_string_lossy(),
        &fib.to_string_lossy(),
    ]);
    assert_eq!((code, stdout.as_str()), (0, ""), "{err}");
    for file in ["init.lua", "fibonacci-dev-1.rockspec"] {
        assert!(out.join("fibonacci").join(file).is_file(), "{file}");
    }
    let _ = fs::remove_dir_all(&out);
}

/// `tempest build` writes each library under `temper.out`, laid out as
/// Kotlin's, with the runtime beside it, and says what it can't build.
#[test]
#[cfg(all(feature = "py", feature = "lua", feature = "cpp"))]
fn build_writes_temper_out() {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let files = [
        ("fib.temper.md", fs::read_to_string(fib).unwrap()),
        (
            "other/config.temper.md",
            "# Other\n\n    export let name = \"other\";\n    export let version = \"1.2.0\";\n"
                .into(),
        ),
        ("other/o.temper", "export let o = 1;\n".into()),
        (
            "user/config.temper.md",
            "# User\n\n    export let name = \"user\";\n".into(),
        ),
        (
            "user/u.temper",
            "let { o } = import(\"other\");\nconsole.log(o.toString());\n".into(),
        ),
        (
            "bad/config.temper.md",
            "# Bad\n\n    export let name = \"bad\";\n".into(),
        ),
        (
            "bad/b.temper",
            "export let f(var x: Int = 1): Int { x }\n".into(),
        ),
    ];
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    with_library("build", &files, |dir| {
        let (code, out, err) = tempest(&[
            "build", "-w", dir, "-b", "py", "-b", "lua", "-b", "cpp", "-v",
        ]);
        // `a`, the root library, holds `fib`; `other` builds; `user`
        // imports `other`, naming it as a dependency; `bad` has a `var`
        // parameter's default, which tmpl can't translate yet.
        // With `-v`, every gap in tmpl first, then each backend's line.
        assert_eq!(code, 1, "{out}{err}");
        let lines: Vec<&str> = err.lines().collect();
        assert_eq!(lines.len(), 4, "{err}");
        assert!(
            lines[0].starts_with("bad: not yet in tmpl: a var parameter's default at "),
            "{err}"
        );
        for (line, backend) in lines[1..].iter().zip(["py", "lua", "cpp"]) {
            let start = format!("bad: {backend}: not yet in tmpl: a var parameter's default at ");
            assert!(line.starts_with(&start), "{err}");
        }
        let out_dir = Path::new(dir).join("temper.out");
        for file in [
            "py/a/pyproject.toml",
            "py/a/a/a.py",
            "py/other/pyproject.toml",
            "py/temper-core/temper_core/__init__.py",
            "lua/a/init.lua",
            "lua/other/other-1.2.0-1.rockspec",
            "lua/temper-core/temper-core-0.6.0-1.rockspec",
            "cpp/a/CMakeLists.txt",
            "cpp/temper-core/core.hpp",
        ] {
            assert!(out_dir.join(file).is_file(), "{file}");
        }
        let pyproject = fs::read_to_string(out_dir.join("py/other/pyproject.toml")).unwrap();
        assert!(pyproject.contains("version = \"1.2.0\"\n"), "{pyproject}");
        let pyproject = fs::read_to_string(out_dir.join("py/a/pyproject.toml")).unwrap();
        assert!(pyproject.contains("version = \"0.0.1\"\n"), "{pyproject}");
        let pyproject = fs::read_to_string(out_dir.join("py/user/pyproject.toml")).unwrap();
        assert!(pyproject.contains("\"other==1.2.0\""), "{pyproject}");
        let user = fs::read_to_string(out_dir.join("py/user/user/user.py")).unwrap();
        assert!(user.contains("from other import o as _o\n"), "{user}");
        // A file left from before is gone.
        fs::write(out_dir.join("py/a/stale.py"), "").unwrap();
        let (code, _, _) = tempest(&["build", "-w", dir, "-b", "py"]);
        assert_eq!(code, 1);
        assert!(!out_dir.join("py/a/stale.py").exists());
    });
}

/// A library's host code goes with it: Python takes `__connected__.py`
/// for a connected function, or its Temper body under `--no-connected`.
#[test]
#[cfg(feature = "py")]
fn build_reads_host_code() {
    let files = [
        (
            "c.temper",
            "@connected([\"py\"])\nexport let twice(i: Int): Int { i * 2 + 1000 }\n",
        ),
        (
            "__connected__.py",
            "class _connected:\n    def twice(i: int) -> int:\n        return i * 2\n",
        ),
    ];
    with_library("build-host", &files, |dir| {
        let out = Path::new(dir).join("temper.out/py/a/a/a.py");
        let (code, _, err) = tempest(&["build", "-w", dir, "-b", "py"]);
        assert_eq!(code, 0, "{err}");
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("class _connected:"), "{text}");
        assert!(text.contains("return _connected.twice(i)"), "{text}");
        let (code, _, err) = tempest(&["build", "-w", dir, "-b", "py", "--no-connected"]);
        assert_eq!(code, 0, "{err}");
        let text = fs::read_to_string(&out).unwrap();
        assert!(!text.contains("return _connected.twice(i)"), "{text}");
    });
}

/// Two libraries of one name, or none, fail the build.
#[test]
fn build_needs_libraries_with_names_of_their_own() {
    let files = [("b/config.temper.md", "    export let name = \"a\";\n")];
    with_library("build-names", &files, |dir| {
        let (code, _, err) = tempest(&["build", "-w", dir]);
        assert_eq!(code, 1);
        assert_eq!(err, format!("Two libraries are named a under {dir}\n"));
    });
    let empty = std::env::temp_dir().join(format!("tempest-cli-{}-empty", std::process::id()));
    fs::create_dir_all(&empty).unwrap();
    let (code, _, err) = tempest(&["build", "-w", &empty.to_string_lossy()]);
    assert_eq!(code, 1);
    assert!(err.starts_with("No Temper library"), "{err}");
    let _ = fs::remove_dir_all(&empty);
}

/// A C++ library's generated CMake project builds and runs, with g++,
/// where CMake is installed (skipped, with a message, otherwise; required
/// with `TEMPEST_REQUIRE_CHECKERS`).
#[test]
#[cfg(feature = "cpp")]
fn cmake_builds_a_library() {
    let cmake_runs = Command::new("cmake")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !cmake_runs {
        assert!(
            std::env::var_os("TEMPEST_REQUIRE_CHECKERS").is_none(),
            "no cmake (needed with TEMPEST_REQUIRE_CHECKERS)"
        );
        eprintln!("skipped: no cmake");
        return;
    }
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let files = [("fib.temper.md", fs::read_to_string(fib).unwrap())];
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    with_library("cmake", &files, |dir| {
        let (code, _, err) = tempest(&["build", "-w", dir, "-b", "cpp"]);
        assert_eq!(code, 0, "{err}");
        let source = Path::new(dir).join("temper.out/cpp/a");
        let build = Path::new(dir).join("cmake-build");
        let run = |args: &[&std::ffi::OsStr]| {
            let out = Command::new("cmake").args(args).output().unwrap();
            assert!(
                out.status.success(),
                "cmake {args:?}:\n{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&[
            "-S".as_ref(),
            source.as_os_str(),
            "-B".as_ref(),
            build.as_os_str(),
        ]);
        run(&[
            "--build".as_ref(),
            build.as_os_str(),
            "-j".as_ref(),
            "2".as_ref(),
        ]);
        let out = Command::new(build.join("a-main")).output().unwrap();
        let expected = temper_funtests::test("algos/fibonacci")
            .unwrap()
            .expected_log("cpp");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(stdout.lines().collect::<Vec<_>>(), expected);
    });
}

/// `tempest test -b cpp` builds a library's tests with CMake and runs
/// them, reading the JUnit XML they write, where CMake is installed
/// (skipped otherwise; required with `TEMPEST_REQUIRE_CHECKERS`).
#[test]
#[cfg(feature = "cpp")]
fn test_a_library_in_cpp() {
    let cmake_runs = Command::new("cmake")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !cmake_runs {
        assert!(
            std::env::var_os("TEMPEST_REQUIRE_CHECKERS").is_none(),
            "no cmake (needed with TEMPEST_REQUIRE_CHECKERS)"
        );
        eprintln!("skipped: no cmake");
        return;
    }
    let files = [
        ("a.temper", "test(\"a\") { assert(1 + 1 == 2) }\n"),
        ("b.temper", "test(\"b\") { assert(1 + 1 == 3) }\n"),
    ];
    with_library("cpp-tests", &files, |dir| {
        let (code, out, err) = tempest(&["test", "-b", "cpp", dir]);
        assert!(out.contains("  failed  b - expected"), "{out}{err}");
        assert!(out.ends_with("Tests passed: 1 of 2\n"), "{out}{err}");
        assert_eq!(code, 1);
        assert!(
            Path::new(dir)
                .join("temper.out/cpp/a/test-results.xml")
                .is_file()
        );
    });
}

/// `tempest test -b BACKEND` on a small library, one test passing and
/// one failing, on every backend whose tools run here: it builds, runs
/// the language's own test tools, and reads their results, so each
/// backend's runner in `test_run.rs` is tested on every push without the
/// real libraries, which run nightly. A backend whose tools are missing is
/// skipped, with a message, unless `TEMPEST_REQUIRE_CHECKERS` is set.
#[test]
fn test_a_library_on_each_backend() {
    let files = [
        ("a.temper", "test(\"a\") { assert(1 + 1 == 2) }\n"),
        ("b.temper", "test(\"b\") { assert(1 + 1 == 3) }\n"),
    ];
    for backend in ["js", "lua", "rust", "java", "csharp"] {
        if !built_in(backend) {
            continue;
        }
        with_library(&format!("each-{backend}"), &files, |dir| {
            let (code, out, err) = tempest(&["test", "-b", backend, dir]);
            if out.contains("couldn't run") || err.contains("couldn't run") {
                assert!(
                    std::env::var_os("TEMPEST_REQUIRE_CHECKERS").is_none(),
                    "{backend}'s tools are missing (needed with TEMPEST_REQUIRE_CHECKERS):\n{out}{err}"
                );
                eprintln!("skipped {backend}: its tools are missing");
                return;
            }
            // As each tool names it: `b`, or C#'s method `B`.
            let failed = out.to_lowercase().contains("  failed  b - expected");
            assert!(failed, "{backend}:\n{out}{err}");
            assert!(
                out.ends_with("Tests passed: 1 of 2\n"),
                "{backend}:\n{out}{err}"
            );
            assert_eq!(code, 1, "{backend}");
        });
    }
}

/// A library that doesn't build for a backend still counts its tests, as
/// not run, so the summary says how many there are.
#[test]
#[cfg(feature = "py")]
fn tests_of_a_library_that_does_not_build_count() {
    let files = [(
        "a.temper",
        "let x: Int = \"s\";\ntest(\"a\") { assert(true) }\ntest(\"b\") { assert(true) }\n",
    )];
    with_library("unbuilt", &files, |dir| {
        let (code, out, err) = tempest(&["test", "-b", "py", dir]);
        assert!(
            out.ends_with("Tests passed: 0 of 2 (2 not run)\n"),
            "{out}{err}"
        );
        assert_eq!(code, 1);
    });
}
