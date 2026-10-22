//! Temper syntax: from source text to tokens, and later to trees.
//!
//! - [`literate`] finds code blocks and paragraphs in `.temper.md` files.
//! - [`lex`] splits source into tokens, as Kotlin Temper's lexer does.

pub mod adapt;
mod id_parts;
pub mod lex;
pub mod literate;
pub mod names;
pub mod operator;
pub mod parse;
pub mod print;
pub mod tree;
