//! Tests of the steps after the grammar step (`module::build_module`) over
//! the cases in `tests/module/*.cases`: call joins rewritten, for a library
//! or, in `docs.cases`, for documentation, and metadata as JSON.
//!
//! The cases have the format described in `tests/common/mod.rs`, with a
//! `--- module` line before the expected tree in the form of
//! `print::write_ast`. To accept new or changed output, run with
//! `UPDATE_SNAPSHOTS=1` and review the diff. `tests/ast.rs` also checks
//! properties of `build_module` on randomly edited inputs.
//!
//! `compat/check_ast.py --module` and `--docs` check the same inputs
//! against Kotlin Temper.

mod common;

use common::check_cases;
use temper_syntax::module::{Genre, build_module};
use temper_syntax::parse::parse;
use temper_syntax::print::write_ast;

#[test]
fn cases_build_as_expected() {
    check_cases("tests/module", "module", |case| {
        let genre = if case.file.starts_with("docs") { Genre::Documentation } else { Genre::Library };
        let mut actual = String::new();
        write_ast(&build_module(&parse(&case.input, case.kind), genre), 80, &mut actual);
        actual
    });
}
