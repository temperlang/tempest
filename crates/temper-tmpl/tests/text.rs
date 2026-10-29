//! Golden tests over `tests/text/*.cases`: hand-written tmpl, and what the
//! reader, writer, and validator make of it.
//!
//! A cases file has a few comment lines, then cases. Each case is a header
//! line `=== name`, then tmpl text, then sections showing only what isn't
//! the default:
//!
//! - `--- canonical`: what the writer gives back, if that isn't the input;
//! - `--- problems`: the validator's, as `RULE @node: message`, if any,
//!   where `@n` is a node's number in writing order and `#n` a
//!   declaration;
//! - `--- error`: why reading failed, if it did.
//!
//! Every case that reads also round trips: writing gives text that reads
//! back to an equal program and writes again the same. To accept new or
//! changed output, run with `UPDATE_SNAPSHOTS=1` and review the diff.

use std::{env, fs, path::Path};
use temper_tmpl::{Profile, difference, text, validate};

struct Case {
    name: String,
    input: String,
    expected: String,
}

/// A file's leading comments, and its cases.
fn read_cases(text: &str) -> (String, Vec<Case>) {
    let mut lines = text.split_inclusive('\n').peekable();
    let mut header = String::new();
    while let Some(line) = lines.next_if(|line| !line.starts_with("=== ")) {
        header.push_str(line);
    }
    let mut cases = Vec::new();
    while let Some(line) = lines.next() {
        let name = line["=== ".len()..].trim_end().to_string();
        let mut input = String::new();
        while let Some(line) =
            lines.next_if(|line| !line.starts_with("--- ") && !line.starts_with("=== "))
        {
            input.push_str(line);
        }
        let mut expected = String::new();
        while let Some(line) = lines.next_if(|line| !line.starts_with("=== ")) {
            expected.push_str(line);
        }
        cases.push(Case {
            name,
            input: trim_blank_end(&input),
            expected: trim_blank_end(&expected),
        });
    }
    (header, cases)
}

/// Text ending in exactly one newline, or empty.
fn trim_blank_end(text: &str) -> String {
    let trimmed = text.trim_end_matches('\n');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

/// The sections a case's input gives, checking its round trip.
fn output(case: &Case) -> String {
    let program = match text::read(&case.input) {
        Ok(program) => program,
        Err(e) => return format!("--- error\n{e}\n"),
    };
    let canonical = text::write(&program);
    let again = text::read(&canonical)
        .unwrap_or_else(|e| panic!("case {:?}: canonical text doesn't read: {e}", case.name));
    if let Some(d) = difference(&program, &again) {
        panic!("case {:?}: reads back different: {d}", case.name);
    }
    assert_eq!(text::write(&again), canonical, "case {:?}", case.name);
    let mut out = String::new();
    if canonical != case.input {
        out.push_str("--- canonical\n");
        out.push_str(&canonical);
    }
    let problems = validate(&program, &Profile::full());
    if !problems.is_empty() {
        out.push_str("--- problems\n");
        for problem in problems {
            out.push_str(&format!("{problem}\n"));
        }
    }
    out
}

#[test]
fn cases_read_write_and_validate_as_expected() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/text");
    let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "cases"))
        .collect();
    paths.sort();
    let (mut count, mut failures) = (0, Vec::new());
    for path in paths {
        let (header, mut cases) = read_cases(&fs::read_to_string(&path).unwrap());
        for case in &mut cases {
            count += 1;
            let actual = output(case);
            if actual != case.expected {
                let file = path.file_name().unwrap().to_string_lossy();
                failures.push(first_difference(&file, case, &actual));
                case.expected = actual;
            }
        }
        if update {
            let mut out = header;
            for (i, case) in cases.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(&format!(
                    "=== {}\n{}{}",
                    case.name, case.input, case.expected
                ));
            }
            fs::write(&path, out).unwrap();
        }
    }
    assert!(count > 0, "no cases found");
    assert!(
        update || failures.is_empty(),
        "{} of {count} cases differ (UPDATE_SNAPSHOTS=1 accepts new output):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn first_difference(file: &str, case: &Case, actual: &str) -> String {
    let place = format!("{file} case {:?}", case.name);
    let (mut expected, mut actual) = (case.expected.lines(), actual.lines());
    for line in 1.. {
        match (expected.next(), actual.next()) {
            (None, None) => break,
            (e, a) if e == a => {}
            (e, a) => {
                let (e, a) = (e.unwrap_or("(end)"), a.unwrap_or("(end)"));
                return format!("{place} output line {line}:\n  expected {e}\n  actual   {a}");
            }
        }
    }
    format!("{place} differs")
}
