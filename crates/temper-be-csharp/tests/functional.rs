//! The C# backend on Kotlin Temper's functional tests that tmpl's
//! translation covers so far, those with classes and interfaces too
//! (`temper_be_testkit::CLASSES`), from the vendored copy
//! (`temper-funtests`), and on the shared programs
//! (`crates/temper-be-testkit/programs/`), which cover operators,
//! jumps, module state, string order, several modules, imports, tests,
//! and classes beyond those tests.
//!
//! - `goldens`: each program's generated library, laid out as Kotlin's,
//!   with its `ROOTGlobal.cs` matching the golden in `tests/goldens/`,
//!   named for the test's path (`algos-fibonacci.cs`), and for a library
//!   of more than one module, with classes, or with tests, its other files
//!   theirs, named
//!   for the path and the file's place (`modules.Helpers.HelpersGlobal.cs`,
//!   `testing.tests.TestingTests.cs`). To accept new or changed output,
//!   run with `UPDATE_SNAPSHOTS=1` and review the diff.
//! - `runs`: without MSBuild or NuGet, Roslyn's `csc` (from the SDK that
//!   `dotnet` finds, or `DOTNET`) compiles the vendored `TemperLang.Core`
//!   (`vendor/temper/be-csharp/temper-core`) once, for .NET 6 as its
//!   `.csproj` says, into `CARGO_TARGET_TMPDIR`, reused until its sources
//!   or the SDK change; then every generated library's sources in one more
//!   `csc` run, with nullable reference types on, every warning, and every
//!   .NET analyzer an error, but for those [`ALLOWED`] lists; then one
//!   generated `Program.cs`, as its own program, and a runner of the
//!   others, which runs each library's static constructor in turn, and
//!   `module-state` from 8 threads, under a culture (`sv-SE`) whose minus
//!   sign isn't `-`, so formatting by the current culture shows. Each logs
//!   what the test expects of Kotlin's `csharp` backend. The runtime's
//!   one dependency, `Microsoft.Extensions.Logging` 8.0.0, comes from
//!   NuGet's cache (`~/.nuget/packages`, or `NUGET_PACKAGES`), never the
//!   network; `dotnet restore` of the runtime's project puts it there.
//!   Libraries that import others compile with them, in the same
//!   assembly. Tests (`tests/`) aren't compiled: they need std's
//!   `testing`, which tmpl doesn't translate yet.
//!   Skipped, with a message, without `dotnet`, the .NET 6 targeting pack,
//!   or those packages; with `TEMPEST_REQUIRE_CHECKERS` set, as in CI,
//!   any of them missing fails it instead.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_be_csharp::{CSHARP, LOGGING_TEMPLATE, OutFile, names, translate};
use temper_be_testkit::{self as kit, Case, Goldens};

/// A library of names that would hide the classes the translator names,
/// for C# alone: `math` is `Math`, `core` `Core`, and `support`
/// `Support`; and a parameter `object`, a C# keyword, is `@object`.
/// Also `Boolean`'s `<`, which C# lacks (Kotlin writes it anyway), and
/// `global::` in an interpolated string's hole, which takes parentheses. (A
/// library's root namespace hides classes too, the translator's and
/// other libraries' alike, but only for the library's own code and its
/// dependents, which are separate assemblies; this test compiles every
/// library into one, so it leaves that out.)
const HIDING: &str = "# Hiding

    export let support = \"s\";
    let math(a: Int, b: Int): Int { a.min(b) }
    let twice(object: Int): Int { object + object }
    let core(a: Int, b: Int): Int throws Bubble { a / b }
    let before(a: Boolean, b: Boolean): Boolean { a < b }
    console.log(\"${math(3, 2).toString()} ${twice(2).toString()} ${core(7, 2).toString()} ${support}\");
    console.log(before(false, true).toString());
    console.log(\"${3.min(2).toString()} ${4.max(1).toString()}\");
";

/// Classes for C# alone: a call through an interface reaches the
/// member Temper finds on the class, the nearest, not the interface's
/// own default that C# would call, which `C`'s `a` hides; and an `if`
/// inside an `if` that has an `else`, which Kotlin's C# printed without
/// braces, so the `else` went with the inner `if`.
const DISPATCH: &str = "# Dispatch

    interface A {
      a(): String { \"A wins!\" }
      get b(): String { \"A's b\" }
    }
    interface C extends A {
      a(): String { \"C wins!\" }
      get b(): String { \"C's b\" }
    }
    class X extends C {}
    let v: A = new X();
    console.log(\"${v.a()} ${v.b}\");
    let pick(a: Boolean, b: Boolean): String {
      if (a) {
        if (b) { return \"ab\"; }
      } else {
        return \"not a\";
      }
      \"a only\"
    }
    console.log(\"${pick(true, true)} ${pick(true, false)} ${pick(false, true)}\");
";

/// The .NET analyzers' rules allowed in generated code, each with why;
/// every other rule is an error (`AnalysisMode` `All`, as warnings, and
/// warnings as errors), in the libraries' sources, Kotlin's `Logging.cs`
/// with them. Only these are allowed so far, ones that Kotlin's public
/// surface, which the first variant keeps, breaks:
///
/// - CA1822, members that don't use `this` could be static: a Temper
///   method is an instance method, whatever its body reads.
/// - CA2211, non-constant fields visible: an exported `var` is a public
///   static field in Kotlin's surface (`Total`), not a property.
///
/// Others that Kotlin's surface will break, once the translation covers
/// what does it: CA1707, no underscores in names (Kotlin's Pascal case
/// gives `A98_Rgb`), and CA1716, names that are other languages'
/// keywords (Kotlin keeps the source's, so a function `next` is `Next`,
/// which VB reserves). `Program.cs`, Kotlin's template, and the runner
/// are compiled without analyzers: `Program` is in the global namespace
/// (CA1050).
const ALLOWED: &[&str] = &["CA1822", "CA2211"];

fn cases() -> Vec<Case> {
    let mut cases = kit::cases_with("csharp", &kit::PROGRAMS, &kit::CLASSES);
    cases.push(Case::inline(
        "hiding",
        "hiding.temper.md",
        HIDING,
        vec!["2 4 3 s".into(), "true".into(), "2 4".into()],
    ));
    cases.push(Case::inline(
        "dispatch",
        "dispatch.temper.md",
        DISPATCH,
        vec!["C wins! C's b".into(), "ab a only not a".into()],
    ));
    cases
}

/// A case's generated files.
fn generate(case: &Case) -> Vec<OutFile> {
    translate(&kit::program(case), &CSHARP).unwrap_or_else(|e| panic!("{}: {e}", case.key))
}

fn vendored() -> PathBuf {
    kit::vendored("be-csharp")
}

#[test]
fn goldens() {
    kit::with_stack(|| {
        let mut goldens = Goldens::new(env!("CARGO_MANIFEST_DIR"));
        let template = |name: &str| {
            let text = fs::read_to_string(vendored().join("library-template").join(name)).unwrap();
            text.trim_start_matches('\u{feff}').to_string()
        };
        let program_template = template("Program.cs");
        for case in cases() {
            let files = generate(&case);
            let library = &case.library;
            let root = names::root_namespace(library);
            let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
            let at = |path: String| {
                (paths.iter().position(|p| *p == path))
                    .unwrap_or_else(|| panic!("{}: no {path} in {paths:?}", case.key))
            };
            let order = [
                at(format!("{library}/src/{root}.csproj")),
                at(format!("{library}/src/{root}Global.cs")),
                at(format!("{library}/src/Logging.cs")),
                at(format!("{library}/program/{root}Program.csproj")),
                at(format!("{library}/program/Program.cs")),
            ];
            let single = files.len() == 5;
            if single {
                assert_eq!(order, [0, 1, 2, 3, 4], "{}", case.key);
            }
            let files_all = files.clone();
            let files: Vec<OutFile> = order.iter().map(|&i| files_all[i].clone()).collect();
            for part in [
                "<TargetFramework>net6.0</TargetFramework>".to_string(),
                format!("<RootNamespace>{root}</RootNamespace>"),
                "<Nullable>enable</Nullable>".into(),
                format!("<InternalsVisibleTo Include=\"{root}Test\"/>"),
                "<ProjectReference Include=\"../../temper-core/TemperLang.Core.csproj\"/>".into(),
            ] {
                assert!(files[0].text.contains(&part), "{}: {part}", case.key);
            }
            assert!(
                files[3].text.contains(&format!(
                    "<OutputType>Exe</OutputType>\n    <RootNamespace>{root}</RootNamespace>"
                )) && files[3].text.contains(&format!(
                    "<ProjectReference Include=\"../src/{root}.csproj\"/>"
                )),
                "{}",
                files[3].text
            );
            // Kotlin's templates, with the library's names.
            let logging = template("Logging.cs")
                .replace("RootNamespaceSpot", &root)
                .replace("SupportNamespaceSpot", "Support");
            assert_eq!(files[2].text, logging, "{}", case.key);
            let main = program_template
                .replace("typeof(Program)", &format!("typeof({root}.{root}Global)"));
            if files[4].text.matches("RunClassConstructor").count() == 1 {
                assert_eq!(files[4].text, main, "{}", case.key);
            } else if single {
                goldens.check(&format!("{}.program.Program.cs", case.key), &files[4].text);
            }
            goldens.check(&format!("{}.cs", case.key), &files[1].text);
            // Other modules' files, tests, and a program that loads
            // several classes have goldens of their own.
            if !single {
                for (i, file) in files_all.iter().enumerate() {
                    if [order[1], order[2]].contains(&i) {
                        continue;
                    }
                    let rest = &file.path[library.len() + 1..];
                    let rest = rest.strip_prefix("src/").unwrap_or(rest);
                    goldens.check(
                        &format!("{}.{}", case.key, rest.replace('/', ".")),
                        &file.text,
                    );
                }
            }
        }
        goldens.finish();
    });
}

#[test]
fn logging_template_is_kotlins() {
    let vendored = fs::read_to_string(vendored().join("library-template/Logging.cs")).unwrap();
    assert_eq!(LOGGING_TEMPLATE, vendored.trim_start_matches('\u{feff}'));
}

/// What compiling and running C# needs, from the .NET SDK and NuGet's
/// cache.
struct Dotnet {
    dotnet: String,
    /// `csc.dll`, Roslyn's compiler, run by `dotnet exec`.
    csc: PathBuf,
    /// The .NET analyzers that come with the SDK.
    analyzers: Vec<PathBuf>,
    /// .NET 6's reference assemblies, and `Microsoft.Extensions.Logging`'s
    /// and its dependencies' assemblies.
    references: Vec<PathBuf>,
    packages: Vec<PathBuf>,
    /// The SDK's version, for the runtime's cache key.
    version: String,
}

/// The packages the runtime depends on for .NET 6 (its `.csproj`'s
/// `Microsoft.Extensions.Logging` 8.0.0, and what that depends on), as
/// NuGet's cache keeps them.
const PACKAGES: &[(&str, &str)] = &[
    (
        "microsoft.extensions.dependencyinjection",
        "Microsoft.Extensions.DependencyInjection",
    ),
    (
        "microsoft.extensions.dependencyinjection.abstractions",
        "Microsoft.Extensions.DependencyInjection.Abstractions",
    ),
    (
        "microsoft.extensions.logging",
        "Microsoft.Extensions.Logging",
    ),
    (
        "microsoft.extensions.logging.abstractions",
        "Microsoft.Extensions.Logging.Abstractions",
    ),
    (
        "microsoft.extensions.options",
        "Microsoft.Extensions.Options",
    ),
    (
        "microsoft.extensions.primitives",
        "Microsoft.Extensions.Primitives",
    ),
];

/// The highest version directory under `dir` that starts with `prefix`.
fn newest(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut found: Vec<(Vec<u32>, PathBuf)> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| {
            let path = e.ok()?.path();
            let name = path.file_name()?.to_str()?.to_string();
            name.starts_with(prefix).then(|| {
                let key = name
                    .split(['.', '-'])
                    .map(|p| p.parse().unwrap_or(0))
                    .collect();
                (key, path)
            })
        })
        .collect();
    found.sort();
    found.pop().map(|(_, p)| p)
}

/// The SDK, .NET 6's targeting pack, and the runtime's packages, or what's
/// missing.
fn find_dotnet() -> Result<Dotnet, String> {
    let dotnet = env::var("DOTNET").unwrap_or_else(|_| "dotnet".into());
    let sdks = Command::new(&dotnet)
        .arg("--list-sdks")
        .output()
        .map_err(|_| "no dotnet (on PATH, or named by DOTNET)".to_string())?;
    let sdks = String::from_utf8_lossy(&sdks.stdout).into_owned();
    // `6.0.402 [/path/to/dotnet/sdk]`, oldest first. The newest 6.x, as
    // Kotlin's projects target .NET 6, so its analyzers are the ones the
    // allowances were made for; else the newest.
    let sdks: Vec<(String, PathBuf)> = sdks
        .lines()
        .filter_map(|l| {
            let (version, dir) = l.split_once(" [")?;
            Some((
                version.to_string(),
                PathBuf::from(dir.trim_end_matches(']')),
            ))
        })
        .collect();
    let (version, sdk) = (sdks.iter().rev())
        .find(|(v, _)| v.starts_with("6."))
        .or(sdks.last())
        .cloned()
        .ok_or("no .NET SDK")?;
    let root = sdk.parent().ok_or("no .NET root")?.to_path_buf();
    let sdk = sdk.join(&version);
    let csc = sdk.join("Roslyn/bincore/csc.dll");
    if !csc.is_file() {
        return Err(format!("no csc.dll in {}", sdk.display()));
    }
    let analyzer_dir = sdk.join("Sdks/Microsoft.NET.Sdk/analyzers");
    let analyzers = [
        "Microsoft.CodeAnalysis.NetAnalyzers.dll",
        "Microsoft.CodeAnalysis.CSharp.NetAnalyzers.dll",
    ]
    .iter()
    .map(|a| analyzer_dir.join(a))
    .collect::<Vec<_>>();
    if !analyzers.iter().all(|a| a.is_file()) {
        return Err(format!("no .NET analyzers in {}", analyzer_dir.display()));
    }
    let pack = newest(&root.join("packs/Microsoft.NETCore.App.Ref"), "6.")
        .ok_or("no .NET 6 targeting pack (Microsoft.NETCore.App.Ref 6.x)")?;
    let mut references = Vec::new();
    kit::files_with(&pack.join("ref/net6.0"), "dll", &mut references);
    let cache = env::var_os("NUGET_PACKAGES")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".nuget/packages")))
        .ok_or("no NuGet cache")?;
    let mut packages = Vec::new();
    for (id, assembly) in PACKAGES {
        let dll = cache
            .join(id)
            .join("8.0.0/lib/net6.0")
            .join(format!("{assembly}.dll"));
        if !dll.is_file() {
            return Err(format!(
                "no {id} 8.0.0 in NuGet's cache ({}); `dotnet restore` of \
                 vendor/temper/be-csharp/temper-core's project, copied elsewhere, puts it there",
                cache.display()
            ));
        }
        packages.push(dll);
    }
    Ok(Dotnet {
        dotnet,
        csc,
        analyzers,
        references,
        packages,
        version,
    })
}

/// The symbols MSBuild defines for a .NET 6 release build, which
/// `TemperLang.Core` and `Logging.cs` test (`TRACE` keeps `Trace`'s
/// calls, which logging falls back to).
const DEFINES: &str = "TRACE;RELEASE;NET;NET6_0;NETCOREAPP;NET5_0_OR_GREATER;NET6_0_OR_GREATER";

impl Dotnet {
    /// `csc` with the references and the options given.
    fn csc(&self, options: &[String], extra: &[&Path], sources: &[PathBuf]) -> Output {
        let mut command = Command::new(&self.dotnet);
        command
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("DOTNET_NOLOGO", "1")
            .arg("exec")
            .arg(&self.csc)
            .args([
                "-nologo",
                "-noconfig",
                "-nostdlib",
                "-langversion:10",
                "-deterministic",
                &format!("-define:{DEFINES}"),
            ])
            .args(options);
        for r in self.references.iter().chain(&self.packages) {
            command.arg(format!("-r:{}", r.display()));
        }
        for r in extra {
            command.arg(format!("-r:{}", r.display()));
        }
        command.args(sources).output().unwrap()
    }
}

/// The runtime's assembly, compiled once and kept under
/// `CARGO_TARGET_TMPDIR` until its sources or the SDK change: without
/// nullable annotations, as its `.csproj` has it, and without warnings,
/// since it isn't generated here.
fn runtime_assembly(dotnet: &Dotnet) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-csharp-runtime");
    let dll = root.join("TemperLang.Core.dll");
    let mut sources = Vec::new();
    kit::files_with(&vendored().join("temper-core"), "cs", &mut sources);
    let options = vec![
        "-target:library".to_string(),
        "-optimize+".into(),
        "-warn:0".into(),
        format!("-out:{}", dll.display()),
    ];
    kit::cached(&root, (&dotnet.version, &options), &sources, &dll, || {
        let build = dotnet.csc(&options, &[], &sources);
        assert!(
            build.status.success(),
            "csc, the runtime:\n{}",
            String::from_utf8_lossy(&build.stdout)
        );
    });
    dll
}

/// The analyzers' configuration for generated code: every rule a warning
/// (`AnalysisMode` `All`), but those [`ALLOWED`].
fn analyzer_config() -> String {
    let mut config =
        String::from("is_global = true\ndotnet_analyzer_diagnostic.severity = warning\n");
    for rule in ALLOWED {
        config.push_str(&format!("dotnet_diagnostic.{rule}.severity = none\n"));
    }
    config
}

/// The runner: runs each library named on its command line, by its
/// generated `Program.cs`'s `Main` (renamed, so they can share one
/// program), in a culture whose minus sign is U+2212; then, for
/// `threads`, `module-state`'s `Tick` and `Report` from 8 threads.
fn runner(mains: &[(String, String)]) -> String {
    let mut cases = String::new();
    for (key, class) in mains {
        cases.push_str(&format!(
            "                case \"{key}\": {class}.Run(); break;\n"
        ));
    }
    format!(
        r#"using System;
using System.Globalization;
using System.Threading;
using TemperLang.Core;

static class Runner
{{
    static void Main(string[] args)
    {{
        try
        {{
            CultureInfo culture = new CultureInfo("sv-SE");
            CultureInfo.DefaultThreadCurrentCulture = culture;
            CultureInfo.CurrentCulture = culture;
        }}
        catch (CultureNotFoundException)
        {{
            // Without ICU, .NET has only the invariant culture.
        }}
        switch (args[0])
        {{
{cases}                case "threads": Threads(); break;
        }}
    }}

    static void Threads()
    {{
        Core.InitSimpleLogging();
        Thread[] threads = new Thread[8];
        for (int i = 0; i < threads.Length; i++)
        {{
            threads[i] = new Thread(() =>
            {{
                for (int j = 0; j < 1000; j++)
                {{
                    ModuleState.ModuleStateGlobal.Tick();
                    ModuleState.ModuleStateGlobal.Report();
                }}
            }});
            threads[i].Start();
        }}
        foreach (Thread thread in threads)
        {{
            thread.Join();
        }}
        Console.WriteLine("ticks " + ModuleState.ModuleStateGlobal.Ticks.ToString(CultureInfo.InvariantCulture));
    }}
}}
"#
    )
}

/// A program's `runtimeconfig.json`: .NET 6, or a later one if that's
/// all there is.
const RUNTIME_CONFIG: &str = r#"{"runtimeOptions":{"tfm":"net6.0","rollForward":"Major","framework":{"name":"Microsoft.NETCore.App","version":"6.0.0"}}}
"#;

#[test]
fn runs() {
    let dotnet = match find_dotnet() {
        Ok(d) => d,
        Err(missing) => {
            kit::missing(&missing);
            return;
        }
    };
    kit::with_stack(move || {
        let runtime = runtime_assembly(&dotnet);
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("be-csharp");
        let _ = fs::remove_dir_all(&out);
        let bin = out.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let cases = cases();
        let mut sources = Vec::new();
        let mut mains = Vec::new();
        let mut first_program = None;
        for case in &cases {
            let root = out.join("src").join(&case.key);
            for file in generate(case) {
                let path = root.join(&file.path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                if file.path.ends_with("/program/Program.cs") {
                    // Each `Program.cs` declares `Program`; the runner
                    // calls each as its own class.
                    let class = format!("Program_{}", case.key.replace('-', "_"));
                    let text = file
                        .text
                        .replace("static class Program", &format!("static class {class}"))
                        .replace("static void Main()", "internal static void Run()");
                    let renamed = path.with_file_name(format!("{class}.cs"));
                    fs::write(&renamed, text).unwrap();
                    mains.push((case.key.clone(), class));
                    first_program.get_or_insert(path.clone());
                    fs::write(&path, &file.text).unwrap();
                    continue;
                }
                fs::write(&path, &file.text).unwrap();
                // Tests need std's `testing`, which tmpl doesn't translate
                // yet, so they aren't compiled.
                if file.path.ends_with(".cs") && !file.path.contains("/tests/") {
                    sources.push(path);
                }
            }
        }
        // Every library, strictly.
        let config = out.join("strict.globalconfig");
        fs::write(&config, analyzer_config()).unwrap();
        let generated = bin.join("Generated.dll");
        let mut options = vec![
            "-target:library".to_string(),
            "-nullable:enable".into(),
            "-warn:9999".into(),
            "-warnaserror+".into(),
            // MSBuild's own `NoWarn`: assembly version unification.
            "-nowarn:CS1701,CS1702".into(),
            format!("-analyzerconfig:{}", config.display()),
            format!("-out:{}", generated.display()),
        ];
        for a in &dotnet.analyzers {
            options.push(format!("-analyzer:{}", a.display()));
        }
        let build = dotnet.csc(&options, &[&runtime], &sources);
        assert!(
            build.status.success(),
            "csc, generated C# (nullable, every warning and analyzer an error):\n{}",
            String::from_utf8_lossy(&build.stdout)
        );
        // One `Program.cs` as it is, and the runner of the others.
        let first = first_program.unwrap();
        let program = bin.join("Program.dll");
        let options = [
            "-target:exe".to_string(),
            "-nullable:enable".into(),
            "-warnaserror+".into(),
            format!("-out:{}", program.display()),
        ];
        let build = dotnet.csc(&options, &[&runtime, &generated], &[first]);
        assert!(
            build.status.success(),
            "csc, Program.cs:\n{}",
            String::from_utf8_lossy(&build.stdout)
        );
        let mut runner_sources: Vec<PathBuf> = mains
            .iter()
            .map(|(key, class)| {
                let dir = out.join("src").join(key);
                let mut found = Vec::new();
                kit::files_with(&dir, "cs", &mut found);
                found
                    .into_iter()
                    .find(|p| p.file_stem().is_some_and(|s| *s == **class))
                    .unwrap()
            })
            .collect();
        let runner_path = out.join("Runner.cs");
        fs::write(&runner_path, runner(&mains)).unwrap();
        runner_sources.push(runner_path);
        let runner_dll = bin.join("Runner.dll");
        let options = [
            "-target:exe".to_string(),
            "-main:Runner".into(),
            "-nullable:enable".into(),
            format!("-out:{}", runner_dll.display()),
        ];
        let build = dotnet.csc(&options, &[&runtime, &generated], &runner_sources);
        assert!(
            build.status.success(),
            "csc, the runner:\n{}",
            String::from_utf8_lossy(&build.stdout)
        );
        // Beside the programs: their configuration, and the assemblies
        // they load.
        for name in ["Program", "Runner"] {
            fs::write(
                bin.join(format!("{name}.runtimeconfig.json")),
                RUNTIME_CONFIG,
            )
            .unwrap();
        }
        for dll in dotnet.packages.iter().chain([&runtime]) {
            fs::copy(dll, bin.join(dll.file_name().unwrap())).unwrap();
        }
        let mut failures = Vec::new();
        let mut run = |name: &str, dll: &Path, args: &[&str], expected: &[String]| {
            let out = Command::new(&dotnet.dotnet)
                .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
                .env("DOTNET_TieredPGO", "0")
                .arg(dll)
                .args(args)
                .output()
                .unwrap();
            kit::check_log(&mut failures, name, &out, expected);
        };
        run("Program.cs", &program, &[], &cases[0].expected);
        for case in &cases {
            run(&case.key, &runner_dll, &[&case.key], &case.expected);
        }
        // Init logs as `module-state` does, then the threads tick.
        let expected = kit::threads_expected(&cases);
        run("module-state-threads", &runner_dll, &["threads"], &expected);
        kit::assert_none(&failures);
    });
}
