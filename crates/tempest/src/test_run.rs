//! `tempest test -b BACKEND`: running built libraries' tests with each
//! language's own tools, as Kotlin Temper's `temper test -b` does, on
//! what `build` wrote to `temper.out/BACKEND`, and reading their results
//! into the interpreter's summary ([`Tally`]), so `-b interp` and `-b py`
//! print alike.
//!
//! - **Python:** `python3 -B -m temper_core.testing result.xml` in each
//!   library's directory, with `temper-core`, `std`, and the libraries on
//!   `PYTHONPATH` (or `PYTHON` for the interpreter), as Kotlin's
//!   `RunPython`.
//! - **JavaScript:** Mocha (`mocha`, or `MOCHA`) in each library's
//!   package, `mocha test --recursive` with its xunit reporter, Node
//!   finding `@temperlang/core`, `@temperlang/std`, and the libraries in
//!   `temper.out/js/node_modules`, linked to their directories, as npm's
//!   workspace that Kotlin makes links them.
//! - **Lua:** each library's `tests/init-test.lua` under `lua` (or
//!   `LUA`), with LuaUnit beside the libraries, as Kotlin copies it, and
//!   `LUA_PATH` of `?.lua;?/init.lua`, writing JUnit XML.
//! - **Rust:** `cargo test` in each library's crate (`CARGO`), offline,
//!   with one target directory, `temper.out/rust/target`, for them all.
//! - **C#:** `dotnet test --logger:junit` in each library's `tests`
//!   project (`DOTNET`), as Kotlin's `RunCSharp`, its packages restored
//!   from NuGet's cache or feed, reading `TestResults/TestResults.xml`;
//!   then `dotnet build-server shutdown`, so nothing stays running.
//! - **Java:** `javac` on the runtime, std, and the libraries, then on
//!   a library's tests, against JUnit 5's jars in Maven's local
//!   repository, as Kotlin's `pom.xml` names them, and a small runner
//!   of JUnit's launcher under `java`, as the Java backend's own tests
//!   do; Kotlin runs Maven instead, which needs its network.
//!
//! - **C++:** CMake (`CMAKE`) on each library's `CMakeLists.txt`, which
//!   tempest writes, in `temper.out/cpp/LIBRARY/build`, building its
//!   `LIBRARY-test` target, its tests with std and the libraries it
//!   imports, by `g++` or whatever CMake finds (`CXX`), then that program
//!   run in the library's directory, which writes `test-results.xml`, as
//!   Kotlin's `RunCpp` runs its `g++` build.
//!
//! Each library's results are its
//! tests' names and failures, as the tools report them; a run with no
//! results fails the library, with the tool's output.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use temper_frontend::library_tests::LibraryTests;

/// A library that built, with how many tests it has.
pub struct Tested {
    pub name: String,
    pub tests: usize,
}

/// Runs the tests of the libraries in `dir` (`temper.out/BACKEND`), by the
/// backend's name, each
/// as its results, or why there are none. `all` is every library built
/// there, which tests may import.
pub fn run(
    backend: &str,
    dir: &Path,
    all: &[Tested],
) -> Vec<(String, Result<LibraryTests, String>)> {
    let mut out = Vec::new();
    // Tools run in libraries' directories, so paths are absolute.
    let dir = &std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let setup = match backend {
        "js" => link_js(dir, all),
        "lua" => write_luaunit(dir),
        _ => Ok(()),
    };
    for library in all.iter().filter(|l| l.tests > 0) {
        let result = setup.clone().and_then(|()| {
            let results = match backend {
                "cpp" => cpp(dir, library),
                "csharp" => csharp(dir, library),
                "java" => java(dir, library, all),
                "js" => js(dir, library),
                "lua" => lua(dir, library),
                "py" => python(dir, library, all),
                "rust" => rust(dir, library),
                _ => Err(format!("no test runner for {backend}")),
            }?;
            Ok(LibraryTests {
                results,
                defined: library.tests,
                output: Vec::new(),
                failure: None,
                diagnostics: String::new(),
                errors: 0,
            })
        });
        out.push((library.name.clone(), result));
    }
    out
}

/// A tool named by an environment variable, or else by its usual name.
fn tool(var: &str, name: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| name.to_string())
}

/// Runs a command, or says why it couldn't start.
fn output(command: &mut Command, what: &str) -> Result<Output, String> {
    command
        .output()
        .map_err(|e| format!("couldn't run {what}: {e}"))
}

/// The tests in a JUnit XML file, or the tool's output where there's
/// none.
fn junit_file(
    path: &Path,
    run: &Output,
    name: impl Fn(&str) -> String,
) -> Result<Vec<(String, Option<String>)>, String> {
    match std::fs::read_to_string(path) {
        Ok(xml) if xml.contains("<testsuite") => Ok(junit(&xml)
            .into_iter()
            .map(|(n, f)| (name(&n), f))
            .collect()),
        _ => Err(format!("no test results\n{}", tool_output(run))),
    }
}

fn tool_output(run: &Output) -> String {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    // The end says most, as a traceback's.
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(40)..].join("\n")
}

fn python(
    dir: &Path,
    library: &Tested,
    all: &[Tested],
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut path = vec![dir.join("temper-core"), dir.join("std")];
    path.extend(all.iter().map(|l| dir.join(&l.name)));
    let path = std::env::join_paths(path).map_err(|e| e.to_string())?;
    let here = dir.join(&library.name);
    let results = here.join("result.xml");
    let _ = std::fs::remove_file(&results);
    let run = output(
        Command::new(tool("PYTHON", "python3"))
            .args(["-B", "-m", "temper_core.testing", "result.xml"])
            .env("PYTHONPATH", path)
            .current_dir(&here),
        "python3",
    )?;
    junit_file(&results, &run, str::to_string)
}

/// Links the runtime, std, and every library into `node_modules`, by the
/// names imports use.
fn link_js(dir: &Path, all: &[Tested]) -> Result<(), String> {
    let modules = dir.join("node_modules");
    let mut links = vec![
        ("@temperlang/core".to_string(), dir.join("temper-core")),
        ("@temperlang/std".to_string(), dir.join("std")),
    ];
    links.extend(all.iter().map(|l| (l.name.clone(), dir.join(&l.name))));
    for (name, target) in links {
        let link = modules.join(&name);
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_dir_all(&link);
        let target = std::path::absolute(&target).unwrap_or(target);
        link_dir(&target, &link).map_err(|e| format!("{}: {e}", link.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

fn js(dir: &Path, library: &Tested) -> Result<Vec<(String, Option<String>)>, String> {
    let here = dir.join(&library.name);
    let results = std::path::absolute(here.join("test-results.xml")).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&results);
    let run = output(
        Command::new(tool("MOCHA", "mocha"))
            .args([
                "test",
                "--recursive",
                "--reporter",
                "xunit",
                "--reporter-option",
            ])
            .arg(format!("output={}", results.display()))
            .current_dir(&here),
        "mocha",
    )?;
    junit_file(&results, &run, str::to_string)
}

/// LuaUnit beside the libraries, as Kotlin copies it there for tests.
fn write_luaunit(dir: &Path) -> Result<(), String> {
    #[cfg(feature = "lua")]
    for (path, text) in temper_be_lua::runtime::LUAUNIT {
        let path = dir.join("luaunit").join(path);
        path.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, text))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let _ = dir;
    Ok(())
}

fn lua(dir: &Path, library: &Tested) -> Result<Vec<(String, Option<String>)>, String> {
    let results = dir.join("test-results.xml");
    let _ = std::fs::remove_file(&results);
    let test = PathBuf::from(&library.name).join("tests/init-test.lua");
    let run = output(
        Command::new(tool("LUA", "lua"))
            .args(["-e", "TEMPER_LUA_DEBUG_FUNCS=false"])
            .arg(&test)
            .args(["-o", "junit", "-n", "test-results.xml"])
            .env("LUA_PATH", "?.lua;?/init.lua")
            .current_dir(dir),
        "lua",
    )?;
    // LuaUnit's names, `Test_.test_name__12`, as the tests' functions'.
    junit_file(&results, &run, |n| {
        let n = n.strip_prefix("Test_.test_").unwrap_or(n);
        let n = match n.rsplit_once("__") {
            Some((n, id)) if id.chars().all(|c| c.is_ascii_digit()) => n,
            _ => n,
        };
        n.to_string()
    })
}

fn rust(dir: &Path, library: &Tested) -> Result<Vec<(String, Option<String>)>, String> {
    let target = std::path::absolute(dir.join("target")).map_err(|e| e.to_string())?;
    let run = output(
        Command::new(tool("CARGO", "cargo"))
            .args(["test", "--offline"])
            .env("CARGO_TARGET_DIR", target)
            .current_dir(dir.join(&library.name)),
        "cargo",
    )?;
    let stdout = String::from_utf8_lossy(&run.stdout);
    let mut results = Vec::new();
    for line in stdout.lines() {
        // `test tests::name ... ok`, or `... FAILED`.
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        let Some((name, result)) = rest.rsplit_once(" ... ") else {
            continue;
        };
        let short = name.rsplit("::").next().unwrap_or(name).to_string();
        match result {
            "ok" => results.push((short, None)),
            "FAILED" => results.push((short, Some(rust_failure(&stdout, name)))),
            _ => {}
        }
    }
    if results.is_empty() {
        return Err(format!("no test results\n{}", tool_output(&run)));
    }
    Ok(results)
}

/// Why a Rust test failed, from its captured output: the message of the
/// error it returned (`Error: Error(Some(MessageError("expected …")))`),
/// or its panic's, else just that it failed.
fn rust_failure(stdout: &str, name: &str) -> String {
    let header = format!("---- {name} stdout ----\n");
    let Some(at) = stdout.find(&header) else {
        return "failed".to_string();
    };
    let section = &stdout[at + header.len()..];
    let section = &section[..section.find("\n\n").unwrap_or(section.len())];
    if let Some((_, rest)) = section.split_once("MessageError(\"") {
        if let Some((message, _)) = rest.split_once("\")") {
            return message.replace("\\\"", "\"");
        }
    }
    let mut lines = section.lines();
    while let Some(line) = lines.next() {
        if line.contains(" panicked at ") {
            if let Some(message) = lines.next() {
                return message.to_string();
            }
        }
    }
    section.lines().next().unwrap_or("failed").to_string()
}

fn csharp(dir: &Path, library: &Tested) -> Result<Vec<(String, Option<String>)>, String> {
    let here = dir.join(&library.name).join("tests");
    let results = here.join("TestResults/TestResults.xml");
    let _ = std::fs::remove_file(&results);
    let run = output(
        Command::new(tool("DOTNET", "dotnet"))
            .args([
                "test",
                "--logger:junit",
                "-p:NuGetAudit=false",
                "-p:WarningLevel=0",
            ])
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("DOTNET_NOLOGO", "1")
            .current_dir(&here),
        "dotnet",
    );
    // No build server left running once the tests are done.
    let _ = Command::new(tool("DOTNET", "dotnet"))
        .args(["build-server", "shutdown"])
        .output();
    junit_file(&results, &run?, str::to_string)
}

/// Builds a library's tests with CMake, two jobs at once, and runs them,
/// reading the JUnit XML they write.
fn cpp(dir: &Path, library: &Tested) -> Result<Vec<(String, Option<String>)>, String> {
    let here = dir.join(&library.name);
    let build = here.join("build");
    let cmake = tool("CMAKE", "cmake");
    let configure = output(
        Command::new(&cmake)
            .args(["-S", ".", "-B", "build", "-DCMAKE_BUILD_TYPE=Debug"])
            .current_dir(&here),
        "cmake",
    )?;
    if !configure.status.success() {
        return Err(format!("cmake failed\n{}", tool_output(&configure)));
    }
    let target = format!("{}-test", library.name);
    let made = output(
        Command::new(&cmake)
            .args(["--build", "build", "--target", &target, "-j", "2"])
            .current_dir(&here),
        "cmake --build",
    )?;
    if !made.status.success() {
        return Err(format!("the build failed\n{}", tool_output(&made)));
    }
    let results = here.join("test-results.xml");
    let _ = std::fs::remove_file(&results);
    let run = output(
        Command::new(build.join(&target)).current_dir(&here),
        "the tests",
    )?;
    junit_file(&results, &run, str::to_string)
}

/// JUnit 5's jars, as Kotlin's `pom.xml` names them, in Maven's local
/// repository (`~/.m2/repository`).
const JUNIT: [&str; 7] = [
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
const RUN_TESTS: &str = r#"package tempest;

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
fn java(
    dir: &Path,
    library: &Tested,
    all: &[Tested],
) -> Result<Vec<(String, Option<String>)>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let jars: Vec<PathBuf> = (JUNIT.iter())
        .map(|j| home.join(".m2/repository").join(j))
        .collect();
    if let Some(missing) = jars.iter().find(|j| !j.exists()) {
        return Err(format!(
            "no {} (JUnit, as Kotlin's pom.xml names it)",
            missing.display()
        ));
    }
    let out = dir.join(&library.name).join("target/tempest");
    let _ = std::fs::remove_dir_all(&out);
    let (main, tests) = (out.join("classes"), out.join("test-classes"));
    let mut sources = Vec::new();
    let roots = ["temper-core", "std"].into_iter().map(String::from);
    for root in roots.chain(all.iter().map(|l| l.name.clone())) {
        java_files(&dir.join(root).join("src/main/java"), &mut sources);
    }
    let runner = out.join("tempest/RunTests.java");
    let written = runner
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&runner, RUN_TESTS));
    written.map_err(|e| format!("{}: {e}", runner.display()))?;
    sources.push(runner);
    let classpath = std::env::join_paths(&jars).map_err(|e| e.to_string())?;
    let javac = |into: &Path, classpath: &std::ffi::OsStr, files: &[PathBuf]| {
        output(
            Command::new(tool("JAVAC", "javac"))
                .args(["-encoding", "UTF-8", "-nowarn", "-d"])
                .arg(into)
                .arg("-cp")
                .arg(classpath)
                .args(files),
            "javac",
        )
    };
    let built = javac(&main, &classpath, &sources)?;
    if !built.status.success() {
        return Err(format!("javac failed\n{}", tool_output(&built)));
    }
    let mut test_sources = Vec::new();
    java_files(
        &dir.join(&library.name).join("src/test/java"),
        &mut test_sources,
    );
    let with_main = std::env::join_paths(std::iter::once(main.clone()).chain(jars.iter().cloned()))
        .map_err(|e| e.to_string())?;
    let built = javac(&tests, &with_main, &test_sources)?;
    if !built.status.success() {
        return Err(format!("javac failed\n{}", tool_output(&built)));
    }
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
    let stdout = String::from_utf8_lossy(&run.stdout);
    let results: Vec<(String, Option<String>)> = (stdout.lines())
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
        .collect();
    if results.is_empty() {
        return Err(format!("no test results\n{}", tool_output(&run)));
    }
    Ok(results)
}

/// The `.java` files under a directory, sorted.
fn java_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            java_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "java") {
            out.push(path);
        }
    }
}

/// Each test case in JUnit XML, by name, with its failure's or error's
/// message or text if it failed.
fn junit(xml: &str) -> Vec<(String, Option<String>)> {
    let mut found = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<testcase") {
        rest = &rest[at + "<testcase".len()..];
        let end = rest.find('>').unwrap_or(rest.len());
        let (attrs, closed) = match rest[..end].strip_suffix('/') {
            Some(attrs) => (attrs, true),
            None => (&rest[..end], false),
        };
        let name = attr(attrs, "name").unwrap_or_default();
        let body = if closed {
            ""
        } else {
            &rest[end..rest.find("</testcase>").unwrap_or(rest.len())]
        };
        let failed = ["<failure", "<error"].iter().find_map(|tag| {
            let at = body.find(tag)?;
            let element = &body[at..];
            let open = element.find('>').unwrap_or(element.len());
            let message = attr(&element[..open], "message").filter(|m| !m.is_empty());
            // LuaUnit's is its `type`, after where it failed:
            // `a/tests/init-test.lua:19: expected …`.
            let message = message.or_else(|| {
                let kind = attr(&element[..open], "type").filter(|k| !k.is_empty())?;
                let said = (kind.split_once(".lua:"))
                    .and_then(|(_, rest)| rest.split_once(": "))
                    .map_or(kind.as_str(), |(_, said)| said);
                Some(said.to_string())
            });
            Some(message.unwrap_or_else(|| {
                let text = element[(open + 1).min(element.len())..].trim_start();
                // CDATA as it is, else up to the next tag, unescaped.
                if let Some(data) = text.strip_prefix("<![CDATA[") {
                    return data[..data.find("]]>").unwrap_or(data.len())]
                        .trim()
                        .to_string();
                }
                let text = &text[..text.find("</").unwrap_or(text.len())];
                unescape(text.trim())
            }))
        });
        found.push((name, failed));
    }
    found
}

/// An attribute's value, unescaped.
fn attr(attrs: &str, name: &str) -> Option<String> {
    let mut rest = attrs;
    loop {
        let at = rest.find(name)?;
        let before = rest[..at].chars().next_back();
        rest = &rest[at + name.len()..];
        let after = rest.trim_start();
        if before.is_some_and(char::is_whitespace) && after.starts_with('=') {
            let value = after[1..].trim_start();
            let quote = value.chars().next()?;
            let value = &value[1..];
            return Some(unescape(&value[..value.find(quote)?]));
        }
    }
}

/// Text with XML's entities, named and numeric, as their characters.
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let entity = rest[1..].split_once(';').map(|(e, _)| e).unwrap_or("");
        let c = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => match entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
            {
                Some(hex) => u32::from_str_radix(hex, 16).ok().and_then(char::from_u32),
                None => (entity.strip_prefix('#'))
                    .and_then(|d| d.parse().ok())
                    .and_then(char::from_u32),
            },
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[entity.len() + 2..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{junit, rust_failure};

    #[test]
    fn reads_junit() {
        let xml = r#"<testsuite tests="3">
  <testcase name="a &amp; b" time="0"/>
  <testcase name="c"><failure message="expected 1 == (2) not (1)">trace</failure></testcase>
  <testcase name="d"><error>Traceback &lt;here&gt; &#x27;x&#39;</error></testcase>
</testsuite>"#;
        assert_eq!(
            junit(xml),
            [
                ("a & b".to_string(), None),
                (
                    "c".to_string(),
                    Some("expected 1 == (2) not (1)".to_string())
                ),
                ("d".to_string(), Some("Traceback <here> 'x'".to_string())),
            ]
        );
    }

    /// A Rust test's failure, from what `cargo test` prints.
    #[test]
    fn reads_rust_failures() {
        let out = "test r#mod::tests::b ... FAILED\n\nfailures:\n\n\
                   ---- r#mod::tests::b stdout ----\n\
                   Error: Error(Some(MessageError(\"expected 1 + 1 == (3) not (2)\")))\n\n";
        assert_eq!(
            rust_failure(out, "r#mod::tests::b"),
            "expected 1 + 1 == (3) not (2)"
        );
        assert_eq!(rust_failure(out, "r#mod::tests::c"), "failed");
    }

    /// LuaUnit's failure says why in its `type`, after where; another's
    /// text may be CDATA, taken as it is.
    #[test]
    fn reads_luaunit_and_cdata() {
        let xml = r#"<testsuite tests="2">
  <testcase name="Test_.test_b">
    <failure type="a/tests/init-test.lua:19: expected 1 + 1 == (3) not (2)">
      <![CDATA[stack traceback:
	a/tests/init-test.lua:19]]></failure>
  </testcase>
  <testcase name="e"><failure><![CDATA[x < y]]></failure></testcase>
</testsuite>"#;
        assert_eq!(
            junit(xml),
            [
                (
                    "Test_.test_b".to_string(),
                    Some("expected 1 + 1 == (3) not (2)".to_string())
                ),
                ("e".to_string(), Some("x < y".to_string())),
            ]
        );
    }
}
