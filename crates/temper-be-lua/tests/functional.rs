//! The Lua backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! ([`temper_funtests`]), on the Rust backend's own programs
//! (`crates/temper-be-rust/tests/programs/`), which cover operators,
//! jumps, module state, and string order beyond those tests, and on
//! programs of Lua's own ([`HIDING`], [`REPEATS`], and [`locals`]).
//!
//! - `goldens`: each program's generated library, laid out as Kotlin's,
//!   with its `init.lua` matching the golden in `tests/goldens/`, named
//!   for the test's path (`algos-fibonacci.lua`), and its rockspec as
//!   Kotlin writes one. To accept new or changed output, run with
//!   `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `programs_log_as_the_interpreter_does`: Lua's own programs log, on
//!   tempest's interpreter, what the test expects of them.
//! - `runs`: with `lua` (or `LUA`), from the directory holding the
//!   generated libraries and the vendored runtime
//!   (`vendor/temper/be-lua/temper-core`), with Kotlin's `LUA_PATH`
//!   (`?.lua;?/init.lua`), one process requires every library in turn,
//!   and each logs what the test expects of Kotlin's `lua` backend; one
//!   library also runs alone, as Kotlin runs a program
//!   (`lua fibonacci/init.lua`); and `module-state`'s exported `var`s,
//!   read from its exports table, stay current as its functions write
//!   them. `luac -p` (or `LUAC`) parses every generated file, if there's
//!   a `luac`, and LuaLS (`lua-language-server --check`, or `LUALS`)
//!   checks them against their LuaCATS annotations and a typed view of
//!   the runtime (`tests/luals/temper-core.lua`), finding no problems at
//!   its `Warning` level. Skipped, with a message, without `lua`, and the
//!   check without LuaLS; with `TEMPEST_REQUIRE_CHECKERS` set, as in CI,
//!   either missing fails it instead. A missing `luac` only skips.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use temper_be_lua::{LUA, OutFile, translate};
use temper_frontend::interp;
use temper_frontend::library::Library;
use temper_frontend::project::load_imports;
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::to_tmpl;
use temper_syntax::lex::SourceKind;
use temper_tmpl::{Profile, Program, text, validate};

/// The functional tests tmpl's translation covers, by their paths.
const COVERED: [&str; 7] = [
    "algos/fibonacci",
    "algos/hello-world",
    "control-flow/if-return",
    "control-flow/loop-reenterable",
    "names/nonascii",
    "types/int/basics",
    "types/string/is-empty",
];

/// The Rust backend's programs, in its `tests/programs/`: Temper
/// (`NAME.temper.md`), or tmpl that the frontend doesn't make yet
/// (`NAME.tmpl`, with the lines it logs in `// log:` comments).
const PROGRAMS: [&str; 4] = ["module-state", "operators", "string-order", "jumps.tmpl"];

/// A library of names that Lua reserves or the translator prints (Lua's
/// keywords `end`, `goto`, `repeat`, and `nil`, which Temper allows, and
/// `temper`, `print`, `tostring`, `math`, `exports`, `module`, `require`,
/// and `_ENV`), each taking `_`; `Boolean`'s order, which Lua can't
/// compare, as logic and, where the right side calls something, through
/// the runtime; an `if` value of each kind; strings Lua would read
/// otherwise (quotes, a backslash, a NUL before a digit, and a
/// right-to-left override); and shifts by more than the width.
const HIDING: &str = r#"# Hiding

    let end(goto: Int): Int { goto + 1 }
    let temper = "t";
    let print(repeat: Boolean, nil: Boolean): Boolean { repeat < nil }
    var calls = 0;
    let tostring(b: Boolean): Boolean { calls += 1; b }
    let math(a: Boolean, b: Boolean): String {
      "${(a <= tostring(b)).toString()} ${(a > b).toString()} ${(a >= b).toString()}"
    }
    let exports(b: Boolean): String { "${if (b) { "yes" } else { "no" }}!" }
    let module(b: Boolean, c: Boolean): String { (if (b) { true } else { c }).toString() }
    let require(b: Boolean, c: Boolean): String { (if (b) { false } else { c }).toString() }
    let _ENV(b: Boolean, c: Boolean, d: Boolean): String { (if (b) { c } else { d }).toString() }
    let shifts(n: Int): String { "${1 << n} ${-8 >> n} ${-8 >>> (n - 4)}" }
    console.log("${end(1).toString()} ${temper} ${print(false, true).toString()} ${print(true, false).toString()}");
    console.log("${math(true, false)} ${calls.toString()} ${exports(true)} ${exports(false)}");
    console.log("${module(false, true)} ${require(false, true)} ${_ENV(true, false, true)}");
    console.log("\"it's\" \\ a\u{0}1 \u{202E}x");
    console.log(shifts(33));
"#;

/// What [`HIDING`] logs.
const HIDING_LOG: [&str; 5] = [
    "2 t true false",
    "false true true 1 yes! no!",
    "true true false",
    "\"it's\" \\ a\u{0}1 \u{202E}x",
    "2 -4 7",
];

/// Jumps the frontend doesn't translate into tmpl yet, as tmpl: a
/// `continue` in a `do … while` whose body declares a local, which Lua's
/// `goto` can't jump past to `until`, so the body takes a `do … end` of
/// its own; a `continue` of an outer loop from an inner one; and a
/// bubble in a function with a result, which `return`s it.
const REPEATS: &str = "tmpl 1 model 1 library repeats 0.0.0

uses `temper-core`#1 0.1.0 {
  interface Console#2 {
    connected method log#3(message: String): Void;
  }
  class Int32#4 {
    connected method toString#5(radix?: Int32): String;
  }
  connected fn getConsole#6(name?: String): Console#2;
}

module repeats#0 {
  let ~console#7: Console#2;

  fn odds#8(n#9: Int32): Int32 {
    var i#10: Int32 = 0;
    var sum#11: Int32 = 0;
    ~loop#12: do {
      i#10 = %Int32.add(i#10, 1);
      let bit#13: Int32 = %Int32.bitAnd(i#10, 1);
      if (%Int32.eq(bit#13, 0)) {
        continue ~loop#12;
      }
      sum#11 = %Int32.add(sum#11, i#10);
    } while (%Int32.lt(i#10, n#9));
    return sum#11;
  }

  fn pairs#14(n#15: Int32): Int32 {
    var count#16: Int32 = 0;
    var i#17: Int32 = 0;
    rows#18: while (%Int32.lt(i#17, n#15)) {
      i#17 = %Int32.add(i#17, 1);
      var j#19: Int32 = 0;
      ~loop#20: while (true) {
        j#19 = %Int32.add(j#19, 1);
        if (%Int32.gt(j#19, i#17)) {
          continue rows#18;
        }
        count#16 = %Int32.add(count#16, 1);
      }
    }
    return count#16;
  }

  fn positive#21(n#22: Int32): Int32 throws Bubble {
    if (%Int32.lt(n#22, 0)) {
      return (error(\"negative\")): Int32;
    }
    return n#22;
  }

  init {
    init ~console#7 = getConsole#6(omitted);
    ~console#7.log#3(odds#8(9).toString#5(omitted));
    ~console#7.log#3(pairs#14(4).toString#5(omitted));
    ~console#7.log#3(positive#21(3)^.toString#5(omitted));
  }
}
";

/// How many module-level `let`s and functions [`locals`] declares, past
/// what Lua allows a chunk as locals.
const MANY: usize = 220;

/// A library with more module-level declarations than Lua allows a chunk
/// locals, so those past the first 100 are fields of a table, and a
/// function that uses the first and the last.
fn locals() -> (String, Vec<String>) {
    let mut text = String::from("# Locals\n\n");
    for i in 0..MANY / 2 {
        text.push_str(&format!("    let v{i} = {i};\n"));
        text.push_str(&format!("    let f{i}(n: Int): Int {{ n + v{i} }}\n"));
    }
    let last = MANY / 2 - 1;
    text.push_str(&format!(
        "    export let sum(): Int {{ f0(1) + f{last}(v{last}) }}\n"
    ));
    text.push_str("    console.log(sum().toString());\n");
    (text, vec![format!("{}", 1 + 2 * last)])
}

/// A program to translate: its library's name, its sources, and the log
/// it expects.
struct Case {
    /// Its golden's name: the test's path with `-` for `/`.
    key: String,
    library: String,
    sources: Vec<(String, String)>,
    expected: Vec<String>,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for path in COVERED {
        let test = temper_funtests::test(path).unwrap_or_else(|| panic!("no test {path}"));
        cases.push(Case {
            key: path.replace('/', "-"),
            library: test.name().to_string(),
            sources: (test.sources().into_iter())
                .map(|s| (s.path, s.text))
                .collect(),
            expected: test.expected_log("lua"),
        });
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../temper-be-rust/tests/programs");
    for file in PROGRAMS {
        if let Some(name) = file.strip_suffix(".tmpl") {
            let path = dir.join(file);
            let text = fs::read_to_string(&path).unwrap();
            let expected = (text.lines())
                .filter_map(|l| l.strip_prefix("// log: "))
                .map(str::to_string)
                .collect();
            cases.push(Case {
                key: name.to_string(),
                library: name.to_string(),
                expected,
                sources: vec![(path.to_string_lossy().into_owned(), text)],
            });
            continue;
        }
        let path = dir.join(format!("{file}.temper.md"));
        let text = fs::read_to_string(&path).unwrap();
        cases.push(Case {
            key: file.to_string(),
            library: file.to_string(),
            expected: temper_funtests::expected_log(&text, "lua"),
            sources: vec![(path.to_string_lossy().into_owned(), text)],
        });
    }
    cases.push(Case {
        key: "hiding".into(),
        library: "hiding".into(),
        sources: vec![("hiding.temper.md".into(), HIDING.into())],
        expected: HIDING_LOG.iter().map(|s| s.to_string()).collect(),
    });
    cases.push(Case {
        key: "repeats".into(),
        library: "repeats".into(),
        sources: vec![("repeats.tmpl".into(), REPEATS.into())],
        expected: vec!["25".into(), "10".into(), "3".into()],
    });
    let (text, expected) = locals();
    cases.push(Case {
        key: "locals".into(),
        library: "locals".into(),
        sources: vec![("locals.temper.md".into(), text)],
        expected,
    });
    cases
}

/// A case's library, settled and translated to tmpl, or read as tmpl.
fn program(case: &Case) -> Program {
    let key = &case.key;
    if let [(path, text)] = &case.sources[..] {
        if path.ends_with(".tmpl") {
            let program = text::read(text).unwrap_or_else(|e| panic!("{key}: {e}"));
            let problems = validate(&program, &Profile::full());
            assert!(problems.is_empty(), "{key}: {problems:#?}");
            return program;
        }
    }
    let mut library = Library::new();
    for (path, text) in &case.sources {
        let kind = if path.ends_with(".md") {
            SourceKind::Markdown
        } else {
            SourceKind::Standalone
        };
        library.add_file(path, text, kind);
    }
    load_imports(&Sources::standard(), &mut library);
    let program = to_tmpl(&settle(library), &case.library).unwrap_or_else(|e| panic!("{key}: {e}"));
    let problems = validate(&program, &Profile::full());
    assert!(problems.is_empty(), "{key}: {problems:#?}");
    program
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&program(case), &LUA).unwrap_or_else(|e| panic!("{}: {e}", case.key))
}

/// Runs a test on a thread with room for the settle loop's recursion.
fn with_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

/// The rockspec Kotlin writes for a library without a version.
fn rockspec(library: &str) -> String {
    format!(
        "package = '{library}'
version = 'dev-1'
rockspec_format = '3.0'
description = {{
}}
build = {{
    type = 'builtin',
    copy_directories = {{'.'}},
    modules = {{}},
}}
dependencies = {{
    'lua >= 5.1',
    'temper-core == 0.6.0',
}}
source = {{
    url = 'file://{library}.zip',
}}
"
    )
}

#[test]
fn goldens() {
    with_stack(|| {
        let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
        let goldens = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens");
        let mut differ = Vec::new();
        for case in cases() {
            let files = generate(&case);
            let library = &case.library;
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(
                paths,
                [
                    format!("{library}/init.lua"),
                    format!("{library}/{library}-dev-1.rockspec"),
                ],
                "{}",
                case.key
            );
            assert_eq!(files[1].text, rockspec(library), "{}", case.key);
            if case.key == "locals" {
                // Too long to keep; `runs` runs it.
                continue;
            }
            let module = &files[0].text;
            let golden = goldens.join(format!("{}.lua", case.key));
            let expected = fs::read_to_string(&golden).unwrap_or_default();
            if *module != expected {
                if update {
                    fs::create_dir_all(&goldens).unwrap();
                    fs::write(&golden, module).unwrap();
                } else {
                    differ.push(format!("{}:\n{module}", golden.display()));
                }
            }
        }
        assert!(
            differ.is_empty(),
            "goldens differ (UPDATE_SNAPSHOTS=1 accepts new output):\n{}",
            differ.join("\n")
        );
    });
}

#[test]
fn programs_log_as_the_interpreter_does() {
    with_stack(|| {
        for case in cases() {
            if !matches!(case.key.as_str(), "hiding" | "locals") {
                continue;
            }
            let mut library = Library::new();
            for (path, text) in &case.sources {
                library.add_file(path, text, SourceKind::Markdown);
            }
            load_imports(&Sources::standard(), &mut library);
            let run = interp::run(&settle(library), Some(interp::FUEL));
            assert_eq!(run.failure, None, "{}", case.key);
            assert_eq!(run.output, case.expected, "{}", case.key);
        }
    });
}

/// A program given by an environment variable, or found on `PATH`, if it
/// runs with `arg`.
fn tool(var: &str, name: &str, arg: &str) -> Option<String> {
    let candidate = env::var(var).unwrap_or_else(|_| name.to_string());
    Command::new(&candidate)
        .arg(arg)
        .output()
        .is_ok_and(|o| o.status.success())
        .then_some(candidate)
}

fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/be-lua/temper-core")
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            copy_dir(&path, &to.join(entry.file_name()));
        } else {
            fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}

/// Kotlin's `LUA_PATH`, for running from the directory that holds the
/// libraries and `temper-core`.
const LUA_PATH: &str = "?.lua;?/init.lua";

/// Requires each library named on the command line in turn, after a line
/// naming it, so one process runs them all; an error is a line too.
const RUNNER: &str = r#"for _, name in ipairs(arg) do
  io.write("==> ", name, "\n")
  local ok, err = pcall(require, name)
  if not ok then
    io.write("!!> ", (tostring(err):gsub("\n", " ")), "\n")
  end
end
"#;

/// Reads `module-state`'s exports after its functions write them.
const LIVE: &str = r#"local state = require("module-state")
state.tick()
state.tick()
print(state.ticks, state.total, state.greeting, state.count)
"#;

/// LuaLS's settings for checking generated code: Lua 5.4, `require` as
/// Kotlin's `LUA_PATH` finds modules, the runtime's typed view as a
/// library in place of the runtime itself, and the type checks on every
/// file. Doc comments aren't required, as with Java's `-missing`, until
/// tmpl carries Temper's docs.
fn luarc(meta: &Path) -> String {
    let meta = meta.to_string_lossy().replace('\\', "/");
    format!(
        r#"{{
  "runtime.version": "Lua 5.4",
  "runtime.path": ["?.lua", "?/init.lua"],
  "workspace.library": ["{meta}"],
  "workspace.ignoreDir": ["temper-core"],
  "workspace.checkThirdParty": false,
  "diagnostics.disable": ["missing-local-export-doc"],
  "diagnostics.groupFileStatus": {{
    "type-check": "Any",
    "luadoc": "Any",
    "unbalanced": "Any",
    "global": "Any",
    "await": "Any"
  }}
}}
"#
    )
}

#[test]
fn runs() {
    let required = env::var_os("TEMPEST_REQUIRE_CHECKERS").is_some();
    let Some(lua) = tool("LUA", "lua", "-v") else {
        assert!(!required, "no lua");
        eprintln!("skipped: no Lua (on PATH, or named by LUA)");
        return;
    };
    let luals = tool("LUALS", "lua-language-server", "--version");
    if luals.is_none() {
        assert!(!required, "no lua-language-server");
        eprintln!("type check skipped: no LuaLS (lua-language-server on PATH, or named by LUALS)");
    }
    let luac = tool("LUAC", "luac", "-v");
    if luac.is_none() {
        eprintln!("syntax check skipped: no luac (on PATH, or named by LUAC)");
    }
    with_stack(move || {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-lua");
        let _ = fs::remove_dir_all(&out);
        let dir = out.join("lua");
        copy_dir(&runtime(), &dir.join("temper-core"));
        let cases = cases();
        let mut generated = Vec::new();
        for case in &cases {
            for file in generate(case) {
                let path = dir.join(&file.path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &file.text).unwrap();
                if file.path.ends_with(".lua") {
                    generated.push(path);
                }
            }
        }
        let mut failures = Vec::new();
        // Every library, in one process.
        let runner = out.join("runner.lua");
        fs::write(&runner, RUNNER).unwrap();
        let run = Command::new(&lua)
            .arg(&runner)
            .args(cases.iter().map(|c| &c.library))
            .current_dir(&dir)
            .env("LUA_PATH", LUA_PATH)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&run.stdout);
        let mut logs: Vec<(String, Vec<String>)> = Vec::new();
        for line in stdout.split('\n') {
            match line.strip_prefix("==> ") {
                Some(name) => logs.push((name.to_string(), Vec::new())),
                None => {
                    if let Some((_, log)) = logs.last_mut() {
                        log.push(line.to_string());
                    }
                }
            }
        }
        if let Some((_, log)) = logs.last_mut() {
            // The newline ending the output.
            if log.last().is_some_and(|l| l.is_empty()) {
                log.pop();
            }
        }
        if !run.status.success() || logs.len() != cases.len() {
            failures.push(format!(
                "the runner failed:\n{stdout}{}",
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        for (case, (name, log)) in cases.iter().zip(&logs) {
            if *name != case.library || *log != case.expected {
                failures.push(format!(
                    "{}: expected {:?}, got {log:?}",
                    case.key, case.expected
                ));
            }
        }
        // One library as Kotlin runs a program.
        let alone = Command::new(&lua)
            .arg("fibonacci/init.lua")
            .current_dir(&dir)
            .env("LUA_PATH", LUA_PATH)
            .output()
            .unwrap();
        let expected = &cases[0].expected;
        let stdout = String::from_utf8_lossy(&alone.stdout);
        if !alone.status.success() || stdout.lines().collect::<Vec<_>>() != *expected {
            failures.push(format!(
                "lua fibonacci/init.lua: expected {expected:?}, got:\n{stdout}{}",
                String::from_utf8_lossy(&alone.stderr)
            ));
        }
        // Exported `var`s, as users see them.
        let live = out.join("live.lua");
        fs::write(&live, LIVE).unwrap();
        let read = Command::new(&lua)
            .arg(&live)
            .current_dir(&dir)
            .env("LUA_PATH", LUA_PATH)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&read.stdout);
        let expected = "hi 1 of 3 11\nhi 2 of 3 13\nlocal 4\n2\t13\thi\tnil\n";
        if !read.status.success() || stdout != expected {
            failures.push(format!(
                "module-state's exports: expected {expected:?}, got {stdout:?}\n{}",
                String::from_utf8_lossy(&read.stderr)
            ));
        }
        if let Some(luac) = &luac {
            let parse = Command::new(luac)
                .arg("-p")
                .args(&generated)
                .output()
                .unwrap();
            if !parse.status.success() {
                failures.push(format!(
                    "luac -p:\n{}",
                    String::from_utf8_lossy(&parse.stderr)
                ));
            }
        }
        if let Some(luals) = &luals {
            failures.extend(check(luals, &out, &dir));
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}

/// Checks the generated libraries in `dir` with LuaLS, against the
/// runtime's typed view, and gives its problems, if any.
fn check(luals: &str, out: &Path, dir: &Path) -> Vec<String> {
    let meta = out.join("luals-meta");
    fs::create_dir_all(&meta).unwrap();
    let view = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/luals/temper-core.lua");
    fs::copy(view, meta.join("temper-core.lua")).unwrap();
    fs::write(dir.join(".luarc.json"), luarc(&meta)).unwrap();
    let logs = out.join("luals-logs");
    let check = Command::new(luals)
        .arg(format!("--check={}", dir.display()))
        .arg(format!(
            "--configpath={}",
            dir.join(".luarc.json").display()
        ))
        .arg("--checklevel=Warning")
        .arg("--locale=en-us")
        .arg(format!("--logpath={}", logs.display()))
        .current_dir(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&check.stdout);
    let report = fs::read_to_string(logs.join("check.json")).unwrap_or_default();
    // LuaLS says so when it finds nothing; otherwise it writes what it
    // found to `check.json` in its log directory.
    let empty = report
        .trim_matches(|c: char| c.is_whitespace() || "{}[]".contains(c))
        .is_empty();
    let said = stdout.to_lowercase();
    let clean = said.contains("no problems found")
        || (check.status.success() && empty && !said.contains("problem"));
    if clean {
        Vec::new()
    } else {
        vec![format!(
            "lua-language-server --check:\n{stdout}{}\n{report}",
            String::from_utf8_lossy(&check.stderr)
        )]
    }
}
