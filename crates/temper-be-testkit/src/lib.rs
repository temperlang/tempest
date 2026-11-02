//! What the backends' tests share: the programs they translate, how a
//! program becomes tmpl, goldens, the tools they run, and runtimes
//! cached between runs. Only the backends' tests use it, as a
//! dev-dependency; it knows no backend.
//!
//! - **Programs:** Kotlin Temper's functional tests that tmpl's
//!   translation covers ([`COVERED`]), from the vendored copy
//!   ([`temper_funtests`]), and the shared programs in this crate's
//!   `programs/` ([`PROGRAMS`]), which cover module state, operators,
//!   jumps, and string order beyond those tests, each a [`Case`] with the
//!   log a backend expects ([`cases`]). A backend adds programs of its
//!   own as more [`Case`]s.
//! - **To tmpl:** [`program`] settles a case's library and translates it,
//!   or reads it as tmpl, and validates it, then gives it the case's host
//!   files and whether it runs under `--no-connected` ([`CONNECTED`]).
//! - **Goldens:** [`Goldens`] compares output with the files in a
//!   backend's `tests/goldens/`, and with `UPDATE_SNAPSHOTS=1` writes new
//!   output there instead.
//! - **Tools:** [`tool`] finds one named by an environment variable or on
//!   `PATH`; a missing one skips its test with a message ([`missing`]),
//!   unless `TEMPEST_REQUIRE_CHECKERS` is set, as in CI, when it fails.
//! - **Runtimes:** [`cached`] builds something once under
//!   `CARGO_TARGET_TMPDIR`, keyed by a hash of its inputs, and keeps it
//!   until they change.
//! - **Runs:** [`check_log`] compares what a program logged with what's
//!   expected, and [`threads_expected`] is what `module-state` from 8
//!   threads logs.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use temper_frontend::interp;
use temper_frontend::library::Library;
use temper_frontend::project::{load_imports, source_kind};
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::to_tmpl;
use temper_tmpl::host::{Host, HostFile};
use temper_tmpl::{Profile, Program, text, validate};

/// The functional tests tmpl's translation covers, by their paths.
pub const COVERED: [&str; 7] = [
    "algos/fibonacci",
    "algos/hello-world",
    "control-flow/if-return",
    "control-flow/loop-reenterable",
    "names/nonascii",
    "types/int/basics",
    "types/string/is-empty",
];

/// The functional tests with classes and interfaces that tmpl's
/// translation covers (subset B's first step): each backend runs them as
/// it covers classes, with [`cases_with`].
pub const CLASSES: [&str; 14] = [
    "algos/hello-from-class-to-top",
    "algos/hello-world-object",
    "classes/call-override-from-subtype",
    "classes/direct-getter",
    "classes/inherited-getter",
    "classes/object-literals/extra-class",
    "classes/private-method",
    "classes/setters",
    "classes/static-properties",
    "classes/static-properties-scope",
    "classes/write-back-through-field",
    "imports/types",
    "interfaces/empty",
    "interfaces/pure-virtual",
];

/// The functional tests with nullable types, `orelse`, and `bubble()`
/// that tmpl's translation covers (subset B's third step): each backend
/// runs them as it covers nullables, with [`cases_with`].
pub const NULLABLES: [&str; 4] = [
    "control-flow/bubble",
    "semantics/mutually-referencing-types",
    "types/float/basics",
    "types/int/limits",
];

/// The functional tests with generic types and core's lists that tmpl's
/// translation covers (subset B's third step, generics), and one with a
/// cast from a nullable type: each backend runs them as it covers
/// generics, with [`cases_with`].
pub const GENERICS: [&str; 3] = ["casts/specific", "imports/values", "types/list/empty"];

/// The functional tests with function values that tmpl's translation
/// covers: function literals with their captures, references to
/// functions, calls of function values, and core's members that take
/// functions. Each backend runs them as it covers function values, with
/// [`cases_with`].
pub const FUNCTIONS: [&str; 6] = [
    "control-flow/for-of-folded",
    "functions/as-values",
    "functions/constructor-callbacks",
    "functions/simple-locals",
    "imports/functions",
    "types/list/sorting",
];

/// The functional tests with parameters' defaults that tmpl's translation
/// covers: optional parameters, typed nullable, whose functions' bodies
/// start with their defaults, and calls that leave them out. Each backend
/// runs them as it covers defaults, with [`cases_with`].
pub const DEFAULTS: [&str; 1] = ["classes/property-order"];

/// The functional tests with core's other types and members that tmpl's
/// translation covers: string indices and `StringIndexOption`,
/// `StringBuilder`, `Empty`, `Deque`, `DenseBitVector`, and more of
/// `String`'s and `Float64`'s members, as `split`, `countBetween`,
/// `near`, and `pi`. Each backend runs them as it covers them, with
/// [`cases_with`].
pub const CORE_TYPES: [&str; 9] = [
    "functions/defaulting",
    "functions/named-args",
    "regression/minimal-repro",
    "types/dense-bit-vector",
    "types/deque",
    "types/float/ops",
    "types/string/build",
    "types/string/indices",
    "types/string/read",
];

/// The functional tests of a library's own connected functions and
/// classes, with host code beside them for each backend: each backend
/// runs them as it reads host files, with [`cases_with`], which gives
/// each twice, as is and under `--no-connected` ([`Case::no_connected`]).
/// Their library is named `work`, as in Kotlin's functional tests, since
/// their host code names it (`package work;`).
pub const CONNECTED: [&str; 2] = ["functions/connected", "types/connected"];

/// The shared programs, in this crate's `programs/`: Temper
/// (`NAME.temper.md`, with what it logs in log blocks, as the functional
/// tests have), or tmpl that the frontend doesn't make yet (`NAME.tmpl`,
/// with the lines it logs in `// log:` comments).
///
/// A program in a directory of its own (`NAME/NAME.temper.md`) is a
/// library of several modules, its other modules in subdirectories, which
/// its main file imports; such libraries may import each other by name.
/// `modules` is three modules importing each other, and
/// `imports-library` imports `modules`. `testing` has tests, which need
/// std's `testing` to run, so backends check only their form for now.
/// `classes` has classes and interfaces, beyond what [`CLASSES`] covers.
/// `nullables` has nullable types, null checks, `??`, and `orelse` with
/// `bubble()` and `panic()`; `generics` generic functions, classes, and
/// interfaces, and core's lists, pairs, and maps; `functions` function
/// values: a closure that assigns what it captures, `@fun` interfaces
/// passed and returned, references to functions and static methods,
/// core's `map`, `filter`, `join`, `sorted`, and `forEach`, and a closure
/// in a field; `defaults` parameters' defaults, left out and given, on
/// functions, methods, and constructors, and called from another module
/// (two modules, so in a directory of its own).
pub const PROGRAMS: [&str; 12] = [
    "classes",
    "nullables",
    "generics",
    "functions",
    "defaults",
    "module-state",
    "operators",
    "string-order",
    "jumps.tmpl",
    "modules",
    "imports-library",
    "testing",
];

/// The shared programs that are libraries of several modules, in
/// directories of their own: what [`Case::libraries`] lists.
pub const LIBRARIES: [&str; 3] = ["modules", "imports-library", "defaults"];

/// The shared programs' directory.
pub fn programs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs")
}

/// A directory of the vendored copy of Kotlin Temper's sources, as
/// `be-py/temper-core`.
pub fn vendored(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/temper")
        .join(path)
}

/// A program to translate: its library's name, its sources and their
/// directory, and the log it expects.
#[derive(Clone, Debug)]
pub struct Case {
    /// Its golden's name: the test's path with `-` for `/`.
    pub key: String,
    pub library: String,
    /// Each source's path and text.
    pub sources: Vec<(String, String)>,
    /// The sources' directory, if they're files.
    pub dir: PathBuf,
    pub expected: Vec<String>,
    /// The other libraries it may import by name, each with its root.
    pub libraries: Vec<(String, PathBuf)>,
    /// Its host files, each a path relative to `dir` and its text.
    pub host_files: Vec<(String, String)>,
    /// Whether it runs under `--no-connected`, using Temper
    /// implementations rather than host code wherever there are any; its
    /// key then ends with `-no-connected`.
    pub no_connected: bool,
}

impl Case {
    /// A library of one source given as text, not a file, its name the
    /// key: `NAME.temper.md`, or `NAME.tmpl` read as tmpl.
    pub fn inline(name: &str, file: &str, text: impl Into<String>, expected: Vec<String>) -> Case {
        Case {
            key: name.to_string(),
            library: name.to_string(),
            sources: vec![(file.to_string(), text.into())],
            dir: PathBuf::new(),
            expected,
            libraries: Vec::new(),
            host_files: Vec::new(),
            no_connected: false,
        }
    }
}

/// The functional tests that tmpl covers, then those of the shared
/// programs named in `programs` (as [`PROGRAMS`] names them), each with
/// the log it expects of `backend` (`py`, `rust`, …).
pub fn cases(backend: &str, programs: &[&str]) -> Vec<Case> {
    cases_with(backend, programs, &[])
}

/// [`cases`], with more functional tests: `tests`, as [`CLASSES`].
pub fn cases_with(backend: &str, programs: &[&str], tests: &[&str]) -> Vec<Case> {
    let mut cases = Vec::new();
    let suite = temper_funtests::suite_dir();
    for &path in COVERED.iter().chain(tests) {
        let test = temper_funtests::test(path).unwrap_or_else(|| panic!("no test {path}"));
        let host_files: Vec<(String, String)> = (test.host_files().into_iter())
            .map(|s| (s.path, s.text))
            .collect();
        let case = Case {
            key: path.replace('/', "-"),
            library: if host_files.is_empty() {
                test.name().to_string()
            } else {
                "work".to_string()
            },
            sources: (test.sources().into_iter())
                .map(|s| (s.path, s.text))
                .collect(),
            dir: suite.join(path),
            expected: test.expected_log(backend),
            libraries: Vec::new(),
            host_files,
            no_connected: false,
        };
        if !case.host_files.is_empty() {
            cases.push(case.clone());
            cases.push(Case {
                key: format!("{}-no-connected", case.key),
                expected: test.expected_log_no_connected(backend),
                no_connected: true,
                ..case
            });
        } else {
            cases.push(case);
        }
    }
    cases.extend(cases_of(backend, programs));
    cases
}

/// The shared programs named in `programs`, as [`cases`] gives them.
fn cases_of(backend: &str, programs: &[&str]) -> Vec<Case> {
    let mut cases = Vec::new();
    let dir = programs_dir();
    for &file in programs {
        assert!(PROGRAMS.contains(&file), "no shared program {file}");
        let in_dir = LIBRARIES.contains(&file);
        let (name, path) = match file.strip_suffix(".tmpl") {
            Some(name) => (name, dir.join(file)),
            None if in_dir => (file, dir.join(file).join(format!("{file}.temper.md"))),
            None => (file, dir.join(format!("{file}.temper.md"))),
        };
        let text = fs::read_to_string(&path).unwrap();
        let expected = if file.ends_with(".tmpl") {
            (text.lines())
                .filter_map(|l| l.strip_prefix("// log: "))
                .map(str::to_string)
                .collect()
        } else {
            temper_funtests::expected_log(&text, backend)
        };
        cases.push(Case {
            key: name.to_string(),
            library: name.to_string(),
            expected,
            sources: vec![(path.to_string_lossy().into_owned(), text)],
            dir: if in_dir { dir.join(file) } else { dir.clone() },
            libraries: if in_dir {
                (LIBRARIES.iter())
                    .filter(|&&l| l != file)
                    .map(|&l| (l.to_string(), dir.join(l)))
                    .collect()
            } else {
                Vec::new()
            },
            host_files: Vec::new(),
            no_connected: false,
        });
    }
    cases
}

/// A case's library, settled and translated to tmpl, or read as tmpl, and
/// validated.
pub fn program(case: &Case) -> Program {
    let key = &case.key;
    if let [(path, text)] = &case.sources[..] {
        if path.ends_with(".tmpl") {
            let program = text::read(text).unwrap_or_else(|e| panic!("{key}: {e}"));
            let problems = validate(&program, &Profile::full());
            assert!(problems.is_empty(), "{key}: {problems:#?}");
            return program;
        }
    }
    let library = load(case);
    let mut program =
        to_tmpl(&settle(library), &case.library).unwrap_or_else(|e| panic!("{key}: {e}"));
    let problems = validate(&program, &Profile::full());
    assert!(problems.is_empty(), "{key}: {problems:#?}");
    program.host = Host {
        files: (case.host_files.iter())
            .map(|(path, text)| HostFile {
                path: path.clone(),
                text: text.clone(),
            })
            .collect(),
        no_connected: case.no_connected,
    };
    program
}

/// The shared programs that are libraries `case` may import, as cases for
/// `backend`, so a backend's run can build them beside it.
pub fn libraries(case: &Case, backend: &str) -> Vec<Case> {
    let names: Vec<&str> = (case.libraries.iter())
        .map(|(name, _)| name.as_str())
        .filter(|name| LIBRARIES.contains(name))
        .filter(|name| {
            let import = format!("import(\"{name}");
            case.sources.iter().any(|(_, text)| text.contains(&import))
        })
        .collect();
    cases_of(backend, &names)
}

/// What a case's Temper sources log on tempest's interpreter, as its
/// failure, if any, and its output.
pub fn interpret(case: &Case) -> (Option<String>, Vec<String>) {
    let library = load(case);
    let run = interp::run(&settle(library), Some(interp::FUEL));
    (run.failure, run.output)
}

/// A case's library, with the modules it imports, and the other
/// libraries it may import by name.
fn load(case: &Case) -> Library {
    let mut library = Library::new();
    library.libraries = (case.libraries.iter())
        .map(|(name, root)| (name.clone(), root.to_string_lossy().into_owned()))
        .collect();
    for (path, text) in &case.sources {
        library.add_file(path, text, source_kind(path));
    }
    load_imports(&Sources::standard(), &mut library);
    library
}

/// Runs a test on a thread with room for the settle loop's recursion.
pub fn with_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

/// Goldens in a directory: output compared with them, or with
/// `UPDATE_SNAPSHOTS=1`, written over them.
pub struct Goldens {
    dir: PathBuf,
    update: bool,
    differ: Vec<String>,
}

impl Goldens {
    /// The goldens in a crate's `tests/goldens/`.
    pub fn new(manifest_dir: &str) -> Goldens {
        Goldens {
            dir: PathBuf::from(manifest_dir).join("tests/goldens"),
            update: env::var_os("UPDATE_SNAPSHOTS").is_some(),
            differ: Vec::new(),
        }
    }

    /// A golden's path, by its file name.
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Compares output with the golden of this file name, or updates it.
    pub fn check(&mut self, name: &str, actual: &str) {
        let golden = self.path(name);
        let expected = fs::read_to_string(&golden).unwrap_or_default();
        if actual != expected {
            if self.update {
                fs::create_dir_all(&self.dir).unwrap();
                fs::write(&golden, actual).unwrap();
            } else {
                self.differ.push(format!("{}:\n{actual}", golden.display()));
            }
        }
    }

    /// Notes a difference found otherwise, such as a file not laid out as
    /// its formatter would.
    pub fn differs(&mut self, what: String) {
        self.differ.push(what);
    }

    /// Fails if any golden differs.
    pub fn finish(self) {
        assert!(
            self.differ.is_empty(),
            "goldens differ (UPDATE_SNAPSHOTS=1 accepts new output):\n{}",
            self.differ.join("\n")
        );
    }
}

/// Whether checkers are required (`TEMPEST_REQUIRE_CHECKERS`, as in CI),
/// so a missing one fails rather than skips.
pub fn required() -> bool {
    env::var_os("TEMPEST_REQUIRE_CHECKERS").is_some()
}

/// Something a test needs is missing: skipped, with a message, unless
/// checkers are required, when it fails.
pub fn missing(what: &str) {
    assert!(!required(), "{what} (needed with TEMPEST_REQUIRE_CHECKERS)");
    eprintln!("skipped: {what}");
}

/// A tool named by an environment variable, or found on `PATH` by
/// `name`, if it runs with `arg` (such as `--version`), with what that
/// printed.
pub fn tool_with(var: &str, name: &str, arg: &str) -> Option<(String, String)> {
    let candidate = env::var(var).unwrap_or_else(|_| name.to_string());
    let out = Command::new(&candidate).arg(arg).output().ok()?;
    let version = [out.stdout, out.stderr].concat();
    out.status
        .success()
        .then(|| (candidate, String::from_utf8_lossy(&version).into_owned()))
}

/// A tool named by an environment variable, or found on `PATH`, if it
/// runs with `--version`.
pub fn tool(var: &str, name: &str) -> Option<String> {
    tool_with(var, name, "--version").map(|(t, _)| t)
}

/// Writes generated files, each a path and its text, under a directory.
pub fn write_files<'a>(root: &Path, files: impl IntoIterator<Item = (&'a str, &'a str)>) {
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
    }
}

/// Writes a file only if its text changed, so incremental builds stay
/// incremental.
pub fn write_if_changed(path: &Path, text: &str) {
    if fs::read_to_string(path).ok().as_deref() == Some(text) {
        return;
    }
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Copies a directory's files, leaving out directories named in `skip`
/// (such as `target`), and leaving files that haven't changed alone.
pub fn copy_dir(from: &Path, to: &Path, skip: &[&str]) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = entry.file_name();
        if entry.file_type().unwrap().is_dir() {
            if !skip.iter().any(|s| name == **s) {
                copy_dir(&path, &to.join(&name), skip);
            }
        } else {
            let target = to.join(&name);
            let bytes = fs::read(&path).unwrap();
            if fs::read(&target).ok().as_deref() != Some(&bytes[..]) {
                fs::write(&target, bytes).unwrap();
            }
        }
    }
}

/// Every file with an extension under a directory, sorted; none if
/// there's no such directory.
pub fn files_with(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files_with(&path, extension, out);
        } else if path.extension().is_some_and(|e| e == extension) {
            out.push(path);
        }
    }
}

/// Something built once, in `root`, and kept until its inputs change:
/// `key` (a tool's version and options, say) and the paths and contents
/// of `sources`. If the stamp in `root` matches and `output` is there,
/// does nothing; else empties `root` and calls `build`, then stamps it.
pub fn cached(
    root: &Path,
    key: impl Hash,
    sources: &[PathBuf],
    output: &Path,
    build: impl FnOnce(),
) {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    for source in sources {
        source.hash(&mut hasher);
        fs::read(source).unwrap().hash(&mut hasher);
    }
    let stamp = format!("{:016x}\n", hasher.finish());
    let stamp_path = root.join("stamp");
    if fs::read_to_string(&stamp_path).ok().as_deref() == Some(&stamp) && output.exists() {
        return;
    }
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root).unwrap();
    build();
    fs::write(stamp_path, stamp).unwrap();
}

/// Compares what a program logged with what's expected, adding a failure
/// if it differs or the program failed.
pub fn check_log(failures: &mut Vec<String>, name: &str, out: &Output, expected: &[String]) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let actual: Vec<&str> = stdout.lines().collect();
    if !out.status.success() || actual != expected {
        failures.push(format!(
            "{name}: expected {expected:?}, got {actual:?}\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
}

/// What `module-state` logs when its init runs and then 8 threads each
/// call `tick` and `report` 1,000 times: its init's log, then `ticks
/// 8000`.
pub fn threads_expected(cases: &[Case]) -> Vec<String> {
    let state = cases.iter().find(|c| c.library == "module-state").unwrap();
    let mut expected = state.expected.clone();
    expected.push("ticks 8000".into());
    expected
}

/// Fails with every failure, if any.
pub fn assert_none(failures: &[String]) {
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
