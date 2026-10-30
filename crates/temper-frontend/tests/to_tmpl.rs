//! Tests of the translation into tmpl ([`temper_frontend::to_tmpl`]) on
//! Kotlin Temper's functional tests, from the vendored copy.
//!
//! - `subset_a_goldens`: each test that subset A covers translates, the
//!   validator finds no problems, the text round trips (with its spans),
//!   and its canonical text, without spans, matches the golden in
//!   `tests/to_tmpl/`, named for the test's path (`algos-fibonacci.tmpl`).
//!   To accept new or changed output, run with `UPDATE_SNAPSHOTS=1` and
//!   review the diff.
//! - `every_functional_test_translates_or_says_why`: no functional test
//!   panics the translation or gives tmpl the validator rejects; the rest
//!   say what isn't covered yet.

use std::{env, fs, path::PathBuf};
use temper_frontend::library::Library;
use temper_frontend::project::load_imports;
use temper_frontend::settle::{Settled, settle};
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::{NotYet, to_tmpl};
use temper_syntax::lex::SourceKind;
use temper_tmpl::{Profile, Program, difference, text, validate};

/// The functional tests subset A covers, by their paths.
const SUBSET_A: [&str; 7] = [
    "algos/fibonacci",
    "algos/hello-world",
    "control-flow/if-return",
    "control-flow/loop-reenterable",
    "names/nonascii",
    "types/int/basics",
    "types/string/is-empty",
];

fn suite() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/functional-tests")
}

/// A functional test, settled as `check_functional.py` builds it: its
/// directory's own files, the main one first, and std's that it imports.
fn settled(test: &str) -> Settled {
    let dir = suite().join(test);
    let name = dir.file_name().unwrap().to_string_lossy().to_string();
    let main = dir.join(format!("{name}.temper.md"));
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.to_string_lossy();
            *p != main && (n.ends_with(".temper") || n.ends_with(".temper.md"))
        })
        .collect();
    files.sort();
    files.insert(0, main);
    let mut library = Library::new();
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        let path = path.to_string_lossy().replace('\\', "/");
        let kind = if path.ends_with(".md") {
            SourceKind::Markdown
        } else {
            SourceKind::Standalone
        };
        library.add_file(&path, &text, kind);
    }
    load_imports(&Sources::standard(), &mut library);
    settle(library)
}

/// A test's library, translated, under its directory's name.
fn translate(test: &str) -> Result<Program, NotYet> {
    let name = test.rsplit('/').next().unwrap();
    to_tmpl(&settled(test), name)
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
fn subset_a_goldens() {
    with_stack(|| {
        let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
        let goldens = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/to_tmpl");
        let mut differ = Vec::new();
        for test in SUBSET_A {
            let program = translate(test).unwrap_or_else(|e| panic!("{test}: {e}"));
            let problems = validate(&program, &Profile::full());
            assert!(problems.is_empty(), "{test}: {problems:#?}");
            assert!(program.has_spans, "{test} has no spans");

            // With spans: read back to an equal program, and the same text.
            let full = text::write(&program);
            let back = text::read(&full).unwrap_or_else(|e| panic!("{test}: {e}\n{full}"));
            assert_eq!(difference(&program, &back), None, "{test}");
            assert_eq!(text::write(&back), full, "{test}");

            let mut plain = program.clone();
            plain.has_spans = false;
            let canonical = text::write(&plain);
            let path = goldens.join(format!("{}.tmpl", test.replace('/', "-")));
            let expected = fs::read_to_string(&path).unwrap_or_default();
            if canonical != expected {
                if update {
                    fs::create_dir_all(&goldens).unwrap();
                    fs::write(&path, &canonical).unwrap();
                } else {
                    differ.push(format!("{}:\n{canonical}", path.display()));
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

#[test]
fn every_functional_test_translates_or_says_why() {
    with_stack(|| {
        let root = suite();
        let mut tests = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_string_lossy();
                    if path.join(format!("{name}.temper.md")).exists() {
                        let test = path.strip_prefix(&root).unwrap();
                        tests.push(test.to_string_lossy().replace('\\', "/"));
                    }
                    stack.push(path);
                }
            }
        }
        tests.sort();
        assert!(tests.len() >= 69, "{} functional tests", tests.len());
        let mut ok = Vec::new();
        for test in &tests {
            match translate(test) {
                Ok(program) => {
                    let problems = validate(&program, &Profile::full());
                    assert!(problems.is_empty(), "{test}: {problems:#?}");
                    ok.push(test.as_str());
                }
                Err(e) => assert!(!e.what.is_empty(), "{test}"),
            }
        }
        assert_eq!(ok, SUBSET_A, "the functional tests that translate");
    });
}
