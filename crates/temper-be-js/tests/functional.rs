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
//! - `source_maps`: each program's `.internal.js.map`, decoded, matching
//!   the golden `tests/goldens/NAME.map.txt`, a line per mapping with the
//!   generated and source text there, updated as `goldens` is; and each
//!   mapping lands where it should (a `while` on a `while`, a function's
//!   name on its source name).
//! - `runs`: each generated package runs under Node (`node`, or `NODE`),
//!   against the vendored `@temperlang/core`
//!   (`vendor/temper/be-js/temper-core`), and logs what the test expects
//!   of Kotlin's `js` backend; and TypeScript's checker (`tsc`, or `TSC`)
//!   finds no problems in it with `--checkJs --strict`, against
//!   declarations it generates from the runtime. Node's own source map
//!   support (`SourceMap` from `node:module`) reads each map to the same
//!   positions, in source files that exist; and with
//!   `--enable-source-maps`, an error thrown in generated code has a stack
//!   trace that points into its Temper source. Skipped, with a message,
//!   without Node, and its type check without `tsc`; with
//!   `TEMPEST_REQUIRE_CHECKERS` set, as in CI, either missing fails it
//!   instead.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use temper_be_js::{JS, OutFile, translate, translate_with_source_maps};
use temper_frontend::library::Library;
use temper_frontend::project::load_imports;
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::to_tmpl;
use temper_syntax::lex::SourceKind;
use temper_tmpl::source_map::{Segment, decode_mappings};
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

/// A program to translate: its library's name, its sources and their
/// directory, and the log it expects.
struct Case {
    /// Its golden's name: the test's path with `-` for `/`.
    key: String,
    library: String,
    sources: Vec<(String, String)>,
    dir: PathBuf,
    expected: Vec<String>,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let suite =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/functional-tests");
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
            dir: suite.join(path),
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
                dir: dir.clone(),
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
            dir: dir.clone(),
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

/// A source map's fields, read from the one line of JSON it's written as.
struct Map {
    sources: Vec<String>,
    names: Vec<String>,
    lines: Vec<Vec<Segment>>,
}

/// A JSON string's content from `at` on, without escapes but `\"` and
/// `\\`, and the rest after its closing quote.
fn json_string(text: &str) -> (String, &str) {
    let mut out = String::new();
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (out, &text[i + 1..]),
            '\\' => out.push(chars.next().unwrap().1),
            c => out.push(c),
        }
    }
    panic!("unterminated string in {text}")
}

fn read_map(json: &str) -> Map {
    let field = |key: &str| {
        let at = json.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
        &json[at..]
    };
    let list = |key: &str| {
        let mut rest = field(key).strip_prefix('[').unwrap();
        let mut items = Vec::new();
        while let Some(item) = rest.strip_prefix('"') {
            let (s, after) = json_string(item);
            items.push(s);
            rest = after.strip_prefix(',').unwrap_or(after);
        }
        assert!(rest.starts_with(']'), "{json}");
        items
    };
    let (mappings, _) = json_string(field("mappings").strip_prefix('"').unwrap());
    Map {
        sources: list("sources"),
        names: list("names"),
        lines: decode_mappings(&mappings).unwrap(),
    }
}

/// A line's text from a column counted in UTF-16 units.
fn from_column(line: &str, column: u32) -> &str {
    let mut units = 0;
    for (i, c) in line.char_indices() {
        if units >= column {
            return &line[i..];
        }
        units += c.len_utf16() as u32;
    }
    ""
}

/// A case's source that a map names, by its file name.
fn source_text<'a>(case: &'a Case, source: &str) -> &'a str {
    fn name(path: &str) -> &str {
        path.rsplit(['/', '\\']).next().unwrap()
    }
    (case.sources.iter())
        .find(|(path, _)| name(path) == name(source))
        .map(|(_, text)| text.as_str())
        .unwrap_or_else(|| panic!("{}: no source {source}", case.key))
}

/// What Temper source a generated statement starting this way starts
/// with: one of these.
fn expected_source(generated: &str) -> Option<&'static [&'static str]> {
    const STARTS: &[(&str, &[&str])] = &[
        ("/**", &["let ", "var "]),
        ("function ", &["let "]),
        ("const ", &["let ", "var "]),
        ("let ", &["let ", "var "]),
        ("while (", &["while"]),
        ("do ", &["do"]),
        ("if (", &["if"]),
        ("console.log(", &["console.log("]),
        ("break", &["break"]),
        ("continue", &["continue"]),
    ];
    STARTS
        .iter()
        .find(|(js, _)| generated.starts_with(js))
        .map(|&(_, temper)| temper)
}

/// A map's mappings, a line each: generated line and column, source,
/// line and column (all from 1), name if any, and the text at both ends;
/// and where a mapping doesn't land where it should.
fn describe<'a>(case: &'a Case, module: &'a str, map: &Map) -> (String, Vec<String>) {
    let mut out =
        String::from("# generated line:column -> source:line:column, and the text at each\n");
    let mut problems = Vec::new();
    let generated: Vec<&str> = module.lines().collect();
    for (i, segments) in map.lines.iter().enumerate() {
        for s in segments {
            let source = &map.sources[s.source as usize];
            let source_line = (source_text(case, source).lines())
                .nth(s.source_line as usize)
                .unwrap_or("");
            let (js, temper) = (
                from_column(generated[i], s.column),
                from_column(source_line, s.source_column),
            );
            let snippet = |t: &str| {
                t.chars()
                    .take(28)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            };
            let name = s.name.map(|n| map.names[n as usize].as_str());
            out.push_str(&format!(
                "{}:{} -> {source}:{}:{}{}  {} <- {}\n",
                i + 1,
                s.column + 1,
                s.source_line + 1,
                s.source_column + 1,
                name.map(|n| format!(" ({n})")).unwrap_or_default(),
                snippet(js),
                snippet(temper)
            ));
            let lands = match name {
                Some(name) => temper.starts_with(name),
                None => {
                    // Exported or not, the same.
                    let unexported = |t: &'a str| t.strip_prefix("export ").unwrap_or(t);
                    let temper = unexported(temper);
                    expected_source(unexported(js))
                        .is_none_or(|starts| starts.iter().any(|t| temper.starts_with(t)))
                }
            };
            if !lands {
                problems.push(format!(
                    "{}: {}:{} `{}` maps to `{}`",
                    case.key,
                    i + 1,
                    s.column + 1,
                    snippet(js),
                    snippet(temper)
                ));
            }
        }
    }
    (out, problems)
}

#[test]
fn source_maps() {
    with_stack(|| {
        let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
        let goldens = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens");
        let (mut differ, mut problems) = (Vec::new(), Vec::new());
        for case in cases() {
            let program = program(&case);
            let plain = translate(&program, &JS).unwrap();
            let files = translate_with_source_maps(&program, &JS, "../").unwrap();
            let golden = goldens.join(format!("{}.map.txt", case.key));
            if !program.has_spans {
                // Read from tmpl's text without spans: no map.
                assert_eq!(files, plain, "{}", case.key);
                assert!(!golden.exists(), "{}", golden.display());
                continue;
            }
            // The same files, the module with a comment naming its map,
            // and the map after it.
            let internal = plain[3].path.rsplit('/').next().unwrap();
            assert_eq!(files.len(), 5, "{}", case.key);
            assert_eq!(files[..3], plain[..3], "{}", case.key);
            assert_eq!(
                files[3].text,
                format!("{}//# sourceMappingURL={internal}.map\n", plain[3].text)
            );
            assert_eq!(files[4].path, format!("{}.map", plain[3].path));
            let json = &files[4].text;
            assert!(
                json.starts_with(&format!("{{\"version\":3,\"file\":\"{internal}\",")),
                "{json}"
            );
            let map = read_map(json);
            assert!(map.sources.iter().all(|s| s.starts_with("../")), "{json}");
            let (description, wrong) = describe(&case, &files[3].text, &map);
            problems.extend(wrong);
            let expected = fs::read_to_string(&golden).unwrap_or_default();
            if description != expected {
                if update {
                    fs::write(&golden, &description).unwrap();
                } else {
                    differ.push(format!("{}:\n{description}", golden.display()));
                }
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
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

/// The relative path from one directory to another, both absolute, with
/// `/` after each part, as a source map's `sources` prefix.
fn relative(from: &Path, to: &Path) -> String {
    let normal = |p: &Path| {
        let mut parts: Vec<std::ffi::OsString> = Vec::new();
        for c in p.components() {
            match c {
                std::path::Component::ParentDir => {
                    parts.pop();
                }
                std::path::Component::CurDir => {}
                c => parts.push(c.as_os_str().to_owned()),
            }
        }
        parts
    };
    let (from, to) = (normal(from), normal(to));
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut out = "../".repeat(from.len() - common);
    for part in &to[common..] {
        out.push_str(&part.to_string_lossy());
        out.push('/');
    }
    out
}

/// Has Node load a generated module with `--enable-source-maps`, then
/// look up each of its map's mappings, and a column after each, with its
/// own source map support; and says where Node reads them differently,
/// or finds a source that isn't there.
fn node_reads_map(node: &str, case: &Case, module: &Path, map: &Map) -> Vec<String> {
    const LOOKUP: &str = r#"
import { findSourceMap } from "node:module";
import { pathToFileURL } from "node:url";
const [file, ...queries] = process.argv.slice(1);
const url = pathToFileURL(file).href;
await import(url);
const map = findSourceMap(url);
for (const query of queries) {
  const [line, column] = query.split(":").map(Number);
  const e = map.findEntry(line, column);
  console.log(["@@", query, e.originalSource, e.originalLine, e.originalColumn, e.name ?? "-"].join("\t"));
}
"#;
    let mut queries = Vec::new();
    for (i, segments) in map.lines.iter().enumerate() {
        for s in segments {
            for column in [s.column, s.column + 1] {
                let next = segments.iter().any(|t| t.column == column && t != s);
                if !next {
                    queries.push((format!("{i}:{column}"), s));
                }
            }
        }
    }
    let run = Command::new(node)
        .args(["--enable-source-maps", "--input-type=module", "-e", LOOKUP])
        .arg(module)
        .args(queries.iter().map(|(q, _)| q))
        .current_dir(module.parent().unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout);
    let answers: Vec<Vec<&str>> = (stdout.lines())
        .filter_map(|l| l.strip_prefix("@@\t"))
        .map(|l| l.split('\t').collect())
        .collect();
    if !run.status.success() || answers.len() != queries.len() {
        return vec![format!(
            "{}: Node's source map lookup failed:\n{stdout}{}",
            case.key,
            String::from_utf8_lossy(&run.stderr)
        )];
    }
    let mut failures = Vec::new();
    for ((query, s), answer) in queries.iter().zip(&answers) {
        let source = &map.sources[s.source as usize];
        let file = source.rsplit('/').next().unwrap();
        let path = answer[1].strip_prefix("file://").unwrap_or(answer[1]);
        let name = s.name.map_or("-", |n| map.names[n as usize].as_str());
        let expected = [
            s.source_line.to_string(),
            s.source_column.to_string(),
            name.into(),
        ];
        if !path.ends_with(&format!("/{file}"))
            || !Path::new(path).exists()
            || answer[2..] != expected
        {
            failures.push(format!(
                "{}: at {query}, Node reads {answer:?}, not {source} {expected:?}",
                case.key
            ));
        }
    }
    failures
}

/// An error thrown in generated code, from a call with a value Temper
/// doesn't allow, has a stack trace that points into its Temper source,
/// with Node's `--enable-source-maps`.
fn stack_trace(node: &str, out: &Path) -> Vec<String> {
    let dir = out.join("traces-src");
    fs::create_dir_all(&dir).unwrap();
    let text =
        "# Traces\n\n    export let twice(n: Int): Int {\n      let m = n * 2;\n      m\n    }\n";
    let path = dir.join("traces.temper.md");
    fs::write(&path, text).unwrap();
    let case = Case {
        key: "traces".into(),
        library: "traces".into(),
        sources: vec![(path.to_string_lossy().into_owned(), text.into())],
        dir: dir.clone(),
        expected: Vec::new(),
    };
    let root = out.join("traces");
    let prefix = relative(&root.join("traces"), &dir);
    let files = translate_with_source_maps(&program(&case), &JS, &prefix).unwrap();
    write_files(&root, &files);
    let harness = root.join("harness.mjs");
    fs::write(
        &harness,
        "import { twice } from \"./traces/index.js\";\n\
         try { twice(Symbol()); } catch (e) { console.log(e.stack); }\n",
    )
    .unwrap();
    let run = Command::new(node)
        .arg("--enable-source-maps")
        .arg(&harness)
        .current_dir(out)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout);
    // `let m = n * 2;`, which throws converting the symbol to a number.
    let frame = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("at twice "));
    match frame {
        Some(frame) if frame.contains("/traces-src/traces.temper.md:4:7)") => Vec::new(),
        _ => vec![format!(
            "the stack trace doesn't point at traces.temper.md:4:7:\n{stdout}{}",
            String::from_utf8_lossy(&run.stderr)
        )],
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
            let root = out.join(&case.key);
            let package = root.join(&case.library);
            let prefix = relative(&package, &case.dir);
            let files = translate_with_source_maps(&program(&case), &JS, &prefix)
                .unwrap_or_else(|e| panic!("{}: {e}", case.key));
            write_files(&root, &files);
            if let Some(map) = files.iter().find(|f| f.path.ends_with(".map")) {
                let module = root.join(map.path.strip_suffix(".map").unwrap());
                failures.extend(node_reads_map(&node, &case, &module, &read_map(&map.text)));
            }
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
        failures.extend(stack_trace(&node, &out));
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
