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
    // Usage errors are 2, as before; Kotlin's are 127, kotlinx-cli's.
    assert_eq!(tempest(&["build", "-b"]).0, 2);
    assert_eq!(tempest(&["build", "-b", "cobol"]).0, 2);
    assert_eq!(tempest(&["build", "-b", "interp"]).0, 2);
    assert_eq!(tempest(&["build", "dir"]).0, 2);
    assert_eq!(tempest(&["build", "--bogus"]).0, 2);
    assert_eq!(tempest(&["build", "--logformat", "xml"]).0, 2);
    assert_eq!(tempest(&["build", "-w", "a", "-w", "b"]).0, 2);
    assert_eq!(tempest(&["build", "--no-connected=yes"]).0, 2);
    assert_eq!(tempest(&["test", "-b", "cobol"]).0, 2);
    assert_eq!(tempest(&["frobnicate"]).0, 2);
    assert_eq!(tempest(&[]).0, 2);
    assert_eq!(tempest(&["test", "a", "b"]).0, 2);
    assert_eq!(tempest(&["test", "-w", "a", "b"]).0, 2);
    assert_eq!(tempest(&["test", "--std"]).0, 2);
    assert_eq!(tempest(&["test", "--bench"]).0, 2);
    assert_eq!(tempest(&["run", "-b", "py"]).0, 2);
    assert_eq!(tempest(&["run", "--library", "a"]).0, 2);
    assert_eq!(
        tempest(&["run", "--library", "a", "-b", "py", "-b", "js"]).0,
        2
    );
    assert_eq!(tempest(&["init", "dir"]).0, 2);
    assert_eq!(tempest(&["lsp", "--stdio-ish"]).0, 2);
    assert_eq!(tempest(&["serve", "--tokens", "some"]).0, 2);
    assert_eq!(tempest(&["version", "now"]).0, 2);
    assert_eq!(tempest(&["help", "frobnicate"]).0, 2);
    // The one-backend commands are gone, and say what does their work.
    for backend in temper_driver::KNOWN {
        let (code, _, err) = tempest(&[backend, "a.temper"]);
        assert_eq!(code, 2);
        let expected = format!("`tempest build -b {backend}`");
        assert!(err.contains(&expected), "{err}");
    }
}

/// `help`, `--help`, and each command's `--help` print to stdout and
/// succeed, as Kotlin's.
#[test]
fn help() {
    for args in [&["help"][..], &["--help"], &["-h"]] {
        let (code, out, _) = tempest(args);
        assert_eq!(code, 0);
        assert!(
            out.starts_with("tempest, Temper's command line tool"),
            "{out}"
        );
        for command in [
            "build", "doc", "help", "init", "license", "licenses", "lsp", "repl", "run", "serve",
            "test", "version", "watch",
        ] {
            assert!(
                out.contains(&format!("\n    {command} - ")),
                "{command}: {out}"
            );
        }
    }
    let (code, out, _) = tempest(&["help", "build"]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with("usage: tempest build [OPTION]...\n"),
        "{out}"
    );
    for flag in [
        "-w, --workroot DIR",
        "-ign, --ignorefile FILE",
        "-o, --output-directory DIR",
        "-b, --backend BACKEND",
        "--no-connected",
        "-v, --verbose",
        "--logformat text|json",
        "--std DIR",
    ] {
        assert!(out.contains(flag), "{flag}: {out}");
    }
    // The work root as Kotlin's code finds it, which its help text
    // words the other way around (notes/kotlin-differences.md).
    let rule = "for the nearest\n            directory holding a temper.out directory, or else the\n            farthest holding a config.temper.md.";
    assert!(out.contains(rule), "{out}");
    assert_eq!(
        tempest(&["build", "--help"]),
        (0, out.clone(), String::new())
    );
    assert_eq!(tempest(&["build", "-h", "-b", "cobol"]).1, out);
    for command in [
        "test", "run", "init", "serve", "lsp", "version", "license", "licenses", "help",
    ] {
        let (code, out, _) = tempest(&["help", command]);
        assert_eq!(code, 0, "{command}");
        assert!(
            out.starts_with(&format!("usage: tempest {command}")),
            "{out}"
        );
    }
}

/// `version` names the build's version, and the commit where git says.
#[test]
fn version() {
    for flag in ["version", "--version"] {
        let (code, out, err) = tempest(&[flag]);
        assert_eq!((code, err.as_str()), (0, ""));
        assert!(out.starts_with("tempest 0."), "{out}");
        assert!(out.ends_with('\n') && out.lines().count() == 1, "{out}");
    }
}

/// `license` prints the repository's legal files, each under a heading,
/// as Kotlin's `temper license` does; `licenses`, the crates built in
/// (kept current by tests/licenses.rs).
#[test]
fn license_and_licenses() {
    let (code, out, _) = tempest(&["license"]);
    assert_eq!(code, 0);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for file in [
        "COPYRIGHT",
        "NOTICE",
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "LICENSE-CC-BY-SA-4.0",
    ] {
        let dashes = "-".repeat(file.len() + 8);
        let heading = format!("{dashes}\n--- {file} ---\n{dashes}\n");
        assert!(out.contains(&heading), "{file}");
        let text = fs::read_to_string(root.join(file)).unwrap();
        assert!(out.contains(&text), "{file}");
    }
    let (code, out, _) = tempest(&["licenses"]);
    assert_eq!(code, 0);
    let file =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/licenses.txt"));
    assert_eq!(out, file.unwrap());
    assert!(out.contains("\n--- serde_json "), "{out}");
}

/// What isn't in tempest yet says so, with a pointer, and exits with 3:
/// Kotlin's `watch`, `repl`, and `doc`, its backends `cppv`, `data`,
/// `java8`, and `mypyc`, and its language server's `--port`.
#[test]
fn not_yet() {
    for command in ["watch", "repl", "doc"] {
        let (code, out, err) = tempest(&[command, "-w", "."]);
        assert_eq!((code, out.as_str()), (3, ""), "{command}");
        let expected = format!(
            "tempest: {command} isn't in tempest yet. Kotlin Temper's `temper {command}` has it; "
        );
        assert!(err.starts_with(&expected), "{err}");
        assert!(err.ends_with(" (docs/tools/cli.md).\n"), "{err}");
        // Its help says the same.
        let (code, out, _) = tempest(&["help", command]);
        assert_eq!(code, 0);
        assert!(out.starts_with(&expected), "{out}");
    }
    for backend in temper_driver::NOT_YET {
        for args in [&["build", "-b", backend][..], &["test", "-b", backend]] {
            let (code, _, err) = tempest(args);
            let expected = format!("tempest: the {backend} backend isn't in tempest yet; ");
            assert_eq!(code, 3, "{args:?}");
            assert!(err.starts_with(&expected), "{err}");
        }
    }
    let (code, _, err) = tempest(&["serve", "--port", "4000"]);
    assert_eq!(code, 3);
    assert!(
        err.contains("over standard input and output, not a TCP port"),
        "{err}"
    );
}

/// Whether this build of `tempest` has a backend (a cargo feature).
fn built_in(backend: &str) -> bool {
    temper_driver::Registry::built_in().get(backend).is_some()
}

#[test]
fn backends_left_out() {
    for backend in temper_driver::KNOWN.into_iter().filter(|b| !built_in(b)) {
        let (code, _, err) = tempest(&["build", "-b", backend]);
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

/// Fibonacci, from Kotlin's functional tests, as a library of its name.
fn with_fibonacci(test: &str, f: impl FnOnce(&str)) {
    let fib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/functional-tests/algos/fibonacci/fibonacci.temper.md");
    let fib = fs::read_to_string(fib).unwrap();
    let files = [
        ("config.temper.md", "    export let name = \"fibonacci\";\n"),
        ("fibonacci.temper.md", fib.as_str()),
    ];
    with_library(test, &files, f);
}

/// `build -b BACKEND -o DIR`, which replaced `tempest BACKEND --out DIR`,
/// writes each backend's files under DIR, as Kotlin lays them out under
/// `temper.out`, and nothing in the work root.
#[test]
fn build_writes_each_backend_to_a_directory() {
    // Each backend, files it writes, and a line one of them has.
    let expected: [(&str, &[&str], &str, &str); 7] = [
        (
            "py",
            &["pyproject.toml", "fibonacci/fibonacci.py"],
            "fibonacci/fibonacci.py",
            "def _fib(i: int, /) -> int:\n",
        ),
        (
            "rust",
            &[
                "Cargo.toml",
                "src/lib.rs",
                "src/main.rs",
                "src/mod.rs",
                "src/support/mod.rs",
            ],
            "src/mod.rs",
            "pub(crate) fn fib(mut i: i32) -> i32 {\n",
        ),
        (
            "js",
            &[
                "package.json",
                "index.js",
                "fibonacci.js",
                "fibonacci.internal.js",
                "fibonacci.internal.js.map",
            ],
            "fibonacci.internal.js",
            "function fib(i) {\n",
        ),
        (
            "java",
            &[
                "pom.xml",
                "src/main/java/fibonacci/FibonacciGlobal.java",
                "src/main/java/fibonacci/FibonacciMain.java",
            ],
            "src/main/java/fibonacci/FibonacciGlobal.java",
            "    static int fib(int i) {\n",
        ),
        (
            "csharp",
            &[
                "src/Fibonacci.csproj",
                "src/FibonacciGlobal.cs",
                "src/Logging.cs",
                "program/FibonacciProgram.csproj",
                "program/Program.cs",
            ],
            "src/FibonacciGlobal.cs",
            "    internal static int Fib(int i)\n",
        ),
        (
            "cpp",
            &["init.hpp", "init.cpp", "main.cpp"],
            "init.cpp",
            "static int32_t fib(int32_t i) {\n",
        ),
        (
            "lua",
            &["init.lua", "fibonacci-dev-1.rockspec"],
            "init.lua",
            "local function fib(i)\n",
        ),
    ];
    with_fibonacci("out-dir", |dir| {
        let out = std::env::temp_dir().join(format!("tempest-cli-{}-out", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        for (backend, files, file, line) in expected {
            if !built_in(backend) {
                continue;
            }
            let out_dir = out.to_string_lossy();
            let (code, stdout, err) = tempest(&["build", "-w", dir, "-b", backend, "-o", &out_dir]);
            assert_eq!((code, stdout.as_str()), (0, ""), "{backend}: {err}");
            let package = out.join(backend).join("fibonacci");
            for file in files {
                assert!(package.join(file).is_file(), "{backend}: {file}");
            }
            let text = fs::read_to_string(package.join(file)).unwrap();
            assert!(text.contains(line), "{backend}: {text}");
            assert!(out.join(backend).join("temper-core").is_dir(), "{backend}");
        }
        assert!(!Path::new(dir).join("temper.out").exists());
        // JS's source map names the source by a path that leads to it.
        if built_in("js") {
            let package = out.join("js/fibonacci");
            let module = fs::read_to_string(package.join("fibonacci.internal.js")).unwrap();
            assert!(module.ends_with("\n//# sourceMappingURL=fibonacci.internal.js.map\n"));
            let map = fs::read_to_string(package.join("fibonacci.internal.js.map")).unwrap();
            let source = map.split("\"sources\":[\"").nth(1).unwrap();
            let source = &source[..source.find('"').unwrap()];
            assert!(source.ends_with("/fibonacci.temper.md"), "{map}");
            assert_eq!(
                fs::canonicalize(package.join(source)).unwrap(),
                fs::canonicalize(Path::new(dir).join("fibonacci.temper.md")).unwrap()
            );
        }
        let _ = fs::remove_dir_all(&out);
    });
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
/// backend's runner (its crate's `tools` module) is tested on every push without the
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

/// A library that doesn't build for a backend, as it uses what tmpl
/// doesn't cover yet, still counts its tests, as not run, so the summary
/// says how many there are.
#[test]
#[cfg(feature = "py")]
fn tests_of_a_library_that_does_not_build_count() {
    let files = [(
        "a.temper",
        "export let f(): Void? { null }\ntest(\"a\") { assert(true) }\ntest(\"b\") { assert(true) }\n",
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

/// Errors don't stop `build` or `test`, as in
/// Kotlin Temper: each is reported, and fails only where it runs, so
/// every backend's code is written, the tests that don't reach an error
/// pass, and the exit status is 1; a type-only error runs on as written
/// on the interpreter and Python (notes/kotlin-differences.md, "Keep on
/// trucking").
#[test]
#[cfg(feature = "py")]
fn errors_fail_only_where_they_run() {
    let files = [
        (
            "a.temper",
            "export let good(x: Int): Int { x + 1 }\n\
         export let mismatched(x: Int): Int { let s: String = x; 1 }\n\
         export let misspelled(s: String): Int { s.lenght }\n\
         export let typo(x: Nope): Int { 1 }\n\
         export let unclosed(x: Int: Int { x * 2 }\n\
         export let after(x: Int): Int { x * 3 }\n",
        ),
        (
            "tests.temper",
            "test(\"good\") { assert(good(1) == 2) }\n\
         test(\"after\") { assert(after(1) + typo(1) == 4) }\n\
         test(\"mismatched\") { assert(mismatched(1) == 1) }\n\
         test(\"misspelled\") { assert(misspelled(\"a\") == 1) }\n",
        ),
    ];
    with_library("errors", &files, |dir| {
        let errors = [
            "a.temper:2:54: error: Cannot assign to String from Int32",
            "a.temper:3:41: error: No member lenght in String",
            "a.temper:4:20: error: Nope has not been declared",
            "a.temper:5:27: error: Expected `)` to close the `(` on line 5",
        ];
        // Every backend's code, each error reported once, and 1.
        let (code, out, err) = tempest(&["build", "-w", dir]);
        for error in errors {
            assert_eq!(err.matches(error).count(), 1, "{error}:\n{err}");
        }
        assert!(err.ends_with("a: 4 errors\n"), "{err}");
        assert_eq!((code, out.as_str()), (1, ""), "{err}");
        for backend in ["csharp", "java", "js", "lua", "py", "rust"] {
            let written = Path::new(dir).join("temper.out").join(backend).join("a");
            assert!(written.is_dir(), "{backend}");
        }
        // One backend, to a directory: the code, and the errors.
        // Outside the library, where it would be host code.
        let py = format!("{dir}-py-out");
        let (code, _, err) = tempest(&["build", "-w", dir, "-b", "py", "-o", &py]);
        let text = fs::read_to_string(format!("{py}/py/a/a/a.py")).unwrap();
        assert!(text.contains("def after(x: int, /) -> int:"), "{text}");
        assert!(err.ends_with("a: 4 errors\n"), "{err}");
        assert_eq!(code, 1);
        let _ = fs::remove_dir_all(&py);
        // Tests run, on the interpreter and on a backend, those that
        // reach an error failing, but for a type-only error, which runs
        // on as written there (`mismatched`), still exiting with 1.
        let mut runs = vec![tempest(&["test", dir])];
        if Command::new("python3").arg("--version").output().is_ok() {
            runs.push(tempest(&["test", "-b", "py", dir]));
        }
        for (code, out, err) in runs {
            assert!(out.ends_with("Tests passed: 3 of 4\n"), "{out}{err}");
            assert!(!out.contains("  failed  mismatched - "), "{out}");
            assert!(out.contains("  failed  misspelled - "), "{out}");
            assert_eq!(code, 1);
        }
    });
}

/// `--logformat json` writes Kotlin's records, a line each, all to
/// stderr: each diagnostic with its position in UTF-16 code units, each
/// failed test, the summary, and the task's result last.
#[test]
fn logformat_json() {
    let files = [(
        "a.temper",
        "export let f(n: Int): Int { n * 3 }\n\
         let g(): Int { tripel(1) }\n\
         test(\"t1\") { assert(f(2) == 6); }\n\
         test(\"t2\") { assert(f(2) == 7); }\n",
    )];
    with_library("json", &files, |dir| {
        let undeclared = format!(
            "{{ \"level\": \"error\", \"template\": \"UndeclaredName\", \"pos\": {{ \"loc\": \
             \"{dir}/a.temper\", \"left\": 51, \"right\": 57 }}, \"values\": [], \"message\": \
             \"tripel has not been declared\" }}"
        );
        // A build needs a backend; one built without js (as CI's
        // --no-default-features run) skips this part.
        if built_in("js") {
            let (code, out, err) =
                tempest(&["build", "-w", dir, "-b", "js", "--logformat", "json"]);
            assert_eq!((code, out.as_str()), (1, ""), "{err}");
            assert_eq!(
                err,
                format!("{undeclared}\n{{ \"task\": \"Build\", \"ok\": false }}\n")
            );
        }
        let (code, out, err) = tempest(&["test", "-w", dir, "--logformat", "json"]);
        assert_eq!((code, out.as_str()), (1, ""), "{err}");
        let expected = [
            undeclared.as_str(),
            "{ \"level\": \"error\", \"template\": \"TestFailed\", \"values\": [ \"interp\", \
             \"t2\", \"expected f(2) == (7) not (6)\" ], \"message\": \"Test failed (interp): t2 \
             - expected f(2) == (7) not (6)\" }",
            "{ \"level\": \"summary\", \"template\": \"TestSummaryAllRun\", \"values\": [ 1, 2 ], \
             \"message\": \"Tests passed: 1 of 2\" }",
            "{ \"task\": \"Test\", \"ok\": false }",
        ];
        assert_eq!(err.lines().collect::<Vec<_>>(), expected);
        // Python's results, as Kotlin's records name the backend.
        if built_in("py") && Command::new("python3").arg("--version").output().is_ok() {
            let (code, _, err) = tempest(&["test", "-w", dir, "-b", "py", "--logformat", "json"]);
            assert_eq!(code, 1);
            assert!(err.contains("\"values\": [ \"py\", \"t2\", "), "{err}");
            assert!(
                err.ends_with("{ \"task\": \"Test\", \"ok\": false }\n"),
                "{err}"
            );
        }
        // Nothing to test is a record too.
        let (code, _, err) = tempest(&["test", "-w", dir, "--library", "b", "--logformat", "json"]);
        assert_eq!(code, 1);
        assert!(
            err.starts_with("{ \"level\": \"error\", \"template\": \"NoSuchLibrary\", "),
            "{err}"
        );
    });
}

/// `init` writes Kotlin's skeleton, byte for byte, named after its
/// directory, and won't write over a project.
#[test]
fn init_writes_kotlins_skeleton() {
    let base = std::env::temp_dir().join(format!("tempest-cli-{}-init", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let dir = base.join("my-Proj");
    let (code, out, err) = tempest(&["init", "-o", &dir.to_string_lossy()]);
    assert_eq!((code, err.as_str()), (0, ""));
    let real = fs::canonicalize(&dir).unwrap();
    assert_eq!(
        out,
        format!(
            "Initialized new Temper project \"my-proj\" in {}\n",
            real.display()
        )
    );
    // As Kotlin's `temper init -o my-Proj` writes them (2026-11-09).
    let config = "# My Proj\n\
                  \n\
                  This file is used for library configuration.\n\
                  \n\
                  The `name` export defines the library name:\n\
                  \n\
                  \x20   export let name = \"my-proj\";\n\
                  \n\
                  By default, the current directory is imported as a module. It can import\n\
                  other module directories. See Temper documentation for additional config\n\
                  options.\n";
    let module = "# Implementation for My Proj\n\
                  \n\
                  Library discussion goes here.\n\
                  \n\
                  \x20   // Library implementation code goes here.\n\
                  \n\
                  Additional documentation and code blocks may follow.\n";
    assert_eq!(
        fs::read_to_string(dir.join("src/config.temper.md")).unwrap(),
        config
    );
    assert_eq!(
        fs::read_to_string(dir.join("src/my-proj.temper.md")).unwrap(),
        module
    );
    assert_eq!(fs::read_dir(dir.join("src")).unwrap().count(), 2);
    // It builds and tests as a library.
    let (code, out, err) = tempest(&["test", "-w", &dir.to_string_lossy()]);
    assert_eq!((code, out.as_str()), (0, "Tests passed: 0 of 0\n"), "{err}");
    // Not over a project.
    let (code, out, err) = tempest(&["init", "-o", &dir.to_string_lossy()]);
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (1, "", "Directory src already exists\nInit failed\n")
    );
    let other = base.join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("config.temper.md"), "").unwrap();
    let (code, _, err) = tempest(&["init", "-o", &other.to_string_lossy()]);
    assert_eq!(
        (code, err.as_str()),
        (1, "Temper config already exists\nInit failed\n")
    );
    assert!(!other.join("src").exists());
    // `config` is `config-lib`, so it isn't the config file.
    let (code, out, _) = tempest(&["init", "-o", &base.join("config").to_string_lossy()]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with("Initialized new Temper project \"config-lib\" in "),
        "{out}"
    );
    assert!(base.join("config/src/config-lib.temper.md").is_file());
    let _ = fs::remove_dir_all(&base);
}

/// `run` builds the work root and runs a library's top level on the
/// interpreter and each backend whose tools are here (skipped where
/// they're missing, unless `TEMPEST_REQUIRE_CHECKERS` is set), what it
/// logs on stdout; a library that imports another runs with it.
#[test]
fn run_a_library() {
    let files = [
        ("a.temper", "export let triple(n: Int): Int { n * 3 }\n"),
        ("b/config.temper.md", "    export let name = \"b\";\n"),
        (
            "b/b.temper",
            "let { triple } = import(\"a\");\nconsole.log(\"hello ${triple(14).toString()}\");\n",
        ),
    ];
    with_library("run", &files, |dir| {
        let (code, out, err) = tempest(&["run", "-w", dir, "--library", "b", "-b", "interp"]);
        assert_eq!((code, out.as_str()), (0, "hello 42\n"), "{err}");
        assert!(!Path::new(dir).join("temper.out").exists());
        for backend in ["py", "js", "lua", "rust", "java", "cpp", "csharp"] {
            if !built_in(backend) {
                continue;
            }
            let (code, out, err) = tempest(&["run", "-w", dir, "--library", "b", "-b", backend]);
            if err.contains("couldn't run") {
                assert!(
                    std::env::var_os("TEMPEST_REQUIRE_CHECKERS").is_none(),
                    "{backend}'s tools are missing (needed with TEMPEST_REQUIRE_CHECKERS):\n{err}"
                );
                eprintln!("skipped {backend}: its tools are missing");
                continue;
            }
            assert_eq!((code, out.trim_end()), (0, "hello 42"), "{backend}: {err}");
            assert!(
                Path::new(dir)
                    .join("temper.out")
                    .join(backend)
                    .join("b")
                    .is_dir()
            );
        }
        // A library that isn't there.
        let (code, _, err) = tempest(&["run", "-w", dir, "--library", "c", "-b", "interp"]);
        assert_eq!(code, 1);
        assert!(err.contains("No library named c under "), "{err}");
    });
    // Errors don't stop it, but fail it.
    let files = [(
        "a.temper",
        "console.log(\"before\");\nlet x: Int = nope;\nconsole.log(\"after\");\n",
    )];
    with_library("run-errors", &files, |dir| {
        let (code, out, err) = tempest(&["run", "-w", dir, "--library", "a", "-b", "interp"]);
        assert!(out.starts_with("before\n"), "{out}");
        assert!(err.contains("nope has not been declared"), "{err}");
        assert_eq!(code, 1);
    });
}

/// `test`'s flags: `-w`, `--library`, several `-b` (each backend's
/// summary, then the total, as Kotlin's), and `--ignorefile` and
/// `.temperignore` beside `.gitignore`.
#[test]
fn test_flags() {
    let files = [
        ("a.temper", "test(\"a\") { assert(true) }\n"),
        ("b/config.temper.md", "    export let name = \"b\";\n"),
        ("b/b.temper", "test(\"b\") { assert(false) }\n"),
    ];
    with_library("test-flags", &files, |dir| {
        let (code, out, _) = tempest(&["test", "-w", dir]);
        assert!(out.ends_with("Tests passed: 1 of 2\n"), "{out}");
        assert_eq!(code, 1);
        let (code, out, _) = tempest(&["test", "-w", dir, "--library", "a"]);
        assert_eq!((code, out.as_str()), (0, "Tests passed: 1 of 1\n"));
        let (code, _, err) = tempest(&["test", "-w", dir, "--library", "c"]);
        assert_eq!(code, 1);
        assert_eq!(err, format!("No library named c under {dir}\n"));
        // Each backend in turn, then the total.
        if built_in("py") && Command::new("python3").arg("--version").output().is_ok() {
            let (code, out, err) = tempest(&[
                "test",
                "-w",
                dir,
                "--library",
                "a",
                "-b",
                "interp",
                "-b",
                "py",
            ]);
            assert_eq!(
                (code, out.as_str()),
                (
                    0,
                    "interp: Tests passed: 1 of 1\npy: Tests passed: 1 of 1\nTests passed: 2 of 2\n"
                ),
                "{err}"
            );
        }
        // An ignore file, from anywhere, relative to its own directory.
        let list = format!("{dir}-ignore");
        fs::write(&list, "b/\n").unwrap();
        let (code, out, _) = tempest(&["test", "-w", dir, "-ign", &list]);
        assert_eq!((code, out.as_str()), (0, "Tests passed: 1 of 1\n"));
        let (code, out, _) = tempest(&["test", "-w", dir, "--ignorefile", "/dev/null"]);
        assert!(out.ends_with("Tests passed: 1 of 2\n"), "{out}");
        assert_eq!(code, 1);
        let (code, _, err) = tempest(&["test", "-w", dir, "-ign", &format!("{list}-not")]);
        assert_eq!(code, 1);
        assert!(err.starts_with("tempest: --ignorefile "), "{err}");
        // A .temperignore is read as a .gitignore is, after it.
        fs::write(Path::new(dir).join(".temperignore"), "b/\n").unwrap();
        let (code, out, _) = tempest(&["test", "-w", dir]);
        assert_eq!((code, out.as_str()), (0, "Tests passed: 1 of 1\n"));
        fs::write(Path::new(dir).join(".gitignore"), "b/\n").unwrap();
        fs::write(Path::new(dir).join(".temperignore"), "!b/\n").unwrap();
        let (_, out, _) = tempest(&["test", "-w", dir]);
        assert!(out.ends_with("Tests passed: 1 of 2\n"), "{out}");
        // And --ignorefile has the last word over both.
        let (_, out, _) = tempest(&["test", "-w", dir, "-ign", &list]);
        assert_eq!(out, "Tests passed: 1 of 1\n");
        fs::write(&list, "!b/\n").unwrap();
        fs::remove_file(Path::new(dir).join(".temperignore")).unwrap();
        let (_, out, _) = tempest(&["test", "-w", dir, "-ign", &list]);
        assert!(out.ends_with("Tests passed: 1 of 2\n"), "{out}");
        // `run` and `build` take it too, where Kotlin's `run` reads no
        // ignore file at all.
        fs::write(&list, "b/\n").unwrap();
        let (code, _, err) = tempest(&[
            "run",
            "-w",
            dir,
            "--library",
            "b",
            "-b",
            "interp",
            "-ign",
            &list,
        ]);
        assert_eq!(code, 1);
        assert!(err.contains("No library named b under "), "{err}");
        if built_in("js") {
            let (code, _, err) = tempest(&["build", "-w", dir, "-b", "js", "-ign", &list]);
            assert_eq!(code, 0, "{err}");
            assert!(!Path::new(dir).join("temper.out/js/b").exists());
        }
        let _ = fs::remove_file(&list);
    });
}
