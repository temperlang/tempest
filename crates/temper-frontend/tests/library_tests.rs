//! Tests of running libraries' tests ([`temper_frontend::library_tests`])
//! on small libraries written to a temporary directory, with std built in
//! or from a directory.

use std::fs;
use temper_frontend::library_tests::{parse_junit, test_dir};
use temper_frontend::project::{Config, LibrarySources, find_libraries, work_root};
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
/// what was written to `out` and `err`, verbosely, listing every test.
fn test(dir: &str) -> (bool, String, String) {
    test_with(dir, None)
}

/// [`test`], with std from `std_dir`, if given.
fn test_with(dir: &str, std_dir: Option<String>) -> (bool, String, String) {
    test_in(dir, true, move || match std_dir {
        Some(std_dir) => Sources::with_std_dir(&std_dir),
        None => Sources::standard(),
    })
}

/// [`test`], in the sources that `sources` makes.
fn test_in(
    dir: &str,
    verbose: bool,
    sources: impl FnOnce() -> Sources + Send + 'static,
) -> (bool, String, String) {
    let dir = dir.to_string();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let passed = test_dir(&sources(), &dir, verbose, &mut out, &mut err);
            let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
            (passed, text(out), text(err))
        })
        .unwrap()
        .join()
        .unwrap()
}

const CONFIG: &str = "# Lib\n\n    export let name = \"lib\";\n";

/// Another library's config, named for its first line.
const OTHER: &str = "# Other\n\nAnother library.\n";

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
fn quiet_by_default() {
    let tree = write_tree(
        "quiet",
        &[
            ("good/config.temper.md", CONFIG),
            ("good/a.temper", "test(\"fine\") { assert(true) }\n"),
            ("bad/config.temper.md", OTHER),
            (
                "bad/b.temper",
                "test(\"fine\") { assert(true) }\n\
                 test(\"broken\") { assert(1 == 2) }\n",
            ),
        ],
    );
    let root = &tree.root;
    let (passed, out, err) = test_in(root, false, Sources::standard);
    assert_eq!(err, "");
    // Like `temper test`: only failures, under their library, and the summary.
    assert_eq!(
        out,
        format!(
            "Testing {root}/bad\n  \
             failed  broken - expected 1 == (2) not (1)\n\
             Tests passed: 2 of 3\n"
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
            ("other/config.temper.md", OTHER),
        ],
    );
    let root = &tree.root;
    // From a library's subdirectory, `temper.out` marks the work root.
    let disk = Sources::standard();
    assert_eq!(work_root(&disk, &format!("{root}/lib/sub")), *root);
    let sources = |dir: &str, files: &[&str], config: &str| LibrarySources {
        root: format!("{root}/{dir}"),
        files: files.iter().map(|f| format!("{root}/{f}")).collect(),
        config: Config::read(config, dir),
    };
    assert_eq!(
        find_libraries(&disk, root),
        [
            sources("lib", &["lib/a.temper.md", "lib/sub/b.temper"], CONFIG),
            sources("other", &[], OTHER),
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
    let (passed, out, err) = test_in("app/b", true, move || Sources::memory(&files));
    assert_eq!(err, "");
    assert_eq!(out, "Testing app\n  ok      date\nTests passed: 1 of 1\n");
    assert!(passed);
}

/// Libraries' names, as Kotlin's `libraryNameFor` gives them: what the
/// config exports, else its first line as a slug, else the root's path.
#[test]
fn library_names() {
    let name = |text: &str, path: &str| Config::read(text, path).name;
    assert_eq!(name(CONFIG, "src"), "lib");
    assert_eq!(name("# Hi There!\n", "src"), "hi-there");
    assert_eq!(
        name("\n    export let version = \"1.2.0\";\n", "source/path"),
        "source-path"
    );
    assert_eq!(name("", ""), "unnamed");
    let config = Config::read(
        "# A\n\n    export let name = \"a\";\n    export let version = \"1.2.0\";\n",
        "",
    );
    assert_eq!(config.get("version"), Some("1.2.0"));
    assert_eq!(config.version_or_default(), "1.2.0");
    assert_eq!(Config::read(CONFIG, "").version_or_default(), "0.0.1");
}

/// A library imports another by its name, as `import("other-lib/sub")`,
/// and each runs only its own tests.
#[test]
fn libraries_import_each_other() {
    let tree = write_tree(
        "imports",
        &[
            ("config.temper.md", "# Root\n"),
            (
                "app/config.temper.md",
                "# App\n\n    export let name = \"app\";\n",
            ),
            (
                "app/a.temper",
                "let { twice } = import(\"util/math\");\n\
                 let { one } = import(\"util\");\n\
                 test(\"uses util\") { assert(twice(one) == 2) }\n",
            ),
            (
                "util/config.temper.md",
                "# Util\n\n    export let name = \"util\";\n",
            ),
            (
                "util/u.temper",
                "export let one = 1;\ntest(\"util's own\") { assert(one == 1) }\n",
            ),
            (
                "util/math/m.temper",
                "export let twice(x: Int): Int { x * 2 }\n",
            ),
        ],
    );
    let root = &tree.root;
    let (passed, out, err) = test(root);
    assert_eq!(err, "");
    assert!(out.contains("/app\n  ok      uses util\n"), "{out}");
    assert!(out.contains("/util\n  ok      util's own\n"), "{out}");
    assert!(out.ends_with("Tests passed: 2 of 2\n"), "{out}");
    assert!(passed);
}

/// Two libraries of one name are an error, as in Kotlin.
#[test]
fn duplicate_names() {
    let tree = write_tree(
        "duplicates",
        &[
            ("a/config.temper.md", CONFIG),
            ("b/config.temper.md", CONFIG),
            ("config.temper.md", "# Root\n"),
        ],
    );
    let root = &tree.root;
    let (passed, _, err) = test(root);
    assert_eq!(err, format!("Two libraries are named lib under {root}\n"));
    assert!(!passed);
}
