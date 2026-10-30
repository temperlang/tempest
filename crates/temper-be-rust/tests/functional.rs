//! The Rust backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! ([`temper_funtests`]), and on this crate's own programs
//! (`tests/programs/`), which cover module state.
//!
//! - `goldens`: each program's generated crate, laid out as Kotlin's,
//!   with its `src/mod.rs` matching the golden in `tests/goldens/`, named
//!   for the test's path (`algos-fibonacci.rs`), and every generated
//!   Rust file as rustfmt would lay it out, where rustfmt is installed. To
//!   accept new or changed output, run with `UPDATE_SNAPSHOTS=1` and
//!   review the diff.
//! - `runs`: the generated crates, in one cargo workspace with one target
//!   directory (under `CARGO_TARGET_TMPDIR`, kept between runs so builds
//!   are incremental), build offline against the vendored `temper-core`
//!   (`vendor/temper/be-rust/temper-core`, with its `Cargo.lock`), and
//!   each program logs what the test expects of Kotlin's `rust` backend;
//!   a harness also runs `module-state` from 8 threads. Then clippy finds
//!   nothing in them with `-D warnings`, apart from three lints that
//!   fire on what the tests do on purpose ([`ALLOWED_LINTS`]). Skipped, with a message, without cargo
//!   or clippy, or if `temper-core`'s one dependency, `indexmap`, isn't
//!   in cargo's cache (`cargo fetch` in the vendored `temper-core` puts
//!   it there); with `TEMPEST_REQUIRE_CHECKERS` set, as in CI, any of
//!   those fails it instead.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_be_rust::{OutFile, RUST, translate};
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

/// This crate's programs, in `tests/programs/`: Temper
/// (`NAME.temper.md`), or tmpl that the frontend doesn't make yet
/// (`NAME.tmpl`, with the lines it logs in `// log:` comments).
const PROGRAMS: [&str; 3] = ["module-state", "operators", "jumps.tmpl"];

/// Clippy's lints that fire on what the functional tests do on purpose:
/// `zero = zero` (`self_assignment`) and `1.min(3)`
/// (`unnecessary_min_or_max`), as in Kotlin's output too; and a local
/// named `foo` (`disallowed_names`), which Kotlin's output avoids only by
/// numbering every name (`foo__3`).
const ALLOWED_LINTS: [&str; 3] = [
    "clippy::self_assignment",
    "clippy::unnecessary_min_or_max",
    "clippy::disallowed_names",
];

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
            sources: test
                .sources()
                .into_iter()
                .map(|s| (s.path, s.text))
                .collect(),
            expected: test.expected_log("rust"),
        });
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/programs");
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
            expected: temper_funtests::expected_log(&text, "rust"),
            sources: vec![(path.to_string_lossy().into_owned(), text)],
        });
    }
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
    translate(&program(case), &RUST).unwrap_or_else(|e| panic!("{}: {e}", case.key))
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

/// Whether a tool runs, as `program args… --version`.
fn runs(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// A tool is missing: skipped, with a message, unless checkers are
/// required.
fn missing(what: &str) {
    assert!(
        env::var_os("TEMPEST_REQUIRE_CHECKERS").is_none(),
        "{what} (needed with TEMPEST_REQUIRE_CHECKERS)"
    );
    eprintln!("skipped: {what}");
}

/// rustfmt's layout of a Rust file.
fn rustfmt(text: &str) -> String {
    use std::io::Write;
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021", "--emit", "stdout"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "rustfmt: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn goldens() {
    let rustfmt_found = runs("rustfmt", &[]);
    if !rustfmt_found {
        missing("no rustfmt, to check the layout");
    }
    with_stack(move || {
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
                    format!("{library}/Cargo.toml"),
                    format!("{library}/src/lib.rs"),
                    format!("{library}/src/main.rs"),
                    format!("{library}/src/mod.rs"),
                    format!("{library}/src/support/mod.rs"),
                ],
                "{}",
                case.key
            );
            let krate = library.replace('-', "_");
            assert_eq!(
                files[2].text,
                format!("fn main() {{\n    {krate}::init(None).unwrap().run_all_blocking();\n}}\n")
            );
            assert!(
                files[0].text.contains(
                    "\n[dependencies]\ntemper-core = { path = \"../temper-core\", version = \"=0.6.0\" }\n"
                ),
                "{}",
                files[0].text
            );
            if rustfmt_found {
                for file in files.iter().filter(|f| f.path.ends_with(".rs")) {
                    let formatted = rustfmt(&file.text);
                    if formatted != file.text {
                        differ.push(format!(
                            "{}: not as rustfmt lays it out:\n{formatted}",
                            file.path
                        ));
                    }
                }
            }
            let module = &files[3].text;
            let golden = goldens.join(format!("{}.rs", case.key));
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

fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/be-rust/temper-core")
}

/// Writes a file only if its text changed, so cargo's builds stay
/// incremental.
fn write(path: &Path, text: &str) {
    if fs::read_to_string(path).ok().as_deref() == Some(text) {
        return;
    }
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Copies a directory's files, leaving out `target`.
fn copy_dir(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            if entry.file_name() != "target" {
                copy_dir(&path, &to.join(entry.file_name()));
            }
        } else {
            write(
                &to.join(entry.file_name()),
                &fs::read_to_string(&path).unwrap(),
            );
        }
    }
}

/// The harness that runs `module-state` from several threads.
const THREADS: &str = "[package]
name = \"module-state-threads\"
version = \"0.0.0\"
edition = \"2021\"

[dependencies]
module-state = { path = \"../module-state\" }
";

const THREADS_MAIN: &str = "fn main() {
    module_state::init(None).unwrap();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..1000 {
                    module_state::tick();
                    module_state::report();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!(\"ticks {}\", module_state::ticks());
}
";

/// Cargo, in the workspace, offline, with its own target directory.
fn cargo(root: &Path, args: &[&str]) -> Output {
    let (command, rest) = args.split_first().unwrap();
    Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .arg(command)
        .arg("--offline")
        .args(rest)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env_remove("RUSTFLAGS")
        .current_dir(root)
        .output()
        .unwrap()
}

#[test]
fn runs_and_lints() {
    if !runs("cargo", &[]) {
        missing("no cargo");
        return;
    }
    with_stack(move || {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-rust");
        let cases = cases();
        let mut members: Vec<String> = Vec::new();
        for case in &cases {
            for file in generate(case) {
                write(&root.join(&file.path), &file.text);
            }
            members.push(case.library.clone());
        }
        write(&root.join("module-state-threads/Cargo.toml"), THREADS);
        write(&root.join("module-state-threads/src/main.rs"), THREADS_MAIN);
        members.push("module-state-threads".into());
        let list: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
        write(
            &root.join("Cargo.toml"),
            &format!(
                "[workspace]\nresolver = \"2\"\nmembers = [{}]\nexclude = [\"temper-core\"]\n",
                list.join(", ")
            ),
        );
        copy_dir(&runtime(), &root.join("temper-core"));
        if !root.join("Cargo.lock").exists() {
            // indexmap and its dependencies at the versions temper-core
            // was tested with.
            write(
                &root.join("Cargo.lock"),
                &fs::read_to_string(runtime().join("Cargo.lock")).unwrap(),
            );
        }
        let fetch = cargo(&root, &["fetch"]);
        if !fetch.status.success() {
            missing(&format!(
                "temper-core's dependencies aren't in cargo's cache (`cargo fetch` in \
                 vendor/temper/be-rust/temper-core gets them):\n{}",
                String::from_utf8_lossy(&fetch.stderr)
            ));
            return;
        }
        let build = cargo(&root, &["build", "--workspace", "--quiet"]);
        assert!(
            build.status.success(),
            "cargo build:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );
        let mut failures = Vec::new();
        let bin = |name: &str| {
            root.join("target/debug")
                .join(format!("{name}{}", env::consts::EXE_SUFFIX))
        };
        let mut run = |name: &str, expected: &[String]| {
            let out = Command::new(bin(name)).output().unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout);
            let actual: Vec<&str> = stdout.lines().collect();
            if !out.status.success() || actual != expected {
                failures.push(format!(
                    "{name}: expected {expected:?}, got {actual:?}\n{}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
        };
        for case in &cases {
            run(&case.library, &case.expected);
        }
        // Init logs as `module-state` does, then the threads tick.
        let state = cases.iter().find(|c| c.library == "module-state").unwrap();
        let mut expected = state.expected.clone();
        expected.push("ticks 8000".into());
        run("module-state-threads", &expected);
        if runs("cargo", &["clippy"]) {
            let mut args = vec!["clippy", "--workspace", "--quiet", "--", "-D", "warnings"];
            for lint in ALLOWED_LINTS {
                args.extend(["-A", lint]);
            }
            let clippy = cargo(&root, &args);
            if !clippy.status.success() {
                failures.push(format!(
                    "clippy:\n{}",
                    String::from_utf8_lossy(&clippy.stderr)
                ));
            }
        } else {
            missing("no clippy");
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}
