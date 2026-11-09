//! How C# runs built libraries' tests, for `tempest test -b csharp`, as
//! Kotlin's `RunCSharp`: `dotnet test --logger:junit` in each library's
//! `tests` project (`DOTNET`), its packages restored from NuGet's cache
//! or feed, reading `TestResults/TestResults.xml`; then `dotnet
//! build-server shutdown`, so nothing stays running. `tempest run -b
//! csharp` builds the library's `program` project quietly, then runs it
//! with `dotnet run --no-build`, as Kotlin's `dotnet run` does.

use std::path::Path;
use std::process::Command;
use temper_tmpl::tools::{junit_file, output, program, step, tool};
use temper_tmpl::{TestResults, Tested, Tools};

pub const TOOLS: Tools = Tools {
    prepare: None,
    test,
    run,
};

fn run(dir: &Path, library: &Tested, _: &[Tested]) -> Result<bool, String> {
    let here = dir.join(&library.name).join("program");
    let dotnet = || {
        let mut command = Command::new(tool("DOTNET", "dotnet"));
        command
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("DOTNET_NOLOGO", "1")
            .current_dir(&here);
        command
    };
    let built = step(
        dotnet().args([
            "build",
            "-p:NuGetAudit=false",
            "-p:WarningLevel=0",
            "-p:CheckEolTargetFramework=false",
        ]),
        "dotnet build",
    );
    let ran = built.and_then(|()| program(dotnet().args(["run", "--no-build"]), "dotnet run"));
    // No build server left running once the program is done.
    let _ = Command::new(tool("DOTNET", "dotnet"))
        .args(["build-server", "shutdown"])
        .output();
    ran
}

fn test(dir: &Path, library: &Tested, _: &[Tested]) -> TestResults {
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
