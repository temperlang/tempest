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
//! - [`typing`]: declarations' [`types`] as facts in the settle loop,
//!   and type errors.
//! - [`order`]: top-level statements in dependency order.
//! - [`builtins`]: the names modules start with.
//! - [`interp`]: an interpreter over core constructs, each function
//!   lowered to a flat form.
//! - [`print`](mod@print): settled trees as text, for tests.
//! - [`project`]: libraries on disk, found as `temper test` finds them.
//! - [`library_tests`]: running libraries' tests, for `tempest test`.
//! - `web_client`: HTTP requests for std's `net`, with the `net` feature.

pub mod builtins;
mod classes;
pub mod core;
pub mod engine;
mod extensions;
mod flow;
mod imports;
pub mod interp;
pub mod library;
pub mod library_tests;
mod macros;
pub mod order;
pub mod print;
pub mod project;
pub mod regex;
pub mod scope;
pub mod settle;
mod testing;
pub mod types;
pub mod typing;
mod web_client;
