//! The C++ backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! (`temper-funtests`), with the class tests (`kit::CLASSES`) and those
//! with nullables and `orelse` (`kit::NULLABLES`), and those with
//! core's other types and members (`kit::CORE_TYPES`: string indices,
//! `StringBuilder`, `Empty`, `Deque`, `DenseBitVector`, and more of
//! `String`'s and `Float64`'s), and those with host code
//! (`kit::CONNECTED`, each as is and under `--no-connected`), those of
//! divisions that can't fail and maps (`kit::DIVISIONS`), those of
//! labeled loops' jumps and classes declared in functions
//! (`kit::LOOPS_AND_CLASSES`), on the
//! shared programs (`crates/temper-be-testkit/programs/`), which cover
//! operators, jumps, module state, string order, nullables, and classes beyond
//! those tests, and on programs of C++'s own: names C++ takes
//! ([`HIDING`]), and objects a call frees while it runs ([`LIFETIMES`]).
//!
//! - `goldens`: each program's generated library, laid out as Kotlin's,
//!   with its `init.cpp` matching the golden in `tests/goldens/`, named
//!   for the test's path (`algos-fibonacci.cpp`), and its header and
//!   `main.cpp` as the test expects; a library of several modules, or
//!   with tests (`modules`, `imports-library`, and `testing`), has every
//!   file in its golden. To accept new or changed output, run with
//!   `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `runs`: without CMake, `g++` (or `CXX`) compiles every generated
//!   `.cpp` against the vendored runtime (`vendor/temper/be-cpp`), C++14
//!   as Kotlin's, with every warning [`STRICT`] lists an error, through a
//!   precompiled header of the runtime, built once per set of options
//!   and kept under `CARGO_TARGET_TMPDIR` until the runtime, the options,
//!   or `g++` change (when it's built, the runtime is checked with every
//!   warning it tolerates, [`RUNTIME_WARNINGS`]). It links one library's
//!   `main.cpp` as Kotlin's program, and a runner of every library, which
//!   runs a library's init by name, entry points with no init first
//!   ([`FIRST_USE`]), which run it themselves, and `module-state` from 8
//!   threads, which all enter it at once with no init first; each logs
//!   what the test expects of Kotlin's `cpp` backend. Then the same under
//!   AddressSanitizer and UndefinedBehaviorSanitizer, and the threads
//!   under ThreadSanitizer, which catch the data races and
//!   use-after-free that Kotlin's C++ can have (the class tests and
//!   `classes` chain calls on returned objects, and `lifetimes` frees
//!   objects and lists in use). Every library is built in
//!   one directory, so `imports-library` finds `modules` beside it, but
//!   a library with host code, which names it `work`, builds alone, its
//!   host `.cpp`s with C++14 alone, since the test's host code isn't
//!   written for the warnings. Tests
//!   need std's `testing`, which tmpl doesn't translate yet, so a stand-in
//!   for its `Test` ([`TESTING_STUB`]) checks their form: each library
//!   with tests builds again with `TEMPER_TESTS`, with its `test.cpp`,
//!   strictly, and runs, writing JUnit's XML. Skipped, with a message,
//!   without `g++`; with `TEMPEST_REQUIRE_CHECKERS` set, as in CI, that
//!   fails it instead.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_be_cpp::{CPP, OutFile, names, translate};
use temper_be_testkit::{self as kit, Case, Goldens};

/// A library of names that C++ reserves or the translator prints
/// (`std`, `core`, `int32_t`, a keyword, and a macro, `M_PI`), each
/// taking `_`; `Boolean`'s `<`; strings that C++ would read otherwise (a
/// trigraph, `??=`, and a NUL, which would end a literal); and shifts by
/// more than the width, which Temper masks and C++ leaves undefined.
const HIDING: &str = "# Hiding

    let std = \"s\";
    let explicit(int32_t: Int): Int { int32_t + 1 }
    let core(a: Int, b: Int): Int throws Bubble { a / b }
    let before(a: Boolean, b: Boolean): Boolean { a < b }
    let M_PI = 3;
    let shifts(n: Int): String { \"${1 << n} ${-8 >> n} ${-8 >>> (n - 4)}\" }
    console.log(\"${std} ${explicit(1).toString()} ${core(7, 2).toString()} ${M_PI.toString()}\");
    console.log(\"${before(false, true).toString()} ${(\"??=\" < \"??>\").toString()}\");
    console.log(shifts(33));
    console.log(\"${(\"a\\u{0}b\" == \"a\\u{0}b\").toString()} ${(\"a\\u{0}b\" == \"a\\u{0}c\").toString()}\");
";

/// Objects that a call frees while it runs, which C++ must keep alive:
/// a method's object that only a field held, which the method replaces;
/// arguments read from a field the callee replaces, which Kotlin's C++
/// passes as references to the field (`this_->replace(this_->box)` logs
/// `new`, not `old`); calls chained on objects nothing else holds; and
/// the same for nullables: a nullable field passed to a method that
/// replaces it, a nullable string field read then replaced, and a
/// nullable object a call returns, used after `??` and a null check.
/// ASan checks it with the rest.
const LIFETIMES: &str = "# Lifetimes

    class Item(public name: String) {
      public leave(h: Holder): String {
        h.box = new Box(new Item(\"other\"));
        name
      }
    }

    class Box(public var item: Item) {}

    class Holder(public var box: Box) {
      public replace(old: Box): String {
        box = new Box(new Item(\"new\"));
        old.item.name
      }
      public replaceItem(old: Item): String {
        box = new Box(new Item(\"newer\"));
        old.name
      }
      public run(): String { replace(box) }
      public runItem(): String { replaceItem(box.item) }
      public drop(): String { box.item.leave(this) }
    }

    class Slot(public var item: Item?, public var tag: String?) {
      public swap(old: Item?): String {
        item = null;
        tag = null;
        if (old == null) { \"none\" } else { old.name }
      }
      public run(): String { swap(item) }
      public relabel(): String {
        let was = tag;
        tag = \"after\";
        was ?? \"unlabeled\"
      }
    }

    let maybeItem(make: Boolean): Item? { if (make) { new Item(\"temp\") } else { null } }

    class Names(public var list: List<String>) {
      public swap(old: List<String>): String {
        list = [\"fresh\"];
        old[0]
      }
      public run(): String { swap(list) }
      public firstThen(): String {
        let first = list[0];
        list = [\"replaced\"];
        first
      }
    }

    let made(): List<String> { [\"made\", \"list\"] }

    let h = new Holder(new Box(new Item(\"old\")));
    console.log(h.run());
    console.log(h.runItem());
    console.log(h.box.item.name);
    console.log(h.drop());
    console.log(h.box.item.name);
    console.log(new Holder(new Box(new Item(\"chained\"))).box.item.name);
    let slot = new Slot(new Item(\"slotted\"), \"before\");
    console.log(slot.run());
    console.log(slot.run());
    slot.tag = \"first\";
    console.log(slot.relabel());
    console.log(slot.relabel());
    console.log((maybeItem(true) ?? new Item(\"fallback\")).name);
    console.log((maybeItem(false) ?? new Item(\"fallback\")).name);
    let got = maybeItem(true);
    if (got != null) {
      console.log(got.name);
    }
    let names = new Names([\"kept\"]);
    console.log(names.run());
    console.log(names.firstThen());
    console.log(names.list[0]);
    let second = made()[1];
    console.log(second);
    console.log(made().length.toString());
";

/// A stand-in for std's `testing` header, as Kotlin's declares `Test`, for
/// building tests until tmpl translates std: every test passes.
const TESTING_STUB: &str = "#pragma once
#include <temper-core/core.hpp>
namespace temper_std {
struct Test : public std::enable_shared_from_this<Test> {
  bool get_passing() const { return true; }
  temper::core::NullableParam<std::string> messagesCombined() const { return nullptr; }
  static std::shared_ptr<Test> make() { return std::make_shared<Test>(); }
};
inline void global_init_testing() {}
}  // namespace temper_std
";

/// Building tests: plain, with the tests defined.
const TESTS: Mode = Mode {
    name: "tests",
    options: &["-O1", "-DTEMPER_TESTS"],
};

/// `g++`'s options for generated code: C++14, as Kotlin's, and every
/// warning below an error. The runtime's headers are a system directory
/// (`-isystem`), so only generated code answers to the four that the
/// runtime breaks ([`RUNTIME_WARNINGS`] has the rest). Not on: `-Weffc++`,
/// whose rules (members initialized in constructors' lists, and the like)
/// are older advice than C++11's, and which the runtime breaks too.
const STRICT: &[&str] = &[
    "-std=c++14",
    "-Wall",
    "-Wextra",
    "-Wpedantic",
    "-Werror",
    // What the runtime tolerates too.
    "-Wcast-align=strict",
    "-Wcast-qual",
    "-Wconversion",
    "-Wdate-time",
    "-Wdeprecated-copy-dtor",
    "-Wdouble-promotion",
    "-Wduplicated-branches",
    "-Wduplicated-cond",
    "-Wextra-semi",
    "-Wformat=2",
    "-Wimplicit-fallthrough=5",
    "-Winvalid-pch",
    "-Wlogical-op",
    "-Wmissing-declarations",
    "-Wnon-virtual-dtor",
    "-Wnull-dereference",
    "-Wold-style-cast",
    "-Woverloaded-virtual",
    "-Wpessimizing-move",
    "-Wredundant-decls",
    "-Wredundant-move",
    "-Wsign-promo",
    "-Wstrict-null-sentinel",
    "-Wsuggest-override",
    "-Wswitch-enum",
    "-Wundef",
    "-Wvla",
    "-Walloca",
    "-Wzero-as-null-pointer-constant",
    // What only generated code answers to: the runtime shadows names,
    // converts between signed and unsigned implicitly, casts to the same
    // type, and compares floats with `==`.
    "-Wshadow",
    "-Wsign-conversion",
    "-Wuseless-cast",
    "-Wfloat-equal",
];

/// The warnings, of [`STRICT`], that the runtime's headers compile
/// without, checked once, with the runtime included as an ordinary
/// directory (`-I`).
const RUNTIME_WARNINGS: usize = STRICT.len() - 4;

fn cases() -> Vec<Case> {
    let tests = [
        &kit::CLASSES[..],
        &kit::NULLABLES[..],
        &kit::GENERICS[..],
        &kit::FUNCTIONS[..],
        &kit::DEFAULTS[..],
        &kit::CORE_TYPES[..],
        &kit::CONNECTED[..],
        &kit::DIVISIONS[..],
        &kit::LOOPS_AND_CLASSES[..],
    ]
    .concat();
    let mut cases = kit::cases_with("cpp", &kit::PROGRAMS, &tests);
    cases.push(Case::inline(
        "hiding",
        "hiding.temper.md",
        HIDING,
        vec![
            "s 2 3 3".into(),
            "true true".into(),
            "2 -4 7".into(),
            "true false".into(),
        ],
    ));
    cases.push(Case::inline(
        "lifetimes",
        "lifetimes.temper.md",
        LIFETIMES,
        [
            "old", "new", "newer", "newer", "other", "chained", "slotted", "none", "first",
            "after", "temp", "fallback", "temp", "kept", "fresh", "replaced", "list", "2",
        ]
        .map(String::from)
        .to_vec(),
    ));
    // Libraries of one name (`types/int/basics` and `types/float/basics`)
    // would be one in a build of every library, so a later one is named
    // for its test, or where that's taken too (the `functions` program,
    // after `imports/functions`), for its test and `-program`.
    let mut seen = std::collections::HashSet::new();
    // Libraries with host code keep their name, `work`, which the host
    // code includes, each built alone ([`runs`]).
    for case in cases.iter_mut().filter(|c| c.host_files.is_empty()) {
        if !seen.insert(case.library.clone()) {
            case.library = if seen.contains(&case.key) {
                format!("{}-program", case.key)
            } else {
                case.key.clone()
            };
            seen.insert(case.library.clone());
        }
    }
    cases
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&kit::program(case), &CPP).unwrap_or_else(|e| panic!("{}: {e}", case.key))
}

fn vendored() -> PathBuf {
    kit::vendored("be-cpp")
}

#[test]
fn goldens() {
    kit::with_stack(|| {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
        for case in cases() {
            let files = generate(&case);
            let library = &case.library;
            let namespace = names::namespace(library);
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            if paths.len() != 4 {
                // Several modules, tests, or host code: every file in one
                // golden, but host code's copies only by their paths.
                let mut all = String::new();
                for file in &files {
                    let rest = &file.path[library.len() + 1..];
                    if case.host_files.iter().any(|(path, _)| path == rest) {
                        assert!(
                            case.host_files
                                .contains(&(rest.to_string(), file.text.clone())),
                            "{}: {rest}",
                            case.key
                        );
                        all.push_str(&format!("// {rest}: host code, as it is\n"));
                        continue;
                    }
                    all.push_str(&format!("// {rest}\n{}\n", file.text));
                }
                goldens.check(&format!("{}.cpp", case.key), &all);
                continue;
            }
            assert_eq!(
                paths,
                [
                    format!("{library}/init.hpp"),
                    format!("{library}/init.cpp"),
                    format!("{library}/main.cpp"),
                    format!("{library}/CMakeLists.txt"),
                ],
                "{}",
                case.key
            );
            let header = &files[0].text;
            assert!(
                header.starts_with("#pragma once\n\n#include <temper-core/core.hpp>\n")
                    && header.contains(&format!("namespace {namespace} {{\n"))
                    && header.contains("void global_init_init();\n"),
                "{}: {header}",
                case.key
            );
            assert_eq!(
                files[2].text,
                format!(
                    "#include \"{library}/init.hpp\"\n\nint main() {{\n  {namespace}::global_init_init();\n}}\n"
                ),
                "{}",
                case.key
            );
            let module = &files[1].text;
            assert!(
                module.starts_with(&format!("#include <{library}/init.hpp>\n")),
                "{}",
                case.key
            );
            // The header and the module, one golden.
            let both = format!("// init.hpp\n{header}\n// init.cpp\n{module}");
            goldens.check(&format!("{}.cpp", case.key), &both);
        }
        goldens.finish();
    });
}

/// `g++`, or `CXX`, and its version, if it runs.
fn find_gxx() -> Option<(String, String)> {
    kit::tool_with("CXX", "g++", "--version")
}

/// A way of building: its name, and the options it adds to [`STRICT`].
struct Mode {
    name: &'static str,
    options: &'static [&'static str],
}

const PLAIN: Mode = Mode {
    name: "plain",
    options: &["-O1"],
};

/// AddressSanitizer and UndefinedBehaviorSanitizer, any report fatal.
const ASAN: Mode = Mode {
    name: "asan",
    options: &[
        "-O1",
        "-g",
        "-fno-omit-frame-pointer",
        "-fsanitize=address,undefined",
        "-fno-sanitize-recover=all",
    ],
};

/// ThreadSanitizer, without position-independent code, which leaves
/// fewer mappings where TSan's shadow memory goes (below, "tsan").
const TSAN: Mode = Mode {
    name: "tsan",
    options: &["-O1", "-g", "-fsanitize=thread", "-fno-pie", "-no-pie"],
};

/// Runs a command, panicking with its errors if it fails.
fn check(what: &str, mut command: Command) -> Output {
    let out = command.output().unwrap();
    assert!(
        out.status.success(),
        "{what}:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// The runtime's precompiled header for a mode, built once and kept
/// under `CARGO_TARGET_TMPDIR` until the runtime's headers, the options,
/// or `g++` change: a directory holding `runtime.hpp`, which includes
/// `<temper-core/core.hpp>`, and its `.gch`, for `-include runtime.hpp`.
/// The first build also checks the runtime itself with every warning it
/// tolerates.
fn runtime_pch(gxx: &str, version: &str, mode: &Mode) -> PathBuf {
    let root =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("be-cpp-runtime-{}", mode.name));
    let mut headers: Vec<PathBuf> = fs::read_dir(vendored().join("temper-core"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    headers.sort();
    let gch = root.join("runtime.hpp.gch");
    kit::cached(
        &root,
        (version, STRICT, mode.options),
        &headers,
        &gch,
        || {
            // The runtime as it is, an ordinary directory.
            let probe = root.join("probe.cpp");
            fs::write(&probe, "#include <temper-core/core.hpp>\n").unwrap();
            let mut command = Command::new(gxx);
            command
                .args(&STRICT[..RUNTIME_WARNINGS])
                .args(mode.options)
                .arg("-I")
                .arg(vendored())
                .args(["-fsyntax-only"])
                .arg(&probe);
            check("g++, the runtime with its warnings", command);
            fs::write(
                root.join("runtime.hpp"),
                "#include <temper-core/core.hpp>\n",
            )
            .unwrap();
            let mut command = Command::new(gxx);
            command
                .args(STRICT)
                .args(mode.options)
                .arg("-isystem")
                .arg(vendored())
                .args(["-x", "c++-header"])
                .arg(root.join("runtime.hpp"))
                .arg("-o")
                .arg(&gch);
            check("g++, the runtime's precompiled header", command);
        },
    );
    root
}

/// Compiles `.cpp` files to objects beside `out`, a few at a time, with
/// the generated libraries' root (`src`) as an include directory.
fn compile(
    gxx: &str,
    mode: &Mode,
    pch: &Path,
    src: &Path,
    out: &Path,
    sources: &[PathBuf],
) -> Vec<PathBuf> {
    compile_with(gxx, STRICT, mode, pch, src, out, sources)
}

/// [`compile`], with these warning options in place of [`STRICT`].
fn compile_with(
    gxx: &str,
    warnings: &[&str],
    mode: &Mode,
    pch: &Path,
    src: &Path,
    out: &Path,
    sources: &[PathBuf],
) -> Vec<PathBuf> {
    let objects: Vec<PathBuf> = sources
        .iter()
        .map(|s| {
            let rel = s.strip_prefix(src).unwrap_or(s);
            out.join(rel).with_extension("o")
        })
        .collect();
    let jobs = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4);
    let pairs: Vec<(&PathBuf, &PathBuf)> = sources.iter().zip(&objects).collect();
    for chunk in pairs.chunks(jobs) {
        std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(source, object)| {
                    scope.spawn(move || {
                        fs::create_dir_all(object.parent().unwrap()).unwrap();
                        let mut command = Command::new(gxx);
                        command
                            .args(warnings)
                            .args(mode.options)
                            .arg("-isystem")
                            .arg(vendored())
                            .arg("-I")
                            .arg(src)
                            .arg("-include")
                            .arg(pch.join("runtime.hpp"))
                            .arg("-c")
                            .arg(source)
                            .arg("-o")
                            .arg(object);
                        check(
                            &format!("g++ ({}), {}", mode.name, source.display()),
                            command,
                        );
                    })
                })
                .collect();
            for h in handles {
                h.join().unwrap();
            }
        });
    }
    objects
}

fn link(gxx: &str, mode: &Mode, objects: &[PathBuf], exe: &Path) {
    let mut command = Command::new(gxx);
    command.args(mode.options).args(objects).arg("-o").arg(exe);
    check(
        &format!("g++ ({}), linking {}", mode.name, exe.display()),
        command,
    );
}

/// An entry point, or a few, that users call with no init first, which
/// run it themselves.
struct FirstUse {
    /// The runner's argument.
    name: &'static str,
    /// The cases whose logs come first.
    logs: &'static [&'static str],
    /// The C++ that prints what each gives, and the lines printed after.
    prints: &'static [(&'static str, &'static str)],
}

/// [`FirstUse`] runs. `first-use-order` enters a module that another
/// imports, then that one, which initializes the first only once, before
/// it.
const FIRST_USE: &[FirstUse] = &[
    FirstUse {
        name: "first-use-function",
        logs: &["modules"],
        prints: &[("modules::sum(3)", "17")],
    },
    FirstUse {
        name: "first-use-order",
        logs: &[],
        prints: &[
            ("modules::twice(2)", "hello, helpers from deep\n4"),
            (
                "modules::sum(3)",
                "hello, modules from deep\nsum 17\ncalls 0\ncalls now 3\n17",
            ),
        ],
    },
    FirstUse {
        name: "first-use-import",
        logs: &["imports-values"],
        prints: &[("values::timesMagic(2)", "84")],
    },
    FirstUse {
        name: "first-use-value",
        logs: &[],
        prints: &[("values::magicNumber()", "42")],
    },
    FirstUse {
        name: "first-use-static",
        logs: &["classes-static-properties"],
        prints: &[("static_properties::Simple::other()", "22")],
    },
];

/// What a [`FIRST_USE`] run logs.
fn first_use_expected(cases: &[Case], run: &FirstUse) -> Vec<String> {
    let mut out = Vec::new();
    for key in run.logs {
        let case = cases.iter().find(|c| c.key == *key).unwrap();
        out.extend(case.expected.iter().cloned());
    }
    for (_, printed) in run.prints {
        out.extend(printed.lines().map(String::from));
    }
    out
}

/// The runner: runs the init of the library named on its command line;
/// a [`FIRST_USE`] run; or, for `threads`, `module-state`'s `tick` and
/// `report` from 8 threads, 1,000 times each, the first calls of which
/// run its init, once.
fn runner(cases: &[Case]) -> String {
    let mut includes = String::new();
    let mut branches = String::new();
    for case in cases {
        includes.push_str(&format!("#include \"{}/init.hpp\"\n", case.library));
        branches.push_str(&format!(
            "  if (name == \"{}\") {{\n    {}::global_init_init();\n  }}\n",
            case.key,
            names::namespace(&case.library)
        ));
    }
    for run in FIRST_USE {
        branches.push_str(&format!("  if (name == \"{}\") {{\n", run.name));
        for (code, _) in run.prints {
            branches.push_str(&format!("    std::cout << {code} << \"\\n\";\n"));
        }
        branches.push_str("  }\n");
    }
    format!(
        r#"{includes}
#include <iostream>
#include <string>
#include <thread>
#include <vector>

namespace {{

void threads() {{
  std::vector<std::thread> threads;
  for (int i = 0; i < 8; i++) {{
    threads.emplace_back([] {{
      for (int j = 0; j < 1000; j++) {{
        module_state::tick();
        module_state::report();
      }}
    }});
  }}
  for (std::thread &thread : threads) {{
    thread.join();
  }}
  std::cout << "ticks " << module_state::ticks() << "\n";
}}

}}  // namespace

int main(int argc, char **argv) {{
  if (argc < 2) {{
    return 2;
  }}
  std::string name = argv[1];
  if (name == "threads") {{
    threads();
    return 0;
  }}
{branches}}}
"#
    )
}

/// Runs an executable, comparing what it logs, retrying where
/// ThreadSanitizer can't place its shadow memory (it exits with 66 and
/// "unexpected memory mapping" when the kernel randomizes mappings over
/// more bits than it expects, as Linux 6's default does; disabling that
/// takes `setarch -R` or a sysctl, which a sandbox may not allow, so a
/// few tries do instead), or crashes silently as it starts.
fn run(failures: &mut Vec<String>, name: &str, exe: &Path, args: &[&str], expected: &[String]) {
    for _ in 0..40 {
        let out = Command::new(exe).args(args).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.code() == Some(66) && stderr.contains("unexpected memory mapping") {
            continue;
        }
        // Or it crashes as it starts, before the program prints or
        // ThreadSanitizer reports anything.
        let silent_crash =
            out.status.code().is_none() && out.stdout.is_empty() && stderr.is_empty();
        if name.ends_with("(tsan)") && silent_crash {
            continue;
        }
        let actual: Vec<&str> = stdout.lines().collect();
        if !out.status.success() || actual != expected || !stderr.is_empty() {
            failures.push(format!(
                "{name}: expected {expected:?}, got {actual:?} ({})\n{stderr}",
                out.status
            ));
        }
        return;
    }
    failures.push(format!(
        "{name}: ThreadSanitizer couldn't map its memory in 40 tries"
    ));
}

#[test]
fn runs() {
    let Some((gxx, version)) = find_gxx() else {
        kit::missing("no g++ (on PATH, or named by CXX)");
        return;
    };
    kit::with_stack(move || {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-cpp");
        let _ = fs::remove_dir_all(&out);
        let src = out.join("src");
        // Libraries with host code are each `work`, so each builds alone.
        let (connected, cases): (Vec<Case>, Vec<Case>) =
            cases().into_iter().partition(|c| !c.host_files.is_empty());
        let mut inits = Vec::new();
        let mut mains = Vec::new();
        // Each library with tests: its modules, then its `test.cpp`.
        let mut tested: Vec<(String, Vec<PathBuf>)> = Vec::new();
        for case in &cases {
            let files = generate(case);
            if files.iter().any(|f| f.path.ends_with("/test.cpp")) {
                let mut sources: Vec<PathBuf> = (files.iter())
                    .filter(|f| f.path.ends_with(".cpp") && !f.path.ends_with("/main.cpp"))
                    .map(|f| src.join(&f.path))
                    .collect();
                sources.sort_by_key(|s| s.ends_with("test.cpp"));
                tested.push((case.key.clone(), sources));
            }
            for file in files {
                let path = src.join(&file.path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &file.text).unwrap();
                if file.path.ends_with("/main.cpp") {
                    mains.push(path);
                } else if file.path.ends_with(".cpp") && !file.path.ends_with("/test.cpp") {
                    inits.push(path);
                }
            }
        }
        let runner_cpp = src.join("runner.cpp");
        fs::write(&runner_cpp, runner(&cases)).unwrap();
        let mut failures = Vec::new();

        // Every library, strictly, each `main.cpp` too, and the runner;
        // one `main.cpp` linked as Kotlin's program is.
        let pch = runtime_pch(&gxx, &version, &PLAIN);
        let bin = out.join("plain");
        let mut sources = inits.clone();
        sources.extend(mains.iter().cloned());
        sources.push(runner_cpp.clone());
        let objects = compile(&gxx, &PLAIN, &pch, &src, &bin, &sources);
        let (init_objects, rest) = objects.split_at(inits.len());
        let (main_objects, runner_object) = rest.split_at(mains.len());
        let program = bin.join("program");
        let first = src.join(&cases[0].library);
        let first_objects = (inits.iter().zip(init_objects))
            .filter(|(source, _)| source.starts_with(&first))
            .map(|(_, object)| object.clone());
        let mut program_objects = vec![main_objects[0].clone()];
        program_objects.extend(first_objects);
        link(&gxx, &PLAIN, &program_objects, &program);
        run(&mut failures, "main.cpp", &program, &[], &cases[0].expected);
        let mut all = init_objects.to_vec();
        all.extend(runner_object.iter().cloned());
        let runner_exe = bin.join("runner");
        link(&gxx, &PLAIN, &all, &runner_exe);
        let threads_expected = kit::threads_expected(&cases);
        for case in &cases {
            run(
                &mut failures,
                &case.key,
                &runner_exe,
                &[&case.key],
                &case.expected,
            );
        }
        run(
            &mut failures,
            "module-state threads",
            &runner_exe,
            &["threads"],
            &threads_expected,
        );
        for first in FIRST_USE {
            let expected = first_use_expected(&cases, first);
            run(
                &mut failures,
                first.name,
                &runner_exe,
                &[first.name],
                &expected,
            );
        }

        // Tests, with the stand-in for std's `testing`.
        if !tested.is_empty() {
            let stub = src.join("std/testing.hpp");
            fs::create_dir_all(stub.parent().unwrap()).unwrap();
            fs::write(&stub, TESTING_STUB).unwrap();
            let pch = runtime_pch(&gxx, &version, &TESTS);
            let bin = out.join("tests");
            for (key, sources) in &tested {
                let objects = compile(&gxx, &TESTS, &pch, &src, &bin.join(key), sources);
                let exe = bin.join(key).join("test");
                link(&gxx, &TESTS, &objects, &exe);
                let dir = exe.parent().unwrap();
                let run = Command::new(&exe).current_dir(dir).output().unwrap();
                let xml = fs::read_to_string(dir.join("test-results.xml")).unwrap_or_default();
                if !run.status.success() || !xml.contains("failures=\"0\"") {
                    failures.push(format!("{key}: tests:\n{xml}"));
                }
            }
        }

        // Each library with host code, alone, its host `.cpp`s with its
        // modules, as its CMake project builds them, and its `main.cpp`
        // linked as Kotlin's program, plainly and under the sanitizers.
        // Host `.cpp`s aren't written for [`STRICT`] (the functional
        // test's `length` converts `size()` to `int32_t`), so they build
        // with C++14 alone; the `_connected.hpp` that generated `.cpp`s
        // include answers to it with them.
        for case in &connected {
            let root = out.join("connected").join(&case.key);
            let src = root.join("src");
            let mut sources = Vec::new();
            let mut host = Vec::new();
            for file in generate(case) {
                let path = src.join(&file.path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &file.text).unwrap();
                let rest = &file.path[case.library.len() + 1..];
                if case.host_files.iter().any(|(p, _)| p == rest) {
                    if rest.ends_with(".cpp") {
                        host.push(path);
                    }
                } else if file.path.ends_with(".cpp") {
                    sources.push(path);
                }
            }
            for mode in [&PLAIN, &ASAN] {
                let pch = runtime_pch(&gxx, &version, mode);
                let bin = root.join(mode.name);
                let mut objects = compile(&gxx, mode, &pch, &src, &bin, &sources);
                objects.extend(compile_with(
                    &gxx,
                    &["-std=c++14"],
                    mode,
                    &pch,
                    &src,
                    &bin,
                    &host,
                ));
                let exe = bin.join("program");
                link(&gxx, mode, &objects, &exe);
                let name = format!("{} ({})", case.key, mode.name);
                run(&mut failures, &name, &exe, &[], &case.expected);
            }
        }

        // The same under AddressSanitizer and UndefinedBehaviorSanitizer.
        let pch = runtime_pch(&gxx, &version, &ASAN);
        let bin = out.join("asan");
        let mut sources = inits.clone();
        sources.push(runner_cpp.clone());
        let objects = compile(&gxx, &ASAN, &pch, &src, &bin, &sources);
        let runner_exe = bin.join("runner");
        link(&gxx, &ASAN, &objects, &runner_exe);
        for case in &cases {
            let name = format!("{} (asan, ubsan)", case.key);
            run(
                &mut failures,
                &name,
                &runner_exe,
                &[&case.key],
                &case.expected,
            );
        }
        let name = "module-state threads (asan, ubsan)";
        run(
            &mut failures,
            name,
            &runner_exe,
            &["threads"],
            &threads_expected,
        );
        for first in FIRST_USE {
            let expected = first_use_expected(&cases, first);
            let label = format!("{} (asan, ubsan)", first.name);
            run(&mut failures, &label, &runner_exe, &[first.name], &expected);
        }

        // The threads under ThreadSanitizer.
        let pch = runtime_pch(&gxx, &version, &TSAN);
        let bin = out.join("tsan");
        let mut sources = inits.clone();
        sources.push(runner_cpp.clone());
        let objects = compile(&gxx, &TSAN, &pch, &src, &bin, &sources);
        let runner_exe = bin.join("runner");
        link(&gxx, &TSAN, &objects, &runner_exe);
        let name = "module-state threads (tsan)";
        run(
            &mut failures,
            name,
            &runner_exe,
            &["threads"],
            &threads_expected,
        );

        kit::assert_none(&failures);
    });
}

/// Host code that a connected function needs, missing: `NotYet`, naming
/// the file, where Kotlin's fails in `g++`.
#[test]
fn missing_host_code() {
    kit::with_stack(|| {
        let mut case = (cases().into_iter())
            .find(|c| c.key == "functions-connected")
            .unwrap();
        case.host_files.retain(|(path, _)| path != "_connected.hpp");
        let err = translate(&kit::program(&case), &CPP).unwrap_err();
        assert!(err.what.contains("_connected.hpp"), "{err}");
    });
}
