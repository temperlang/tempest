//! The `tempest` command, a thin layer over the other crates, which do
//! the work as libraries.
//!
//!     tempest test [--std DIR] [DIR]
//!
//! `test` runs the tests of the libraries under DIR's work root on the
//! interpreter, as Kotlin Temper's `temper test` does
//! ([`temper_frontend::library_tests`]). DIR defaults to the current
//! directory. Std modules are built in ([`temper_frontend::stdlib`]), or
//! come from `--std`'s directory, to test against another copy.
//!
//! Exits with 0 on success, 1 on failure, and 2 on a usage error.

use std::io::{self, Write};
use std::process::ExitCode;
use temper_frontend::library_tests::test_dir;
use temper_frontend::sources::Sources;

const USAGE: &str = "usage: tempest test [--std DIR] [DIR]";

fn main() -> ExitCode {
    // Calls and lowering recurse; give them room (see `interp`).
    let thread = std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let args: Vec<String> = std::env::args().skip(1).collect();
        ExitCode::from(run(&args))
    });
    thread
        .and_then(|t| t.join().map_err(|_| io::Error::other("panicked")))
        .unwrap_or(ExitCode::FAILURE)
}

/// Runs a command, `args` not including the program's name, and returns
/// the exit code.
fn run(args: &[String]) -> u8 {
    match args.first().map(String::as_str) {
        Some("test") => test(&args[1..]),
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            0
        }
        _ => usage(),
    }
}

/// `tempest test [--std DIR] [DIR]`.
fn test(args: &[String]) -> u8 {
    let (mut std_dir, mut dir) = (None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--std" => match args.next() {
                Some(d) => std_dir = Some(d.clone()),
                None => return usage(),
            },
            _ if arg.starts_with('-') || dir.is_some() => return usage(),
            _ => dir = Some(arg.clone()),
        }
    }
    let sources = match std_dir {
        Some(std_dir) => Sources::with_std_dir(&std_dir),
        None => Sources::standard(),
    };
    let dir = dir.as_deref().unwrap_or(".");
    let mut out = io::BufWriter::new(io::stdout().lock());
    let passed = test_dir(&sources, dir, &mut out, &mut io::stderr());
    let _ = out.flush();
    if passed { 0 } else { 1 }
}

fn usage() -> u8 {
    eprintln!("{USAGE}");
    2
}
