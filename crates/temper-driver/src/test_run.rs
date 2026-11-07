//! `tempest test -b BACKEND`: running built libraries' tests with each
//! language's own tools, as Kotlin Temper's `temper test -b` does, on
//! what `build` wrote to `temper.out/BACKEND`, and reading their results
//! into the interpreter's summary ([`Tally`](temper_frontend::library_tests::Tally)),
//! so `-b interp` and `-b py` print alike. How each language runs its
//! tests is its backend's ([`Backend::tools`], each crate's `tools`
//! module); this runs them for any backend that has them.
//!
//! Each library's results are its tests' names and failures, as the
//! tools report them; a run with no results fails the library, with the
//! tool's output.

use std::path::Path;
use temper_frontend::library_tests::LibraryTests;
use temper_tmpl::Backend;
pub use temper_tmpl::Tested;

/// Runs the tests of the libraries in `dir` (`temper.out/BACKEND`) with
/// the backend's tools, each as its results, or why there are none.
/// `all` is every library built there, which tests may import.
pub fn run(
    backend: &Backend,
    dir: &Path,
    all: &[Tested],
) -> Vec<(String, Result<LibraryTests, String>)> {
    // Tools run in libraries' directories, so paths are absolute.
    let dir = &std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let tools = (backend.tools).ok_or_else(|| format!("no test runner for {}", backend.name));
    let setup = tools
        .clone()
        .and_then(|t| t.prepare.map_or(Ok(()), |prepare| prepare(dir, all)));
    let mut out = Vec::new();
    for library in all.iter().filter(|l| l.tests > 0) {
        let result = setup.clone().and_then(|()| {
            let results = (tools.clone()?.test)(dir, library, all)?;
            Ok(LibraryTests {
                results,
                defined: library.tests,
                output: Vec::new(),
                failure: None,
                diagnostics: String::new(),
                errors: 0,
            })
        });
        out.push((library.name.clone(), result));
    }
    out
}
