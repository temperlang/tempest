//! Builds the given files as one library, settles it and runs it with the
//! interpreter, printing what it logs.
//!
//!     run-temper [--tree] [--untyped] [--tmpl] [--steps] [--test] [--fuel none|STEPS] [--std DIR]
//!                [--test-server-port PORT] FILE...
//!     run-temper --bench ROUNDS FILE...
//!
//! `--tree` prints the settled tree and diagnostics instead of running.
//! `--untyped` prints, instead of running, what typing left unknown, per
//! file ([`temper_frontend::typing::coverage`]): a line `file PATH exprs
//! N untyped U calls C unresolved R identity I`, then one per expression
//! of unknown type, call not resolved, and `==` relying on identity, as
//! `untyped LINE:COLUMN WHAT | TEXT`, `unresolved …` and `identity …`.
//! compat/count_untyped.py sums them. Its diagnostics go to stderr as
//! `PATH:LINE:COLUMN: LEVEL: MESSAGE`.
//! `--tmpl` prints, instead of running, the library as tmpl's canonical
//! text, without its spans section ([`temper_frontend::to_tmpl`]), and to
//! stderr a status line that compat/count_tmpl.py reads: `tmpl: ok`,
//! `tmpl: not yet in tmpl: WHAT at PATH:LINE:COLUMN`, or `tmpl: invalid,
//! N problems` then each of the validator's problems (printing the
//! invalid tmpl too, to show where); it fails unless ok.
//! `--test` also runs the tests, `test("name") { … }`, printing JUnit
//! XML after the program's output.
//! `--fuel` stops a run after that many evaluation steps; the default is
//! none, running to the end as other languages' runners do. Callers that
//! must finish, like tests, set it.
//! Modules that imports name are loaded from their directories, and
//! those under `std/` are built in, or with `--std`, from its directory,
//! such as Kotlin Temper's `frontend/src/commonMain/resources/std`.
//! `--test-server-port` declares `testServerPort`, for std `net`'s
//! functional tests, as Kotlin Temper does.
//! Diagnostics and a run's failure go to stderr. With `--bench`, it reads
//! all files, then per round builds each as its own library and settles
//! it, printing milliseconds per round.

use std::{
    env,
    io::{self, Write},
    process::ExitCode,
    time::Instant,
};
use temper_frontend::interp::{run, run_tests};
use temper_frontend::library::Library;
use temper_frontend::library_tests::write_located_diagnostics;
use temper_frontend::print::{write_diagnostics, write_settled};
use temper_frontend::project::load_imports;
use temper_frontend::settle::{Settled, settle};
use temper_frontend::sources::Sources;
use temper_frontend::to_tmpl::to_tmpl;
use temper_frontend::typing::coverage;
use temper_syntax::lex::SourceKind;
use temper_tmpl::{Profile, text, validate};

fn main() -> ExitCode {
    // Calls and lowering recurse; give them room (see `interp`).
    let thread = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(run_main);
    thread
        .and_then(|t| t.join().map_err(|_| io::Error::other("panicked")))
        .unwrap_or(ExitCode::FAILURE)
}

fn run_main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let (mut tree, mut steps, mut test, mut rounds, mut fuel, mut std_dir) =
        (false, false, false, None, None, None);
    let (mut untyped, mut tmpl) = (false, false);
    let mut test_server_port = None;
    while let Some(arg) = args.first() {
        match arg.as_str() {
            "--tree" => tree = true,
            "--untyped" => untyped = true,
            "--tmpl" => tmpl = true,
            "--steps" => steps = true,
            "--test" => test = true,
            "--bench" => match args.get(1).and_then(|n| n.parse::<usize>().ok()) {
                Some(n) => {
                    rounds = Some(n);
                    args.remove(0);
                }
                None => {
                    eprintln!("usage: run-temper --bench ROUNDS FILE...");
                    return ExitCode::FAILURE;
                }
            },
            "--std" if args.len() > 1 => {
                std_dir = Some(args.remove(1));
            }
            "--test-server-port" => match args.get(1).and_then(|n| n.parse::<u16>().ok()) {
                Some(port) => {
                    test_server_port = Some(port.into());
                    args.remove(0);
                }
                None => {
                    eprintln!("usage: run-temper --test-server-port PORT FILE...");
                    return ExitCode::FAILURE;
                }
            },
            "--fuel" => {
                fuel = match args.get(1).map(|n| (n == "none", n.parse::<u64>())) {
                    Some((true, _)) => None,
                    Some((_, Ok(n))) => Some(n),
                    _ => {
                        eprintln!("usage: run-temper --fuel none|STEPS FILE...");
                        return ExitCode::FAILURE;
                    }
                };
                args.remove(0);
            }
            _ => break,
        }
        args.remove(0);
    }
    let sources = match &std_dir {
        Some(dir) => Sources::with_std_dir(dir),
        None => Sources::standard(),
    };
    let mut texts = Vec::new();
    for path in &args {
        match sources.read(path) {
            Ok(text) => texts.push((path.clone(), text.into_owned())),
            Err(err) => {
                eprintln!("{path}: {err}");
                return ExitCode::FAILURE;
            }
        }
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    if let Some(rounds) = rounds {
        let bytes: usize = texts.iter().map(|(_, t)| t.len()).sum();
        let _ = writeln!(out, "# {} files, {bytes} bytes", texts.len());
        for _ in 0..rounds {
            let begin = Instant::now();
            let mut count = 0;
            for (path, text) in &texts {
                let mut library = Library::new();
                library.add_file(path, text, kind_of(path));
                count += settle(library).steps;
            }
            let millis = begin.elapsed().as_secs_f64() * 1000.0;
            let _ = writeln!(out, "{millis:.3}\t{count}");
        }
        return ExitCode::SUCCESS;
    }
    let mut library = Library::new();
    library.test_server_port = test_server_port;
    for (path, text) in &texts {
        library.add_file(path, text, kind_of(path));
    }
    load_imports(&sources, &mut library);
    let settled = settle(library);
    let mut text = String::new();
    if untyped {
        write_located_diagnostics(&settled, &mut text);
        eprint!("{text}");
        let _ = out.write_all(coverage_text(&settled).as_bytes());
        return ExitCode::SUCCESS;
    }
    if tmpl {
        write_diagnostics(&settled, &mut text);
        eprint!("{text}");
        let name = library_name(args.first().map_or("main", |p| p.as_str()));
        return match tmpl_text(&settled, &name) {
            Ok((text, problems)) if problems.is_empty() => {
                let _ = out.write_all(text.as_bytes());
                eprintln!("tmpl: ok");
                ExitCode::SUCCESS
            }
            Ok((text, problems)) => {
                // The invalid tmpl too, to see where the problems are.
                let _ = out.write_all(text.as_bytes());
                eprintln!("tmpl: {problems}");
                ExitCode::FAILURE
            }
            Err(why) => {
                eprintln!("tmpl: {why}");
                ExitCode::FAILURE
            }
        };
    }
    if tree {
        write_settled(&settled, 100, &mut text);
        let _ = out.write_all(text.as_bytes());
        return ExitCode::SUCCESS;
    }
    write_diagnostics(&settled, &mut text);
    if steps {
        text.push_str(&format!("{} steps\n", settled.steps));
    }
    eprint!("{text}");
    let result = if test {
        run_tests(&settled, fuel)
    } else {
        run(&settled, fuel)
    };
    for line in &result.output {
        let _ = writeln!(out, "{line}");
    }
    let _ = out.flush();
    match result.failure {
        Some(failure) => {
            eprintln!("{failure}");
            ExitCode::FAILURE
        }
        None => ExitCode::SUCCESS,
    }
}

/// What typing left unknown, per file, as `--untyped` prints it.
fn coverage_text(settled: &Settled) -> String {
    use std::fmt::Write;
    let lib = &settled.library;
    let mut out = String::new();
    for file in coverage(settled) {
        let _ = writeln!(
            out,
            "file {} exprs {} untyped {} calls {} unresolved {} identity {}",
            file.path,
            file.exprs,
            file.untyped.len(),
            file.calls,
            file.unresolved.len(),
            file.identity_eqs.len()
        );
        let lists = [
            ("untyped", &file.untyped),
            ("unresolved", &file.unresolved),
            ("identity", &file.identity_eqs),
        ];
        for (label, list) in lists {
            for (node, what) in list {
                let span = lib.tree.span(*node);
                let (f, offset) = lib.locate(span.start);
                let source = lib.files[f.0 as usize].parsed.source();
                let before = source.get(..offset as usize).unwrap_or(source);
                let line = before.matches('\n').count() + 1;
                let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
                let text: String = lib
                    .source_text(span)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                let text: String = text.chars().take(60).collect();
                let _ = writeln!(out, "  {label} {line}:{column} {what} | {text}");
            }
        }
    }
    out
}

/// A library's name from its first file's: `fibonacci` for
/// `algos/fibonacci/fibonacci.temper.md`.
fn library_name(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = (file.strip_suffix(".temper.md"))
        .or_else(|| file.strip_suffix(".temper"))
        .unwrap_or(file);
    stem.to_string()
}

/// The library as tmpl's canonical text, without spans, if it translates
/// and validates; else why not.
/// The library as tmpl text, and the validator's problems with it, if
/// any, as the status line gives them.
fn tmpl_text(settled: &Settled, name: &str) -> Result<(String, String), String> {
    let mut program = to_tmpl(settled, name).map_err(|e| e.to_string())?;
    let problems = validate(&program, &Profile::full());
    let mut why = String::new();
    if !problems.is_empty() {
        why = format!("invalid, {} problems", problems.len());
        for problem in &problems {
            why.push_str(&format!("\n  {problem}"));
        }
    }
    program.has_spans = false;
    Ok((text::write(&program), why))
}

fn kind_of(path: &str) -> SourceKind {
    if path.ends_with(".md") {
        SourceKind::Markdown
    } else {
        SourceKind::Standalone
    }
}
