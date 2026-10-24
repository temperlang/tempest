//! Builds the given files as one library, settles it and runs it with the
//! tree interpreter, printing what it logs.
//!
//!     run-temper [--tree] [--steps] [--fuel none|STEPS] [--std DIR] FILE...
//!     run-temper --bench ROUNDS FILE...
//!
//! `--tree` prints the settled tree and diagnostics instead of running.
//! `--fuel` stops a run after that many evaluation steps; the default is
//! none, running to the end as other languages' runners do. Callers that
//! must finish, like tests, set it.
//! Modules that imports name are loaded from their directories, those
//! under `std/` from `--std`'s, such as Kotlin Temper's
//! `frontend/src/commonMain/resources/std`.
//! Diagnostics and a run's failure go to stderr. With `--bench`, it reads
//! all files, then per round builds each as its own library and settles
//! it, printing milliseconds per round.

use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
    time::Instant,
};
use temper_frontend::interp::run;
use temper_frontend::library::Library;
use temper_frontend::print::{write_diagnostics, write_settled};
use temper_frontend::settle::settle;
use temper_syntax::lex::SourceKind;

fn main() -> ExitCode {
    // The interpreter recurses; give it room (see `interp`).
    let thread = std::thread::Builder::new().stack_size(64 << 20).spawn(run_main);
    thread.and_then(|t| t.join().map_err(|_| io::Error::other("panicked"))).unwrap_or(ExitCode::FAILURE)
}

fn run_main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let (mut tree, mut steps, mut rounds, mut fuel, mut std_dir) = (false, false, None, None, None);
    while let Some(arg) = args.first() {
        match arg.as_str() {
            "--tree" => tree = true,
            "--steps" => steps = true,
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
    let mut sources = Vec::new();
    for path in &args {
        match fs::read_to_string(path) {
            Ok(text) => sources.push((path.clone(), text)),
            Err(err) => {
                eprintln!("{path}: {err}");
                return ExitCode::FAILURE;
            }
        }
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    if let Some(rounds) = rounds {
        let bytes: usize = sources.iter().map(|(_, t)| t.len()).sum();
        let _ = writeln!(out, "# {} files, {bytes} bytes", sources.len());
        for _ in 0..rounds {
            let begin = Instant::now();
            let mut count = 0;
            for (path, text) in &sources {
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
    library.std_dir = std_dir;
    for (path, text) in &sources {
        library.add_file(path, text, kind_of(path));
    }
    load_imports(&mut library);
    let settled = settle(library);
    let mut text = String::new();
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
    let result = run(&settled, fuel);
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

fn kind_of(path: &str) -> SourceKind {
    if path.ends_with(".md") { SourceKind::Markdown } else { SourceKind::Standalone }
}

/// Adds the files of each module that a relative `import` names, and of
/// those they import in turn. A directory that can't be read is left out,
/// so the import fails when settled.
fn load_imports(library: &mut Library) {
    let mut tried = Vec::new();
    loop {
        let missing: Vec<String> = library.missing_imports().into_iter().filter(|d| !tried.contains(d)).collect();
        if missing.is_empty() {
            return;
        }
        for dir in missing {
            let mut paths: Vec<String> = fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok().map(|e| e.path().to_string_lossy().into_owned()))
                .filter(|p| p.ends_with(".temper") || p.ends_with(".temper.md"))
                .collect();
            paths.sort();
            for path in paths {
                if let Ok(text) = fs::read_to_string(&path) {
                    library.add_file(&path, &text, kind_of(&path));
                }
            }
            tried.push(dir);
        }
    }
}
