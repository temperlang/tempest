//! The Java backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! ([`temper_funtests`]), and on the Rust backend's own programs
//! (`crates/temper-be-rust/tests/programs/`), which cover operators,
//! jumps, and module state beyond those tests.
//!
//! - `goldens`: each program's generated package, laid out as Kotlin's,
//!   with its `NAMEGlobal.java` matching the golden in `tests/goldens/`,
//!   named for the test's path (`algos-fibonacci.java`). To accept new or
//!   changed output, run with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `runs`: without Maven, `javac` (or `JAVAC`) compiles the vendored
//!   `temper-core` (`vendor/temper/be-java/temper-core`) once, for Java 8
//!   as its `pom.xml` says, into `CARGO_TARGET_TMPDIR`, reused until its
//!   sources or the JDK change; then every generated package, and a
//!   harness that runs `module-state` from 8 threads, in one more `javac`,
//!   for Java 17, with every lint as an error ([`LINTS`]); then `java` (or
//!   `JAVA`) runs each program's main class, which logs what the test
//!   expects of Kotlin's `java` backend. Skipped, with a message, without
//!   `javac` or `java`; with `TEMPEST_REQUIRE_CHECKERS` set, as in CI,
//!   either missing fails it instead.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_be_java::{JAVA, OutFile, translate};
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

/// javac's options for generated code: Java 17, as Kotlin's `pom.xml`
/// has it, UTF-8, every lint (`-Xlint:all`) and every Javadoc check but
/// for missing comments (`-Xdoclint:all,-missing`, as Kotlin's `pom.xml`
/// leaves them out of `javadoc`'s), all as errors. No lint is allowed.
const LINTS: &[&str] = &[
    "--release",
    "17",
    "-encoding",
    "UTF-8",
    "-Xlint:all",
    "-Xdoclint:all,-missing",
    "-Werror",
];

/// javac's options for the runtime: Java 8, as its `pom.xml` has it,
/// without lints, since it isn't generated here, and without the warning
/// that Java 8 is obsolete.
const RUNTIME_OPTIONS: &[&str] = &[
    "--release",
    "8",
    "-encoding",
    "UTF-8",
    "-nowarn",
    "-Xlint:-options",
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
            expected: test.expected_log("java"),
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
            expected: temper_funtests::expected_log(&text, "java"),
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
    translate(&program(case), &JAVA).unwrap_or_else(|e| panic!("{}: {e}", case.key))
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

/// A library's package and class names' start, as Kotlin makes them.
fn names(library: &str) -> (String, String) {
    let package = library.replace('-', "_");
    let class: String = package
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect();
    (package, class)
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
            let (package, class) = names(library);
            let dir = format!("{library}/src/main/java/{package}");
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(
                paths,
                [
                    format!("{library}/pom.xml"),
                    format!("{dir}/{class}Global.java"),
                    format!("{dir}/{class}Main.java"),
                ],
                "{}",
                case.key
            );
            let pom = &files[0].text;
            for part in [
                format!("<groupId>{package}</groupId>\n  <artifactId>{library}</artifactId>"),
                "<artifactId>temper-core</artifactId>\n      <version>0.6.0</version>".into(),
                "<release>17</release>".into(),
                format!("<mainClass>{package}.{class}Main</mainClass>"),
            ] {
                assert!(pom.contains(&part), "{}: {part} in {pom}", case.key);
            }
            assert!(
                files[2]
                    .text
                    .contains(&format!("Class.forName(\"{package}.{class}Global\");\n")),
                "{}",
                files[2].text
            );
            let module = &files[1].text;
            let golden = goldens.join(format!("{}.java", case.key));
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
/// runs, and its version.
fn tool(var: &str, name: &str) -> Option<(String, String)> {
    let candidate = env::var(var).unwrap_or_else(|_| name.to_string());
    let out = Command::new(&candidate).arg("--version").output().ok()?;
    let version = [out.stdout, out.stderr].concat();
    out.status
        .success()
        .then(|| (candidate, String::from_utf8_lossy(&version).into_owned()))
}

fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper/be-java/temper-core/src/main/java")
}

/// Every `.java` file under a directory, sorted.
fn java_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            java_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "java") {
            out.push(path);
        }
    }
}

fn javac(
    javac: &str,
    options: &[&str],
    classpath: Option<&Path>,
    out: &Path,
    files: &[PathBuf],
) -> Output {
    let mut command = Command::new(javac);
    command.args(options).arg("-d").arg(out);
    if let Some(cp) = classpath {
        command.arg("-cp").arg(cp);
    }
    command.args(files).output().unwrap()
}

/// The runtime's classes, compiled once and kept under
/// `CARGO_TARGET_TMPDIR` until its sources or the JDK change.
fn runtime_classes(javac_tool: &str, version: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-java-runtime");
    let classes = root.join("classes");
    let mut sources = Vec::new();
    java_files(&runtime(), &mut sources);
    let mut hasher = DefaultHasher::new();
    version.hash(&mut hasher);
    RUNTIME_OPTIONS.hash(&mut hasher);
    for source in &sources {
        source.hash(&mut hasher);
        fs::read(source).unwrap().hash(&mut hasher);
    }
    let stamp = format!("{:016x}\n", hasher.finish());
    let stamp_path = root.join("stamp");
    if fs::read_to_string(&stamp_path).ok().as_deref() == Some(&stamp) {
        return classes;
    }
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&classes).unwrap();
    let build = javac(javac_tool, RUNTIME_OPTIONS, None, &classes, &sources);
    assert!(
        build.status.success(),
        "javac, the runtime:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    fs::write(stamp_path, stamp).unwrap();
    classes
}

/// The harness that runs `module-state` from several threads.
const THREADS: &str = "package threads;

import module_state.ModuleStateGlobal;
import temper.core.Core;

public final class ModuleStateThreads {
    private ModuleStateThreads() {}

    public static void main(String[] args) throws InterruptedException {
        Core.initSimpleLogging();
        Thread[] threads = new Thread[8];
        for (int i = 0; i < threads.length; i++) {
            threads[i] = new Thread(() -> {
                for (int j = 0; j < 1000; j++) {
                    ModuleStateGlobal.tick();
                    ModuleStateGlobal.report();
                }
            });
            threads[i].start();
        }
        for (Thread thread : threads) {
            thread.join();
        }
        System.out.println(\"ticks \" + ModuleStateGlobal.ticks);
    }
}
";

#[test]
fn runs() {
    let required = env::var_os("TEMPEST_REQUIRE_CHECKERS").is_some();
    let (Some((javac_tool, version)), Some((java, _))) =
        (tool("JAVAC", "javac"), tool("JAVA", "java"))
    else {
        assert!(!required, "no javac or java");
        eprintln!("skipped: no javac or java (on PATH, or named by JAVAC and JAVA)");
        return;
    };
    with_stack(move || {
        let runtime = runtime_classes(&javac_tool, &version);
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-java");
        let _ = fs::remove_dir_all(&out);
        let cases = cases();
        let mut sources = Vec::new();
        let mut mains = Vec::new();
        for case in &cases {
            let root = out.join("src").join(&case.key);
            for file in generate(case) {
                let path = root.join(&file.path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &file.text).unwrap();
                if file.path.ends_with(".java") {
                    sources.push(path);
                }
            }
            let (package, class) = names(&case.library);
            mains.push((case, format!("{package}.{class}Main")));
        }
        let threads = out.join("src/threads/ModuleStateThreads.java");
        fs::create_dir_all(threads.parent().unwrap()).unwrap();
        fs::write(&threads, THREADS).unwrap();
        sources.push(threads);
        let classes = out.join("classes");
        let build = javac(&javac_tool, LINTS, Some(&runtime), &classes, &sources);
        assert!(
            build.status.success(),
            "javac {}:\n{}",
            LINTS.join(" "),
            String::from_utf8_lossy(&build.stderr)
        );
        let classpath = env::join_paths([&classes, &runtime]).unwrap();
        let mut failures = Vec::new();
        let mut run = |name: &str, main: &str, expected: &[String]| {
            let out = Command::new(&java)
                .args(["-XX:TieredStopAtLevel=1", "-XX:+UseSerialGC", "-cp"])
                .arg(&classpath)
                .arg(main)
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout);
            let actual: Vec<&str> = stdout.lines().collect();
            if !out.status.success() || actual != expected {
                failures.push(format!(
                    "{name}: expected {expected:?}, got {actual:?}\n{}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
        };
        for (case, main) in &mains {
            run(&case.key, main, &case.expected);
        }
        // Init logs as `module-state` does, then the threads tick.
        let state = cases.iter().find(|c| c.library == "module-state").unwrap();
        let mut expected = state.expected.clone();
        expected.push("ticks 8000".into());
        run(
            "module-state-threads",
            "threads.ModuleStateThreads",
            &expected,
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}
