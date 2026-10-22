//! Temper syntax: from source text to tokens to trees.
//!
//! - [`literate`] finds code blocks and paragraphs in `.temper.md` files.
//! - [`lex`] splits source into tokens, as Kotlin Temper's lexer does.
//! - [`adapt`] adjusts tokens for the parser: semicolons, strings, call
//!   joins and modifier words.
//! - [`parse`] builds a [`tree::Tree`] of operators, using the table in
//!   [`operator`] and interning words in [`names`].
//! - [`ast`] turns a parse tree into calls, blocks and declarations, with
//!   the productions in `grammar` and token values from [`unpack`].
//! - [`module`] finishes a module's tree as Kotlin's `ParseStage` does:
//!   metadata as JSON, and call joins rewritten.
//! - [`print`](mod@print) writes trees as text, for tests and comparisons.

pub mod adapt;
pub mod ast;
mod grammar;
mod id_parts;
pub mod lex;
pub mod literate;
pub mod module;
pub mod names;
pub mod operator;
pub mod parse;
pub mod print;
pub mod tree;
pub mod unpack;
