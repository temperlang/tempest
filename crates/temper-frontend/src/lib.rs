//! Temper's frontend: from a library's files to a settled tree, and a small
//! interpreter to run it.
//!
//! - [`library`]: one arena per library build, with every file's tree.
//! - [`engine`]: the settle loop's scheduler: work items, facts and waits.
//! - [`settle`]: the settle loop's steps: reading blocks, resolving names in
//!   [`scope`]s and expanding [`macros`] into [`core`] constructs.
//! - [`builtins`]: the names modules start with.
//! - [`interp`]: a tree interpreter over core constructs.
//! - [`print`](mod@print): settled trees as text, for tests.

pub mod builtins;
pub mod core;
pub mod engine;
pub mod interp;
pub mod library;
mod macros;
pub mod print;
pub mod scope;
pub mod settle;
