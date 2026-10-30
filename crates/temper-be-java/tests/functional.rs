//! The Java backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, from the vendored copy
//! (`temper-funtests`), and on the shared programs
//! (`crates/temper-be-testkit/programs/`), which cover operators,
//! jumps, module state, several modules, imports, and tests beyond those
//! tests.
//!
//! - `goldens`: each program's generated package, laid out as Kotlin's,
//!   with its root `NAMEGlobal.java` matching the golden in
//!   `tests/goldens/`, named for the test's path (`algos-fibonacci.java`),
//!   and each other generated Java file, if any, its own, named for the
//!   path and the file's place (`modules.helpers.HelpersGlobal.java`,
//!   `testing.TestingTest.java`). To accept new or changed output,
//!   run with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `runs`: without Maven, `javac` (or `JAVAC`) compiles the vendored
//!   `temper-core` (`vendor/temper/be-java/temper-core`) once, for Java 8
//!   as its `pom.xml` says, into `CARGO_TARGET_TMPDIR`, reused until its
//!   sources or the JDK change; then every generated package, and a
//!   harness that runs `module-state` from 8 threads, in one more `javac`,
//!   for Java 17, with every lint as an error ([`LINTS`]); then `java` (or
//!   `JAVA`) runs each program's main class, which logs what the test
//!   expects of Kotlin's `java` backend. Libraries that import others
//!   compile with them, in the same `javac`. Tests (`src/test`) aren't
//!   compiled: they need std's `testing`, which tmpl doesn't translate
//!   yet. Skipped, with a message, without
//!   `javac` or `java`; with `TEMPEST_REQUIRE_CHECKERS` set, as in CI,
//!   either missing fails it instead.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_be_java::{JAVA, OutFile, translate};
use temper_be_testkit::{self as kit, Case, Goldens};

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

fn cases() -> Vec<Case> {
    kit::cases("java", &kit::PROGRAMS)
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&kit::program(case), &JAVA).unwrap_or_else(|e| panic!("{}: {e}", case.key))
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
    kit::with_stack(|| {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
        for case in cases() {
            let files = generate(&case);
            let library = &case.library;
            let (package, class) = names(library);
            let dir = format!("{library}/src/main/java/{package}");
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            let global = format!("{dir}/{class}Global.java");
            let main = format!("{dir}/{class}Main.java");
            assert_eq!(paths[0], format!("{library}/pom.xml"), "{}", case.key);
            let at = |path: &str| paths.iter().position(|p| *p == path);
            let (Some(g), Some(m)) = (at(&global), at(&main)) else {
                panic!("{}: no {global} or {main} in {paths:?}", case.key);
            };
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
                files[m]
                    .text
                    .contains(&format!("Class.forName(\"{package}.{class}Global\");\n")),
                "{}",
                files[m].text
            );
            goldens.check(&format!("{}.java", case.key), &files[g].text);
            // A single module's entry point is as above; others' files
            // have goldens of their own.
            if files.len() > 3 {
                for (i, file) in files.iter().enumerate().skip(1) {
                    if i == g {
                        continue;
                    }
                    let rest = &file.path[library.len() + 1..];
                    let rest = (rest.strip_prefix("src/main/java/"))
                        .or_else(|| rest.strip_prefix("src/test/java/"))
                        .unwrap_or(rest);
                    let rest = rest.strip_prefix(&format!("{package}/")).unwrap_or(rest);
                    goldens.check(
                        &format!("{}.{}", case.key, rest.replace('/', ".")),
                        &file.text,
                    );
                }
                goldens.check(&format!("{}.pom.xml", case.key), &files[0].text);
            }
        }
        goldens.finish();
    });
}

fn runtime() -> PathBuf {
    kit::vendored("be-java/temper-core/src/main/java")
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
    kit::files_with(&runtime(), "java", &mut sources);
    kit::cached(
        &root,
        (version, RUNTIME_OPTIONS),
        &sources,
        &classes,
        || {
            fs::create_dir_all(&classes).unwrap();
            let build = javac(javac_tool, RUNTIME_OPTIONS, None, &classes, &sources);
            assert!(
                build.status.success(),
                "javac, the runtime:\n{}",
                String::from_utf8_lossy(&build.stderr)
            );
        },
    );
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
    let (Some((javac_tool, version)), Some((java, _))) = (
        kit::tool_with("JAVAC", "javac", "--version"),
        kit::tool_with("JAVA", "java", "--version"),
    ) else {
        kit::missing("no javac or java (on PATH, or named by JAVAC and JAVA)");
        return;
    };
    kit::with_stack(move || {
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
                if file.path.ends_with(".java") && !file.path.contains("/src/test/") {
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
            kit::check_log(&mut failures, name, &out, expected);
        };
        for (case, main) in &mains {
            run(&case.key, main, &case.expected);
        }
        // Init logs as `module-state` does, then the threads tick.
        run(
            "module-state-threads",
            "threads.ModuleStateThreads",
            &kit::threads_expected(&cases),
        );
        kit::assert_none(&failures);
    });
}
