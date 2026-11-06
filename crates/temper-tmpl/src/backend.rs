//! What tools need of a backend, in one description each backend crate
//! exports as `BACKEND`, so a tool keeps one list of backends and loops
//! over it. tmpl never lists backends; the `tempest` crate does.

use crate::Program;
use crate::naming::NamingRules;

/// A backend: its name, runtime, naming, and translation.
pub struct Backend {
    /// Its name on the command line and in `temper.out`, as Kotlin's.
    pub name: &'static str,
    /// Its line comment, which marks each file printed together.
    pub comment: &'static str,
    /// Its runtime's files, as `temper.out` has them beside the
    /// libraries, by paths in its `temper-core` directory; none in wasm,
    /// which writes no runtime, so the playground's download stays small.
    pub runtime: &'static [(&'static str, &'static str)],
    /// Its naming rules, which also name what other libraries use of a
    /// library ([`crate::naming::Interface`]).
    pub rules: &'static NamingRules,
    /// A program's files, or what isn't covered yet. Given a prefix that
    /// leads from the library's output directory to its sources, a
    /// backend that writes source maps (JS) writes them, naming the
    /// sources by their paths after it.
    pub translate: fn(&Program, Option<&str>) -> Result<Vec<OutFile>, String>,
}

/// A generated file: its path relative to the output directory, and its
/// text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub text: String,
}
