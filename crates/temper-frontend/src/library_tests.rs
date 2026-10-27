//! Running libraries' tests on the interpreter, as Kotlin Temper's
//! `temper test` does, for the `tempest test` command.
//!
//! [`test_dir`] finds the libraries under a directory's work root
//! ([`crate::project`]), in its [`Sources`], settles each, and runs its
//! tests, `test("name")
//! { … }`. It prints each test's result, then Kotlin's summary, `Tests
//! passed: 29 of 29`, and fails if a test failed or didn't run, or a
//! library has errors.

use crate::interp::run_tests;
use crate::library::Level;
use crate::project::{LibrarySources, find_libraries, load_library, work_root};
use crate::settle::{Settled, settle};
use crate::sources::Sources;
use std::io::Write;

/// Tests the libraries under `dir`'s work root, writing each test's
/// result and the summary to `out`, and diagnostics and problems to
/// `err`. Whether all passed, without errors.
pub fn test_dir(sources: &Sources, dir: &str, out: &mut dyn Write, err: &mut dyn Write) -> bool {
    let root = work_root(sources, dir);
    let libraries = find_libraries(sources, &root);
    if libraries.is_empty() {
        let _ = writeln!(err, "No Temper library (config.temper.md) under {root}");
        return false;
    }
    let mut tally = Tally::default();
    let mut ok = true;
    for library in &libraries {
        let tests = match test_library(sources, library) {
            Ok(tests) => tests,
            Err(message) => {
                let _ = writeln!(err, "{message}");
                ok = false;
                continue;
            }
        };
        let _ = err.write_all(tests.diagnostics.as_bytes());
        ok &= tests.errors == 0;
        let _ = writeln!(out, "Testing {}", library.root);
        for line in &tests.output {
            let _ = writeln!(out, "{line}");
        }
        for (name, failure) in &tests.results {
            let _ = match failure {
                None => writeln!(out, "  ok      {name}"),
                Some(cause) => writeln!(out, "  failed  {name} - {cause}"),
            };
        }
        if let Some(failure) = &tests.failure {
            let _ = writeln!(out, "  {failure}");
        }
        tally.add(&tests);
    }
    let _ = writeln!(out, "{}", tally.summary());
    let _ = out.flush();
    ok && tally.all_passed()
}

/// A library's tests, run.
#[derive(Debug)]
pub struct LibraryTests {
    /// Each test that ran, by name, with why it failed, if it did.
    pub results: Vec<(String, Option<String>)>,
    /// How many tests the library declares.
    pub defined: usize,
    /// What the library logged.
    pub output: Vec<String>,
    /// How the run failed, if it stopped before the tests finished.
    pub failure: Option<String>,
    /// The library's diagnostics, a line each, with file, line and column.
    pub diagnostics: String,
    /// How many of those are errors.
    pub errors: usize,
}

/// Settles a library and runs its tests. The error names a file that
/// can't be read.
pub fn test_library(sources: &Sources, library: &LibrarySources) -> Result<LibraryTests, String> {
    let settled = settle(load_library(sources, library)?);
    let mut diagnostics = String::new();
    write_located_diagnostics(&settled, &mut diagnostics);
    let errors = (settled.library.diagnostics.iter())
        .filter(|d| d.level == Level::Error)
        .count();
    let defined = settled
        .modules
        .iter()
        .filter_map(|m| m.tests)
        .map(|run| {
            // `r#([t#test, …])`: the list's items after its callee.
            let list = settled.library.tree.children(run)[1];
            settled.library.tree.children(list).len() - 1
        })
        .sum();
    let run = run_tests(&settled, None);
    let (mut output, mut report) = (Vec::new(), Vec::new());
    let mut in_report = false;
    for line in run.output {
        in_report |= line == "<testsuites>";
        if in_report {
            in_report = line != "</testsuites>";
            report.push(line);
        } else {
            output.push(line);
        }
    }
    Ok(LibraryTests {
        results: parse_junit(&report),
        defined,
        output,
        failure: run.failure,
        diagnostics,
        errors,
    })
}

/// Each test case in JUnit XML as std's `reportTestResults` writes it, a
/// tag a line, with its failure message, if any.
pub fn parse_junit(lines: &[String]) -> Vec<(String, Option<String>)> {
    let mut results: Vec<(String, Option<String>)> = Vec::new();
    for line in lines {
        let line = line.trim_start();
        if line.starts_with("<testcase ") {
            results.push((attribute(line, "name").unwrap_or_default(), None));
        } else if line.starts_with("<failure ") {
            if let Some(last) = results.last_mut() {
                last.1 = Some(attribute(line, "message").unwrap_or_default());
            }
        }
    }
    results
}

/// An attribute's value in a tag, quoted with `'`, unescaped.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let start = tag.find(&format!(" {name}='"))? + name.len() + 3;
    let end = start + tag[start..].find('\'')?;
    Some(
        tag[start..end]
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&#39;", "'")
            .replace("&#34;", "\"")
            .replace("&amp;", "&"),
    )
}

/// Diagnostics as `path:line:column: level: message`, a line each, with
/// the column in characters, both from 1.
pub fn write_located_diagnostics(s: &Settled, out: &mut String) {
    use std::fmt::Write;
    for d in &s.library.diagnostics {
        let level = format!("{:?}", d.level).to_lowercase();
        let (file, offset) = s.library.locate(d.span.start);
        let file = &s.library.files[file.0 as usize];
        let source = file.parsed.source();
        let before = source.get(..offset as usize).unwrap_or(source);
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        let path = &file.path;
        let _ = writeln!(out, "{path}:{line}:{column}: {level}: {}", d.message.text());
    }
}

/// Tests across libraries, counted as Kotlin's `TestTally` counts them.
#[derive(Debug, Default, PartialEq)]
pub struct Tally {
    pub run: usize,
    pub failed: usize,
    pub defined: usize,
    pub not_run: usize,
    pub unexpected: usize,
}

impl Tally {
    pub fn add(&mut self, tests: &LibraryTests) {
        let run = tests.results.len();
        self.run += run;
        self.failed += tests.results.iter().filter(|r| r.1.is_some()).count();
        self.defined += tests.defined;
        self.not_run += tests.defined.saturating_sub(run);
        self.unexpected += run.saturating_sub(tests.defined);
    }

    /// Whether every test declared ran and passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.not_run == 0 && self.unexpected == 0
    }

    /// Kotlin's summary line: `Tests passed: 28 of 29 (1 not run)`.
    pub fn summary(&self) -> String {
        let passed = self.run - self.failed;
        let total = self.defined;
        match (self.not_run, self.unexpected) {
            (0, 0) => format!("Tests passed: {passed} of {total}"),
            (n, 0) => format!("Tests passed: {passed} of {total} ({n} not run)"),
            (0, u) => format!("Tests passed: {passed} of {total} ({u} unexpected)"),
            (n, u) => format!("Tests passed: {passed} of {total} ({n} not run, {u} unexpected)"),
        }
    }
}
