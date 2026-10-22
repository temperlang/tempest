//! Lexer tests over the cases in `tests/*.cases`.
//!
//! Each case is a header line `=== name`, or `=== name (markdown)` for a
//! `.temper.md` source, then the input text exactly, then a `--- tokens` line
//! and the expected tokens, one per line in the format of
//! `lex::write_token_lines`. The input ends with the newline before
//! `--- tokens`, so an input that ends in a newline has a blank line there.
//!
//! - `cases_lex_as_expected` checks each case's tokens. To accept new or
//!   changed output, run with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `mutated_cases_keep_lexer_properties` lexes randomly edited copies of
//!   the cases and checks properties that hold for any text. Set
//!   `LEX_FUZZ_SEED` and `LEX_FUZZ_ROUNDS` (edits per case) to vary it.
//!
//! `compat/check_lex.py` checks the same cases against Kotlin Temper, and
//! `compat/fuzz_lex.py` compares edited copies against it.

use std::{env, fs, path::Path};
use temper_syntax::lex::{SourceKind, Synthetic, Token, TokenKind, lex, write_token_lines};

struct Case {
    name: String,
    kind: SourceKind,
    input: String,
    tokens: String,
}

#[test]
fn cases_lex_as_expected() {
    let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut failures = Vec::new();
    let mut count = 0;
    for (path, mut cases) in case_files() {
        for case in &mut cases {
            count += 1;
            let mut actual = String::new();
            write_token_lines(&case.input, &lex(&case.input, case.kind), &mut actual);
            if actual != case.tokens {
                failures.push(first_difference(&path, case, &actual));
                case.tokens = actual;
            }
        }
        if update {
            fs::write(&path, write_cases(&path, &cases)).unwrap();
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

#[test]
fn mutated_cases_keep_lexer_properties() {
    let seed = env_number("LEX_FUZZ_SEED", 1);
    let rounds = env_number("LEX_FUZZ_ROUNDS", 40);
    let mut rng = Rng(seed.max(1));
    for (path, cases) in case_files() {
        for case in cases {
            let context = format!("{} case {:?}", path.display(), case.name);
            check_both(&case.input, &context);
            for round in 0..rounds {
                let text = mutate(&case.input, &mut rng);
                check_both(&text, &format!("{context}, seed {seed} round {round}"));
            }
        }
    }
}

// Reading and writing cases.

fn case_files() -> Vec<(std::path::PathBuf, Vec<Case>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "cases"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let cases = read_cases(&fs::read_to_string(&path).unwrap());
            (path, cases)
        })
        .collect()
}

fn read_cases(text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    let mut rest = text;
    while let Some(start) = find_line(rest, "=== ") {
        let header_end = rest[start..].find('\n').map_or(rest.len(), |i| start + i + 1);
        let header = rest[start + 4..header_end].trim_end();
        let (name, kind) = match header.strip_suffix(" (markdown)") {
            Some(name) => (name, SourceKind::Markdown),
            None => (header, SourceKind::Standalone),
        };
        rest = &rest[header_end..];
        let separator = find_line(rest, "--- tokens\n").expect("case without --- tokens");
        let input = rest[..separator].strip_suffix('\n').unwrap_or("");
        rest = &rest[separator + "--- tokens\n".len()..];
        let tokens_end = find_line(rest, "=== ").unwrap_or(rest.len());
        let tokens = rest[..tokens_end].trim_end_matches('\n');
        cases.push(Case {
            name: name.to_string(),
            kind,
            input: input.to_string(),
            tokens: if tokens.is_empty() { String::new() } else { format!("{tokens}\n") },
        });
        rest = &rest[tokens_end..];
    }
    cases
}

/// Where a line starting with `prefix` begins, if any.
fn find_line(text: &str, prefix: &str) -> Option<usize> {
    if text.starts_with(prefix) {
        return Some(0);
    }
    text.find(&format!("\n{prefix}")).map(|i| i + 1)
}

/// Rewrites a cases file with new tokens, keeping its leading comments.
fn write_cases(path: &Path, cases: &[Case]) -> String {
    let old = fs::read_to_string(path).unwrap();
    let mut out = old[..find_line(&old, "=== ").unwrap_or(old.len())].to_string();
    for case in cases {
        let markdown = if case.kind == SourceKind::Markdown { " (markdown)" } else { "" };
        out.push_str(&format!("=== {}{markdown}\n", case.name));
        if !case.input.is_empty() {
            out.push_str(&case.input);
            out.push('\n');
        }
        out.push_str("--- tokens\n");
        out.push_str(&case.tokens);
        out.push('\n');
    }
    out.truncate(out.trim_end_matches('\n').len());
    out.push('\n');
    out
}

fn first_difference(path: &Path, case: &Case, actual: &str) -> String {
    let place = format!("{} case {:?}", path.file_name().unwrap().to_string_lossy(), case.name);
    let mut expected_lines = case.tokens.lines();
    let mut actual_lines = actual.lines();
    for token in 1.. {
        match (expected_lines.next(), actual_lines.next()) {
            (None, None) => break,
            (e, a) if e == a => {}
            (e, a) => {
                let e = e.unwrap_or("(end)");
                let a = a.unwrap_or("(end)");
                return format!("{place} token {token}:\n  expected {e}\n  actual   {a}");
            }
        }
    }
    format!("{place} differs")
}

// Properties.

/// Checks both source kinds, since any text is valid input to either.
fn check_both(text: &str, context: &str) {
    for kind in [SourceKind::Standalone, SourceKind::Markdown] {
        let tokens = lex(text, kind);
        if let Err(problem) = check_properties(text, &tokens) {
            panic!("{context} as {kind:?}: {problem}\ninput: {text:?}");
        }
        assert!(lex(text, kind) == tokens, "{context}: lexing again differs");
    }
}

/// Tokens from the source tile it exactly, in order, none empty. Synthetic
/// tokens are zero-width at the current position, except paragraph
/// comments, which lie within the prose comment before them.
fn check_properties(text: &str, tokens: &[Token]) -> Result<(), String> {
    let mut pos = 0;
    let mut last_source: Option<Token> = None;
    for (i, &token) in tokens.iter().enumerate() {
        let at = || format!("token {i} {token:?}");
        if token.start > token.end || token.end as usize > text.len() {
            return Err(format!("{} is out of bounds", at()));
        }
        if token.synthetic.is_some() {
            if token.start == token.end {
                if token.start != pos {
                    return Err(format!("{} isn't at position {pos}", at()));
                }
            } else {
                let inside = last_source.is_some_and(|prose| {
                    prose.kind == TokenKind::Comment
                        && prose.start <= token.start
                        && token.end <= prose.end
                });
                if token.synthetic != Some(Synthetic::Paragraph) || !inside {
                    return Err(format!("{} has width but isn't a paragraph comment", at()));
                }
            }
            continue;
        }
        if token.start != pos {
            return Err(format!("{} doesn't start at position {pos}", at()));
        }
        if token.start == token.end {
            return Err(format!("{} is empty but not synthetic", at()));
        }
        if text.get(token.range()).is_none() {
            return Err(format!("{} doesn't span whole characters", at()));
        }
        pos = token.end;
        last_source = Some(token);
    }
    if pos as usize != text.len() {
        return Err(format!("tokens end at {pos} of {}", text.len()));
    }
    Ok(())
}

/// Characters that matter to the lexer, plus some non-ASCII ones. Keep in
/// sync with `PIECES` in `compat/fuzz_lex.py`.
#[rustfmt::skip]
const PIECES: &[&str] = &[
    "\"", "\"\"\"", "'", "`", "{", "}", "(", ")", "[", "]", "<", ">", "/", "*", "\\", "$", "${",
    "\\u{", ":", "~", "#", ";", ".", ",", "-", "=", "!", "0", "1e", "x", "nym`", "\n", "\r\n",
    "  ", "\t", "\n```temper\n", "\n```\n", "\n    ", "é", "ό", "\u{FE0F}", "’", "😀", "\0",
    "return /", "/*", "*/", "//",
];

/// Makes a few random edits: inserting pieces, deleting, duplicating or
/// truncating.
fn mutate(text: &str, rng: &mut Rng) -> String {
    let mut text = text.to_string();
    for _ in 0..1 + rng.below(4) {
        let at = boundary(&text, rng.below(text.len() + 1));
        match rng.below(4) {
            0 | 1 => text.insert_str(at, PIECES[rng.below(PIECES.len())]),
            2 => {
                let end = boundary(&text, at + rng.below(8));
                text.replace_range(at..end, "");
            }
            _ => {
                let end = boundary(&text, at + rng.below(16));
                let copy = text[at..end].to_string();
                text.insert_str(end, &copy);
            }
        }
    }
    if rng.below(10) == 0 {
        let at = boundary(&text, rng.below(text.len() + 1));
        text.truncate(at);
    }
    text
}

/// The nearest char boundary at or before `i`, clamped to the text.
fn boundary(text: &str, i: usize) -> usize {
    let mut i = i.min(text.len());
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// A small xorshift generator, so runs repeat for a given seed.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

fn env_number(name: &str, default: u64) -> u64 {
    env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
