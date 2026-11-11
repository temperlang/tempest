//! A REPL session: Temper inputs settled and run one at a time against
//! what earlier inputs declared, for `tempest repl`, and later the
//! playground's console and the language server.
//!
//! - **One growing library.** Each input is a file of one module, with a
//!   scope of its own chained to the previous input's, so later inputs
//!   see earlier names directly, and a later declaration shadows an
//!   earlier one: code that already resolved to the old one keeps it.
//!   Each input is settled and typed once, incrementally
//!   ([`Incremental`]); a top-level `let` is visible from its end, so
//!   `let s = "${s}!"` reads the earlier `s`, and a top-level `var`
//!   can be assigned from later inputs, checked against its type.
//! - **One interpreter** whose globals grow by each input
//!   ([`crate::interp::repl`]): code lowered once stays lowered, and
//!   `async` blocks waiting on promises stay parked until a later input
//!   resolves them. An input may `await` at its top level.
//! - **Errors never end a session.** An input with errors still settles
//!   and runs as far as it can, as libraries do; a declaration whose
//!   input failed stays declared, and a read of it fails saying why. A
//!   panic in tempest itself is caught and reported, and the session
//!   goes on. An interrupt, as from Ctrl-C, stops the input running.
//!
//! Each input reports, in order, its diagnostics, what it logs as it
//! runs, why it failed if no diagnostic says, and its result, as
//! [`ReplEvent`]s; [`ReplEvent::text`] gives each as `tempest repl`
//! prints it, the result in Kotlin's form, `interactive#N: value`.
//! [`Chunker`] splits typed or piped lines into inputs.
//!
//! The REPL's own functions, `help`, `describe`, and `translate`, are
//! declared before the first input ([`specials`]).
//!
//! What the REPL prints of its own, diagnostics, failures, and its
//! functions' output, has control characters escaped
//! ([`escape_controls`]), so it can't move the cursor, recolor the
//! terminal, or reorder text; a value's strings are quoted, escaped as
//! Temper's are. What an input logs is printed as it's logged.

mod specials;

pub use specials::{ReplTools, STEPS};

use crate::interp::repl::{Outcome, Runner, render};
use crate::library::{Level, Library, Message};
use crate::project::load_imports;
use crate::scope::{Lookup, ScopeId};
use crate::settle::{Incremental, Settled};
use crate::sources::Sources;
use specials::{HOST_NAME, PRELUDE, PRELUDE_PATH, Specials};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use temper_syntax::lex::{SourceKind, Synthetic, TokenKind, lex};
use temper_syntax::tree::{NodeId, NodeKind};

/// The directory the inputs' files are in, as their module's; never a
/// real one, so it's no library's.
const DIR: &str = "-repl-";

/// How a session is set up.
pub struct ReplOptions {
    /// Where std and the libraries that inputs import are read from.
    pub sources: Sources,
    /// The libraries an import can name, each a name and its root
    /// directory, as [`crate::project::find_libraries`] finds them under
    /// a work root.
    pub libraries: Vec<(String, String)>,
    /// Steps each input may take; `None` for no limit, as `tempest run`.
    pub fuel: Option<u64>,
    /// Stops the input running once set, as by Ctrl-C; whoever sets it
    /// clears it.
    pub interrupt: Option<Arc<AtomicBool>>,
    /// Reports hints too, not only errors and warnings.
    pub hints: bool,
    /// What `help`, `describe`, and `translate` need beyond the
    /// frontend; without them, they say what's missing.
    pub tools: Option<Rc<dyn ReplTools>>,
    /// What the front end adds to `help()`, as its keys and options.
    pub help_footer: String,
}

impl Default for ReplOptions {
    fn default() -> ReplOptions {
        ReplOptions {
            sources: Sources::standard(),
            libraries: Vec::new(),
            fuel: None,
            interrupt: None,
            hints: false,
            tools: None,
            help_footer: String::new(),
        }
    }
}

/// Something an input reports, in the order it happens.
#[derive(Clone, Debug, PartialEq)]
pub enum ReplEvent {
    Diagnostic(ReplDiagnostic),
    /// A line the input logged, as it's logged.
    Log(String),
    /// Why the input, or a module it loaded, failed as it ran, where no
    /// diagnostic of its already says.
    Failure(String),
    /// The input was interrupted.
    Interrupted,
    /// The input awaits a promise nothing has resolved; it goes on if a
    /// later input resolves it.
    Unresolved,
    /// A bug in tempest, a panic, caught; the session goes on.
    InternalError(String),
    /// What one of the REPL's functions shows, as `help()`.
    Output(String),
    /// Something the REPL notes, as that an earlier input used a name
    /// before this one declared it.
    Note(String),
    /// What the input gave, as Kotlin's REPL prints it: `fail` if it
    /// failed.
    Result {
        input: usize,
        value: String,
    },
}

/// A diagnostic, located in its input (`interactive#N`) or file.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplDiagnostic {
    pub level: Level,
    pub message: String,
    /// `interactive#N` for an input, else the file's path.
    pub location: String,
    /// From 1, the column in characters.
    pub line: usize,
    pub column: usize,
    /// The source line, and a line of `^` under the span.
    pub snippet: [String; 2],
}

impl ReplEvent {
    /// The event as `tempest repl` prints it, a line or more, without a
    /// final newline.
    ///
    /// All but logs have control characters escaped
    /// ([`escape_controls`]).
    pub fn text(&self) -> String {
        match self {
            ReplEvent::Diagnostic(d) => {
                let level = format!("{:?}", d.level).to_lowercase();
                format!(
                    "{}:{}:{}: {level}: {}\n    {}\n    {}",
                    escape_controls(&d.location),
                    d.line,
                    d.column,
                    escape_controls(&d.message),
                    d.snippet[0],
                    d.snippet[1]
                )
            }
            ReplEvent::Log(line) => line.clone(),
            ReplEvent::Failure(text) | ReplEvent::Output(text) | ReplEvent::Note(text) => {
                escape_controls(text).into_owned()
            }
            ReplEvent::Interrupted => "Interrupted".into(),
            ReplEvent::Unresolved => "1 promise is unresolved.".into(),
            ReplEvent::InternalError(why) => format!(
                "tempest: internal error (a bug in tempest, not in your code): {}",
                escape_controls(why)
            ),
            ReplEvent::Result { input, value } => {
                format!("interactive#{input}: {}", escape_controls(value))
            }
        }
    }
}

/// Where events go.
type Sink = Rc<RefCell<Box<dyn FnMut(&ReplEvent)>>>;

/// A REPL session ([module docs](self)).
pub struct ReplSession {
    settle: Incremental,
    runner: Runner,
    /// What the REPL's own functions read, shared with the interpreter's
    /// host that runs them.
    specials: Rc<RefCell<Specials>>,
    sink: Sink,
    hints: bool,
    /// Inputs fed so far.
    inputs: usize,
    /// Each input's scope, with its number.
    segments: HashMap<ScopeId, usize>,
    /// Names an input found undeclared, with the first such input, so a
    /// later input declaring one can note it.
    undeclared: HashMap<String, usize>,
    /// Makes the next input panic, to test that a panic is caught.
    #[cfg(test)]
    panic_next: bool,
}

impl ReplSession {
    /// A session with nothing fed yet, giving each event to `sink` as it
    /// happens.
    pub fn new(options: ReplOptions, sink: Box<dyn FnMut(&ReplEvent)>) -> ReplSession {
        let mut library = Library::new();
        library.libraries = options.libraries;
        let sink: Sink = Rc::new(RefCell::new(sink));
        let logs = sink.clone();
        let log =
            Box::new(move |line: &str| (*logs.borrow_mut())(&ReplEvent::Log(line.to_string())));
        let specials = Rc::new(RefCell::new(Specials {
            tools: options.tools,
            sources: options.sources,
            libraries: library.libraries.clone(),
            help_footer: options.help_footer,
            inputs: Vec::new(),
            prelude: None,
            host: None,
        }));
        let host = specials::host(specials.clone(), sink.clone());
        let mut session = ReplSession {
            settle: Incremental::new(library),
            runner: Runner::new(log, options.interrupt, options.fuel, Some(host)),
            specials,
            sink,
            hints: options.hints,
            inputs: 0,
            segments: HashMap::new(),
            undeclared: HashMap::new(),
            #[cfg(test)]
            panic_next: false,
        };
        session.settle_prelude();
        session
    }

    /// Declares the REPL's own functions ([`specials`]) in a scope before
    /// the first input's.
    fn settle_prelude(&mut self) {
        let library = self.settle.library_mut();
        let file = library.add_file(PRELUDE_PATH, PRELUDE, SourceKind::Standalone);
        let root = library.files[file.0 as usize].root;
        // The host's function gets a name no input can write.
        let host = library.names.find(HOST_NAME);
        let hidden = library.names.temporary(HOST_NAME);
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let tree = &library.tree;
            stack.extend(tree.children(node));
            if host.is_some() && library.name(node) == host {
                let kind = tree.kind(node);
                library.tree.replace(node, kind, hidden.0, &[]);
            }
        }
        let round = self.settle.settle_input(file);
        let scope = round.input.scope;
        let runner = &mut self.runner;
        self.settle.view(|s| {
            runner.add_module(s, scope);
            runner.run_input(s, &round.input, root, false);
        });
        let host = (self.settle.scopes().get(scope).names.iter())
            .find(|&&(name, _)| name == hidden)
            .and_then(|&(_, symbol)| self.settle.scopes().symbol(symbol).decl);
        let mut specials = self.specials.borrow_mut();
        specials.prelude = Some(scope);
        specials.host = host;
    }

    /// Lends what's settled so far, with the scope that the next input's
    /// names are looked up from, as for completion.
    pub fn view<R>(&mut self, f: impl FnOnce(&Settled, ScopeId) -> R) -> R {
        let scope = self.settle.last_segment().unwrap_or(crate::scope::BUILTINS);
        self.settle.view(|s| f(s, scope))
    }

    /// Input `n`, with the inputs it uses, as one Temper module, as
    /// `describe(n, "module")` shows it; `None` if there's no input `n`.
    pub fn module_source(&mut self, n: usize) -> Option<String> {
        if n >= self.inputs {
            return None;
        }
        let specials = self.specials.clone();
        self.settle.view(|s| Some(specials.borrow().module(s, n)))
    }

    /// Inputs fed so far, which is the next one's number.
    pub fn inputs(&self) -> usize {
        self.inputs
    }

    /// Settles and runs one input, reporting what happens to the sink,
    /// and returns its result as printed, `None` if it failed.
    pub fn feed(&mut self, text: &str) -> Option<String> {
        let input = self.inputs;
        self.inputs += 1;
        let scopes_before = self.settle.scopes().scopes.len();
        let fed = catch_unwind(AssertUnwindSafe(|| self.feed_input(input, text)));
        let value = match fed {
            Ok(value) => value,
            Err(payload) => {
                let why = (payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "a panic".into());
                self.settle.abandon_round(scopes_before);
                self.runner.recover();
                self.emit(&ReplEvent::InternalError(why));
                None
            }
        };
        let shown = value.clone().unwrap_or_else(|| "fail".into());
        self.emit(&ReplEvent::Result {
            input,
            value: shown,
        });
        value
    }

    fn emit(&self, event: &ReplEvent) {
        (*self.sink.borrow_mut())(event);
    }

    fn feed_input(&mut self, input: usize, text: &str) -> Option<String> {
        let path = format!("{DIR}/interactive#{input}");
        let library = self.settle.library_mut();
        let diagnostics_before = library.diagnostics.len();
        let file = library.add_file(&path, text, SourceKind::Standalone);
        load_imports(&self.specials.borrow().sources, library);
        #[cfg(test)]
        if std::mem::take(&mut self.panic_next) {
            panic!("as a test asked");
        }
        let round = self.settle.settle_input(file);
        self.segments.insert(round.input.scope, input);
        (self.specials.borrow_mut().inputs).push((file, round.input.scope));
        let errors = self.report_diagnostics(input, diagnostics_before);
        self.note_late_declarations(input, round.input.scope);

        let root = self.settle.library().files[file.0 as usize].root;
        let semicolon = ends_with_semicolon(text);
        let kind = round.last.map(|n| self.settle.library().tree.kind(n));
        // The last statement's value, unless it's a declaration, or a
        // `;` follows it.
        let keep =
            !semicolon && kind.is_some_and(|k| !matches!(k, NodeKind::Let | NodeKind::Class));
        // A class or interface declared last gives itself.
        let class = round
            .last
            .filter(|_| !semicolon && kind == Some(NodeKind::Class));
        let latest = round.input.scope;
        let (runner, segments, sink) = (&mut self.runner, &self.segments, &self.sink);
        let emit = |event: ReplEvent| (*sink.borrow_mut())(&event);
        self.settle.view(|s| {
            for module in &round.modules {
                runner.add_module(s, module.scope);
                match runner.run_module(s, module) {
                    Outcome::Value(_) | Outcome::Unresolved => {}
                    Outcome::Interrupted => emit(ReplEvent::Interrupted),
                    Outcome::Failed(why) => {
                        let dir = &s.library.modules[module.id.0 as usize].dir;
                        emit(ReplEvent::Failure(format!("{dir}: {why}")));
                    }
                }
            }
            runner.add_module(s, latest);
            let outcome = runner.run_input(s, &round.input, root, keep);
            let label = |class| class_label(s, segments, latest, class);
            match outcome {
                Outcome::Value(value) if keep => Some(render(s, &value, &label)),
                Outcome::Value(_) => Some(class.map_or("void".into(), label)),
                Outcome::Failed(why) => {
                    // Not again if a diagnostic of the input said it; nor
                    // a bare `Failed`, a bubble's, which `fail` says.
                    let said = why
                        .strip_prefix("Failed: ")
                        .is_some_and(|m| errors.contains(m));
                    if !said && why != "Failed" {
                        emit(ReplEvent::Failure(why));
                    }
                    runner.note_unset(s, latest, &format!("interactive#{input} failed"));
                    None
                }
                Outcome::Interrupted => {
                    emit(ReplEvent::Interrupted);
                    let why = format!("interactive#{input} was interrupted");
                    runner.note_unset(s, latest, &why);
                    None
                }
                Outcome::Unresolved => {
                    emit(ReplEvent::Unresolved);
                    None
                }
            }
        })
    }

    /// Notes names an earlier input found undeclared that input `input`
    /// declares, as for mutual recursion over two inputs, once each.
    fn note_late_declarations(&mut self, input: usize, scope: ScopeId) {
        if self.undeclared.is_empty() {
            return;
        }
        let library = self.settle.library();
        let mut notes = Vec::new();
        for &(name, _) in &self.settle.scopes().get(scope).names {
            let name = library.names.get(name);
            if let Some(earlier) = self.undeclared.remove(name).filter(|&e| e < input) {
                notes.push(format!(
                    "Note: interactive#{earlier} used {name} before this input declared it. \
                     An input sees only what earlier inputs declared, so declarations that \
                     use each other go in one input: on one line, or with \
                     --separator semisemi, ended by ;;"
                ));
            }
        }
        for note in notes {
            self.emit(&ReplEvent::Note(note));
        }
    }

    /// Reports the diagnostics made since `before`, each once, returning
    /// the errors' messages, and notes the names input `input` found
    /// undeclared.
    fn report_diagnostics(&mut self, input: usize, before: usize) -> HashSet<String> {
        let lib = self.settle.library();
        let mut undeclared = Vec::new();
        let mut seen = HashSet::new();
        let mut errors = HashSet::new();
        for d in &lib.diagnostics[before..] {
            if d.level == Level::Hint && !self.hints {
                continue;
            }
            let message = d.message.text();
            if !seen.insert((d.span.start, d.span.end, message.clone())) {
                continue;
            }
            if d.level == Level::Error {
                errors.insert(message.clone());
            }
            let mut message = message;
            if let Message::UndeclaredName(name) = &d.message {
                // A `test(…)` names std's `Test`, which only a library's
                // tests import.
                if name == "Test" && lib.source_text(d.span).starts_with("test") {
                    message = "Tests don't run in the REPL: write them in a library's \
                               files, and run tempest test"
                        .into();
                } else {
                    undeclared.push(name.to_string());
                }
            }
            let (file, start) = lib.locate(d.span.start);
            let file = &lib.files[file.0 as usize];
            let location = match file.path.strip_prefix(&format!("{DIR}/")) {
                Some(name) => name.to_string(),
                None => file.path.clone(),
            };
            let source = file.parsed.source();
            let length = d.span.end.saturating_sub(d.span.start) as usize;
            let (line, column, snippet) = snippet(source, start as usize, length);
            self.emit(&ReplEvent::Diagnostic(ReplDiagnostic {
                level: d.level,
                message,
                location,
                line,
                column,
                snippet,
            }));
        }
        for name in undeclared {
            self.undeclared.entry(name).or_insert(input);
        }
        errors
    }
}

/// A class's or interface's name as a result shows it: plain, as `C`,
/// unless a later input's `C` shadows it, then with its input's, as
/// `interactive#2.C`.
fn class_label(
    s: &Settled,
    segments: &HashMap<ScopeId, usize>,
    latest: ScopeId,
    class: NodeId,
) -> String {
    let name_node = s.library.tree.children(class)[0];
    let Some(name) = s.library.name(name_node) else {
        return "?".into();
    };
    let text = s.library.names.get(name);
    let symbol = s.scopes.resolved(name_node);
    let shadowed = match s.scopes.lookup(latest, name, false, u32::MAX) {
        Lookup::Found(b) => Some(b) != symbol,
        _ => false,
    };
    let owner = symbol.map(|b| s.scopes.symbol(b).owner);
    match owner.and_then(|o| segments.get(&o)) {
        Some(input) if shadowed => format!("interactive#{input}.{text}"),
        _ => text.to_string(),
    }
}

/// The 1-based line and column (in characters) of `start` in `source`,
/// and the line with `^` under the span, at least one, up to the line's
/// end; tabs are shown as spaces, so the marks line up, and control
/// characters escaped ([`escape_controls`]), the marks under their
/// escapes.
fn snippet(source: &str, start: usize, length: usize) -> (usize, usize, [String; 2]) {
    let mut start = start.min(source.len());
    while !source.is_char_boundary(start) {
        start -= 1;
    }
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = source[start..]
        .find('\n')
        .map_or(source.len(), |i| start + i);
    let line = source[..start].matches('\n').count() + 1;
    let column = source[line_start..start].chars().count() + 1;
    let mut end = (start + length).min(line_end);
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    // The line as shown, and where the span starts and ends in it, in
    // characters.
    let mut text = String::new();
    let (mut from, mut to) = (0, 0);
    let mut shown = 0;
    let body = source[line_start..line_end].trim_end_matches('\r');
    for (i, c) in body.char_indices() {
        let at = line_start + i;
        if at == start {
            from = shown;
        }
        if at == end {
            to = shown;
        }
        let before = text.len();
        match c {
            '\t' => text.push(' '),
            c if needs_escape(c) => text.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => text.push(c),
        }
        shown += text[before..].chars().count();
    }
    if start >= line_start + body.len() {
        from = shown;
    }
    if end >= line_start + body.len() {
        to = shown;
    }
    let marks = to.saturating_sub(from).max(1);
    let under = format!("{}{}", " ".repeat(from), "^".repeat(marks));
    (line, column, [text, under])
}

/// Whether a character would change the terminal, or how text around it
/// reads, if printed as is: a C0 control other than a line break or tab,
/// DEL, a C1 control, or a bidirectional control.
fn needs_escape(c: char) -> bool {
    let n = c as u32;
    (n < 0x20 && c != '\n' && c != '\t')
        || (0x7f..=0x9f).contains(&n)
        || matches!(n, 0x61c | 0x200e | 0x200f | 0x202a..=0x202e | 0x2066..=0x2069)
}

/// Text with the characters that could rewrite the terminal or hide text
/// escaped as Temper writes them, `\u{1b}`: C0 controls but for line
/// breaks and tabs, DEL, C1 controls, and bidirectional controls.
pub fn escape_controls(text: &str) -> Cow<'_, str> {
    if !text.chars().any(needs_escape) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match needs_escape(c) {
            true => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            false => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Whether code's last token is `;`, which makes an input give `void`.
fn ends_with_semicolon(text: &str) -> bool {
    let tokens = lex(text, SourceKind::Standalone);
    let last = tokens.iter().rev().find(|t| !t.kind.is_ignorable());
    last.is_some_and(|t| t.kind == TokenKind::Punctuation && &text[t.range()] == ";")
}

/// What ends an input, as Kotlin's `--separator` chooses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Separator {
    /// A line break outside brackets, strings, and comments (the default).
    #[default]
    Newline,
    /// `;;` at the end of a line, as OCaml's, dropped from the input.
    SemiSemi,
    /// The end of all input: it's one input.
    Eof,
}

impl Separator {
    /// A separator by its name on the command line.
    pub fn named(name: &str) -> Option<Separator> {
        match name {
            "newline" => Some(Separator::Newline),
            "semisemi" => Some(Separator::SemiSemi),
            "eof" => Some(Separator::Eof),
            _ => None,
        }
    }
}

/// Splits lines into inputs, by Temper's own lexer, so strings and UTF-8
/// are exactly as the language reads them. With [`Separator::Newline`],
/// a line ends an input unless a bracket (`(`, `[`, `{`, `${`), a
/// multi-quoted string, or a block comment is still open, or the input
/// so far is only comments, as a doc comment waits for what it
/// documents. A `"` string never spans lines, so one left open ends with
/// its line, an error, rather than swallowing the lines after it.
#[derive(Debug, Default)]
pub struct Chunker {
    separator: Separator,
    pending: String,
}

impl Chunker {
    pub fn new(separator: Separator) -> Chunker {
        Chunker {
            separator,
            pending: String::new(),
        }
    }

    /// Whether an input is begun but not complete, as for a continuation
    /// prompt.
    pub fn is_pending(&self) -> bool {
        !self.pending.trim().is_empty()
    }

    /// Drops the input begun so far.
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Whether `text`, its lines fed to a new chunker, leaves nothing
    /// pending: what an editor's Enter asks before running a buffer of
    /// one or more lines, so typed and piped lines split alike.
    pub fn completes(separator: Separator, text: &str) -> bool {
        let mut chunker = Chunker::new(separator);
        for line in text.split('\n') {
            chunker.push_line(line);
        }
        separator != Separator::Eof && !chunker.is_pending()
    }

    /// How many brackets (`(`, `[`, `{`, `${`) are open at the end of
    /// `text`, as for indenting the line after it.
    pub fn open_brackets(text: &str) -> usize {
        let tokens = lex(text, SourceKind::Standalone);
        let mut depth = 0usize;
        for t in &tokens {
            if t.kind != TokenKind::Punctuation || t.synthetic.is_some() {
                continue;
            }
            match &text[t.range()] {
                "(" | "[" | "{" | "${" => depth += 1,
                ")" | "]" | "}" => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        depth
    }

    /// Adds a line, with or without its line break, giving the input it
    /// completes, if it completes one.
    pub fn push_line(&mut self, line: &str) -> Option<String> {
        self.pending.push_str(line);
        if !line.ends_with('\n') {
            self.pending.push('\n');
        }
        if self.pending.trim().is_empty() {
            self.pending.clear();
            return None;
        }
        match self.separator {
            Separator::Eof => None,
            Separator::Newline => complete(&self.pending).then(|| self.take()),
            Separator::SemiSemi => {
                let tokens = lex(&self.pending, SourceKind::Standalone);
                let last = tokens.iter().rev().find(|t| !t.kind.is_ignorable())?;
                let ends = last.kind == TokenKind::Punctuation
                    && last.synthetic.is_none()
                    && &self.pending[last.range()] == ";;";
                if !ends {
                    return None;
                }
                let mut input = self.take();
                input.replace_range(last.range(), "  ");
                Some(input.trim_end().to_string())
            }
        }
    }

    /// At the end of input: what's pending, if it's more than space and
    /// comments.
    pub fn finish(&mut self) -> Option<String> {
        let input = self.take();
        has_code(&input).then_some(input)
    }

    fn take(&mut self) -> String {
        let mut input = std::mem::take(&mut self.pending);
        while input.ends_with(['\n', '\r']) {
            input.pop();
        }
        input
    }
}

/// Whether text's last line, before its final line break, is blank.
fn blank_last_line(text: &str) -> bool {
    let text = text.strip_suffix('\n').unwrap_or(text);
    text.rsplit('\n')
        .next()
        .is_some_and(|line| line.trim().is_empty())
}

/// Whether text has more than space and comments.
fn has_code(text: &str) -> bool {
    let tokens = lex(text, SourceKind::Standalone);
    (tokens.iter()).any(|t| !matches!(t.kind, TokenKind::Comment | TokenKind::Space))
}

/// Whether text is a complete input ([`Chunker`]).
fn complete(text: &str) -> bool {
    let tokens = lex(text, SourceKind::Standalone);
    let mut depth = 0i32;
    let mut code = false;
    for t in &tokens {
        let token = &text[t.range()];
        match t.kind {
            TokenKind::Comment if token.starts_with("/*") && !token.ends_with("*/") => {
                return false;
            }
            TokenKind::Comment | TokenKind::Space => {}
            TokenKind::Punctuation if t.synthetic.is_none() => {
                code = true;
                match token {
                    "(" | "[" | "{" | "${" => depth += 1,
                    ")" | "]" | "}" => depth -= 1,
                    _ => {}
                }
            }
            // A multi-quoted string open at the end, whose next line may
            // go on with it, unless a blank line ended it.
            TokenKind::RightDelimiter
                if t.synthetic == Some(Synthetic::TripleQuote)
                    && t.start as usize == text.len()
                    && !blank_last_line(text) =>
            {
                return false;
            }
            _ => code = true,
        }
    }
    code && depth <= 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcript(inputs: &[&str]) -> Vec<String> {
        let events = Rc::new(RefCell::new(Vec::new()));
        let seen = events.clone();
        let sink = Box::new(move |e: &ReplEvent| seen.borrow_mut().push(e.text()));
        let mut session = ReplSession::new(ReplOptions::default(), sink);
        for input in inputs {
            session.feed(input);
        }
        events.take()
    }

    #[test]
    fn chunks_by_brackets_strings_and_comments() {
        let mut c = Chunker::new(Separator::Newline);
        assert_eq!(c.push_line("1 + 2\n"), Some("1 + 2".into()));
        assert_eq!(c.push_line("   \n"), None);
        assert!(!c.is_pending());
        assert_eq!(c.push_line("f(1,\n"), None);
        assert!(c.is_pending());
        assert_eq!(c.push_line("  2)\n"), Some("f(1,\n  2)".into()));
        assert_eq!(c.push_line("// a doc comment\n"), None);
        assert_eq!(
            c.push_line("let x = 1\n"),
            Some("// a doc comment\nlet x = 1".into())
        );
        assert_eq!(c.push_line("/* open\n"), None);
        assert_eq!(
            c.push_line("closed */ 3\n"),
            Some("/* open\nclosed */ 3".into())
        );
        // A `"` string ends with its line.
        assert_eq!(c.push_line("\"open\n"), Some("\"open".into()));
        assert_eq!(c.push_line("let s = \"\"\"\n"), None);
        assert_eq!(c.push_line("  \"two\n"), None);
        assert_eq!(c.push_line("  \"lines\n"), None);
        assert_eq!(
            c.push_line("\n"),
            Some("let s = \"\"\"\n  \"two\n  \"lines".into())
        );
        assert_eq!(c.push_line("\"${ 1 +\n"), None);
        assert_eq!(c.push_line("2 }\"\n"), Some("\"${ 1 +\n2 }\"".into()));
        // A closer too many ends it, as an error.
        assert_eq!(c.push_line("1 }\n"), Some("1 }".into()));
        assert_eq!(c.push_line("[1,\n"), None);
        assert_eq!(c.finish(), Some("[1,".into()));
        // Comments alone at the end aren't an input.
        assert_eq!(c.push_line("// done\n"), None);
        assert_eq!(c.finish(), None);
    }

    #[test]
    fn says_whether_a_buffer_completes() {
        let newline = Separator::Newline;
        assert!(Chunker::completes(newline, "1 + 2"));
        assert!(Chunker::completes(newline, "f(1,\n  2)"));
        assert!(!Chunker::completes(newline, "f(1,"));
        assert!(!Chunker::completes(newline, "// a doc comment"));
        assert!(!Chunker::completes(newline, "let s = \"\"\"\n  \"text"));
        assert!(Chunker::completes(newline, "let s = \"\"\"\n  \"text\n"));
        assert!(Chunker::completes(newline, "1\n2"));
        assert!(!Chunker::completes(Separator::SemiSemi, "1"));
        assert!(Chunker::completes(Separator::SemiSemi, "1 ;;"));
        assert!(!Chunker::completes(Separator::Eof, "1"));
        assert_eq!(Chunker::open_brackets("f(["), 2);
        assert_eq!(Chunker::open_brackets("\"${ {"), 2);
        assert_eq!(Chunker::open_brackets("f(1)"), 0);
    }

    #[test]
    fn escapes_controls() {
        assert_eq!(escape_controls("plain\ttext\n"), "plain\ttext\n");
        assert_eq!(
            escape_controls("a\u{1b}[2Jb\u{7f}\u{9b}c\u{202e}d\u{2066}\u{61c}"),
            "a\\u{1b}[2Jb\\u{7f}\\u{9b}c\\u{202e}d\\u{2066}\\u{61c}"
        );
        let (_, _, lines) = snippet("x = \u{1b}y", 5, 1);
        assert_eq!(
            lines,
            ["x = \\u{1b}y".to_string(), "          ^".to_string()]
        );
    }

    #[test]
    fn chunks_by_semisemi_and_eof() {
        let mut c = Chunker::new(Separator::SemiSemi);
        assert_eq!(c.push_line("let f(): Int { 1 }\n"), None);
        assert_eq!(
            c.push_line("f() ;;\n"),
            Some("let f(): Int { 1 }\nf()".into())
        );
        let mut c = Chunker::new(Separator::Eof);
        assert_eq!(c.push_line("1\n"), None);
        assert_eq!(c.push_line("2\n"), None);
        assert_eq!(c.finish(), Some("1\n2".into()));
    }

    #[test]
    fn quotes_as_kotlin_does() {
        let mut out = String::new();
        crate::interp::repl::quote("a\"b\\c\n$\u{a0}x\u{1}é😀", &mut out);
        assert_eq!(out, "\"a\\\"b\\\\c\\n\\u{24,a0}x\\u{1}é😀\"");
    }

    #[test]
    fn snippets_underline_spans() {
        let (line, column, lines) = snippet("ab\n\tcdé fg\n", 6, 4);
        assert_eq!((line, column), (2, 4));
        assert_eq!(lines, [" cdé fg".to_string(), "   ^^^".to_string()]);
        let (_, _, lines) = snippet("x = 1", 0, 0);
        assert_eq!(lines[1], "^");
    }

    #[test]
    fn a_panic_is_caught_and_the_session_goes_on() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let seen = events.clone();
        let sink = Box::new(move |e: &ReplEvent| seen.borrow_mut().push(e.text()));
        let mut session = ReplSession::new(ReplOptions::default(), sink);
        session.feed("let x = 1");
        session.panic_next = true;
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        session.feed("x + 1");
        std::panic::set_hook(hook);
        session.feed("x + 2");
        assert_eq!(
            events.take(),
            [
                "interactive#0: void",
                "tempest: internal error (a bug in tempest, not in your code): as a test asked",
                "interactive#1: fail",
                "interactive#2: 3",
            ]
        );
    }

    #[test]
    fn names_carry_over_and_shadow() {
        assert_eq!(
            transcript(&["let x = 1", "x + 1", "let x = \"${x}!\"", "x"]),
            [
                "interactive#0: void",
                "interactive#1: 2",
                "interactive#2: void",
                "interactive#3: \"1!\""
            ]
        );
    }
}
