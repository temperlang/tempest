//! Settle loop tests over the cases in `tests/settle/*.cases`.
//!
//! The cases have the format described in temper-syntax's
//! `tests/common/mod.rs`, with a `--- settled` line before the expected
//! output: the settled tree and diagnostics as `print::write_settled`
//! writes them, then what running it logs, as `log` lines, and how it
//! failed, if it did.
//!
//! - `cases_settle_as_expected` checks each case. To accept new or
//!   changed output, run with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `mutated_inputs_settle` settles randomly edited copies of these cases
//!   and of temper-syntax's grammar cases, and checks that settling never
//!   panics (nor fails the debug check that no step read a node rewritten
//!   later), finishes within its step budget, gives the same result twice,
//!   and that running the result never panics. Set `SETTLE_FUZZ_SEED` and
//!   `SETTLE_FUZZ_ROUNDS` (edits per case) to vary it, and
//!   `SETTLE_FUZZ_DIR` to add the Temper files under a directory, such as
//!   Kotlin Temper's functional tests.

#[path = "../../temper-syntax/tests/common/mod.rs"]
mod common;

use common::{Rng, case_files, check_cases, env_number, mutate};
use temper_frontend::interp::run;
use temper_frontend::library::{Library, Message};
use temper_frontend::print::write_settled;
use temper_frontend::settle::settle;
use temper_syntax::lex::SourceKind;

/// The settled tree, diagnostics and run of a source.
fn settled(input: &str, kind: SourceKind, fuel: u64) -> (String, bool) {
    let mut library = Library::new();
    library.add_file("test.temper", input, kind);
    let settled = settle(library);
    let out_of_steps = settled.library.diagnostics.iter().any(|d| matches!(d.message, Message::OutOfSteps(_)));
    let mut text = String::new();
    write_settled(&settled, 80, &mut text);
    let run = run(&settled, Some(fuel));
    for line in run.output {
        text.push_str(&format!("log {line}\n"));
    }
    if let Some(failure) = run.failure {
        text.push_str(&format!("{failure}\n"));
    }
    (text, out_of_steps)
}

/// Runs a test on a thread with room for the interpreter's recursion.
fn with_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().stack_size(64 << 20).spawn(test).unwrap().join().unwrap();
}

#[test]
fn cases_settle_as_expected() {
    with_stack(|| check_cases("tests/settle", "settled", |case| settled(&case.input, case.kind, 1_000_000).0));
}

/// Imports between modules, which the cases' single files can't show.
#[test]
fn imports_across_modules() {
    with_stack(|| {
        let mut library = Library::new();
        let main = "let { twice, C as D } = import(\"./lib/\");\nlet n = 1;\n\
                    console.log(\"${twice(n)} ${new D(3).n}\");\nlet { hidden } = import(\"./lib\");\nlet { z } = import(\"./nope\");";
        library.add_file("app/main.temper", main, SourceKind::Standalone);
        assert_eq!(library.missing_imports(), ["app/lib", "app/nope"]);
        // `n` here is its own; the importer's is another.
        let lib = "export let twice(x: Int): Int { x * n }\nlet n = 2;\nlet hidden = 0;\n\
                   export class C(public n: Int) {}\nconsole.log(\"lib\");";
        library.add_file("app/lib/lib.temper", lib, SourceKind::Standalone);
        assert!(library.missing_imports() == ["app/nope"]);
        let settled = settle(library);
        let messages: Vec<String> = settled.library.diagnostics.iter().map(|d| d.message.text()).collect();
        assert_eq!(messages, ["./lib does not export symbol hidden", "Import of ./nope failed"]);
        let run = run(&settled, Some(1_000_000));
        assert_eq!((run.output, run.failure.as_deref()), (vec!["lib".to_string(), "2 3".into()], Some("Failed: ./lib does not export symbol hidden")));
    });
}

/// Pieces that matter to the settle loop, on top of the lexer's.
#[rustfmt::skip]
const SETTLE_PIECES: &[&str] = &[
    "let ", "var ", "let f(x: Int): Int { x }\n", "if (x) {", "} else {", "} else if (y) {", "while (",
    "for (var i = 0; i < 3; ++i) {", "break;", "continue;", "break outer;", "outer: ", "return ",
    "x", "f(", "=", "+=", "++", "--", "fn (", "=>", "do {", "} orelse ", "bubble()", "${",
    "console.log(", "@var ", "&&", "||", ";", "{", "}", "(", ")", "\n",
];

#[test]
fn mutated_inputs_settle() {
    with_stack(|| {
        let seed = env_number("SETTLE_FUZZ_SEED", 1);
        let rounds = env_number("SETTLE_FUZZ_ROUNDS", 20);
        let mut rng = Rng(seed.max(1));
        let pieces: Vec<&str> = common::PIECES.iter().chain(SETTLE_PIECES).copied().collect();
        let mut inputs = Vec::new();
        for (dir, separator) in [("tests/settle", "settled"), ("../temper-syntax/tests/ast", "ast")] {
            for (path, cases) in case_files(dir, separator) {
                for case in cases {
                    inputs.push((format!("{} case {:?}", path.display(), case.name), case.input, case.kind));
                }
            }
        }
        if let Ok(dir) = std::env::var("SETTLE_FUZZ_DIR") {
            add_files(std::path::Path::new(&dir), &mut inputs);
        }
        for (context, input, kind) in inputs {
            check(&input, kind, &context);
            for round in 0..rounds {
                let text = mutate(&input, &pieces, &mut rng);
                check(&text, kind, &format!("{context}, seed {seed} round {round}"));
            }
        }
    });
}

fn add_files(dir: &std::path::Path, inputs: &mut Vec<(String, String, SourceKind)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        let name = path.to_string_lossy().to_string();
        if path.is_dir() {
            add_files(&path, inputs);
        } else if name.ends_with(".temper") || name.ends_with(".temper.md") {
            let kind = if name.ends_with(".md") { SourceKind::Markdown } else { SourceKind::Standalone };
            inputs.push((name, std::fs::read_to_string(&path).unwrap(), kind));
        }
    }
}

fn check(text: &str, kind: SourceKind, context: &str) {
    let result = std::panic::catch_unwind(|| settled(text, kind, 20_000));
    let Ok((once, out_of_steps)) = result else {
        panic!("{context}: panicked\ninput: {text:?}");
    };
    assert!(!out_of_steps, "{context}: ran out of steps\ninput: {text:?}");
    let (again, _) = settled(text, kind, 20_000);
    assert_eq!(once, again, "{context}: settled differently twice\ninput: {text:?}");
}
