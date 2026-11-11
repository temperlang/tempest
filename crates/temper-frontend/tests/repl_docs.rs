//! The docs' REPL transcripts, run through a REPL session: each page of
//! the tutorial (`docs/guide/tutorial/`) and the REPL's own page
//! (`docs/tools/repl.md`) is a session, and each `$ ` line in a code
//! block is an input, typed as shown, with the lines after it what the
//! session prints, exactly. Markers in HTML comments, on the line
//! before a block, say more:
//!
//! - `<!-- repl: paste -->`: the block is typed in as it is, one input
//!   or several; what they print is the next transcript's lines before
//!   its first `$ `, if it has any.
//! - `<!-- repl: skip -->`: the block isn't run.
//! - `<!-- repl: restart -->`: a new session begins at the block.
//! - `<!-- repl: output -->`: the block is what the pasted block before
//!   it printed, all of it.
//!
//! In a pasted block, a comment `// Result: R` says that the input
//! before it gave `R`.
//!
//! In a transcript, a line starting `//` outside an input is commentary,
//! and an input's continuation lines are indented by the prompt's width,
//! two spaces, which are dropped. A marker may hold more than one word,
//! as `<!-- repl: restart paste -->`.
//!
//! `REPL_DOCS_FIX=1` rewrites the pages' transcripts as the session
//! prints them, to review as a diff; `REPL_DOCS_PAGE=name` runs only the
//! pages whose file names contain it.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use temper_frontend::repl::{Chunker, ReplEvent, ReplOptions, ReplSession, Separator};

/// A session and what it has printed since last asked.
struct Session {
    session: ReplSession,
    printed: Rc<RefCell<Vec<String>>>,
}

impl Session {
    fn new() -> Session {
        let printed = Rc::new(RefCell::new(Vec::new()));
        let sink = printed.clone();
        let options = ReplOptions {
            fuel: Some(10_000_000),
            ..ReplOptions::default()
        };
        let session = ReplSession::new(
            options,
            Box::new(move |e: &ReplEvent| {
                sink.borrow_mut().extend(e.text().lines().map(String::from))
            }),
        );
        Session { session, printed }
    }

    fn feed(&mut self, input: &str) -> Vec<String> {
        self.session.feed(input);
        self.printed.take()
    }
}

/// A fenced code block, its marker, and where its body starts.
struct Block {
    marker: Option<String>,
    line: usize,
    body: Vec<String>,
}

fn blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = text.lines().enumerate().peekable();
    let mut marker = None;
    while let Some((i, line)) = lines.next() {
        let trimmed = line.trim();
        if let Some(m) = (trimmed.strip_prefix("<!-- repl: ")).and_then(|m| m.strip_suffix(" -->"))
        {
            marker = Some(m.to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("```") {
            let indent = line.len() - line.trim_start().len();
            let mut body = Vec::new();
            for (_, line) in lines.by_ref() {
                if line.trim() == "```" {
                    break;
                }
                body.push(line.get(indent..).unwrap_or(line.trim_start()).to_string());
            }
            let _ = rest;
            blocks.push(Block {
                marker: marker.take(),
                line: i + 2,
                body,
            });
            continue;
        }
        if !trimmed.is_empty() {
            marker = None;
        }
    }
    blocks
}

/// What a transcript shows after an input, or before its first one for
/// a pasted block: the lines expected, by line number in the page, and
/// what the session printed; and the line after which it's shown.
struct Region {
    after: usize,
    expected: Vec<(usize, String)>,
    actual: Vec<String>,
}

/// A result line's value, as `3` in `interactive#2: 3`.
fn result_of(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("interactive#")?;
    let (number, value) = rest.split_once(": ")?;
    number.chars().all(|c| c.is_ascii_digit()).then_some(value)
}

/// A line of what was printed that says something went wrong, if any.
fn went_wrong(printed: &[String]) -> Option<String> {
    let wrong = |l: &&String| l.contains(": error: ") || l.ends_with(": fail");
    printed.iter().find(wrong).cloned()
}

/// Runs a page's transcripts, giving each difference, and with `fix`,
/// the page with the transcripts as the session printed them.
fn check_page(path: &Path, fix: bool) -> (Vec<String>, Option<String>) {
    let text = fs::read_to_string(path).unwrap();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let mut problems = Vec::new();
    let mut regions: Vec<Region> = Vec::new();
    let mut session = Session::new();
    // What pasted blocks printed, for the next transcript to show.
    let mut pasted: Option<Vec<String>> = None;
    for block in blocks(&text) {
        let words: Vec<&str> = block.marker.as_deref().unwrap_or("").split(' ').collect();
        if words.contains(&"skip") {
            continue;
        }
        if words.contains(&"restart") {
            session = Session::new();
            pasted = None;
        }
        if words.contains(&"output") {
            // What the paste before printed, all of it.
            let expected = (block.body.iter().enumerate())
                .map(|(i, l)| (block.line + i, l.clone()))
                .collect();
            let actual = pasted.take().unwrap_or_default();
            let after = block.line - 1;
            regions.push(Region {
                after,
                expected,
                actual,
            });
            continue;
        }
        if words.contains(&"paste") {
            let mut chunker = Chunker::new(Separator::Newline);
            let mut printed = Vec::new();
            for (i, line) in block.body.iter().enumerate() {
                if let Some(input) = chunker.push_line(line) {
                    printed.extend(session.feed(&input));
                }
                // `// Result: R` says the input before printed `R`.
                if let Some(result) = line.strip_prefix("// Result: ") {
                    let last = printed.iter().rev().find_map(|l| result_of(l));
                    let at = block.line + i;
                    regions.push(Region {
                        after: at - 1,
                        expected: vec![(at, format!("// Result: {result}"))],
                        actual: vec![format!("// Result: {}", last.unwrap_or("nothing"))],
                    });
                }
            }
            if let Some(input) = chunker.finish() {
                printed.extend(session.feed(&input));
            }
            // A failure a `// Result:` expects needn't be shown again.
            let expects_failure = block.body.iter().any(|l| l == "// Result: fail");
            let printed = (!expects_failure).then_some(printed);
            // What a pasted block printed must be shown if it went wrong.
            if let Some(previous) = std::mem::replace(&mut pasted, printed) {
                if let Some(wrong) = went_wrong(&previous) {
                    problems.push(format!(
                        "{name}: a paste before line {} printed {wrong}",
                        block.line
                    ));
                }
            }
            continue;
        }
        if !block.body.iter().any(|l| l.starts_with("$ ")) {
            continue;
        }
        let mut chunker = Chunker::new(Separator::Newline);
        // Before the first input, what a pasted block printed.
        let mut region = Region {
            after: block.line - 1,
            expected: Vec::new(),
            actual: Vec::new(),
        };
        let mut first = true;
        for (i, line) in block.body.iter().enumerate() {
            let at = block.line + i;
            if chunker.is_pending() {
                let line = line.strip_prefix("  ").unwrap_or(line);
                if let Some(input) = chunker.push_line(line) {
                    region.after = at;
                    region.actual = session.feed(&input);
                }
            } else if let Some(input) = line.strip_prefix("$ ") {
                if first {
                    let printed = pasted.take().unwrap_or_default();
                    match region.expected.is_empty() {
                        true => {
                            if let Some(wrong) = went_wrong(&printed) {
                                let at = block.line;
                                problems.push(format!(
                                    "{name}: a paste before line {at} printed {wrong}"
                                ));
                            }
                        }
                        false => region.actual = printed,
                    }
                }
                if !first || !region.expected.is_empty() {
                    regions.push(region);
                }
                first = false;
                region = Region {
                    after: at,
                    expected: Vec::new(),
                    actual: Vec::new(),
                };
                if let Some(input) = chunker.push_line(input) {
                    region.actual = session.feed(&input);
                }
            } else if !line.trim_start().starts_with("//") {
                region.expected.push((at, line.clone()));
            }
        }
        if chunker.is_pending() {
            problems.push(format!("{name}:{}: an input doesn't end", block.line));
        }
        regions.push(region);
    }
    if let Some(wrong) = pasted.as_deref().and_then(went_wrong) {
        problems.push(format!("{name}: the last paste printed {wrong}"));
    }
    let mut edits: Vec<(usize, usize, Vec<String>)> = Vec::new();
    for r in &regions {
        let lines: Vec<String> = r.expected.iter().map(|(_, l)| l.clone()).collect();
        if lines == r.actual {
            continue;
        }
        let at = r.expected.first().map_or(r.after + 1, |&(i, _)| i);
        problems.push(format!(
            "{name}:{at}: expected\n    {}\n  but the REPL printed\n    {}",
            lines.join("\n    "),
            r.actual.join("\n    ")
        ));
        // Lines `first..=last` replaced, or inserted after `after`.
        match (r.expected.first(), r.expected.last()) {
            (Some(&(first, _)), Some(&(last, _))) => {
                edits.push((first, last + 1, r.actual.clone()))
            }
            _ => edits.push((r.after + 1, r.after + 1, r.actual.clone())),
        }
    }
    let fixed = fix.then(|| {
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        for (from, to, with) in edits.into_iter().rev() {
            lines.splice(from - 1..to - 1, with);
        }
        lines.join("\n") + "\n"
    });
    (problems, fixed)
}

fn pages() -> Vec<PathBuf> {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let mut pages: Vec<PathBuf> = fs::read_dir(docs.join("guide/tutorial"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    pages.sort();
    pages.push(docs.join("tools/repl.md"));
    pages
}

#[test]
fn docs_transcripts_run_as_shown() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let only = std::env::var("REPL_DOCS_PAGE").ok();
            let mut problems = Vec::new();
            let mut transcripts = 0;
            for page in pages() {
                let name = page.file_name().unwrap().to_string_lossy().to_string();
                if only.as_ref().is_some_and(|o| !name.contains(o.as_str())) {
                    continue;
                }
                let text = fs::read_to_string(&page).unwrap();
                transcripts += text
                    .lines()
                    .filter(|l| l.trim_start().starts_with("$ "))
                    .count();
                let fix = std::env::var_os("REPL_DOCS_FIX").is_some();
                let (found, fixed) = check_page(&page, fix);
                problems.extend(found);
                if let Some(fixed) = fixed {
                    fs::write(&page, fixed).unwrap();
                }
            }
            assert!(
                only.is_some() || transcripts > 80,
                "only {transcripts} inputs"
            );
            assert!(problems.is_empty(), "{}", problems.join("\n"));
        })
        .unwrap()
        .join()
        .unwrap();
}
