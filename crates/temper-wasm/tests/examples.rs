//! Every example the playground bundles (`site/playground/examples/`)
//! compiles as the playground compiles it, through `compile` with
//! `RUN | TESTS` and every backend: no errors, every backend's files,
//! and the interpreter's output and tests' results as pinned here; and
//! as the REPL tab's input 0, the same output and tests' summary.

use serde_json::Value;
use std::path::Path;
use temper_frontend::repl::ReplOptions;
use temper_syntax::lex::SourceKind;
use temper_wasm::{RUN, TESTS, built_in, compile_into};

/// What each example logs, and its tests' results, by file name: each
/// test's name, passing.
const EXPECTED: &[(&str, &[&str], &[&str])] = &[
    ("1-hello.temper", &["Hello, world!"], &["greets by name"]),
    (
        "2-interpolation.temper",
        &["Total: $49.99"],
        &["formats whole dollars", "pads cents"],
    ),
    (
        "3-generic-class.temper",
        &[],
        &["pops in reverse order", "fails when empty"],
    ),
    ("4-async.temper", &["total 6"], &[]),
    (
        "5-generator.temper",
        &["0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89"],
        &["takes only what it asks for"],
    ),
    (
        "6-tests.temper",
        &[],
        &[
            "every fourth year",
            "but not most centuries",
            "except every fourth century",
        ],
    ),
    ("7-triple.temper", &["triple(14) = 42"], &[]),
];

/// Compiles as the playground does, on a thread with room for the
/// settle loop's recursion, giving `compile`'s JSON.
fn compile(source: String) -> Value {
    let mask = (1u32 << built_in().count()) - 1;
    let json = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let mut out = String::new();
            compile_into(&source, SourceKind::Standalone, RUN | TESTS, mask, &mut out);
            out
        })
        .unwrap()
        .join()
        .unwrap();
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}: {json}"))
}

/// Starts the REPL tab's session with `source` as input 0, as the
/// playground does, giving its answer.
fn repl_start(source: String) -> Value {
    let json = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || temper_wasm::repl::start_with(ReplOptions::default(), &source))
        .unwrap()
        .join()
        .unwrap();
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}: {json}"))
}

#[test]
fn every_example_works() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site/playground/examples");
    let mut names: Vec<String> = (std::fs::read_dir(&dir).unwrap())
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".temper"))
        .collect();
    names.sort();
    let pinned: Vec<&str> = EXPECTED.iter().map(|e| e.0).collect();
    assert_eq!(names, pinned, "each example needs its expectations here");
    let backends: Vec<&str> = built_in().collect();
    #[cfg(all(
        feature = "cpp",
        feature = "csharp",
        feature = "java",
        feature = "js",
        feature = "lua",
        feature = "py",
        feature = "rust"
    ))]
    assert_eq!(backends, temper_driver::KNOWN);
    for &(name, output, tests) in EXPECTED {
        let result = compile(std::fs::read_to_string(dir.join(name)).unwrap());
        assert_eq!(result["diagnostics"], Value::Array(vec![]), "{name}");
        let run = &result["run"];
        assert_eq!(run["output"], serde_json::json!(output), "{name}");
        assert_eq!(run["failure"], Value::Null, "{name}");
        let results: Vec<Value> = (tests.iter())
            .map(|t| serde_json::json!({"name": t, "failure": null}))
            .collect();
        assert_eq!(run["tests"], Value::Array(results), "{name}");
        assert_eq!(run["defined"], tests.len(), "{name}");
        let summary = format!("Tests passed: {0} of {0}", tests.len());
        assert_eq!(run["summary"], summary.as_str(), "{name}");
        let mut expected: Vec<Value> = (output.iter())
            .map(|line| serde_json::json!({"kind": "log", "text": line}))
            .collect();
        if !tests.is_empty() {
            expected.push(serde_json::json!({"kind": "note", "text": summary}));
        }
        expected.push(serde_json::json!({"kind": "result", "text": "interactive#0: void"}));
        let repl = repl_start(std::fs::read_to_string(dir.join(name)).unwrap());
        assert_eq!(repl["events"], Value::Array(expected), "{name} in the REPL");
        for backend in &backends {
            let generated = &result["backends"][backend];
            assert!(
                generated["error"].is_null(),
                "{name} {backend}: {}",
                generated["error"]
            );
            let files = generated["files"]
                .as_array()
                .unwrap_or_else(|| panic!("{name} {backend}"));
            assert!(
                files
                    .iter()
                    .any(|f| !f["text"].as_str().unwrap_or("").is_empty()),
                "{name} {backend}: no files"
            );
        }
    }
}
