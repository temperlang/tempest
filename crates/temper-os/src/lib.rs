//! The operating system calls tempest needs that Rust's std doesn't
//! offer, each behind a small safe function. This crate holds all of
//! the workspace's unsafe code but the playground's wasm exports; every
//! other crate forbids it (`unsafe_code`, the workspace's lints).
//!
//! - [`process`]: children in process groups of their own, killed with
//!   tempest's own end, and their memory limit.
//! - [`interrupt`]: Ctrl-C as a flag, rather than the end of tempest.
//! - [`console`]: ANSI escape codes on Windows' console, and Ctrl-Break
//!   for tests.
//! - [`fs`]: on Unix, opening a file relative to a directory's
//!   descriptor, and the effective user ID.
//! - [`host`]: on wasm, the web page's functions the module imports, a
//!   log line's stream and the clock.
//!
//! It calls the platforms through `libc` on Unix and `windows-sys` on
//! Windows, and on wasm the page's imports, declared by hand. Elsewhere,
//! there's nothing to call, and each function does what tempest did
//! before it had the call, as each one says.
//!
//! Each unsafe block does one unsafe thing and says, in a safety
//! comment above it, why it's sound; clippy checks both. Signal and
//! console handlers live here too, as what they may do
//! (async-signal-safe calls and atomics only) is part of why installing
//! them is sound.
//! `tests/unsafe_inventory.rs` lists every file in the repository with
//! unsafe code, and how much, so a change to either shows in review.

pub mod console;
pub mod fs;
pub mod host;
pub mod interrupt;
pub mod process;
