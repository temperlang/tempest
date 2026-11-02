//! The Lua backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! (`temper-funtests`), its class tests among them (`kit::CLASSES`),
//! those with nullable types and `orelse` (`kit::NULLABLES`), those
//! with generics (`kit::GENERICS`), and those with core's other types
//! and members (`kit::CORE_TYPES`), on
//! the shared programs (`crates/temper-be-testkit/programs/`), which
//! cover operators, jumps, module state, string order, classes, and
//! nullables beyond those tests, and on programs of Lua's own ([`HIDING`],
//! [`HOISTING`], [`MEMBERS`], [`REPEATS`], and [`locals`]).
//!
//! - `goldens`: each program's generated library, laid out as Kotlin's,
//!   with its `init.lua` matching the golden in `tests/goldens/`, named
//!   for the test's path (`algos-fibonacci.lua`), and its rockspec as
//!   Kotlin writes one. To accept new or changed output, run with
//!   `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `programs_log_as_the_interpreter_does`: Lua's own Temper programs
//!   log, on tempest's interpreter, what the test expects of them.
//! - `runs`: with `lua` (or `LUA`), from the directory holding the
//!   generated libraries and the vendored runtime
//!   (`vendor/temper/be-lua/temper-core`), with Kotlin's `LUA_PATH`
//!   (`?.lua;?/init.lua`), one process requires every library in turn,
//!   and each logs what the test expects of Kotlin's `lua` backend (a
//!   library named as one before it, as both `basics`, in a directory
//!   and process of its own); one
//!   library also runs alone, as Kotlin runs a program
//!   (`lua fibonacci/init.lua`); `module-state`'s exported `var`s, read
//!   from its exports table, stay current as its functions write them,
//!   as `members`' does as a method writes it, and its static, property,
//!   and instance's `tostring` read as Lua's users read them. `luac -p`
//!   (or `LUAC`) parses every generated file, if there's a `luac`, and
//!   LuaLS (`lua-language-server --check`, or `LUALS`)
//!   checks them against their LuaCATS annotations and a typed view of
//!   the runtime (`tests/luals/temper-core.lua`), finding no problems at
//!   its `Warning` level. Skipped, with a message, without `lua`, and the
//!   check without LuaLS; with `TEMPEST_REQUIRE_CHECKERS` set, as in CI,
//!   either missing fails it instead. A missing `luac` only skips.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use temper_be_lua::{LUA, OutFile, translate};
use temper_be_testkit::{self as kit, Case, Goldens};

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

/// What Lua can't write inline, hoisted before its statement: an `if`
/// value of `Boolean`s whose condition does something, `orelse` on the
/// right of `&&` and `||`, `orelse` in arguments after others, and `??`
/// and `orelse` in a string template.
const HOISTING: &str = r#"# Hoisting

    var calls = 0;
    let noisy(b: Boolean): Boolean { calls += 1; b }
    let half(n: Int): Int throws Bubble {
      if (n % 2 != 0) { bubble(); }
      n / 2
    }
    let pick(c: Boolean, a: Boolean, b: Boolean): Boolean { if (noisy(c)) { a } else { b } }
    let isPos(n: Int): Boolean { n > 0 }
    let both(a: Boolean, n: Int): Boolean { a && isPos(half(n) orelse 0) }
    let either(a: Boolean, n: Int): Boolean { a || isPos(half(n) orelse 0) }
    let sum(a: Int, b: Int): Int { a + b }
    let label(x: String?): String { "got ${x ?? "nothing"} and ${(half(3) orelse -1).toString()}" }
    console.log("${pick(true, false, true)} ${pick(false, false, true)} ${calls}");
    console.log("${both(true, 4)} ${both(true, 3)} ${both(false, 4)}");
    console.log("${either(false, 4)} ${either(false, 3)} ${either(true, 3)}");
    console.log(sum(half(8) orelse 0, half(7) orelse 100).toString());
    console.log(label(null));
    console.log(label("x"));
"#;

/// What [`HOISTING`] logs.
const HOISTING_LOG: [&str; 6] = [
    "false true 2",
    "true false false",
    "true false true",
    "104",
    "got nothing and -1",
    "got x and -1",
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

/// Classes where Lua's translation takes care: an exported `var` that only
/// a method writes, which stays current in the exports table; a static
/// named as a key of the runtime's type table (`methods`), which takes a
/// `_`; a method call on the right of `Boolean`'s `<`, which Lua's `and`
/// would skip; a field that implements an interface's default getter,
/// stored under a key of its own, with a getter; and a plain `toString()`,
/// which `tostring` shows ([`MEMBERS_READ`]).
const MEMBERS: &str = r#"# Members

    let make(n: Int): Box { new Box(n) }
    export var made = 0;
    export class Box(public var size: Int) extends Sized {
      public static methods = 7;
      public bump(): Int { made += 1; size += 1; size }
      public positive(): Boolean { made += 1; size > 0 }
      public toString(): String { "Box(${size})" }
    }
    export interface Sized {
      public get size(): Int;
      public twice(): Int { size * 2 }
    }
    export interface Named {
      public get name(): String { "anon" }
    }
    export class Pet(public name: String) extends Named {}
    let order(b: Box): Boolean { true < b.positive() }
    let b = make(1);
    console.log("${b.bump().toString()} ${b.twice().toString()} ${made.toString()}");
    console.log("${order(b).toString()} ${made.toString()} ${Box.methods.toString()}");
    let n: Named = new Pet("rex");
    console.log(n.name);
"#;

/// What [`MEMBERS`] logs.
const MEMBERS_LOG: [&str; 3] = ["2 4 1", "false 2 7", "rex"];

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

fn cases() -> Vec<Case> {
    let tests = [
        &kit::CLASSES[..],
        &kit::NULLABLES[..],
        &kit::GENERICS[..],
        &kit::FUNCTIONS[..],
        &kit::DEFAULTS[..],
        &kit::CORE_TYPES[..],
    ]
    .concat();
    let mut cases = kit::cases_with("lua", &kit::PROGRAMS, &tests);
    let hiding_log = HIDING_LOG.iter().map(|s| s.to_string()).collect();
    cases.push(Case::inline(
        "hiding",
        "hiding.temper.md",
        HIDING,
        hiding_log,
    ));
    cases.push(Case::inline(
        "repeats",
        "repeats.tmpl",
        REPEATS,
        vec!["25".into(), "10".into(), "3".into()],
    ));
    let members_log = MEMBERS_LOG.iter().map(|s| s.to_string()).collect();
    cases.push(Case::inline(
        "members",
        "members.temper.md",
        MEMBERS,
        members_log,
    ));
    let hoisting_log = HOISTING_LOG.iter().map(|s| s.to_string()).collect();
    cases.push(Case::inline(
        "hoisting",
        "hoisting.temper.md",
        HOISTING,
        hoisting_log,
    ));
    let (text, expected) = locals();
    cases.push(Case::inline("locals", "locals.temper.md", text, expected));
    cases
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&kit::program(case), &LUA).unwrap_or_else(|e| panic!("{}: {e}", case.key))
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
    kit::with_stack(|| {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
        for case in cases() {
            let files = generate(&case);
            let library = &case.library;
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            let spec = format!("{library}/{library}-dev-1.rockspec");
            assert_eq!(paths[0], format!("{library}/init.lua"), "{}", case.key);
            assert_eq!(paths.last(), Some(&spec.as_str()), "{}", case.key);
            if case.key == "locals" {
                // Too long to keep; `runs` runs it.
                continue;
            }
            goldens.check(&format!("{}.lua", case.key), &files[0].text);
            // Other modules, tests, and a rockspec with dependencies.
            for f in &files[1..] {
                if f.path == spec && f.text == rockspec(library) {
                    continue;
                }
                let rest = &f.path[library.len() + 1..];
                goldens.check(&format!("{}.{}", case.key, rest.replace('/', ".")), &f.text);
            }
        }
        goldens.finish();
    });
}

#[test]
fn programs_log_as_the_interpreter_does() {
    kit::with_stack(|| {
        for case in cases() {
            if !matches!(
                case.key.as_str(),
                "hiding" | "members" | "locals" | "hoisting"
            ) {
                continue;
            }
            let (failure, output) = kit::interpret(&case);
            assert_eq!(failure, None, "{}", case.key);
            assert_eq!(output, case.expected, "{}", case.key);
        }
    });
}

/// A program given by an environment variable, or found on `PATH`, if it
/// runs with `arg`.
fn tool(var: &str, name: &str, arg: &str) -> Option<String> {
    kit::tool_with(var, name, arg).map(|(t, _)| t)
}

fn runtime() -> PathBuf {
    kit::vendored("be-lua/temper-core")
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

/// Runs a tests file with a stand-in for LuaUnit, which runs each
/// function of `Test_`, in order of name, printing `ok NAME` or `fail NAME
/// why`.
const TESTS: &str = r#"package.preload["luaunit"] = function()
  return { LuaUnit = { run = function()
    local names = {}
    for name in pairs(Test_) do names[#names + 1] = name end
    table.sort(names)
    for _, name in ipairs(names) do
      local ok, err = pcall(Test_[name])
      io.write(ok and "ok " or "fail ", name, ok and "" or (" " .. tostring(err)), "\n")
    end
  end } }
end
arg = {}
dofile(...)
print(require("testing")["$test"].runs__get())
"#;

/// Reads `module-state`'s exports after its functions write them.
const LIVE: &str = r#"local state = require("module-state")
state.tick()
state.tick()
print(state.ticks, state.total, state.greeting, state.count)
"#;

/// Uses [`MEMBERS`]' exports: its `var`, current after a method writes
/// it, its static, a field read through its getter, and `tostring` of an
/// instance.
const MEMBERS_READ: &str = r#"local m = require("members")
print(m.made, m.Box.methods_, m.Pet("x").name, tostring(m.Box(5)))
m.Box(1):bump()
print(m.made)
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

/// Tests whose `Int64`s need Lua 5.3's 64-bit integers, which LuaJIT
/// lacks: this backend writes `Int64`s as Lua integers, where Kotlin's
/// makes them with the runtime's `int64_constructor`, which LuaJIT's
/// half of the runtime (`intold.lua`) keeps as tables. `types/int/limits`
/// also finds LuaJIT's `int32_mod` giving `-0` for `-2147483648 % -1`.
const NEEDS_INTEGERS: [&str; 2] = ["types-float-basics", "types-int-limits"];

/// Tests that the runtime gets wrong on LuaJIT (and Lua 5.1), Kotlin's
/// bugs, which these run on Lua 5.4 to show: without Lua 5.3's `utf8`,
/// its stand-in `len` ignores the range, so `countBetween` and
/// `hasAtLeast` count the whole string (`types/string/read`); LuaJIT's
/// `tonumber(s, 10)` takes `"2.0"` and `"0x10"` (`String.toInt32()`,
/// also `types/string/read`); and `float64_max` and `float64_min` check
/// only their second argument for NaN, where LuaJIT's `math.max(nan, 2)`
/// gives 2 (`types/float/ops`).
const NEEDS_LUA53: [&str; 2] = ["types-float-ops", "types-string-read"];

#[test]
fn runs() {
    let Some((lua, version)) = kit::tool_with("LUA", "lua", "-v") else {
        kit::missing("no Lua (on PATH, or named by LUA)");
        return;
    };
    let jit = version.contains("LuaJIT");
    let luals = tool("LUALS", "lua-language-server", "--version");
    if luals.is_none() {
        kit::missing("type check: no LuaLS (lua-language-server on PATH, or named by LUALS)");
    }
    let luac = tool("LUAC", "luac", "-v");
    if luac.is_none() {
        eprintln!("syntax check skipped: no luac (on PATH, or named by LUAC)");
    }
    kit::with_stack(move || {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-lua");
        let _ = fs::remove_dir_all(&out);
        let dir = out.join("lua");
        kit::copy_dir(&runtime(), &dir.join("temper-core"), &[]);
        let mut cases = cases();
        if jit {
            cases.retain(|c| {
                !NEEDS_INTEGERS.contains(&c.key.as_str()) && !NEEDS_LUA53.contains(&c.key.as_str())
            });
        }
        let mut generated = Vec::new();
        // A library named as one before it (`types/int/basics` and
        // `types/float/basics` are both `basics`) goes in a directory of
        // its own, and runs and is checked there.
        let mut named = std::collections::HashSet::new();
        let mut apart: Vec<(&Case, PathBuf)> = Vec::new();
        for case in &cases {
            let here = if named.insert(case.library.clone()) {
                dir.clone()
            } else {
                let own = out.join(format!("lua-{}", case.key));
                kit::copy_dir(&runtime(), &own.join("temper-core"), &[]);
                apart.push((case, own.clone()));
                own
            };
            for file in generate(case) {
                let path = here.join(&file.path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &file.text).unwrap();
                if file.path.ends_with(".lua") {
                    generated.push(path);
                }
            }
        }
        let mut failures = Vec::new();
        // Every library, in one process, but those that import another,
        // whose output they show only when they're the first to require
        // it, each in one of its own.
        let runner = out.join("runner.lua");
        fs::write(&runner, RUNNER).unwrap();
        let (cases, importers): (Vec<&Case>, Vec<&Case>) = cases
            .iter()
            .filter(|c| !apart.iter().any(|(a, _)| a.key == c.key))
            .partition(|c| c.libraries.is_empty() || kit::libraries(c, "lua").is_empty());
        let alone: Vec<(&Case, PathBuf)> = (importers.iter())
            .map(|&c| (c, dir.clone()))
            .chain(apart.iter().cloned())
            .collect();
        for (case, here) in &alone {
            let run = Command::new(&lua)
                .arg(&runner)
                .arg(&case.library)
                .current_dir(here)
                .env("LUA_PATH", LUA_PATH)
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&run.stdout);
            let log: Vec<&str> = stdout.lines().skip(1).collect();
            if !run.status.success() || log != case.expected {
                failures.push(format!(
                    "{}: expected {:?}, got {log:?}\n{}",
                    case.key,
                    case.expected,
                    String::from_utf8_lossy(&run.stderr)
                ));
            }
        }
        // The tests of `testing`, which need nothing of std's on Lua.
        let tests = out.join("tests.lua");
        fs::write(&tests, TESTS).unwrap();
        let run = Command::new(&lua)
            .arg(&tests)
            .arg("testing/tests/init-test.lua")
            .current_dir(&dir)
            .env("LUA_PATH", LUA_PATH)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&run.stdout);
        let expected = "init 2\n\
            ok test_names_punctuation_quotes_and_n_code\n\
            ok test_twice_works\n\
            14\n";
        if !run.status.success() || stdout != expected {
            failures.push(format!(
                "testing's tests: expected {expected:?}, got {stdout:?}\n{}",
                String::from_utf8_lossy(&run.stderr)
            ));
        }
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
        // `members`' exports.
        let members = out.join("members.lua");
        fs::write(&members, MEMBERS_READ).unwrap();
        let read = Command::new(&lua)
            .arg(&members)
            .current_dir(&dir)
            .env("LUA_PATH", LUA_PATH)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&read.stdout);
        let expected = "2 4 1\nfalse 2 7\nrex\n2\t7\tx\tBox(5)\n3\n";
        if !read.status.success() || stdout != expected {
            failures.push(format!(
                "members' exports: expected {expected:?}, got {stdout:?}\n{}",
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
            for (_, own) in &apart {
                failures.extend(check(luals, &out, own));
            }
        }
        kit::assert_none(&failures);
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
