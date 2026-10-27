//! Tests of running libraries' tests ([`temper_frontend::library_tests`])
//! on small libraries written to a temporary directory, with std built in
//! or from a directory.

use std::fs;
use temper_frontend::library_tests::{parse_junit, test_dir};
use temper_frontend::project::{LibrarySources, find_libraries, work_root};
use temper_frontend::sources::Sources;
use temper_frontend::stdlib;

/// A temporary directory, removed when dropped.
struct TempTree {
    root: String,
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A fresh directory for a test, with these files, by relative path.
fn write_tree(test: &str, files: &[(&str, &str)]) -> TempTree {
    let dir = std::env::temp_dir().join(format!(
        "tempest-library-tests-{}-{test}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    for (path, text) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    TempTree {
        root: dir.to_string_lossy().replace('\\', "/"),
    }
}

/// Tests the libraries under `dir`, with std built in, on a thread with
/// room for the interpreter's recursion, giving whether all passed, and
/// what was written to `out` and `err`.
fn test(dir: &str) -> (bool, String, String) {
    test_with(dir, None)
}

/// [`test`], with std from `std_dir`, if given.
fn test_with(dir: &str, std_dir: Option<String>) -> (bool, String, String) {
    test_in(dir, move || match std_dir {
        Some(std_dir) => Sources::with_std_dir(&std_dir),
        None => Sources::standard(),
    })
}

/// [`test`], in the sources that `sources` makes.
fn test_in(
    dir: &str,
    sources: impl FnOnce() -> Sources + Send + 'static,
) -> (bool, String, String) {
    let dir = dir.to_string();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let passed = test_dir(&sources(), &dir, &mut out, &mut err);
            let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
            (passed, text(out), text(err))
        })
        .unwrap()
        .join()
        .unwrap()
}

const CONFIG: &str = "# Lib\n\n    export let name = \"lib\";\n";

#[test]
fn passing_and_failing_tests() {
    let tree = write_tree(
        "results",
        &[
            ("src/config.temper.md", CONFIG),
            (
                "src/a.temper",
                "export let double(x: Int): Int { x * 2 }\n\
                 test(\"doubles\") { assert(double(2) == 4) }\n\
                 test(\"<odd> & 'quoted'\") { assert(double(2) == 5) }\n",
            ),
        ],
    );
    let root = &tree.root;
    let (passed, out, err) = test(root);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "Testing {root}/src\n  \
             ok      doubles\n  \
             failed  <odd> & 'quoted' - expected double(2) == (5) not (4)\n\
             Tests passed: 1 of 2\n"
        )
    );
    assert!(!passed);
}

#[test]
fn work_root_and_skipped_directories() {
    let tree = write_tree(
        "root",
        &[
            ("temper.out/x", ""),
            ("lib/config.temper.md", CONFIG),
            (
                "lib/a.temper.md",
                "# A\n\n    test(\"a\") { assert(true) }\n",
            ),
            ("lib/sub/b.temper", "let b = 1;\n"),
            ("lib/.hidden/c.temper", "this is not Temper (\n"),
            ("lib/temper.keep/d.temper", "nor this (\n"),
            ("other/config.temper.md", CONFIG),
        ],
    );
    let root = &tree.root;
    // From a library's subdirectory, `temper.out` marks the work root.
    let disk = Sources::standard();
    assert_eq!(work_root(&disk, &format!("{root}/lib/sub")), *root);
    let sources = |dir: &str, files: &[&str]| LibrarySources {
        root: format!("{root}/{dir}"),
        files: files.iter().map(|f| format!("{root}/{f}")).collect(),
    };
    assert_eq!(
        find_libraries(&disk, root),
        [
            sources("lib", &["lib/a.temper.md", "lib/sub/b.temper"]),
            sources("other", &[]),
        ]
    );
    let (passed, out, _) = test(&format!("{root}/lib/sub"));
    assert!(out.contains("/lib\n  ok      a\nTesting "), "{out}");
    assert!(out.ends_with("/other\nTests passed: 1 of 1\n"), "{out}");
    assert!(passed);
}

#[test]
fn tests_not_run_and_errors() {
    let tree = write_tree(
        "not-run",
        &[
            ("config.temper.md", CONFIG),
            (
                "a.temper",
                "console.log(\"starting\");\nlet x = y;\ntest(\"a\") { assert(true) }\n",
            ),
        ],
    );
    let root = &tree.root;
    let (passed, out, err) = test(root);
    assert_eq!(
        err,
        format!("{root}/a.temper:2:9: error: y has not been declared\n")
    );
    assert_eq!(
        out,
        format!(
            "Testing {root}\nstarting\n  Failed: y has not been declared\nTests passed: 0 of 1 (1 not run)\n"
        )
    );
    assert!(!passed);
}

#[test]
fn no_libraries() {
    let tree = write_tree("empty", &[("a.temper", "let a = 1;\n")]);
    let root = &tree.root;
    let (passed, _, err) = test(root);
    assert_eq!(
        err,
        format!("No Temper library (config.temper.md) under {root}\n")
    );
    assert!(!passed);
}

#[test]
fn std_built_in_or_from_a_directory() {
    let testing = stdlib::FILES
        .iter()
        .find(|&&(path, _)| path == "testing/testing.temper.md")
        .unwrap()
        .1;
    let tree = write_tree(
        "std",
        &[
            ("lib/config.temper.md", CONFIG),
            (
                "lib/a.temper",
                "let { Date } = import(\"std/temporal\");\n\
                 test(\"date\") { assert((new Date(2024, 3, 5) orelse panic()).year == 2024) }\n",
            ),
            ("other/config.temper.md", CONFIG),
            (
                "other/b.temper",
                "let { m } = import(\"std/m\");\ntest(\"m\") { assert(m == 1) }\n",
            ),
            ("std/testing/testing.temper.md", testing),
            ("std/m/m.temper", "export let m = 1;\n"),
        ],
    );
    let root = &tree.root;
    // Built in: temporal and testing, but no `m`.
    let (passed, out, err) = test(&format!("{root}/lib"));
    assert_eq!(err, "");
    assert!(
        out.ends_with("  ok      date\nTests passed: 1 of 1\n"),
        "{out}"
    );
    assert!(passed);
    let (passed, _, err) = test(&format!("{root}/other"));
    assert!(err.contains("Import of std/m failed"), "{err}");
    assert!(!passed);
    // From a directory, which has `m` and testing.
    let std_dir = Some(format!("{root}/std"));
    let (passed, out, err) = test_with(&format!("{root}/other"), std_dir);
    assert_eq!(err, "");
    assert!(
        out.ends_with("  ok      m\nTests passed: 1 of 1\n"),
        "{out}"
    );
    assert!(passed);
}

#[test]
fn junit_reports() {
    let lines: Vec<String> = [
        "<testsuites>",
        "  <testsuite name='suite' tests='2' failures='1' time='0.0'>",
        "    <testcase name='a &amp; b' classname='a &amp; b' time='0.0' />",
        "    <testcase name='c' classname='c' time='0.0'>",
        "      <failure message='&lt;x&gt; &#39;y&#39; &#34;z&#34;' />",
        "    </testcase>",
        "  </testsuite>",
        "</testsuites>",
    ]
    .map(String::from)
    .into();
    assert_eq!(
        parse_junit(&lines),
        [
            ("a & b".to_string(), None),
            ("c".to_string(), Some("<x> 'y' \"z\"".to_string())),
        ]
    );
}

/// A library in memory, as the wasm build or an editor's unsaved
/// buffers would give it, with std built in.
#[test]
fn library_in_memory() {
    let files = [
        ("app/config.temper.md", CONFIG),
        (
            "app/a.temper",
            "let { Date } = import(\"std/temporal\");\nlet { b } = import(\"./b\");\n\
             test(\"date\") { assert((new Date(2024, 3, 5) orelse panic()).year == b) }\n",
        ),
        ("app/b/b.temper", "export let b = 2024;\n"),
    ];
    let sources = Sources::memory(&files);
    assert_eq!(work_root(&sources, "app/b"), "app");
    let (passed, out, err) = test_in("app/b", move || Sources::memory(&files));
    assert_eq!(err, "");
    assert_eq!(out, "Testing app\n  ok      date\nTests passed: 1 of 1\n");
    assert!(passed);
}
