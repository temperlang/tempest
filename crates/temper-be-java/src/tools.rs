//! How Java runs built libraries' tests, for `tempest test -b java`:
//! `javac` on the runtime, std, and the libraries, then on a library's
//! tests, against JUnit 5's jars in Maven's local repository, as
//! Kotlin's `pom.xml` names them, and a small runner of JUnit's launcher
//! under `java`, as this crate's own tests do; Kotlin runs Maven
//! instead, which needs its network. `tempest run -b java` compiles the
//! runtime, std, and the libraries the same way, quietly, and runs the
//! library's `…Main` class under `java`, as Kotlin runs it by exec.

use std::path::{Path, PathBuf};
use std::process::Command;
use temper_tmpl::tools::{files_with, output, program, tool, tool_output};
use temper_tmpl::{TestResults, Tested, Tools};

pub const TOOLS: Tools = Tools {
    prepare: None,
    test,
    run,
};

/// Compiles the runtime, std, and the libraries' main sources with
/// `javac` (`JAVAC`) into `classes`, with `extra` sources and `classpath`.
fn compile_main(
    dir: &Path,
    all: &[Tested],
    classes: &Path,
    extra: &[PathBuf],
    classpath: &std::ffi::OsStr,
) -> Result<(), String> {
    let mut sources = Vec::new();
    let roots = ["temper-core", "std"].into_iter().map(String::from);
    for root in roots.chain(all.iter().map(|l| l.name.clone())) {
        files_with(&dir.join(root).join("src/main/java"), "java", &mut sources);
    }
    sources.extend(extra.iter().cloned());
    javac(classes, classpath, &sources)
}

/// `javac` on `files` into a directory, against a class path.
fn javac(into: &Path, classpath: &std::ffi::OsStr, files: &[PathBuf]) -> Result<(), String> {
    let built = output(
        Command::new(tool("JAVAC", "javac"))
            .args(["-encoding", "UTF-8", "-nowarn", "-d"])
            .arg(into)
            .arg("-cp")
            .arg(classpath)
            .args(files),
        "javac",
    )?;
    match built.status.success() {
        true => Ok(()),
        false => Err(format!("javac failed\n{}", tool_output(&built))),
    }
}

/// Compiles as [`test`] does, without JUnit, into
/// `temper.out/java/LIBRARY/target/tempest/classes`, and runs the
/// library's main class, the one its `…Main.java` declares.
fn run(dir: &Path, library: &Tested, all: &[Tested]) -> Result<bool, String> {
    let here = dir.join(&library.name);
    let mut mains = Vec::new();
    files_with(&here.join("src/main/java"), "java", &mut mains);
    let main = (mains.iter())
        .filter(|f| {
            f.file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with("Main.java"))
        })
        .find_map(|f| f.strip_prefix(here.join("src/main/java")).ok())
        .map(|f| {
            f.with_extension("")
                .to_string_lossy()
                .replace(['/', '\\'], ".")
        })
        .ok_or_else(|| format!("no main class in {}", here.display()))?;
    let classes = here.join("target/tempest/classes");
    let _ = std::fs::remove_dir_all(&classes);
    compile_main(dir, all, &classes, &[], std::ffi::OsStr::new(""))?;
    program(
        Command::new(tool("JAVA", "java"))
            .args(["-XX:TieredStopAtLevel=1", "-XX:+UseSerialGC", "-cp"])
            .arg(&classes)
            .arg(main),
        "java",
    )
}

/// [`JUNIT`]'s jars in Maven's local repository (`~/.m2/repository`),
/// or the first that's missing.
pub fn junit_jars() -> Result<Vec<PathBuf>, PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let jars: Vec<PathBuf> = (JUNIT.iter())
        .map(|j| home.join(".m2/repository").join(j))
        .collect();
    match jars.iter().find(|j| !j.exists()) {
        Some(missing) => Err(missing.clone()),
        None => Ok(jars),
    }
}

/// JUnit 5's jars, as Kotlin's `pom.xml` names them (`junit-jupiter`
/// 5.9.2), with what they need to compile tests and run them through
/// JUnit's launcher, by their paths in Maven's local repository.
pub const JUNIT: [&str; 7] = [
    "org/junit/jupiter/junit-jupiter-api/5.9.2/junit-jupiter-api-5.9.2.jar",
    "org/junit/jupiter/junit-jupiter-engine/5.9.2/junit-jupiter-engine-5.9.2.jar",
    "org/junit/platform/junit-platform-commons/1.9.2/junit-platform-commons-1.9.2.jar",
    "org/junit/platform/junit-platform-engine/1.9.2/junit-platform-engine-1.9.2.jar",
    "org/junit/platform/junit-platform-launcher/1.9.2/junit-platform-launcher-1.9.2.jar",
    "org/opentest4j/opentest4j/1.2.0/opentest4j-1.2.0.jar",
    "org/apiguardian/apiguardian-api/1.1.2/apiguardian-api-1.1.2.jar",
];

/// Runs the tests compiled into the directory it's given through JUnit's
/// launcher, printing a line per test: `ok`, or `failed` and its
/// message, then its name, tab separated, with newlines as `\n`.
pub const RUN_TESTS: &str = r#"package tempest;

import java.nio.file.Paths;
import java.util.Set;
import org.junit.platform.engine.TestExecutionResult;
import org.junit.platform.launcher.Launcher;
import org.junit.platform.launcher.TestExecutionListener;
import org.junit.platform.launcher.TestIdentifier;
import org.junit.platform.launcher.core.LauncherFactory;

import static org.junit.platform.engine.discovery.DiscoverySelectors.selectClasspathRoots;
import static org.junit.platform.launcher.core.LauncherDiscoveryRequestBuilder.request;

public final class RunTests {
    private RunTests() {}

    public static void main(String[] args) {
        Launcher launcher = LauncherFactory.create();
        TestExecutionListener listener = new TestExecutionListener() {
            @Override
            public void executionFinished(TestIdentifier id, TestExecutionResult result) {
                if (!id.isTest()) {
                    return;
                }
                String name = id.getDisplayName().replace("\t", " ");
                if (result.getStatus() == TestExecutionResult.Status.SUCCESSFUL) {
                    System.out.println("ok\t" + name);
                } else {
                    String message = result.getThrowable().map(t -> String.valueOf(t.getMessage())).orElse("failed");
                    System.out.println("failed\t" + name + "\t" + message.replace("\n", "\\n"));
                }
            }
        };
        launcher.execute(request().selectors(selectClasspathRoots(Set.of(Paths.get(args[0])))).build(), listener);
    }
}
"#;

/// Compiles the runtime, std, the libraries, and a library's tests with
/// `javac` (`JAVAC`), against JUnit's jars in Maven's local repository,
/// into `temper.out/java/LIBRARY/target/tempest`, and runs them with
/// `java` (`JAVA`) through JUnit's launcher ([`RUN_TESTS`]).
fn test(dir: &Path, library: &Tested, all: &[Tested]) -> TestResults {
    let jars = junit_jars().map_err(|missing| {
        format!(
            "no {} (JUnit, as Kotlin's pom.xml names it)",
            missing.display()
        )
    })?;
    let out = dir.join(&library.name).join("target/tempest");
    let _ = std::fs::remove_dir_all(&out);
    let (main, tests) = (out.join("classes"), out.join("test-classes"));
    let runner = out.join("tempest/RunTests.java");
    let written = runner
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&runner, RUN_TESTS));
    written.map_err(|e| format!("{}: {e}", runner.display()))?;
    let classpath = std::env::join_paths(&jars).map_err(|e| e.to_string())?;
    compile_main(dir, all, &main, &[runner], &classpath)?;
    let mut test_sources = Vec::new();
    files_with(
        &dir.join(&library.name).join("src/test/java"),
        "java",
        &mut test_sources,
    );
    let with_main = std::env::join_paths(std::iter::once(main.clone()).chain(jars.iter().cloned()))
        .map_err(|e| e.to_string())?;
    javac(&tests, &with_main, &test_sources)?;
    let everything = std::env::join_paths([tests.clone(), main].into_iter().chain(jars))
        .map_err(|e| e.to_string())?;
    let run = output(
        Command::new(tool("JAVA", "java"))
            .args(["-XX:TieredStopAtLevel=1", "-XX:+UseSerialGC", "-cp"])
            .arg(everything)
            .arg("tempest.RunTests")
            .arg(&tests),
        "java",
    )?;
    let results = results(&String::from_utf8_lossy(&run.stdout));
    if results.is_empty() {
        return Err(format!("no test results\n{}", tool_output(&run)));
    }
    Ok(results)
}

/// Each test's name and failure, as [`RUN_TESTS`] prints them.
pub fn results(stdout: &str) -> Vec<(String, Option<String>)> {
    (stdout.lines())
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            // JUnit's display name of a method, `b()`, as the test `b`.
            let (status, name, cause) = (parts.next(), parts.next(), parts.next());
            let name = name.map(|n| n.strip_suffix("()").unwrap_or(n));
            match (status, name, cause) {
                (Some("ok"), Some(name), _) => Some((name.to_string(), None)),
                (Some("failed"), Some(name), cause) => {
                    let cause = cause.unwrap_or("failed").replace("\\n", "\n");
                    Some((name.to_string(), Some(cause)))
                }
                _ => None,
            }
        })
        .collect()
}
