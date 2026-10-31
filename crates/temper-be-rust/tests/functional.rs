//! The Rust backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! (`temper-funtests`), with the class tests (`kit::CLASSES`) and those
//! with nullable types and `orelse` (`kit::NULLABLES`), and on
//! the shared programs (`crates/temper-be-testkit/programs/`), which
//! cover classes and interfaces (`classes`), nullables and `orelse`
//! (`nullables`), module state, operators,
//! jumps, string order, modules and imports (`modules`, and
//! `imports-library`, which imports it), and tests (`testing`), and on
//! this crate's own (`tests/programs/`), which cover what's particular
//! to Rust ([`OWN`]).
//! - `goldens`: each program's generated crate, laid out as Kotlin's,
//!   with its `src/mod.rs` matching the golden in `tests/goldens/`, named
//!   for the test's path (`algos-fibonacci.rs`), its other modules'
//!   files theirs (`modules.helpers.deep.rs`), and every generated
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
//!   fire on what the tests do on purpose ([`ALLOWED_LINTS`]), and dead
//!   code in a test that declares what it never uses ([`UNUSED`]). Tests
//!   need std's `testing`, which tmpl doesn't translate yet, so the
//!   workspace has a stand-in `temper-std` ([`STD_STUB`]) with just what
//!   the test form calls, and `testing`'s tests build and run against it
//!   (`cargo test`), checking the form, not std. Skipped, with a message, without cargo
//!   or clippy, or if `temper-core`'s one dependency, `indexmap`, isn't
//!   in cargo's cache (`cargo fetch` in the vendored `temper-core` puts
//!   it there); with `TEMPEST_REQUIRE_CHECKERS` set, as in CI, any of
//!   those fails it instead.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_be_rust::{OutFile, RUST, translate};
use temper_be_testkit::{self as kit, Case, Goldens};

/// Clippy's lints that fire on what the functional tests do on purpose:
/// `zero = zero` (`self_assignment`), `1.min(3)`
/// (`unnecessary_min_or_max`), and `infinity - infinity` (`eq_op`), as in
/// Kotlin's output too; and a local named `foo` (`disallowed_names`),
/// which Kotlin's output avoids only by numbering every name (`foo__3`).
const ALLOWED_LINTS: [&str; 4] = [
    "clippy::self_assignment",
    "clippy::unnecessary_min_or_max",
    "clippy::eq_op",
    "clippy::disallowed_names",
];

/// The tests that declare what they never use, a function, a default
/// method that every class overrides, a parameter, or locals, on purpose,
/// so rustc's
/// `dead_code` and `unused_variables` are allowed in them.
const UNUSED: [&str; 3] = [
    "call-override-from-subtype",
    "mutually-referencing-types",
    "bubble",
];

/// This crate's own programs, in `tests/programs/`, which test what's
/// particular to Rust.
const OWN: [&str; 2] = ["class-edges", "nullable-edges"];

fn cases() -> Vec<Case> {
    let tests = [&kit::CLASSES[..], &kit::NULLABLES[..]].concat();
    let mut cases = kit::cases_with("rust", &kit::PROGRAMS, &tests);
    for name in OWN {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/programs")
            .join(format!("{name}.temper.md"));
        let text = fs::read_to_string(&path).unwrap();
        // Its one log block.
        let log = text.split("```log\n").nth(1).unwrap_or_default();
        let expected = log.split("```").next().unwrap_or_default();
        let expected = expected.lines().map(str::to_string).collect();
        let file = format!("{name}.temper.md");
        cases.push(Case::inline(name, &file, text, expected));
    }
    cases
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&kit::program(case), &RUST).unwrap_or_else(|e| panic!("{}: {e}", case.key))
}

/// Whether a tool runs, as `program args… --version`.
fn runs(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
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
        kit::missing("no rustfmt, to check the layout");
    }
    kit::with_stack(move || {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
        for case in cases() {
            let files = generate(&case);
            let library = &case.library;
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(
                paths[..5],
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
            let main = &files[2].text;
            assert!(
                main.starts_with(&format!("fn main() {{\n    {krate}::init(None)")),
                "{main}"
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
                        goldens.differs(format!(
                            "{}: not as rustfmt lays it out:\n{formatted}",
                            file.path
                        ));
                    }
                }
            }
            goldens.check(&format!("{}.rs", case.key), &files[3].text);
            // `lib.rs` and `Cargo.toml` where modules or libraries show.
            if !case.sources[0].0.contains("functional-tests")
                && (files.len() > 5 || files[0].text.contains("std"))
                || !case.libraries.is_empty()
            {
                goldens.check(&format!("{}.lib.rs", case.key), &files[1].text);
                goldens.check(&format!("{}.Cargo.toml", case.key), &files[0].text);
            }
            for file in &files[5..] {
                let rest = &file.path[library.len() + "/src/".len()..];
                let rest = rest.strip_suffix("/mod.rs").unwrap_or(rest);
                goldens.check(
                    &format!("{}.{}.rs", case.key, rest.replace('/', ".")),
                    &file.text,
                );
            }
        }
        goldens.finish();
    });
}

/// A shift of a literal, `0x7F >> n`: Kotlin's backend writes
/// `127.wrapping_shr(…)`, which rustc rejects, since it can't call an
/// inherent method of `{integer}`; here the literal gets its type's
/// suffix, `127i32.wrapping_shr(…)`.
#[test]
fn literal_shifts() {
    let text = "export let lead(first: Int, n: Int): Int { first & (0x7F >> n) }\n";
    let case = Case::inline("shifts", "shifts.temper", text, Vec::new());
    kit::with_stack(move || {
        let lib = &generate(&case)[3].text;
        assert!(
            lib.contains("first & 127i32.wrapping_shr(n as u32)"),
            "{lib}"
        );
    });
}

fn runtime() -> PathBuf {
    kit::vendored("be-rust/temper-core")
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

/// A stand-in for std's crate, `temper-std`, until tmpl translates std:
/// `init`, and `testing::Test` with what the test form calls.
const STD_STUB: (&str, &str) = (
    "[package]
name = \"temper-std\"
version = \"0.6.0\"
edition = \"2021\"

[dependencies]
temper-core = { path = \"../temper-core\" }
",
    "pub fn init(_config: Option<temper_core::Config>) -> temper_core::Result<()> {
    Ok(())
}

pub mod testing {
    pub struct Test;

    impl Test {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Test {
            Test
        }

        pub fn soft_fail_to_hard(&self) -> temper_core::Result<()> {
            Ok(())
        }
    }
}
",
);

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
        kit::missing("no cargo");
        return;
    }
    kit::with_stack(move || {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-rust");
        // Each library a package of its own in one workspace, so one
        // named as another (`basics`, for `types/int/basics` and
        // `types/float/basics`) is named by its key instead.
        let mut cases = cases();
        let mut seen = std::collections::HashSet::new();
        for case in &mut cases {
            if !seen.insert(case.library.clone()) {
                case.library = case.key.clone();
            }
        }
        let mut members: Vec<String> = Vec::new();
        for case in &cases {
            for file in generate(case) {
                kit::write_if_changed(&root.join(&file.path), &file.text);
            }
            members.push(case.library.clone());
        }
        kit::write_if_changed(&root.join("std/Cargo.toml"), STD_STUB.0);
        kit::write_if_changed(&root.join("std/src/lib.rs"), STD_STUB.1);
        members.push("std".into());
        kit::write_if_changed(&root.join("module-state-threads/Cargo.toml"), THREADS);
        kit::write_if_changed(&root.join("module-state-threads/src/main.rs"), THREADS_MAIN);
        members.push("module-state-threads".into());
        let list: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
        kit::write_if_changed(
            &root.join("Cargo.toml"),
            &format!(
                "[workspace]\nresolver = \"2\"\nmembers = [{}]\nexclude = [\"temper-core\"]\n",
                list.join(", ")
            ),
        );
        kit::copy_dir(&runtime(), &root.join("temper-core"), &["target"]);
        if !root.join("Cargo.lock").exists() {
            // indexmap and its dependencies at the versions temper-core
            // was tested with.
            kit::write_if_changed(
                &root.join("Cargo.lock"),
                &fs::read_to_string(runtime().join("Cargo.lock")).unwrap(),
            );
        }
        let fetch = cargo(&root, &["fetch"]);
        if !fetch.status.success() {
            kit::missing(&format!(
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
            kit::check_log(&mut failures, name, &out, expected);
        };
        for case in &cases {
            run(&case.library, &case.expected);
        }
        // Init logs as `module-state` does, then the threads tick.
        run("module-state-threads", &kit::threads_expected(&cases));
        // The test form builds and runs, against the stand-in std.
        let tests = cargo(&root, &["test", "--quiet", "-p", "testing"]);
        if !tests.status.success() {
            failures.push(format!(
                "cargo test -p testing:\n{}{}",
                String::from_utf8_lossy(&tests.stdout),
                String::from_utf8_lossy(&tests.stderr)
            ));
        }
        if runs("cargo", &["clippy"]) {
            // Most crates strictly, then those that declare what they
            // never use, allowing that.
            let mut strict = vec!["clippy", "--workspace", "--all-targets", "--quiet"];
            for package in UNUSED {
                strict.extend(["--exclude", package]);
            }
            let mut lax = vec!["clippy", "--all-targets", "--quiet"];
            for package in UNUSED {
                lax.extend(["-p", package]);
            }
            let unused = ["dead_code", "unused_variables"];
            for (mut args, extra) in [(strict, &[][..]), (lax, &unused[..])] {
                args.extend(["--", "-D", "warnings"]);
                for lint in ALLOWED_LINTS.iter().chain(extra) {
                    args.extend(["-A", lint]);
                }
                let clippy = cargo(&root, &args);
                if !clippy.status.success() {
                    failures.push(format!(
                        "clippy:\n{}",
                        String::from_utf8_lossy(&clippy.stderr)
                    ));
                }
            }
        } else {
            kit::missing("no clippy");
        }
        kit::assert_none(&failures);
    });
}
