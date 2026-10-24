//! Temper's frontend: from a library's files to a settled tree, and a small
//! interpreter to run it.
//!
//! - [`library`]: one arena per library build, with every file's tree.
//! - [`engine`]: the settle loop's scheduler: work items, facts and waits.
//! - [`settle`]: the settle loop's steps: reading blocks, resolving names in
//!   [`scope`]s and expanding [`macros`] into [`core`] constructs.
//! - `imports`: `let { a } = import("./dir")` across modules.
//! - `extensions`: `@staticExtension`, so `Int.jsonAdapter()` calls a
//!   function.
//! - [`regex`]: std's regex model as data, Temper's regex dialect, a
//!   matcher, and regex literals.
//! - `testing`: `test` and `assert`, and running a module's tests.
//! - [`order`]: top-level statements in dependency order.
//! - [`builtins`]: the names modules start with.
//! - [`interp`]: a tree interpreter over core constructs.
//! - [`print`](mod@print): settled trees as text, for tests.

pub mod builtins;
pub mod core;
pub mod engine;
pub mod interp;
pub mod library;
mod classes;
mod extensions;
mod imports;
mod macros;
mod testing;
pub mod order;
pub mod print;
pub mod regex;
pub mod scope;
pub mod settle;
