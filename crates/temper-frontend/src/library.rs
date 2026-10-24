//! One arena per library build: every file's tree, appended into one
//! [`Tree`] with one set of [`Names`] and literal values, and the
//! diagnostics so far.
//!
//! Positions are library-wide: each file's byte offsets are shifted by the
//! file's base, so a [`Span`] alone says which file it's in
//! ([`Library::locate`]). Each node keeps an origin link to the parse node
//! it came from, also shifted, so the files' parse results stay usable for
//! formatting and the language server.

use temper_syntax::ast::{self, Value};
use temper_syntax::lex::SourceKind;
use temper_syntax::module::{Genre, build_module};
use temper_syntax::names::{NameId, Names};
use temper_syntax::parse::{Parsed, parse};
use temper_syntax::tree::{NodeId, NodeKind, Span, Tree};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

/// A module: the files in one directory, which `import` names by its path.
#[derive(Debug)]
pub struct Module {
    /// Its directory, as `/a/b`, without a trailing `/`; `.` for the
    /// current directory.
    pub dir: String,
    pub files: Vec<FileId>,
}

/// A source file in the library.
#[derive(Debug)]
pub struct File {
    pub path: String,
    pub module: ModuleId,
    pub parsed: Parsed,
    /// Where its offsets start in library-wide positions.
    pub base: u32,
    /// Where its parse nodes start in library-wide origins.
    pub origin_base: u32,
    /// Its module code, a block.
    pub root: NodeId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// A problem found while building, at a library-wide span.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub level: Level,
    pub message: Message,
    pub span: Span,
}

/// What a diagnostic says, named as Kotlin Temper's message templates
/// where there's one.
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// From the grammar step.
    Syntax(ast::Message),
    UndeclaredName(String),
    /// A builtin used in a way it doesn't support, by its name.
    MalformedSpecial(String),
    MalformedDeclaration,
    MalformedFunction,
    /// Assigning to something that isn't a `var`.
    IllegalAssignment(String),
    /// `break` or `continue` with no loop or label to go to.
    NoJumpTarget(&'static str),
    ReturnOutsideFunction,
    /// `for (x of xs)` without `let`.
    ExpectedDeclarationForOf,
    /// `for (let x = 1 of xs)`.
    OfDeclarationInitializer,
    /// A name declared after a statement's macro name resolved past it.
    DeclaredAfterUse(String),
    /// In a `when` block, something that's not a case.
    InvalidBlockContent,
    /// A `when` case without `->` and a value.
    MissingCaseValue,
    /// A `when` case after `else`.
    ElseMustBeLast,
    /// `import` of a module that isn't there, or isn't supported yet.
    ImportFailed(String),
    /// Importing a name the module doesn't export: the module, the name.
    NotExported(String, String),
    /// An object literal whose property names no visible constructor takes.
    NoSignatureMatches,
    /// An object literal that several classes' constructors match: their names.
    MultipleConstructorsMatch(String),
    /// A constructor parameter without a default that an object literal skips.
    NoValuePassed(String),
    /// Ours: a feature the settle loop doesn't have yet.
    Unsupported(String),
    /// Ours: the item never settled; the chain of what it waited on.
    NeverSettled(String),
    /// Ours: the step budget ran out.
    OutOfSteps(u64),
}

impl Message {
    pub fn text(&self) -> String {
        match self {
            Message::Syntax(m) => m.text(),
            Message::UndeclaredName(n) => format!("{n} has not been declared"),
            Message::MalformedSpecial(n) => format!("Use of builtin {n} is malformed"),
            Message::MalformedDeclaration => "Declaration is malformed".into(),
            Message::MalformedFunction => "Function is malformed".into(),
            Message::IllegalAssignment(n) => format!("Cannot assign to {n}, which is not a var"),
            Message::NoJumpTarget(what) => format!("No enclosing loop or label for {what}"),
            Message::ReturnOutsideFunction => "return outside a function".into(),
            Message::ExpectedDeclarationForOf => "Declaration required for `of`".into(),
            Message::OfDeclarationInitializer => "Initializer not allowed on `of` declaration".into(),
            Message::DeclaredAfterUse(n) => format!("{n} is declared after a use that it would shadow"),
            Message::InvalidBlockContent => "Invalid block content".into(),
            Message::MissingCaseValue => "Missing case value".into(),
            Message::ElseMustBeLast => "Other cases are invalid after else".into(),
            Message::ImportFailed(path) => format!("Import of {path} failed"),
            Message::NotExported(module, name) => format!("{module} does not export symbol {name}"),
            Message::NoSignatureMatches => "No signature matches".into(),
            Message::MultipleConstructorsMatch(names) => format!("Multiple types have matching constructors: {names}"),
            Message::NoValuePassed(name) => format!("No value passed for {name}"),
            Message::Unsupported(what) => format!("Not supported yet: {what}"),
            Message::NeverSettled(chain) => format!("Never settled, waiting on {chain}"),
            Message::OutOfSteps(n) => format!("Gave up after {n} steps"),
        }
    }
}

/// The arena for one library build.
#[derive(Debug, Default)]
pub struct Library {
    pub tree: Tree,
    pub names: Names,
    values: Vec<Value>,
    origins: Vec<NodeId>,
    pub files: Vec<File>,
    pub modules: Vec<Module>,
    pub diagnostics: Vec<Diagnostic>,
    next_base: u32,
    next_origin: u32,
}

impl Library {
    pub fn new() -> Library {
        Library::default()
    }

    /// Parses a file and appends its tree, in the module for its directory.
    pub fn add_file(&mut self, path: &str, source: &str, kind: SourceKind) -> FileId {
        let dir = normalize(path.rsplit_once('/').map_or(".", |(dir, _)| if dir.is_empty() { "/" } else { dir }));
        let module = match self.modules.iter().position(|m| m.dir == dir) {
            Some(i) => ModuleId(i as u32),
            None => {
                self.modules.push(Module { dir, files: Vec::new() });
                ModuleId(self.modules.len() as u32 - 1)
            }
        };
        self.modules[module.0 as usize].files.push(FileId(self.files.len() as u32));
        let parsed = parse(source, kind);
        let ast = build_module(&parsed, Genre::Library);
        let base = self.next_base;
        let origin_base = self.next_origin;
        // Library-wide names for the file's.
        let names: Vec<NameId> = (0..ast.names().len() as u32)
            .map(|i| {
                let id = NameId(i);
                let text = ast.names().get(id);
                if ast.names().is_temporary(id) { self.names.temporary(text) } else { self.names.intern(text) }
            })
            .collect();
        let node_base = self.tree.len() as u32;
        let tree = ast.tree();
        let mut children = Vec::new();
        for i in 0..tree.len() as u32 {
            let id = NodeId(i);
            let kind = tree.kind(id);
            let data = match kind {
                NodeKind::LeftName | NodeKind::RightName | NodeKind::BuiltinName => names[tree.data(id) as usize].0,
                NodeKind::Value => {
                    let value = match ast.value(id).unwrap() {
                        Value::Symbol(name) => Value::Symbol(names[name.0 as usize]),
                        value => value.clone(),
                    };
                    self.values.push(value);
                    self.values.len() as u32 - 1
                }
                // Error nodes point at diagnostics, appended below.
                NodeKind::Error if tree.data(id) != u32::MAX => tree.data(id) + self.diagnostics.len() as u32,
                _ => tree.data(id),
            };
            children.clear();
            children.extend(tree.children(id).iter().map(|c| NodeId(c.0 + node_base)));
            let span = tree.span(id);
            let span = Span { start: span.start + base, end: span.end + base };
            self.tree.add_node(kind, span, data, &children);
            self.origins.push(NodeId(ast.origin(id).0 + origin_base));
        }
        for d in ast.diagnostics() {
            let level = match d.level {
                ast::Level::Info => Level::Info,
                ast::Level::Warn => Level::Warn,
                ast::Level::Error => Level::Error,
            };
            let span = Span { start: d.span.start + base, end: d.span.end + base };
            self.diagnostics.push(Diagnostic { level, message: Message::Syntax(d.message.clone()), span });
        }
        let mut root = NodeId(ast.root().0 + node_base);
        // Code that's one labeled statement is that statement's block;
        // wrap it, so the module's block holds the labeled one.
        let labeled = self.tree.children(root).first().is_some_and(|&c| {
            self.symbol(c).is_some_and(|s| self.names.get(s) == "label")
        });
        if labeled {
            root = self.add(NodeKind::Block, self.tree.span(root), 0, &[root], root);
        }
        // One past the end, so an empty span at a file's end is still its.
        self.next_base = base + source.len() as u32 + 1;
        self.next_origin = origin_base + parsed.tree().len() as u32;
        self.files.push(File { path: path.to_string(), module, parsed, base, origin_base, root });
        FileId(self.files.len() as u32 - 1)
    }

    /// The directory a relative import path, as `./b/` or `../c`, names
    /// from a module, or `None` for other paths, like `std/temporal`.
    pub fn import_dir(&self, from: ModuleId, path: &str) -> Option<String> {
        if !(path.starts_with("./") || path.starts_with("../")) {
            return None;
        }
        Some(normalize(&format!("{}/{path}", self.modules[from.0 as usize].dir)))
    }

    /// The module for a directory, if it's loaded.
    pub fn module_at(&self, dir: &str) -> Option<ModuleId> {
        self.modules.iter().position(|m| m.dir == dir).map(|i| ModuleId(i as u32))
    }

    /// The directories that relative `import("…")` calls name and that
    /// aren't loaded yet, for a driver to load before settling.
    pub fn missing_imports(&self) -> Vec<String> {
        let mut missing = Vec::new();
        for i in 0..self.tree.len() as u32 {
            let node = NodeId(i);
            let [callee, arg] = *self.tree.children(node) else { continue };
            if self.tree.kind(node) != NodeKind::Call || self.name(callee).is_none_or(|n| self.names.get(n) != "import") {
                continue;
            }
            let Some(Value::String(path)) = self.value(arg) else { continue };
            let module = self.files[self.locate(self.tree.span(node).start).0.0 as usize].module;
            if let Some(dir) = self.import_dir(module, path) {
                if self.module_at(&dir).is_none() && !missing.contains(&dir) {
                    missing.push(dir);
                }
            }
        }
        missing
    }

    /// The file a library-wide position is in, and the offset in it.
    pub fn locate(&self, pos: u32) -> (FileId, u32) {
        let i = self.files.partition_point(|f| f.base <= pos).saturating_sub(1);
        (FileId(i as u32), pos - self.files.get(i).map_or(0, |f| f.base))
    }

    /// The parse node a node came from, in its file's parse tree.
    pub fn origin(&self, id: NodeId) -> (FileId, NodeId) {
        let global = self.origins[id.0 as usize].0;
        let i = self.files.partition_point(|f| f.origin_base <= global).saturating_sub(1);
        (FileId(i as u32), NodeId(global - self.files[i].origin_base))
    }

    /// A value node's value.
    pub fn value(&self, id: NodeId) -> Option<&Value> {
        (self.tree.kind(id) == NodeKind::Value).then(|| &self.values[self.tree.data(id) as usize])
    }

    /// A name node's name.
    pub fn name(&self, id: NodeId) -> Option<NameId> {
        matches!(self.tree.kind(id), NodeKind::LeftName | NodeKind::RightName | NodeKind::BuiltinName)
            .then(|| NameId(self.tree.data(id)))
    }

    /// A symbol value's name, as `\else`.
    pub fn symbol(&self, id: NodeId) -> Option<NameId> {
        match self.value(id) {
            Some(Value::Symbol(name)) => Some(*name),
            _ => None,
        }
    }

    /// Adds a node made by a rewrite, with the origin of `like`.
    pub fn add(&mut self, kind: NodeKind, span: Span, data: u32, children: &[NodeId], like: NodeId) -> NodeId {
        self.origins.push(self.origins[like.0 as usize]);
        self.tree.add_node(kind, span, data, children)
    }

    /// A copy of a subtree, with fresh node ids, sharing values.
    pub fn copy(&mut self, id: NodeId) -> NodeId {
        let children: Vec<NodeId> = self.tree.children(id).to_vec().into_iter().map(|c| self.copy(c)).collect();
        self.add(self.tree.kind(id), self.tree.span(id), self.tree.data(id), &children, id)
    }

    pub fn add_value(&mut self, value: Value, span: Span, like: NodeId) -> NodeId {
        self.values.push(value);
        let index = self.values.len() as u32 - 1;
        self.add(NodeKind::Value, span, index, &[], like)
    }

    pub fn add_name(&mut self, kind: NodeKind, name: NameId, span: Span, like: NodeId) -> NodeId {
        self.add(kind, span, name.0, &[], like)
    }

    /// Reports a problem.
    pub fn report(&mut self, level: Level, message: Message, span: Span) -> u32 {
        self.diagnostics.push(Diagnostic { level, message, span });
        self.diagnostics.len() as u32 - 1
    }

    /// Rewrites a node into an error node, reporting why.
    pub fn error(&mut self, id: NodeId, message: Message) {
        let index = self.report(Level::Error, message, self.tree.span(id));
        self.tree.replace(id, NodeKind::Error, index, &[]);
    }

    /// The message an error node carries.
    pub fn error_text(&self, id: NodeId) -> String {
        match self.diagnostics.get(self.tree.data(id) as usize) {
            Some(d) => d.message.text(),
            None => "Malformed code".to_string(),
        }
    }
}

/// A path with `.` and `..` parts resolved and no trailing `/`, as far as
/// that can be done from the text: `a/./b/../c/` is `a/c`.
fn normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|&p| p != "..") => {
                parts.pop();
            }
            ".." if absolute => {}
            part => parts.push(part),
        }
    }
    match (absolute, parts.is_empty()) {
        (true, _) => format!("/{}", parts.join("/")),
        (false, true) => ".".to_string(),
        (false, false) => parts.join("/"),
    }
}
