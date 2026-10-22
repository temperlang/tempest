//! Prints the trees `temper_syntax::ast::build` makes from parse trees, in
//! the format of `compat/oracle/AstTree.java`, for comparison with Kotlin
//! Temper.
//!
//!     ast-tree [--pretty] [--module | --docs] FILE...
//!     ast-tree [--module | --docs] --bench ROUNDS FILE...
//!
//! `--pretty` prints the readable form used in `tests/ast/*.cases`
//! instead. `--module` adds the steps of `module::build_module` for a
//! library (call joins rewritten, metadata as JSON), and `--docs` those for
//! documentation. With `--bench`, it reads all files, then lexes, parses and
//! builds trees for them all once per round, and prints the time per round
//! in milliseconds, one line per round.

use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
    time::Instant,
};
use temper_syntax::lex::SourceKind;
use temper_syntax::ast::{Ast, build};
use temper_syntax::module::{Genre, build_module};
use temper_syntax::parse::{Parsed, parse};
use temper_syntax::print::{write_ast, write_ast_lines};

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut pretty = false;
    let mut rounds = None;
    let mut genre = None;
    while let Some(arg) = args.first() {
        match arg.as_str() {
            "--pretty" => pretty = true,
            "--module" => genre = Some(Genre::Library),
            "--docs" => genre = Some(Genre::Documentation),
            "--bench" => match args.get(1).and_then(|n| n.parse::<usize>().ok()) {
                Some(n) => {
                    rounds = Some(n);
                    args.remove(0);
                }
                None => {
                    eprintln!("usage: ast-tree --bench ROUNDS FILE...");
                    return ExitCode::FAILURE;
                }
            },
            _ => break,
        }
        args.remove(0);
    }

    let mut ok = true;
    let mut sources = Vec::with_capacity(args.len());
    for path in args {
        match fs::read_to_string(&path) {
            Ok(text) => sources.push((path, text)),
            Err(err) => {
                eprintln!("{path}: {err}");
                ok = false;
            }
        }
    }

    let mut out = io::BufWriter::new(io::stdout().lock());
    match rounds {
        Some(rounds) => {
            let bytes: usize = sources.iter().map(|(_, text)| text.len()).sum();
            let _ = writeln!(out, "# {} files, {bytes} bytes", sources.len());
            for _ in 0..rounds {
                let begin = Instant::now();
                let mut count = 0;
                for (path, text) in &sources {
                    count += build_any(&parse(text, kind_of(path)), genre).tree().len();
                }
                let millis = begin.elapsed().as_secs_f64() * 1000.0;
                // Print the count so the work can't be optimized away.
                let _ = writeln!(out, "{millis:.3}\t{count}");
            }
        }
        None => {
            let mut lines = String::new();
            for (path, text) in &sources {
                lines.clear();
                lines.push_str("# ");
                lines.push_str(path);
                lines.push('\n');
                let ast = build_any(&parse(text, kind_of(path)), genre);
                if pretty {
                    write_ast(&ast, 100, &mut lines);
                } else {
                    write_ast_lines(&ast, &mut lines);
                }
                let _ = out.write_all(lines.as_bytes());
            }
        }
    }
    let _ = out.flush();
    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn build_any(parsed: &Parsed, genre: Option<Genre>) -> Ast {
    match genre {
        Some(genre) => build_module(parsed, genre),
        None => build(parsed),
    }
}

fn kind_of(path: &str) -> SourceKind {
    if path.ends_with(".md") { SourceKind::Markdown } else { SourceKind::Standalone }
}
