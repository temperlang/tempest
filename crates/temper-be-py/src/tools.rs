//! How Python runs built libraries' tests, for `tempest test -b py`, as
//! Kotlin's `RunPython`: `python3 -B -m temper_core.testing result.xml`
//! in each library's directory, with `temper-core`, `std`, and the
//! libraries on `PYTHONPATH` (or `PYTHON` for the interpreter), reading
//! the JUnit XML it writes. `tempest run -b py` imports the library's
//! package as Kotlin's `top.py` does, with logging to standard output,
//! then waits for its async blocks.

use std::path::Path;
use std::process::Command;
use temper_tmpl::tools::{junit_file, limit_memory, output, program, tool};
use temper_tmpl::{TestResults, Tested, Tools};

pub const TOOLS: Tools = Tools {
    prepare: None,
    test,
    run,
};

/// `PYTHONPATH` for the runtime, std, and every library.
fn python_path(dir: &Path, all: &[Tested]) -> Result<std::ffi::OsString, String> {
    let mut path = vec![dir.join("temper-core"), dir.join("std")];
    path.extend(all.iter().map(|l| dir.join(&l.name)));
    std::env::join_paths(path).map_err(|e| e.to_string())
}

fn run(dir: &Path, library: &Tested, all: &[Tested]) -> Result<bool, String> {
    let package = crate::names::library_package(&library.name);
    let top = format!(
        "from temper_core import init_simple_logging, await_safe_to_exit\n\
         init_simple_logging()\n\
         import {package}\n\
         await_safe_to_exit()\n"
    );
    program(
        limit_memory(&mut Command::new(tool("PYTHON", "python3")))
            .args(["-B", "-c", &top])
            .env("PYTHONPATH", python_path(dir, all)?)
            .current_dir(dir),
        "python3",
    )
}

fn test(dir: &Path, library: &Tested, all: &[Tested]) -> TestResults {
    let path = python_path(dir, all)?;
    let here = dir.join(&library.name);
    let results = here.join("result.xml");
    let _ = std::fs::remove_file(&results);
    let run = output(
        limit_memory(&mut Command::new(tool("PYTHON", "python3")))
            .args(["-B", "-m", "temper_core.testing", "result.xml"])
            .env("PYTHONPATH", path)
            .current_dir(&here),
        "python3",
    )?;
    junit_file(&results, &run, str::to_string)
}
