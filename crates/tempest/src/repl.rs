//! `tempest repl`: reads Temper a line at a time and runs each complete
//! input in one [`ReplSession`], printing what it reports. Lines are read
//! plainly, without editing yet; input that isn't a terminal gets no
//! prompt, so piped input gives clean output.
//!
//! Ctrl-C stops the input running, through the interpreter's interrupt
//! check, and the session goes on; at the prompt, it drops the input
//! begun so far once the line is entered. If tempest itself is stuck,
//! pressing Ctrl-C again ends it (a third time on Unix). Ctrl-D, the end of input, runs what's
//! pending and exits with 0, whatever failed.

use crate::{Flags, NOT_YET_CODE, USAGE, parse, parse_fuel};
use std::io::{self, BufRead, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use temper_frontend::project::{CONFIG_FILE, find_libraries_ignoring, work_root};
use temper_frontend::repl::{Chunker, ReplEvent, ReplOptions, ReplSession, Separator};
use temper_frontend::sources::Sources;

/// `tempest repl`.
pub fn repl_command(args: &[String]) -> u8 {
    let allowed = [
        "std",
        "workroot",
        "ignorefile",
        "backend",
        "exec-before",
        "separator",
        "prompt",
        "fuel",
        "verbose",
    ];
    let flags = match parse("repl", args, &allowed, 0) {
        Ok(flags) => flags,
        Err(code) => return code,
    };
    if let Some(backend) = flags.backends.first() {
        eprintln!(
            "tempest repl: -b {backend} isn't in tempest yet (Kotlin's opens the backend \
             language's own REPL with the work root's libraries loaded); the REPL runs \
             Temper on its interpreter."
        );
        return NOT_YET_CODE;
    }
    let separator = match flags.separator.as_deref().map(Separator::named) {
        None => Separator::Newline,
        Some(Some(separator)) => separator,
        Some(None) => {
            let value = flags.separator.as_deref().unwrap_or_default();
            eprintln!("tempest repl: --separator is newline, semisemi, or eof, not {value}");
            return USAGE;
        }
    };
    let prompt = match flags.prompt.as_deref() {
        None => io::stdin().is_terminal(),
        Some("normal") => true,
        Some("hide") => false,
        Some(other) => {
            eprintln!("tempest repl: --prompt is normal or hide, not {other}");
            return USAGE;
        }
    };
    let fuel = match flags.fuel.as_deref().map(parse_fuel) {
        None => None,
        Some(Ok(fuel)) => fuel,
        Some(Err(())) => {
            let value = flags.fuel.as_deref().unwrap_or_default();
            eprintln!("tempest repl: --fuel is a number of steps above 0, or none, not {value}");
            return USAGE;
        }
    };
    let sources = flags.sources();
    let libraries = match libraries(&flags, &sources) {
        Ok(libraries) => libraries,
        Err(code) => return code,
    };
    let interrupt = sigint::install();
    let options = ReplOptions {
        sources,
        libraries,
        fuel,
        interrupt: Some(interrupt.clone()),
        hints: flags.verbose,
    };
    let mut session = ReplSession::new(options, Box::new(print_event));
    for code in &flags.exec {
        interrupt.store(false, Ordering::Relaxed);
        session.feed(code);
    }
    read_inputs(&mut session, separator, prompt, &interrupt);
    0
}

/// Each event as it happens, on standard output.
fn print_event(event: &ReplEvent) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", event.text());
    let _ = out.flush();
}

/// The libraries imports can name: those under `-w`'s work root, or
/// else the work root around the current directory, if it's in one.
fn libraries(flags: &Flags, sources: &Sources) -> Result<Vec<(String, String)>, u8> {
    let root = match &flags.workroot {
        Some(root) => root.clone(),
        None => {
            let root = work_root(sources, ".");
            let entries = sources.list(&root).unwrap_or_default();
            let marked = (entries.iter())
                .any(|e| e.name == CONFIG_FILE && !e.is_dir || e.name == "temper.out" && e.is_dir);
            if !marked {
                return Ok(Vec::new());
            }
            root
        }
    };
    let ignore = flags.ignore(sources, &root)?;
    let found = find_libraries_ignoring(sources, &root, ignore.as_ref());
    Ok((found.into_iter())
        .map(|l| (l.config.name, l.root))
        .collect())
}

/// Reads lines until the end of input, feeding each complete input.
fn read_inputs(
    session: &mut ReplSession,
    separator: Separator,
    prompt: bool,
    interrupt: &AtomicBool,
) {
    let mut chunker = Chunker::new(separator);
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = Vec::new();
    loop {
        if prompt {
            let mut out = io::stdout().lock();
            let _ = write!(out, "{}", if chunker.is_pending() { "  " } else { "$ " });
            let _ = out.flush();
        }
        line.clear();
        let read = reader.read_until(b'\n', &mut line);
        if matches!(read, Ok(0) | Err(_)) {
            break;
        }
        // Ctrl-C at the prompt drops what's begun.
        if interrupt.swap(false, Ordering::Relaxed) {
            sigint::reset();
            chunker.clear();
            continue;
        }
        let text = String::from_utf8_lossy(&line);
        if let Some(input) = chunker.push_line(&text) {
            run(session, &input, interrupt);
        }
    }
    if let Some(input) = chunker.finish() {
        run(session, &input, interrupt);
    }
    if prompt {
        println!();
    }
}

/// Feeds an input; a Ctrl-C while it runs is for it alone.
fn run(session: &mut ReplSession, input: &str, interrupt: &AtomicBool) {
    interrupt.store(false, Ordering::Relaxed);
    session.feed(input);
    interrupt.store(false, Ordering::Relaxed);
    sigint::reset();
}

/// Ctrl-C, as a flag for the interpreter rather than the end of the
/// process: SIGINT on Unix, the console's Ctrl-C event on Windows, with
/// the platform's own calls (no crate). While a Ctrl-C is still
/// unhandled, another one goes to the platform's default: on Unix, the
/// next ends tempest, and on Windows, that one does, for when tempest
/// itself is stuck.
mod sigint {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};

    static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    /// Ctrl-Cs since the flag was last handled.
    static PRESSES: AtomicU32 = AtomicU32::new(0);

    /// The flag Ctrl-C sets, with the handler installed.
    pub fn install() -> Arc<AtomicBool> {
        let flag = FLAG
            .get_or_init(|| Arc::new(AtomicBool::new(false)))
            .clone();
        platform::install();
        flag
    }

    /// After a Ctrl-C is handled: the next ones count from none, and the
    /// handler is back if a second one took it away.
    pub fn reset() {
        if PRESSES.swap(0, Ordering::Relaxed) > 1 {
            platform::install();
        }
    }

    /// What the handler does: sets the flag, and says whether it's the
    /// first press since the last was handled; if not, the platform's
    /// default takes over.
    fn pressed() -> bool {
        if let Some(flag) = FLAG.get() {
            flag.store(true, Ordering::Relaxed);
        }
        PRESSES.fetch_add(1, Ordering::Relaxed) == 0
    }

    #[cfg(unix)]
    mod platform {
        use std::os::raw::c_int;

        const SIGINT: c_int = 2;
        const SIG_DFL: usize = 0;

        unsafe extern "C" {
            fn signal(signum: c_int, handler: usize) -> usize;
        }

        extern "C" fn on_sigint(_: c_int) {
            if !super::pressed() {
                // SAFETY: `signal` is async-signal-safe; this restores
                // the default for the next SIGINT.
                unsafe { signal(SIGINT, SIG_DFL) };
            }
        }

        pub fn install() {
            let handler: extern "C" fn(c_int) = on_sigint;
            // SAFETY: the handler only touches atomics and calls `signal`,
            // which are async-signal-safe.
            unsafe { signal(SIGINT, handler as usize) };
        }
    }

    #[cfg(windows)]
    mod platform {
        const CTRL_C_EVENT: u32 = 0;

        unsafe extern "system" {
            fn SetConsoleCtrlHandler(
                handler: Option<unsafe extern "system" fn(u32) -> i32>,
                add: i32,
            ) -> i32;
        }

        unsafe extern "system" fn on_ctrl(kind: u32) -> i32 {
            // Handled unless it's the third Ctrl-C in a row, or another
            // event, such as closing the console.
            (kind == CTRL_C_EVENT && super::pressed()) as i32
        }

        /// Once: the handler stays, passing on what it doesn't handle.
        pub fn install() {
            static ONCE: std::sync::Once = std::sync::Once::new();
            // SAFETY: registers a handler that only touches atomics.
            ONCE.call_once(|| unsafe {
                SetConsoleCtrlHandler(Some(on_ctrl), 1);
            });
        }
    }

    #[cfg(not(any(unix, windows)))]
    mod platform {
        pub fn install() {}
    }
}
