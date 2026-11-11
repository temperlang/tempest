//! REPL sessions with the tools that the REPL's own functions need
//! beyond the frontend ([`ReplTools`]): `help` with hover's text,
//! `describe(n, "tmpl")`, and `translate`. The transcripts in
//! `tests/repl/*.cases` are as temper-frontend's `tests/repl.rs` reads
//! them; run with `UPDATE_SNAPSHOTS=1` to accept new output. The
//! transcripts translate into every language, so they need every
//! backend built in.
#![cfg(all(
    feature = "cpp",
    feature = "csharp",
    feature = "java",
    feature = "js",
    feature = "lua",
    feature = "py",
    feature = "rust"
))]

#[path = "../../temper-syntax/tests/common/mod.rs"]
mod common;

use common::check_cases;
use std::cell::RefCell;
use std::rc::Rc;
use temper_driver::repl::ReplTools;
use temper_frontend::repl::{Chunker, ReplEvent, ReplOptions, ReplSession, Separator};

/// What a session with the tools prints for `text`, typed a line at a
/// time.
fn transcript(text: &str) -> String {
    let out = Rc::new(RefCell::new(String::new()));
    let sink = out.clone();
    let options = ReplOptions {
        fuel: Some(10_000_000),
        tools: Some(Rc::new(ReplTools::default())),
        help_footer: "(The front end's keys and options.)".into(),
        ..ReplOptions::default()
    };
    let mut session = ReplSession::new(
        options,
        Box::new(move |e: &ReplEvent| {
            let mut out = sink.borrow_mut();
            out.push_str(&e.text());
            out.push('\n');
        }),
    );
    let mut chunker = Chunker::new(Separator::Newline);
    for line in text.split_inclusive('\n') {
        if let Some(input) = chunker.push_line(line) {
            session.feed(&input);
        }
    }
    if let Some(input) = chunker.finish() {
        session.feed(&input);
    }
    out.take()
}

#[test]
fn cases_run_as_expected() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| check_cases("tests/repl", "transcript", |case| transcript(&case.input)))
        .unwrap()
        .join()
        .unwrap();
}
