//! What `tempest help` prints: the commands, each with a line, as Kotlin
//! Temper's `temper help` lists them, and each command's options, with
//! Kotlin's names, short forms, and defaults.

/// Each command, alphabetically, with its line in `tempest help`, and
/// whether tempest has it yet.
pub const COMMANDS: [(&str, &str, bool); 13] = [
    (
        "build",
        "Brings Temper libraries under the work root up to date.",
        true,
    ),
    (
        "doc",
        "Generates documentation for Temper libraries (not in tempest yet).",
        false,
    ),
    ("help", "Prints help text for a command and exits.", true),
    (
        "init",
        "Generates initial contents for a Temper project.",
        true,
    ),
    (
        "license",
        "Displays the copyright and license attributions for the Temper toolchain and documentation.",
        true,
    ),
    (
        "licenses",
        "Displays dependencies and licenses bundled with them.",
        true,
    ),
    (
        "lsp",
        "Starts the language server over standard input and output (as serve).",
        true,
    ),
    (
        "repl",
        "Interactive shell for Temper code (not in tempest yet).",
        false,
    ),
    (
        "run",
        "Loads the named Temper-built library after processing libraries in the work root.",
        true,
    ),
    (
        "serve",
        "Starts the Temper language server, expecting to be called from a language client.",
        true,
    ),
    ("test", "Runs the tests with the configured backends.", true),
    ("version", "Reports tempest's version.", true),
    (
        "watch",
        "Like build, but keeps running, rebuilding as files change (not in tempest yet).",
        false,
    ),
];

/// `tempest help`, without a command.
pub fn overview() -> String {
    let mut out = String::from(
        "tempest, Temper's command line tool, in Rust.\n\
         \n\
         usage: tempest COMMAND [OPTION]...\n\
         \n\
         Commands:\n",
    );
    for (name, line, _) in COMMANDS {
        out.push_str(&format!("    {name} - {line}\n"));
    }
    out.push_str(
        "\n`tempest help COMMAND` or `tempest COMMAND --help` shows a command's options.\n",
    );
    out
}

const WORKROOT: &str = "    -w, --workroot DIR
            Root directory to scan for Temper libraries; each
            config.temper.md under it starts one. By default, found by
            searching up from the current directory for the nearest
            directory holding a temper.out directory, or else the
            farthest holding a config.temper.md.
";

const IGNOREFILE: &str = "    -ign, --ignorefile FILE
            A file like .gitignore whose patterns, relative to its own
            directory, say what isn't a source file, with the last word
            over the .gitignore and .temperignore files that tempest
            always reads.
";

const BACKEND: &str = "    -b, --backend BACKEND
            A backend; may be given more than once: cpp, csharp, java,
            js, lua, py, or rust.
";

const NO_CONNECTED: &str = "    --no-connected
            Use @connected functions' Temper bodies, where they have
            one, instead of connected code.
";

const VERBOSE: &str = "    -v, --verbose
            Say more, including hints: advice about correct code, such
            as an unnecessary type check, which editors show anyway.
";

const LOGFORMAT: &str = "    --logformat text|json
            text (the default) for people; json for one JSON record
            per line, as Kotlin's.
";

const TIMEOUT: &str = "    --timeout S|none
            On a backend, the seconds each command its tools start (a
            tests file, a build) may take, killed with what it started
            if it runs over; as TEMPEST_TIMEOUT. none for no limit.
";

const STD: &str = "    --std DIR
            Read std from DIR instead of the copy built in.
";

use temper_frontend::interp::TEST_FUEL;

/// A command's help, or `None` for a command there isn't.
pub fn command(name: &str) -> Option<String> {
    let text = match name {
        "build" => format!(
            "usage: tempest build [OPTION]...\n\
             \n\
             Brings Temper libraries under the work root up to date, writing\n\
             temper.out/BACKEND/LIBRARY for each backend.\n\
             \n\
             Options:\n\
             {WORKROOT}{IGNOREFILE}\
             \x20   -o, --output-directory DIR\n\
             \x20           Write each backend's directory under DIR instead of the\n\
             \x20           work root's temper.out.\n\
             {BACKEND}\
             \x20           Defaults to csharp, java, js, lua, py, and rust.\n\
             {NO_CONNECTED}{VERBOSE}{LOGFORMAT}{STD}"
        ),
        "test" => format!(
            "usage: tempest test [OPTION]... [DIR]\n\
             \n\
             Runs the libraries' tests on each backend given, in turn, or on the\n\
             interpreter. DIR, if given, is where the work root's search starts.\n\
             \n\
             Options:\n\
             {WORKROOT}{IGNOREFILE}\
             \x20   --library NAME\n\
             \x20           Only this library's tests, by its dashed name, as foo-bar.\n\
             \x20   -b, --backend BACKEND\n\
             \x20           A backend, or interp for the interpreter; may be given\n\
             \x20           more than once. Defaults to interp.\n\
             \x20   --fuel N|none\n\
             \x20           On the interpreter, the steps each test may take, and the\n\
             \x20           top-level code before them (default {TEST_FUEL}); a test\n\
             \x20           that runs out fails alone. none for no limit.\n\
             {TIMEOUT}\
             \x20           Default 600.\n\
             {NO_CONNECTED}{VERBOSE}{LOGFORMAT}{STD}"
        ),
        "run" => format!(
            "usage: tempest run --library NAME -b BACKEND [OPTION]...\n\
             \n\
             Builds the work root's libraries for the backend, then runs the named\n\
             library's top level with the backend's language, or on the interpreter\n\
             with -b interp.\n\
             \n\
             Options:\n\
             \x20   --library NAME\n\
             \x20           The library to run, by its dashed name (required).\n\
             \x20   -b, --backend BACKEND\n\
             \x20           cpp, csharp, java, js, lua, py, rust, or interp (required).\n\
             {WORKROOT}{IGNOREFILE}\
             \x20   -o, --output-directory DIR\n\
             \x20           As for build.\n\
             {TIMEOUT}\
             \x20           The program itself has none by default; each step that\n\
             \x20           builds it, 600.\n\
             {NO_CONNECTED}{VERBOSE}{LOGFORMAT}{STD}"
        ),
        "init" => "usage: tempest init [-o DIR]\n\
             \n\
             Generates a new project: DIR/src/config.temper.md and a first source\n\
             file, DIR/src/NAME.temper.md, where NAME is DIR's name, dashed.\n\
             \n\
             Options:\n\
             \x20   -o, --output-directory DIR\n\
             \x20           New or empty directory in which to create the project;\n\
             \x20           defaults to the current directory.\n"
            .to_string(),
        "serve" | "lsp" => format!(
            "usage: tempest {name} [--std DIR]\n\
             \n\
             Starts the Temper language server, talking to its client over standard\n\
             input and output. `serve` and `lsp` are the same.\n\
             \n\
             Options:\n\
             \x20   --port N\n\
             \x20           Kotlin's: a TCP port to serve on. tempest serves over\n\
             \x20           standard input and output only, so it says so and stops.\n\
             \x20   --tokens add|full|none\n\
             \x20           Kotlin's semantic tokens; tempest sends none yet, so all\n\
             \x20           three are as none.\n\
             {STD}"
        ),
        "help" => "usage: tempest help [COMMAND]\n\
             \n\
             Prints the commands, or a command's options.\n"
            .to_string(),
        "version" => "usage: tempest version\n\nReports tempest's version.\n".to_string(),
        "license" => "usage: tempest license\n\n\
             Displays the copyright and license attributions for the Temper\n\
             toolchain and documentation.\n"
            .to_string(),
        "licenses" => "usage: tempest licenses\n\n\
             Displays the third-party crates built into tempest and their licenses.\n"
            .to_string(),
        "watch" | "repl" | "doc" => format!("{}\n", crate::not_yet_command(name)),
        _ => return None,
    };
    Some(text)
}
