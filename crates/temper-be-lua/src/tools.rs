//! How Lua runs built libraries' tests, for `tempest test -b lua`: each
//! library's `tests/init-test.lua` under `lua` (or `LUA`), with LuaUnit
//! beside the libraries, as Kotlin copies it, and `LUA_PATH` of
//! `?.lua;?/init.lua`, writing JUnit XML. `tempest run -b lua` runs the
//! library's `init.lua` the same way, as Kotlin's `Lua51Specifics` does.

use std::path::{Path, PathBuf};
use std::process::Command;
use temper_tmpl::tools::{junit_file, output, program, tool};
use temper_tmpl::{TestResults, Tested, Tools};

pub const TOOLS: Tools = Tools {
    prepare: Some(write_luaunit),
    test,
    run,
};

fn run(dir: &Path, library: &Tested, _: &[Tested]) -> Result<bool, String> {
    program(
        Command::new(tool("LUA", "lua"))
            .args(["-e", "TEMPER_LUA_DEBUG_FUNCS=false"])
            .arg(PathBuf::from(&library.name).join("init.lua"))
            .env("LUA_PATH", "?.lua;?/init.lua")
            .current_dir(dir),
        "lua",
    )
}

/// LuaUnit beside the libraries, as Kotlin copies it there for tests.
fn write_luaunit(dir: &Path, _: &[Tested]) -> Result<(), String> {
    for (path, text) in crate::runtime::LUAUNIT {
        let path = dir.join("luaunit").join(path);
        path.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, text))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

fn test(dir: &Path, library: &Tested, _: &[Tested]) -> TestResults {
    let results = dir.join("test-results.xml");
    let _ = std::fs::remove_file(&results);
    let test = PathBuf::from(&library.name).join("tests/init-test.lua");
    let run = output(
        Command::new(tool("LUA", "lua"))
            .args(["-e", "TEMPER_LUA_DEBUG_FUNCS=false"])
            .arg(&test)
            .args(["-o", "junit", "-n", "test-results.xml"])
            .env("LUA_PATH", "?.lua;?/init.lua")
            .current_dir(dir),
        "lua",
    )?;
    // LuaUnit's names, `Test_.test_name__12`, as the tests' functions'.
    junit_file(&results, &run, |n| {
        let n = n.strip_prefix("Test_.test_").unwrap_or(n);
        let n = match n.rsplit_once("__") {
            Some((n, id)) if id.chars().all(|c| c.is_ascii_digit()) => n,
            _ => n,
        };
        n.to_string()
    })
}
