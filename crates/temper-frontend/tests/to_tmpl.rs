//! Tests of the translation into tmpl ([`temper_frontend::to_tmpl`]) on
//! Kotlin Temper's functional tests, from the vendored copy.
//!
//! - `subset_a_goldens`: each test that subset A covers, and those with
//!   classes, nullable types, generics, function values, parameters'
//!   defaults, and core's other types that subset B's steps cover,
//!   translates, the validator finds no problems, the text round trips
//!   (with its spans), and its canonical text, without spans, matches the
//!   golden in `tests/to_tmpl/`, named for the test's path
//!   (`algos-fibonacci.tmpl`).
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

/// The functional tests with classes and interfaces that translate
/// (subset B's first step), as the backends' testkit lists them.
const CLASSES: [&str; 14] = [
    "algos/hello-from-class-to-top",
    "algos/hello-world-object",
    "classes/call-override-from-subtype",
    "classes/direct-getter",
    "classes/inherited-getter",
    "classes/object-literals/extra-class",
    "classes/private-method",
    "classes/setters",
    "classes/static-properties",
    "classes/static-properties-scope",
    "classes/write-back-through-field",
    "imports/types",
    "interfaces/empty",
    "interfaces/pure-virtual",
];

/// The functional tests with nullable types, `orelse`, `bubble()`, and
/// `panic()` that translate, as the backends' testkit lists them.
const NULLABLES: [&str; 4] = [
    "control-flow/bubble",
    "semantics/mutually-referencing-types",
    "types/float/basics",
    "types/int/limits",
];

/// The functional tests with generic types and core's lists that
/// translate, as the backends' testkit lists them.
const GENERICS: [&str; 3] = ["casts/specific", "imports/values", "types/list/empty"];

/// The functional tests with function values (literals, references,
/// calls of values, and core's members that take functions) that
/// translate, as the backends' testkit lists them.
const FUNCTIONS: [&str; 6] = [
    "control-flow/for-of-folded",
    "functions/as-values",
    "functions/constructor-callbacks",
    "functions/simple-locals",
    "imports/functions",
    "types/list/sorting",
];

/// The functional tests with parameters' defaults that translate, as the
/// backends' testkit lists them.
const DEFAULTS: [&str; 1] = ["classes/property-order"];

/// The functional tests with core's other types and members (string
/// indices, `StringBuilder`, `Empty`, `Deque`, `DenseBitVector`, and more
/// of `String`'s and `Float64`'s members) that translate, as the
/// backends' testkit lists them.
const CORE_TYPES: [&str; 9] = [
    "functions/defaulting",
    "functions/named-args",
    "regression/minimal-repro",
    "types/dense-bit-vector",
    "types/deque",
    "types/float/ops",
    "types/string/build",
    "types/string/indices",
    "types/string/read",
];

/// A functional test, settled as `check_functional.py` builds it: its
/// directory's own files, the main one first, and std's that it imports.
fn settled(test: &str) -> Settled {
    let test = temper_funtests::test(test).unwrap_or_else(|| panic!("no test {test}"));
    let mut library = Library::new();
    for source in test.sources() {
        let kind = if source.path.ends_with(".md") {
            SourceKind::Markdown
        } else {
            SourceKind::Standalone
        };
        library.add_file(&source.path, &source.text, kind);
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
        for test in SUBSET_A
            .iter()
            .chain(&CLASSES)
            .chain(&NULLABLES)
            .chain(&GENERICS)
            .chain(&FUNCTIONS)
            .chain(&DEFAULTS)
            .chain(&CORE_TYPES)
        {
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
        let tests: Vec<String> = temper_funtests::tests()
            .into_iter()
            .map(|t| t.path)
            .collect();
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
        let mut covered: Vec<&str> = (SUBSET_A
            .iter()
            .chain(&CLASSES)
            .chain(&NULLABLES)
            .chain(&GENERICS)
            .chain(&FUNCTIONS)
            .chain(&DEFAULTS)
            .chain(&CORE_TYPES))
        .copied()
        .collect();
        covered.sort();
        assert_eq!(ok, covered, "the functional tests that translate");
    });
}
