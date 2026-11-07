//! How Python runs built libraries' tests, for `tempest test -b py`, as
//! Kotlin's `RunPython`: `python3 -B -m temper_core.testing result.xml`
//! in each library's directory, with `temper-core`, `std`, and the
//! libraries on `PYTHONPATH` (or `PYTHON` for the interpreter), reading
//! the JUnit XML it writes.

use std::path::Path;
use std::process::Command;
use temper_tmpl::tools::{junit_file, output, tool};
use temper_tmpl::{TestResults, Tested, Tools};

pub const TOOLS: Tools = Tools {
    prepare: None,
    test,
};

fn test(dir: &Path, library: &Tested, all: &[Tested]) -> TestResults {
    let mut path = vec![dir.join("temper-core"), dir.join("std")];
    path.extend(all.iter().map(|l| dir.join(&l.name)));
    let path = std::env::join_paths(path).map_err(|e| e.to_string())?;
    let here = dir.join(&library.name);
    let results = here.join("result.xml");
    let _ = std::fs::remove_file(&results);
    let run = output(
        Command::new(tool("PYTHON", "python3"))
            .args(["-B", "-m", "temper_core.testing", "result.xml"])
            .env("PYTHONPATH", path)
            .current_dir(&here),
        "python3",
    )?;
    junit_file(&results, &run, str::to_string)
}
