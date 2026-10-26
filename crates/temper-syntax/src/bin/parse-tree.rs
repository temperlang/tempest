//! Prints the trees `temper_syntax::parse` builds, in the format of
//! `compat/oracle/ParseTree.java`, for comparison with Kotlin Temper.
//!
//!     parse-tree [--pretty] FILE...
//!     parse-tree --bench ROUNDS FILE...
//!
//! `--pretty` prints the readable
//! form used in `tests/parse/*.cases` instead. With `--bench`, it reads all
//! files, lexes and parses them all once per round, and prints the time per
//! round in milliseconds, one line per round.

use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
    time::Instant,
};
use temper_syntax::lex::SourceKind;
use temper_syntax::parse::parse;
use temper_syntax::print::{write_tree, write_tree_lines};

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut pretty = false;
    let mut rounds = None;
    while let Some(arg) = args.first() {
        match arg.as_str() {
            "--pretty" => pretty = true,
            "--bench" => match args.get(1).and_then(|n| n.parse::<usize>().ok()) {
                Some(n) => {
                    rounds = Some(n);
                    args.remove(0);
                }
                None => {
                    eprintln!("usage: parse-tree --bench ROUNDS FILE...");
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
                    count += parse(text, kind_of(path)).tree().len();
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
                let parsed = parse(text, kind_of(path));
                if pretty {
                    write_tree(&parsed, 100, &mut lines);
                } else {
                    write_tree_lines(&parsed, &mut lines);
                }
                let _ = out.write_all(lines.as_bytes());
            }
        }
    }
    let _ = out.flush();
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn kind_of(path: &str) -> SourceKind {
    if path.ends_with(".md") {
        SourceKind::Markdown
    } else {
        SourceKind::Standalone
    }
}
