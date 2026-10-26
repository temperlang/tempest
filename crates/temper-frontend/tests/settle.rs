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
use temper_frontend::interp::{run, run_tests};
use temper_frontend::library::{Library, Message};
use temper_frontend::print::write_settled;
use temper_frontend::settle::settle;
use temper_syntax::lex::SourceKind;

/// The settled tree, diagnostics and run of a source.
fn settled(input: &str, kind: SourceKind, fuel: u64) -> (String, bool) {
    let mut library = Library::new();
    library.add_file("test.temper", input, kind);
    let settled = settle(library);
    let out_of_steps = settled
        .library
        .diagnostics
        .iter()
        .any(|d| matches!(d.message, Message::OutOfSteps(_)));
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
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn cases_settle_as_expected() {
    with_stack(|| {
        check_cases("tests/settle", "settled", |case| {
            settled(&case.input, case.kind, 1_000_000).0
        })
    });
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
        let messages: Vec<String> = settled
            .library
            .diagnostics
            .iter()
            .map(|d| d.message.text())
            .collect();
        assert_eq!(
            messages,
            [
                "./lib does not export symbol hidden",
                "Import of ./nope failed"
            ]
        );
        let run = run(&settled, Some(1_000_000));
        assert_eq!(
            (run.output, run.failure.as_deref()),
            (
                vec!["lib".to_string(), "2 3".into()],
                Some("Failed: ./lib does not export symbol hidden")
            )
        );
    });
}

/// `std/…` imports, from `Library::std_dir`, and `...`, which imports
/// every export not named otherwise.
#[test]
fn std_imports_and_spread() {
    with_stack(|| {
        let mut library = Library::new();
        library.std_dir = Some("/lib/std".into());
        let main = "let { ... } = import(\"std/m\");\nlet { a as b, ... } = import(\"./n\");\n\
                    console.log(\"${a} ${c2} ${b} ${d}\");";
        library.add_file("app/main.temper", main, SourceKind::Standalone);
        assert_eq!(library.missing_imports(), ["/lib/std/m", "app/n"]);
        // Std modules import each other relatively.
        library.add_file(
            "/lib/std/m/m.temper",
            "export let a = 1;\nlet { c } = import(\"../o\");\nexport let c2 = c;",
            SourceKind::Standalone,
        );
        library.add_file(
            "/lib/std/o/o.temper",
            "export let c = 3;",
            SourceKind::Standalone,
        );
        library.add_file(
            "app/n/n.temper",
            "export let a = 2;\nexport let d = 4;\nlet hidden = 5;",
            SourceKind::Standalone,
        );
        assert!(library.missing_imports().is_empty());
        let settled = settle(library);
        assert!(settled.library.diagnostics.is_empty());
        // `a` is std/m's, as n's is named and renamed.
        let run = run(&settled, Some(1_000_000));
        assert_eq!(
            (run.output, run.failure),
            (vec!["1 3 2 4".to_string()], None)
        );
        // Without a std directory, std imports aren't supported.
        let mut library = Library::new();
        library.add_file(
            "main.temper",
            "let { a } = import(\"std/m\");",
            SourceKind::Standalone,
        );
        assert!(library.missing_imports().is_empty());
        let messages: Vec<String> = settle(library)
            .library
            .diagnostics
            .iter()
            .map(|d| d.message.text())
            .collect();
        assert_eq!(messages, ["Not supported yet: importing std/m"]);
    });
}

/// `test` and `assert`, over a small stand-in for std's `testing`, and
/// running the tests after the program.
#[test]
fn tests_and_asserts() {
    with_stack(|| {
        let mut library = Library::new();
        library.std_dir = Some("/lib/std".into());
        let main = "test(\"one ${n}\") { assert(n == 2); assert(n > 2) }\nlet n = 1;\n\
                    test(\"two\") { t => helper(t); assert(false) { \"no\" } }\n\
                    let helper(test: Test): Void { assert(n != 1) }\nlet f(x: Int): Void { assert(true) }\nconsole.log(\"ran\");";
        library.add_file("main.temper", main, SourceKind::Standalone);
        assert_eq!(library.missing_imports(), ["/lib/std/testing"]);
        let testing = "export class Test {\n\
                         public assert(ok: Boolean, message: fn (): String): Void { if (!ok) { messages.add(message()) } }\n\
                         public messages: ListBuilder<String> = new ListBuilder<String>();\n}\n\
                       export let runTestCases(cases: List<Pair<String, fn (Test): Void>>): String {\n\
                         cases.map { (c): String => let t = new Test(); let f = c.value; f(t); \"${c.key}: ${t.messages.join(\", \") { it => it }}\" }\n\
                           .join(\"\\n\") { it => it }\n}";
        library.add_file(
            "/lib/std/testing/testing.temper",
            testing,
            SourceKind::Standalone,
        );
        let settled = settle(library);
        let messages: Vec<String> = settled
            .library
            .diagnostics
            .iter()
            .map(|d| d.message.text())
            .collect();
        assert_eq!(messages, ["Invalid block content"]);
        let run = run(&settled, Some(1_000_000));
        assert_eq!((run.output, run.failure), (vec!["ran".to_string()], None));
        let run = run_tests(&settled, Some(1_000_000));
        let expected = [
            "ran",
            "one 1: expected n == (2) not (1), expected n > 2",
            "two: expected n != 1, no",
        ];
        assert_eq!(
            (run.output, run.failure),
            (expected.map(String::from).to_vec(), None)
        );
    });
}

/// Regex literals, which build std's regex objects, over a small stand-in
/// for std's `regex` whose objects describe themselves.
#[test]
fn regex_literals() {
    with_stack(|| {
        let mut library = Library::new();
        library.std_dir = Some("/lib/std".into());
        let main = "let x = \"-\";\nconsole.log(rgx\"a${x}b+^(?n=c)\");\nlet bad = /a{2,1}/;";
        library.add_file("main.temper", main, SourceKind::Standalone);
        assert_eq!(library.missing_imports(), ["/lib/std/regex"]);
        let regex = "export interface RegexNode { public compiled(): String { this.toString() } }\n\
                     export class CodePoints(public value: String) extends RegexNode { public toString(): String { \"'${value}'\" } }\n\
                     export class Sequence(public items: List<RegexNode>) extends RegexNode {\n\
                       public toString(): String { \"seq(${items.join(\", \") { it => it.toString() }})\" }\n}\n\
                     export class Repeat(public item: RegexNode, public min: Int, public max: Int?, public reluctant: Boolean) extends RegexNode {\n\
                       public toString(): String { \"rep(${item}, ${min}, ${max ?? -1}, ${reluctant})\" }\n}\n\
                     export class Capture(public name: String, public item: RegexNode) extends RegexNode {\n\
                       public toString(): String { \"cap(${name}, ${item})\" }\n}\n\
                     class B extends RegexNode { public toString(): String { \"^\" } }\nexport let Begin = new B();";
        library.add_file("/lib/std/regex/regex.temper", regex, SourceKind::Standalone);
        let settled = settle(library);
        let messages: Vec<String> = settled
            .library
            .diagnostics
            .iter()
            .map(|d| d.message.text())
            .collect();
        assert_eq!(
            messages,
            ["Invalid regex: at position 6: quantifier has min greater than max"]
        );
        let run = run(&settled, Some(1_000_000));
        let expected = "seq('a-', rep('b', 1, -1, false), ^, cap(n, 'c'))";
        let failure = Some(
            "Failed: Invalid regex: at position 6: quantifier has min greater than max".to_string(),
        );
        assert_eq!(
            (run.output, run.failure),
            (vec![expected.to_string()], failure)
        );
    });
}

/// std `net`'s `sendRequest`, over a small stand-in for std's `net`,
/// against a local server that answers one request, echoing it. Without
/// the `net` feature, the promise breaks, saying why.
#[test]
fn net_requests() {
    with_stack(|| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut library = Library::new();
        library.std_dir = Some("/lib/std".into());
        library.test_server_port = Some(port.into());
        let main = "let { send, NetResponse } = import(\"std/net\");\n\
                    let p = send(\"http://127.0.0.1:${testServerPort}/echo\");\n\
                    console.log(\"sent\");\n\
                    async { (): GeneratorResult<Empty> extends GeneratorFn =>\n\
                      let r = await p;\n\
                      let body = (await r.bodyContent) ?? \"none\";\n\
                      console.log(\"${r.status} ${r.contentType ?? \"none\"} ${r is NetResponse}: ${body}\");\n\
                    }";
        library.add_file("main.temper", main, SourceKind::Standalone);
        let net = "@connected\n\
                   let sendRequest(url: String, method: String, bodyContent: String?, bodyMimeType: String?): Promise<NetResponse>;\n\
                   export let send(url: String): Promise<NetResponse> { sendRequest(url, \"POST\", \"[]\", \"application/json\") }\n\
                   @connected export interface NetResponse {\n\
                     @connected get status(): Int;\n\
                     @connected get contentType(): String?;\n\
                     @connected get bodyContent(): Promise<String?>;\n\
                   }";
        library.add_file("/lib/std/net/net.temper", net, SourceKind::Standalone);
        let settled = settle(library);
        assert!(settled.library.diagnostics.is_empty());
        let server = cfg!(feature = "net").then(|| std::thread::spawn(move || echo_once(listener)));
        let run = run(&settled, Some(1_000_000));
        let (output, failure) = if cfg!(feature = "net") {
            let echo = "201 text/plain true: POST /echo HTTP/1.1, application/json, []";
            (vec!["sent", echo], None)
        } else {
            (vec!["sent"], Some("Failed: web client not supported here"))
        };
        let output: Vec<String> = output.into_iter().map(String::from).collect();
        assert_eq!((run.output, run.failure.as_deref()), (output, failure));
        if let Some(server) = server {
            server.join().unwrap();
        }
    });
}

/// Answers one HTTP request with 201 and, as text, its request line,
/// content type and body.
fn echo_once(listener: std::net::TcpListener) {
    use std::io::{BufRead, BufReader, Read, Write};
    let (stream, _) = listener.accept().unwrap();
    let mut reader = BufReader::new(stream);
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        match line.trim_end() {
            "" => break,
            line => lines.push(line.to_string()),
        }
    }
    let header = |name: &str| {
        lines.iter().find_map(|l| {
            let (key, value) = l.split_once(':')?;
            key.eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    };
    let length = header("content-length").map_or(0, |n| n.parse().unwrap());
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    let kind = header("content-type").unwrap_or_default();
    let echo = format!("{}, {kind}, {}", lines[0], String::from_utf8_lossy(&body));
    let response = format!(
        "HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{echo}",
        echo.len()
    );
    reader.get_mut().write_all(response.as_bytes()).unwrap();
}

/// Pieces that matter to the settle loop, on top of the lexer's.
#[rustfmt::skip]
const SETTLE_PIECES: &[&str] = &[
    "let ", "var ", "let f(x: Int): Int { x }\n", "if (x) {", "} else {", "} else if (y) {", "while (",
    "for (var i = 0; i < 3; ++i) {", "break;", "continue;", "break outer;", "outer: ", "return ",
    "x", "f(", "=", "+=", "++", "--", "fn (", "=>", "do {", "} orelse ", "bubble()", "${",
    "console.log(", "@var ", "&&", "||", ";", "{", "}", "(", ")", "\n", "yield ", " extends GeneratorFn",
    ".next()", "await ", "async { (): GeneratorResult<Empty> extends GeneratorFn => ", "new PromiseBuilder<Int>()", ".promise",
    ".complete(1)", ".breakPromise()", "@connected ",
];

#[test]
fn mutated_inputs_settle() {
    with_stack(|| {
        let seed = env_number("SETTLE_FUZZ_SEED", 1);
        let rounds = env_number("SETTLE_FUZZ_ROUNDS", 20);
        let mut rng = Rng(seed.max(1));
        let pieces: Vec<&str> = common::PIECES
            .iter()
            .chain(SETTLE_PIECES)
            .copied()
            .collect();
        let mut inputs = Vec::new();
        for (dir, separator) in [
            ("tests/settle", "settled"),
            ("../temper-syntax/tests/ast", "ast"),
        ] {
            for (path, cases) in case_files(dir, separator) {
                for case in cases {
                    inputs.push((
                        format!("{} case {:?}", path.display(), case.name),
                        case.input,
                        case.kind,
                    ));
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
                check(
                    &text,
                    kind,
                    &format!("{context}, seed {seed} round {round}"),
                );
            }
        }
    });
}

fn add_files(dir: &std::path::Path, inputs: &mut Vec<(String, String, SourceKind)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.to_string_lossy().to_string();
        if path.is_dir() {
            add_files(&path, inputs);
        } else if name.ends_with(".temper") || name.ends_with(".temper.md") {
            let kind = if name.ends_with(".md") {
                SourceKind::Markdown
            } else {
                SourceKind::Standalone
            };
            inputs.push((name, std::fs::read_to_string(&path).unwrap(), kind));
        }
    }
}

fn check(text: &str, kind: SourceKind, context: &str) {
    let result = std::panic::catch_unwind(|| settled(text, kind, 20_000));
    let Ok((once, out_of_steps)) = result else {
        panic!("{context}: panicked\ninput: {text:?}");
    };
    assert!(
        !out_of_steps,
        "{context}: ran out of steps\ninput: {text:?}"
    );
    let (again, _) = settled(text, kind, 20_000);
    assert_eq!(
        once, again,
        "{context}: settled differently twice\ninput: {text:?}"
    );
}
