//! Builds the given files as one library, settles it and runs it with the
//! tree interpreter, printing what it logs.
//!
//!     run-temper [--tree] [--steps] FILE...
//!     run-temper --bench ROUNDS FILE...
//!
//! `--tree` prints the settled tree and diagnostics instead of running.
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
    let (mut tree, mut steps, mut rounds) = (false, false, None);
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
    for (path, text) in &sources {
        library.add_file(path, text, kind_of(path));
    }
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
    let result = run(&settled);
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
