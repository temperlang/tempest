//! How commands report, as Kotlin Temper's `--logformat` chooses: as
//! text for people (the default), or as JSON, one record per line, for
//! tools ([`LogFormat::Json`]).
//!
//! A JSON record has Kotlin's shape (its `ConsoleBackedContextualLogSink`):
//!
//! ```text
//! { "level": "error", "template": "UndeclaredName", "pos": { "loc": "lib/a.temper", "left": 51, "right": 57 }, "values": [], "message": "tripel has not been declared" }
//! ```
//!
//! - `level` is `error`, `warn`, `info`, or `summary`.
//! - `template` names the message: a diagnostic's kind, as tempest's
//!   [`Message`] names it (after Kotlin's templates where there's one),
//!   or one of the commands' own, as `TestFailed` and
//!   `TestSummaryAllRun`, which are Kotlin's.
//! - `pos`, where there is one, is the file, as the text form prints its
//!   path, and the start and end as offsets in UTF-16 code units, as
//!   Kotlin's are.
//! - `values` are the message's arguments, where tempest keeps them
//!   apart: the commands' own messages, as Kotlin's; empty for
//!   diagnostics, which keep only their text.
//! - `message` is the text the text form prints. Kotlin's records don't
//!   have it.
//!
//! A command ends with Kotlin's `{ "task": "Build", "ok": true }`.

use crate::library::{Diagnostic, Level, Message};
use crate::settle::Settled;

/// How a command reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogFormat {
    /// Lines for people.
    #[default]
    Text,
    /// A JSON record per line, for tools, as Kotlin's `--logformat json`.
    Json,
}

impl LogFormat {
    /// A format by its name on the command line, `text` or `json`.
    pub fn named(name: &str) -> Option<LogFormat> {
        match name {
            "text" => Some(LogFormat::Text),
            "json" => Some(LogFormat::Json),
            _ => None,
        }
    }
}

/// One of a record's values.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::Str(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::Str(s)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Value {
        Value::Int(n as i64)
    }
}

/// Where a record points: a file, and a start and end in UTF-16 code
/// units.
#[derive(Clone, Debug, PartialEq)]
pub struct Pos {
    pub loc: String,
    pub left: usize,
    pub right: usize,
}

/// A record as a JSON line, without its newline.
pub fn record(
    level: &str,
    template: &str,
    pos: Option<&Pos>,
    values: &[Value],
    message: &str,
) -> String {
    let mut out = format!(
        "{{ \"level\": {}, \"template\": {}",
        quoted(level),
        quoted(template)
    );
    if let Some(pos) = pos {
        out.push_str(&format!(
            ", \"pos\": {{ \"loc\": {}, \"left\": {}, \"right\": {} }}",
            quoted(&pos.loc),
            pos.left,
            pos.right
        ));
    }
    let values: Vec<String> = (values.iter())
        .map(|v| match v {
            Value::Str(s) => quoted(s),
            Value::Int(n) => n.to_string(),
        })
        .collect();
    match values.is_empty() {
        true => out.push_str(", \"values\": []"),
        false => out.push_str(&format!(", \"values\": [ {} ]", values.join(", "))),
    }
    out.push_str(&format!(", \"message\": {} }}", quoted(message)));
    out
}

/// Kotlin's last record of a command: `{ "task": "Build", "ok": true }`.
pub fn task(label: &str, ok: bool) -> String {
    format!("{{ \"task\": {}, \"ok\": {ok} }}", quoted(label))
}

/// A string as JSON writes it.
pub fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A level's name in records, as Kotlin's.
pub fn level_name(level: Level) -> &'static str {
    match level {
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}

/// A diagnostic's template: its message's kind, as [`Message`] names it,
/// or the grammar's message's for a syntax error.
pub fn template(message: &Message) -> String {
    let debug = match message {
        Message::Syntax(m) => format!("{m:?}"),
        m => format!("{m:?}"),
    };
    debug
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// A settled library's diagnostics as JSON records, a line each.
pub fn write_json_diagnostics(s: &Settled, out: &mut String) {
    for d in &s.library.diagnostics {
        write_json_diagnostic(s, d, out);
    }
}

/// One diagnostic of a settled library's as a JSON record, as
/// [`write_json_diagnostics`] writes each.
pub fn write_json_diagnostic(s: &Settled, d: &Diagnostic, out: &mut String) {
    let (file, start) = s.library.locate(d.span.start);
    let (_, end) = s.library.locate(d.span.end.max(d.span.start));
    let file = &s.library.files[file.0 as usize];
    let source = file.parsed.source();
    let units = |offset: u32| {
        let offset = (offset as usize).min(source.len());
        // A boundary at or before the offset, should one be mid-character.
        let offset = (0..=offset)
            .rev()
            .find(|&i| source.is_char_boundary(i))
            .unwrap_or(0);
        source[..offset].encode_utf16().count()
    };
    let pos = Pos {
        loc: file.path.clone(),
        left: units(start),
        right: units(end.max(start)),
    };
    out.push_str(&record(
        level_name(d.level),
        &template(&d.message),
        Some(&pos),
        &[],
        &d.message.text(),
    ));
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_have_kotlins_shape() {
        let pos = Pos {
            loc: "lib/a.temper".into(),
            left: 51,
            right: 57,
        };
        assert_eq!(
            record(
                "error",
                "UndeclaredName",
                Some(&pos),
                &[],
                "tripel has not been declared"
            ),
            "{ \"level\": \"error\", \"template\": \"UndeclaredName\", \"pos\": { \"loc\": \
             \"lib/a.temper\", \"left\": 51, \"right\": 57 }, \"values\": [], \"message\": \
             \"tripel has not been declared\" }"
        );
        assert_eq!(
            record(
                "summary",
                "TestSummaryAllRun",
                None,
                &[1usize.into(), 2usize.into()],
                "Tests passed: 1 of 2"
            ),
            "{ \"level\": \"summary\", \"template\": \"TestSummaryAllRun\", \"values\": [ 1, 2 ], \
             \"message\": \"Tests passed: 1 of 2\" }"
        );
        assert_eq!(
            task("Build", false),
            "{ \"task\": \"Build\", \"ok\": false }"
        );
        assert_eq!(quoted("a\"b\\\n\u{1}"), "\"a\\\"b\\\\\\n\\u0001\"");
    }

    #[test]
    fn templates_name_messages() {
        assert_eq!(
            template(&Message::UndeclaredName("x".into())),
            "UndeclaredName"
        );
        assert_eq!(template(&Message::VoidAsValue), "VoidAsValue");
    }
}
