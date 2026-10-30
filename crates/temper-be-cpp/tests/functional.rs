//! The C++ backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! ([`temper_funtests`]), on the Rust backend's own programs
//! (`crates/temper-be-rust/tests/programs/`), which cover operators,
//! jumps, module state, and string order beyond those tests, and on a
//! program of C++'s own ([`HIDING`]).
//!
//! - `goldens`: each program's generated library, laid out as Kotlin's,
//!   with its `init.cpp` matching the golden in `tests/goldens/`, named
//!   for the test's path (`algos-fibonacci.cpp`), and its header and
//!   `main.cpp` as the test expects. To accept new or changed output, run
//!   with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `runs`: without CMake, `g++` (or `CXX`) compiles every generated
//!   `.cpp` against the vendored runtime (`vendor/temper/be-cpp`), C++14
//!   as Kotlin's, with every warning [`STRICT`] lists an error, through a
//!   precompiled header of the runtime, built once per set of options
//!   and kept under `CARGO_TARGET_TMPDIR` until the runtime, the options,
//!   or `g++` change (when it's built, the runtime is checked with every
//!   warning it tolerates, [`RUNTIME_WARNINGS`]). It links one library's
//!   `main.cpp` as Kotlin's program, and a runner of every library, which
//!   runs a library's init by name, and `module-state` from 8 threads;
//!   each logs what the test expects of Kotlin's `cpp` backend. Then the
//!   same under AddressSanitizer and UndefinedBehaviorSanitizer, and the
//!   threads under ThreadSanitizer, which catch the data races and
//!   use-after-free that Kotlin's C++ can have. Skipped, with a message,
//!   without `g++`; with `TEMPEST_REQUIRE_CHECKERS` set, as in CI, that
//!   fails it instead.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_be_cpp::{CPP, OutFile, names, translate};
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
            expected: test.expected_log("cpp"),
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
            expected: temper_funtests::expected_log(&text, "cpp"),
            sources: vec![(path.to_string_lossy().into_owned(), text)],
        });
    }
    cases.push(Case {
        key: "hiding".into(),
        library: "hiding".into(),
        sources: vec![("hiding.temper.md".into(), HIDING.into())],
        expected: vec![
            "s 2 3 3".into(),
            "true true".into(),
            "2 -4 7".into(),
            "true false".into(),
        ],
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
    translate(&program(case), &CPP).unwrap_or_else(|e| panic!("{}: {e}", case.key))
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

fn vendored() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/be-cpp")
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
            let namespace = names::namespace(library);
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(
                paths,
                [
                    format!("{library}/init.hpp"),
                    format!("{library}/init.cpp"),
                    format!("{library}/main.cpp"),
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
            let golden = goldens.join(format!("{}.cpp", case.key));
            let expected = fs::read_to_string(&golden).unwrap_or_default();
            if both != expected {
                if update {
                    fs::create_dir_all(&goldens).unwrap();
                    fs::write(&golden, &both).unwrap();
                } else {
                    differ.push(format!("{}:\n{both}", golden.display()));
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

/// `g++`, or `CXX`, and its version, if it runs.
fn find_gxx() -> Option<(String, String)> {
    let gxx = env::var("CXX").unwrap_or_else(|_| "g++".into());
    let out = Command::new(&gxx).arg("--version").output().ok()?;
    out.status
        .success()
        .then(|| (gxx, String::from_utf8_lossy(&out.stdout).into_owned()))
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
    let mut hasher = DefaultHasher::new();
    version.hash(&mut hasher);
    STRICT.hash(&mut hasher);
    mode.options.hash(&mut hasher);
    for h in &headers {
        h.hash(&mut hasher);
        fs::read(h).unwrap().hash(&mut hasher);
    }
    let stamp = format!("{:016x}\n", hasher.finish());
    let stamp_path = root.join("stamp");
    let gch = root.join("runtime.hpp.gch");
    if fs::read_to_string(&stamp_path).ok().as_deref() == Some(&stamp) && gch.is_file() {
        return root;
    }
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
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
    fs::write(stamp_path, stamp).unwrap();
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
                            .args(STRICT)
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

/// The runner: runs the init of the library named on its command line;
/// or, for `threads`, `module-state`'s init, then its `tick` and `report`
/// from 8 threads, 1,000 times each.
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
    format!(
        r#"{includes}
#include <iostream>
#include <string>
#include <thread>
#include <vector>

namespace {{

void threads() {{
  module_state::global_init_init();
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
  std::cout << "ticks " << module_state::ticks << "\n";
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
/// few tries do instead).
fn run(failures: &mut Vec<String>, name: &str, exe: &Path, args: &[&str], expected: &[String]) {
    for _ in 0..40 {
        let out = Command::new(exe).args(args).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.code() == Some(66) && stderr.contains("unexpected memory mapping") {
            continue;
        }
        let actual: Vec<&str> = stdout.lines().collect();
        if !out.status.success() || actual != expected || !stderr.is_empty() {
            failures.push(format!(
                "{name}: expected {expected:?}, got {actual:?}\n{stderr}"
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
    let required = env::var_os("TEMPEST_REQUIRE_CHECKERS").is_some();
    let Some((gxx, version)) = find_gxx() else {
        assert!(!required, "no g++");
        eprintln!("skipped: no g++ (on PATH, or named by CXX)");
        return;
    };
    with_stack(move || {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-cpp");
        let _ = fs::remove_dir_all(&out);
        let src = out.join("src");
        let cases = cases();
        let mut inits = Vec::new();
        let mut mains = Vec::new();
        for case in &cases {
            for file in generate(case) {
                let path = src.join(&file.path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &file.text).unwrap();
                if file.path.ends_with("/init.cpp") {
                    inits.push(path);
                } else if file.path.ends_with("/main.cpp") {
                    mains.push(path);
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
        link(
            &gxx,
            &PLAIN,
            &[main_objects[0].clone(), init_objects[0].clone()],
            &program,
        );
        run(&mut failures, "main.cpp", &program, &[], &cases[0].expected);
        let mut all = init_objects.to_vec();
        all.extend(runner_object.iter().cloned());
        let runner_exe = bin.join("runner");
        link(&gxx, &PLAIN, &all, &runner_exe);
        let state = cases.iter().find(|c| c.library == "module-state").unwrap();
        let mut threads_expected = state.expected.clone();
        threads_expected.push("ticks 8000".into());
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

        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}
