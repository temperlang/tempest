//! Temper's C# backend: tmpl in, C# out.
//!
//! Works from tmpl alone (notes/tmpl/overview.md, "Boundaries"). Its
//! variants share the translation and differ only in data
//! (notes/tmpl/profiles.md, "Variants and the first profiles"); the first,
//! [`CSHARP`], is Kotlin-compatible: the same project layout (a `src`
//! project, `ROOT.csproj`, for the library, with its root module in the
//! root namespace's `ROOTGlobal` static class and `Logging.cs` in
//! `ROOT.Support`, and a `program` project to run it), `TemperLang.Core`
//! runtime, public names (Pascal case for functions and exported values),
//! and .NET 6, as Kotlin Temper's `csharp` backend, with prettier bodies:
//! no numbered names, the source's names for parameters and locals,
//! `using`s rather than one-letter namespace aliases, a file-scoped
//! namespace, and blank lines between members. Unlike Kotlin's, it never
//! leaves string or number formatting to the current culture (see
//! [`connected`]), and compiles without warnings with nullable reference
//! types on and every .NET analyzer an error but those its tests list.
//!
//! Parameter names are part of a C# API, since callers may pass arguments
//! by name, so they're exactly the source's (a keyword as `@name`), never
//! renamed for looks; only a clash with a module-level name a function
//! uses would number one. Temper means them to be positional but for
//! constructors' and factories' (notes/tmpl/surface.md, "Parameter names
//! where languages have named arguments"), which a Roslyn analyzer in the
//! generated NuGet package is to say; it isn't built yet.
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! C#'s rules ([`names::RULES`]), then [`translate`] into C#'s output
//! tree ([`tree`]), with operators spelled by [`ops`] and core's connected
//! declarations by [`connected`], then the printer.
//!
//! So far it covers tmpl's subset A, one module per library; anything
//! else is [`NotYet`]. No source maps yet.

pub mod connected;
pub mod names;
pub mod ops;
pub mod translate;
pub mod tree;

use std::fmt;
use temper_tmpl::naming::{NamingRules, assign};
use temper_tmpl::{Features, Profile, Program};

/// A named set of translation choices for C#.
pub struct Variant {
    /// Its name on the command line: `csharp`.
    pub name: &'static str,
    /// What it handles natively, which lowering and validation read.
    pub profile: Profile,
    pub rules: NamingRules,
    /// The `TemperLang.Core` version its output depends on, as a project
    /// beside the library's (`../../temper-core/TemperLang.Core.csproj`),
    /// as Kotlin's.
    pub runtime_version: &'static str,
    /// The .NET target framework, `<TargetFramework>` in each `.csproj`.
    pub target_framework: &'static str,
}

/// The first variant: Kotlin-compatible API and names, pretty bodies,
/// .NET 6 (C# 10). Everything is native except statements inside
/// expressions, which the hoisting pass will lower (not built yet: a
/// program that needs it is [`NotYet`]).
pub const CSHARP: Variant = Variant {
    name: "csharp",
    profile: Profile {
        native: Features::NONE,
    },
    rules: names::RULES,
    runtime_version: "0.6.0",
    target_framework: "net6.0",
};

/// Every variant, by name.
pub const VARIANTS: &[&Variant] = &[&CSHARP];

/// Something the translation doesn't cover yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub what: String,
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not yet in the C# backend: {}", self.what)
    }
}

impl std::error::Error for NotYet {}

/// A generated file: its path relative to the output directory, and its
/// text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub text: String,
}

/// Translates a library, laid out as Kotlin's C# backend lays it out:
///
/// ```text
/// hello-world/src/HelloWorld.csproj               the library's project, on TemperLang.Core
/// hello-world/src/HelloWorldGlobal.cs             its root module
/// hello-world/src/Logging.cs                      its logging setup, in HelloWorld.Support
/// hello-world/program/HelloWorldProgram.csproj    a program to run it
/// hello-world/program/Program.cs                  its entry point
/// ```
///
/// The root namespace takes the library's name in Pascal case
/// ([`names::root_namespace`]). Kotlin also copies `TemperLang.Core`
/// beside the library, as `temper-core/`, which the projects refer to;
/// this doesn't, yet.
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    let [module] = p.modules.as_slice() else {
        return Err(NotYet {
            what: format!("a library of {} modules", p.modules.len()),
        });
    };
    if !module.path.is_empty() {
        return Err(NotYet {
            what: "a module that isn't at the library's root".into(),
        });
    }
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    let library = &p.library.name;
    let root = names::root_namespace(library);
    let global = format!("{root}Global");
    let mut translator = translate::Translator::new(p, &names, &root, &global);
    let class = translator.module(module)?;
    Ok(vec![
        OutFile {
            path: format!("{library}/src/{root}.csproj"),
            text: library_project(&root, &p.library.version, variant),
        },
        OutFile {
            path: format!("{library}/src/{global}.cs"),
            text: tree::class(&class),
        },
        OutFile {
            path: format!("{library}/src/Logging.cs"),
            text: logging(&root),
        },
        OutFile {
            path: format!("{library}/program/{root}Program.csproj"),
            text: program_project(&root, &p.library.version, variant),
        },
        OutFile {
            path: format!("{library}/program/Program.cs"),
            text: program(&root, &global),
        },
    ])
}

/// A `<Version>` line, as Kotlin writes one only for a library that sets
/// its version; the frontend gives `0.0.0` to one that doesn't.
fn version_line(version: &str) -> String {
    if version == "0.0.0" {
        String::new()
    } else {
        format!("    <Version>{version}</Version>\n")
    }
}

/// The library's project, as Kotlin's `CsProj` writes it for a library
/// without tests or dependencies: its tests' project may see its
/// internals, and it refers to the runtime's project beside it, which
/// packing turns into a package reference.
fn library_project(root: &str, version: &str, variant: &Variant) -> String {
    let framework = variant.target_framework;
    let version = version_line(version);
    format!(
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>{framework}</TargetFramework>
    <RootNamespace>{root}</RootNamespace>
{version}    <IsPackable>true</IsPackable>
    <Nullable>enable</Nullable>
  </PropertyGroup>
  <ItemGroup>
    <InternalsVisibleTo Include="{root}Test"/>
  </ItemGroup>
  <ItemGroup>
    <ProjectReference Include="../../temper-core/TemperLang.Core.csproj"/>
  </ItemGroup>
</Project>
"#
    )
}

/// The program's project, as Kotlin's.
fn program_project(root: &str, version: &str, variant: &Variant) -> String {
    let framework = variant.target_framework;
    let version = version_line(version);
    format!(
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>{framework}</TargetFramework>
    <OutputType>Exe</OutputType>
    <RootNamespace>{root}</RootNamespace>
{version}    <IsPackable>true</IsPackable>
    <Nullable>enable</Nullable>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="../src/{root}.csproj"/>
  </ItemGroup>
</Project>
"#
    )
}

/// The entry point, Kotlin's template: simple logging to stdout, then the
/// module's class initialized, which runs its static constructor, then
/// any async tasks waited for.
fn program(root: &str, global: &str) -> String {
    format!(
        "using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{{
    static void Main()
    {{
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof({root}.{global}).TypeHandle);
        Async.WaitUntilSafeToExit();
    }}
}}
"
    )
}

/// Kotlin's `library-template/Logging.cs`, with the library's namespace
/// for `RootNamespaceSpot` and `Support` for `SupportNamespaceSpot`, as
/// Kotlin's, but without the template's byte order mark. Its public types
/// (`Logging.Init`, `ILoggingConsoleFactory`, `ILoggingConsole`) are how
/// a host sends a library's logs where it likes, so it's kept as it is.
pub const LOGGING_TEMPLATE: &str = r#"using System.Collections.Concurrent;
#if NET6_0_OR_GREATER
using Microsoft.Extensions.Logging;
#endif

namespace RootNamespaceSpot.SupportNamespaceSpot
{
    /// <summary>
    /// Only a single Init call of any kind can be made at most once.
    /// </summary>
    public static class Logging
    {
        public static void Init(ILoggingConsoleFactory factory)
        {
            LoggingConsoleFactory.Internal = new WrappedLoggingConsoleFactory(factory);
        }

#if NET6_0_OR_GREATER
        public static void Init(ILoggerFactory factory)
        {
            LoggingConsoleFactory.Internal = new TemperLang.Core.StandardLoggingConsoleFactory(
                factory
            );
        }
#endif

        internal static readonly TemperLang.Core.LazyInitLoggingConsoleFactory LoggingConsoleFactory =
            new TemperLang.Core.LazyInitLoggingConsoleFactory();
    }

    public interface ILoggingConsoleFactory
    {
        ILoggingConsole CreateConsole(string categoryName);
    }

    public interface ILoggingConsole
    {
        void Log(string message);
    }

    class WrappedLoggingConsoleFactory : TemperLang.Core.ILoggingConsoleFactory
    {
        readonly ILoggingConsoleFactory factory;
        readonly ConcurrentDictionary<string, WrappedLoggingConsole> consoles =
            new ConcurrentDictionary<string, WrappedLoggingConsole>();

        public WrappedLoggingConsoleFactory(ILoggingConsoleFactory factory)
        {
            this.factory = factory;
        }

        public TemperLang.Core.ILoggingConsole CreateConsole(string categoryName) =>
            // Cache because LoggerFactory also caches in similar circumstances.
            // Though we don't expect repeated calls on same categoryName.
            consoles.GetOrAdd(
                categoryName,
                (key) => new WrappedLoggingConsole(factory.CreateConsole(categoryName))
            );
    }

    class WrappedLoggingConsole : TemperLang.Core.ILoggingConsole
    {
        private ILoggingConsole console;

        public WrappedLoggingConsole(ILoggingConsole console)
        {
            this.console = console;
        }

        public void Log(string message)
        {
            console.Log(message);
        }
    }
}
"#;

/// The library's `Logging.cs`.
fn logging(root: &str) -> String {
    LOGGING_TEMPLATE
        .replace("RootNamespaceSpot", root)
        .replace("SupportNamespaceSpot", "Support")
}
