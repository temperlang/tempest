//! Tests of the translation into tmpl ([`temper_frontend::to_tmpl`]) on
//! Kotlin Temper's functional tests, from the vendored copy.
//!
//! - `subset_a_goldens`: each test that subset A covers, and those with
//!   classes, nullable types, generics, function values, parameters'
//!   defaults, core's other types, and std's classes' members that
//!   subset B's steps cover,
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
use temper_tmpl::coroutines::{CoroutineStrategy, lower};
use temper_tmpl::{Node, Profile, Program, difference, text, validate};

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
const FUNCTIONS: [&str; 7] = [
    "control-flow/for-of-folded",
    "functions/as-values",
    "functions/constructor-callbacks",
    "functions/locals",
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

/// The functional tests of a library's own connected functions and
/// classes, with host code, that translate, as the backends' testkit
/// lists them.
const CONNECTED: [&str; 2] = ["functions/connected", "types/connected"];

/// The functional tests that divide by a constant that can't fail them,
/// where nothing may bubble, that translate, as the backends' testkit
/// lists them.
const DIVISIONS: [&str; 3] = ["types/int/shifty", "types/list/reduce", "types/map"];

/// The functional tests with jumps out of labeled loops, classes declared
/// in functions, type aliases, explicit type arguments, and `as` casts to
/// a supertype, that translate, as the backends' testkit lists them.
const LOOPS_AND_CLASSES: [&str; 4] = [
    "algos/myers-diff",
    "classes/object-literals",
    "control-flow/loops",
    "interfaces/property-members",
];

/// The functional tests that typing needed more of, that translate, as
/// the backends' testkit lists them.
const TYPING: [&str; 1] = ["semantics/constness"];

/// The functional tests with builtin values as `AnyValue`s and `List.of`,
/// that translate, as the backends' testkit lists them.
const ANY_VALUES: [&str; 2] = ["casts/as-expr", "types/list/operations"];

/// The functional tests of a library with errors, each a tmpl `Error`
/// that fails when run, that translate, as the backends' testkit lists
/// them.
const ERRORS: [&str; 2] = ["semantics/broken", "semantics/type-checked-locals"];

/// The functional tests with types as values, that translate, as the
/// backends' testkit lists them.
const TYPE_VALUES: [&str; 1] = ["classes/angle-call"];

/// The functional tests of std's `net`, that translate, as the backends'
/// testkit lists them.
const NET: [&str; 2] = ["types/netresponse", "types/netresponse-not-found"];

/// The functional tests with generators, `async`, `await`, and `yield`,
/// that translate, as the backends' testkit lists them.
const ASYNC: [&str; 5] = [
    "control-flow/actor-run",
    "control-flow/async",
    "control-flow/async-broken-early",
    "control-flow/async-waiters",
    "control-flow/await-in-loop",
];

/// The functional tests that use std's classes' members and
/// constructors, declared in tmpl as another library's, that translate;
/// the backends run them once std translates as a library of its own.
const STD_MEMBERS: [&str; 5] = [
    "regex/match",
    "regex/zero-advance",
    "testing/asserts",
    "types/date",
    "types/json-syntax-tree",
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
    // `testServerPort`, which std `net`'s tests read, at the testkit's
    // goldens' port; translating sends nothing.
    library.test_server_port = Some(28461);
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
            .chain(&CONNECTED)
            .chain(&DIVISIONS)
            .chain(&LOOPS_AND_CLASSES)
            .chain(&TYPING)
            .chain(&ANY_VALUES)
            .chain(&ERRORS)
            .chain(&TYPE_VALUES)
            .chain(&NET)
            .chain(&ASYNC)
            .chain(&STD_MEMBERS)
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
            let mut outputs = vec![(String::new(), text::write(&plain))];
            // Generators as state machines, as Rust, C++, and Java write
            // them: valid, and as the golden `….state-machine.tmpl` has it.
            if ASYNC.contains(test) {
                let lowered = lower(&program, CoroutineStrategy::TranslateToRegularFunction)
                    .unwrap_or_else(|e| panic!("{test}: {e}"))
                    .unwrap_or_else(|| panic!("{test}: no generators"));
                let problems = validate(&lowered, &Profile::full());
                let shown = text::write(&lowered);
                assert!(problems.is_empty(), "{test}: {problems:#?}\n{shown}");
                let back = text::read(&shown).unwrap_or_else(|e| panic!("{test}: {e}\n{shown}"));
                assert_eq!(difference(&lowered, &back), None, "{test}");
                let mut plain = lowered;
                plain.has_spans = false;
                outputs.push((".state-machine".into(), text::write(&plain)));
            }
            for (suffix, canonical) in outputs {
                let name = format!("{}{suffix}.tmpl", test.replace('/', "-"));
                let path = goldens.join(name);
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
            .chain(&CORE_TYPES)
            .chain(&CONNECTED)
            .chain(&DIVISIONS)
            .chain(&LOOPS_AND_CLASSES)
            .chain(&TYPING)
            .chain(&ANY_VALUES)
            .chain(&ERRORS)
            .chain(&TYPE_VALUES)
            .chain(&NET)
            .chain(&ASYNC)
            .chain(&STD_MEMBERS))
        .copied()
        .collect();
        covered.sort();
        assert_eq!(ok, covered, "the functional tests that translate");
    });
}

/// std's modules that build as its own library
/// ([`temper_frontend::stdlib::program`]) translate, validate, round
/// trip, and match `tests/to_tmpl/std.tmpl`; their `@connected` members
/// keep their Temper bodies, for backends that don't connect them.
#[test]
fn std_library() {
    with_stack(|| {
        let program = temper_frontend::stdlib::program(&Sources::standard())
            .unwrap_or_else(|e| panic!("std: {e}"));
        let problems = validate(&program, &Profile::full());
        assert!(problems.is_empty(), "std: {problems:#?}");
        let full = text::write(&program);
        let back = text::read(&full).unwrap_or_else(|e| panic!("std: {e}\n{full}"));
        assert_eq!(difference(&program, &back), None, "std");
        let mut plain = program.clone();
        plain.has_spans = false;
        let canonical = text::write(&plain);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/to_tmpl/std.tmpl");
        if env::var_os("UPDATE_SNAPSHOTS").is_some() {
            fs::write(&path, &canonical).unwrap();
        } else {
            let expected = fs::read_to_string(&path).unwrap_or_default();
            assert!(
                canonical == expected,
                "{} differs (UPDATE_SNAPSHOTS=1 accepts new output):\n{canonical}",
                path.display()
            );
        }
    });
}

/// Each state machine's step reads its state, then sets it to `-1`
/// before anything else runs, so a step that fails stays failed: stepped
/// again, it reads `-1` and says it's done, rather than running again.
/// No Temper program can show it, since a generator's body can't bubble
/// (rule F4) and `panic()` ends the program on every backend; host code
/// stepping it again could.
#[test]
fn failed_steps_are_done() {
    with_stack(|| {
        for test in ASYNC {
            let program = translate(test).unwrap_or_else(|e| panic!("{test}: {e}"));
            let lowered = lower(&program, CoroutineStrategy::TranslateToRegularFunction)
                .unwrap_or_else(|e| panic!("{test}: {e}"))
                .unwrap_or_else(|| panic!("{test}: no generators"));
            let p = &lowered;
            let mut steps = 0;
            for m in &p.modules {
                for &root in m.items.iter().chain([&m.init]) {
                    temper_tmpl::analysis::walk(p, root, &mut |id| {
                        let Node::FunctionLiteral { decl, body, .. } = p.nodes.get(id) else {
                            return;
                        };
                        if !p.name(decl).starts_with("convertedCoroutine") {
                            return;
                        }
                        steps += 1;
                        // Its loop's body, or one state's, its own.
                        let Node::Block { stmts } = p.nodes.get(body) else {
                            panic!("{test}: a step's body isn't a block");
                        };
                        let stmts = match p.nodes.list(stmts).first().map(|&s| p.nodes.get(s)) {
                            Some(Node::While { body: each, .. }) => match p.nodes.get(each) {
                                Node::Block { stmts } => stmts,
                                _ => panic!("{test}: a step's loop body isn't a block"),
                            },
                            _ => stmts,
                        };
                        let first: Vec<Node> = p
                            .nodes
                            .list(stmts)
                            .iter()
                            .take(2)
                            .map(|&s| p.nodes.get(s))
                            .collect();
                        let [
                            Node::Let {
                                init: Some(read), ..
                            },
                            Node::Assign { local, value },
                        ] = first[..]
                        else {
                            panic!(
                                "{test}: a step doesn't start by reading its state, then setting it"
                            );
                        };
                        assert_eq!(p.nodes.get(read), Node::Local { local }, "{test}");
                        assert_eq!(p.nodes.get(value), Node::Int32Lit { value: -1 }, "{test}");
                    });
                }
            }
            assert!(steps > 0, "{test}: no steps");
        }
    });
}

/// A closure in a generator's loop that captures a loop `let` assigned
/// after it's set, which a state machine's one copy per closure can't
/// share with the pass: lowering fails, saying so, rather than giving
/// each closure a stale copy.
#[test]
fn reassigned_loop_captures_fail_clearly() {
    with_stack(|| {
        let source = "
    @fun interface Counter(): Int;
    let drive(body: fn (): SafeGenerator<Empty>): Void { body().nextSafe(); }
    drive { (): GeneratorResult<Empty> extends GeneratorFn =>
      let fs = new ListBuilder<Counter>();
      for (var i = 0; i < 2; i += 1) {
        var j = i;
        fs.add(fn (): Int { j });
        j += 10;
        yield();
      }
    }
";
        let mut library = Library::new();
        library.add_file("reassigned.temper.md", source, SourceKind::Markdown);
        load_imports(&Sources::standard(), &mut library);
        let program = to_tmpl(&settle(library), "reassigned").unwrap();
        assert!(validate(&program, &Profile::full()).is_empty());
        let error = lower(&program, CoroutineStrategy::TranslateToRegularFunction).unwrap_err();
        assert_eq!(
            error,
            "a closure in a generator's loop capturing j, which is assigned after its `let`"
        );
    });
}

#[test]
fn constant_divisors_fold_as_typing_does() {
    with_stack(|| {
        // As Hrefine's `s / (jamoVCount * jamoTCount)`: module `let`s and
        // arithmetic on them make a divisor that can't fail, which typing
        // lets a function that can't bubble use, so it's `divSafe`. One
        // that folds to 0 may fail, which typing reports.
        let source = "
    let jamoVCount = 21;
    let jamoTCount = 28;
    let none = jamoTCount - 28;
    export let split(s: Int): Int {
      s / (jamoVCount * jamoTCount) + s % (jamoTCount - 1) + s / -(1 + 1)
    }
    export let broken(s: Int): Int { s / none }
";
        let mut library = Library::new();
        library.add_file("divisors.temper.md", source, SourceKind::Markdown);
        load_imports(&Sources::standard(), &mut library);
        let settled = settle(library);
        let errors: Vec<String> = (settled.library.diagnostics.iter())
            .map(|d| d.message.text())
            .collect();
        assert_eq!(
            errors,
            ["Cannot bubble from a function without Bubble in its return type"]
        );
    });
    with_stack(|| {
        let source = "
    let jamoVCount = 21;
    let jamoTCount = 28;
    export let split(s: Int): Int {
      s / (jamoVCount * jamoTCount) + s % (jamoTCount - 1) + s / -(1 + 1)
    }
";
        let mut library = Library::new();
        library.add_file("divisors.temper.md", source, SourceKind::Markdown);
        load_imports(&Sources::standard(), &mut library);
        let program = to_tmpl(&settle(library), "divisors").unwrap();
        let written = text::write(&program);
        assert_eq!(written.matches("%Int32.divSafe(").count(), 2, "{written}");
        assert_eq!(written.matches("%Int32.modSafe(").count(), 1, "{written}");
        assert!(!written.contains("%Int32.div("), "{written}");
    });
}
