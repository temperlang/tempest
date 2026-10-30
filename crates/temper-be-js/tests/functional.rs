//! The JS backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! ([`temper_funtests`]), and on the Rust backend's own programs
//! (`crates/temper-be-rust/tests/programs/`), which cover operators,
//! jumps, and module state beyond those tests.
//!
//! - `goldens`: each program's generated package, laid out as Kotlin's,
//!   with its `.internal.js` module matching the golden in
//!   `tests/goldens/`, named for the test's path (`algos-fibonacci.js`).
//!   To accept new or changed output, run with `UPDATE_SNAPSHOTS=1` and
//!   review the diff.
//! - `runs`: each generated package runs under Node (`node`, or `NODE`),
//!   against the vendored `@temperlang/core`
//!   (`vendor/temper/be-js/temper-core`), and logs what the test expects
//!   of Kotlin's `js` backend; and TypeScript's checker (`tsc`, or `TSC`)
//!   finds no problems in it with `--checkJs --strict`, against
//!   declarations it generates from the runtime. Skipped, with a message,
//!   without Node, and its type check without `tsc`; with
//!   `TEMPEST_REQUIRE_CHECKERS` set, as in CI, either missing fails it
//!   instead.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use temper_be_js::{JS, OutFile, translate};
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
const PROGRAMS: [&str; 3] = ["module-state", "operators", "jumps.tmpl"];

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
            expected: test.expected_log("js"),
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
            expected: temper_funtests::expected_log(&text, "js"),
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
    translate(&program(case), &JS).unwrap_or_else(|e| panic!("{}: {e}", case.key))
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

#[test]
fn goldens() {
    with_stack(|| {
        let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
        let goldens = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens");
        let mut differ = Vec::new();
        for case in cases() {
            let files = generate(&case);
            let library = &case.library;
            let file = library.replace('-', "_");
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(
                paths,
                [
                    format!("{library}/package.json"),
                    format!("{library}/index.js"),
                    format!("{library}/{file}.js"),
                    format!("{library}/{file}.internal.js"),
                ],
                "{}",
                case.key
            );
            assert!(files[0].text.contains("\"type\": \"module\""));
            assert!(files[0].text.contains("\"@temperlang/core\": \"0.6.0\""));
            assert_eq!(files[1].text, format!("export * from \"./{file}.js\";\n"));
            assert!(
                files[2]
                    .text
                    .ends_with(&format!(" from \"./{file}.internal.js\";\n")),
                "{}",
                files[2].text
            );
            let module = &files[3].text;
            let golden = goldens.join(format!("{}.js", case.key));
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

/// A tool named by an environment variable, or found on `PATH`, if it
/// runs.
fn tool(var: &str, name: &str) -> Option<String> {
    let candidate = env::var(var).unwrap_or_else(|_| name.to_string());
    Command::new(&candidate)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
        .then_some(candidate)
}

fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/be-js/temper-core")
}

fn write_files(root: &Path, files: &[OutFile]) {
    for file in files {
        let path = root.join(&file.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &file.text).unwrap();
    }
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

/// TypeScript's options for checking generated code: strict, as ES
/// modules under Node's rules, with the runtime's declarations taken as
/// given (`--skipLibCheck`), since they don't all check themselves.
const TSC_CHECK: &[&str] = &[
    "--checkJs",
    "--allowJs",
    "--noEmit",
    "--strict",
    "--skipLibCheck",
    "--module",
    "nodenext",
    "--target",
    "es2022",
];

#[test]
fn runs() {
    let required = env::var_os("TEMPEST_REQUIRE_CHECKERS").is_some();
    let Some(node) = tool("NODE", "node") else {
        assert!(!required, "no node");
        eprintln!("skipped: no Node (on PATH, or named by NODE)");
        return;
    };
    let tsc = tool("TSC", "tsc");
    if tsc.is_none() {
        assert!(!required, "no tsc");
        eprintln!("type check skipped: no tsc (on PATH, or named by TSC)");
    }
    with_stack(move || {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-js");
        let _ = fs::remove_dir_all(&out);
        // The runtime, where Node and tsc look for `@temperlang/core`.
        let core = out.join("node_modules/@temperlang/core");
        copy_dir(&runtime(), &core);
        let mut failures = Vec::new();
        let mut checked = Vec::new();
        for case in cases() {
            let files = generate(&case);
            let root = out.join(&case.key);
            write_files(&root, &files);
            let index = root.join(&case.library).join("index.js");
            let run = Command::new(&node)
                .arg(&index)
                .current_dir(&out)
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&run.stdout);
            let actual: Vec<&str> = stdout.lines().collect();
            if !run.status.success() || actual != case.expected {
                failures.push(format!(
                    "{}: expected {:?}, got {actual:?}\n{}",
                    case.key,
                    case.expected,
                    String::from_utf8_lossy(&run.stderr)
                ));
            }
            checked.extend(
                files
                    .iter()
                    .filter(|f| f.path.ends_with(".js"))
                    .map(|f| root.join(&f.path)),
            );
        }
        if let Some(tsc) = &tsc {
            // Declarations for the runtime, beside its modules, generated
            // from a copy outside `node_modules`, which tsc won't emit for.
            let source = out.join("temper-core");
            copy_dir(&runtime(), &source);
            let emit = Command::new(tsc)
                .args(["--allowJs", "--declaration", "--emitDeclarationOnly"])
                .args(["--module", "nodenext", "--target", "es2022", "--outDir"])
                .arg(&core)
                .arg(source.join("index.js"))
                .current_dir(&out)
                .output()
                .unwrap();
            if !core.join("index.d.ts").exists() {
                failures.push(format!(
                    "tsc made no declarations for the runtime:\n{}",
                    String::from_utf8_lossy(&emit.stdout)
                ));
            } else {
                let check = Command::new(tsc)
                    .args(TSC_CHECK)
                    .args(&checked)
                    .current_dir(&out)
                    .output()
                    .unwrap();
                if !check.status.success() {
                    failures.push(format!(
                        "tsc --checkJs --strict:\n{}{}",
                        String::from_utf8_lossy(&check.stdout),
                        String::from_utf8_lossy(&check.stderr)
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}
