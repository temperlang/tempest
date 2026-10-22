//! Prints the tokens `temper_syntax::lex` finds, in the same format as
//! `compat/oracle/LexTokens.java`, for comparison with Kotlin Temper.

use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
};
use temper_syntax::lex::{SourceKind, lex};

fn main() -> ExitCode {
    let mut ok = true;
    let mut out = io::BufWriter::new(io::stdout().lock());
    for path in env::args().skip(1) {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("{path}: {err}");
                ok = false;
                continue;
            }
        };
        let kind =
            if path.ends_with(".md") { SourceKind::Markdown } else { SourceKind::Standalone };
        let _ = writeln!(out, "# {path}");
        for token in lex(&text, kind) {
            let flags = match (token.synthetic, token.may_bracket) {
                (true, true) => "sb",
                (true, false) => "s",
                (false, true) => "b",
                (false, false) => "-",
            };
            let error = token.error.map_or("-".to_string(), |e| format!("{e:?}"));
            let _ = writeln!(
                out,
                "{:?}\t{}\t{}\t{flags}\t{error}\t{}",
                token.kind,
                token.start,
                token.end,
                escape(&token.text)
            );
        }
    }
    let _ = out.flush();
    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// Escapes text to fit on one line, as the oracle does.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}
