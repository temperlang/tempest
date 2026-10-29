//! One arena per library build: every file's tree, appended into one
//! [`Tree`] with one set of [`Names`] and literal values, and the
//! diagnostics so far.
//!
//! Positions are library-wide: each file's byte offsets are shifted by the
//! file's base, so a [`Span`] alone says which file it's in
//! ([`Library::locate`]). Each node keeps an origin link to the parse node
//! it came from, also shifted, so the files' parse results stay usable for
//! formatting and the language server.

use crate::stdlib;
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
    /// Assigning to something that isn't a name, member or element.
    MalformedAssignment,
    /// Assigning to something that isn't a `var`.
    IllegalAssignment(String),
    /// `break` or `continue` with no loop or label to go to.
    NoJumpTarget(&'static str),
    ReturnOutsideFunction,
    /// `yield` or `await` outside any function.
    YieldingOutsideGeneratorFn(&'static str),
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
    /// A `when` case's test that isn't one, as a `case` without a pattern.
    InvalidCaseCondition,
    /// A second `as` for a destructured name.
    MultipleRenames,
    /// `...` in a destructuring declaration that isn't an import.
    WildcardWithoutImport,
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
    /// An object literal whose names fit another class's constructor than
    /// the type wanted there takes: the type wanted and that class.
    ObjectLiteralWantsOther(String, String),
    /// `class C extends D` where `D` is a class: their names.
    CannotExtendConcrete(String),
    /// `interface I(x: Int)`.
    ConstructorArgumentInInterfaceType,
    /// Ours: `interface I { x: Int = 1 }`; interfaces may define only
    /// abstract properties.
    InterfaceBackedProperty(String),
    /// A type parameter of an interface's method, which may be
    /// overridden, by its name.
    TypeParameterInInterfaceMethod(String),
    /// A class's members without `public`, `protected` or `private`: the
    /// class, and the members as Kotlin lists them, `[.x, get.y(...)]`.
    MissingMemberVisibility(String, String),
    /// A concrete class lacking an abstract member it inherits: the class,
    /// the member, the type declaring it, and a declaration to add.
    MissingMethodDefinition(String, String, String, String),
    /// An override narrower than what it overrides: the class, the member,
    /// its visibility, and the type declaring the overridden one.
    IncompatibleVisibility(String, String, String, String),
    /// An override whose signature differs from what it overrides: the
    /// class, the member as [`Message::IncompatibleVisibility`] names it,
    /// its signature, the one it should have, and the supertype.
    IncompatibleSignature(String, String, String, String, String),
    /// `this` outside a class or interface.
    ThisOutsideClassBody,
    /// `new I()` for an interface `I`.
    CannotInstantiateAbstractType(String),
    /// Assigning a member that isn't a `var`: the member, its class.
    IncompatibleUsage(String, String),
    /// A value of the wrong type stored: the target's type, the value's.
    /// Kotlin's `IllegalAssignment`.
    CannotAssign(String, String),
    /// A value of the wrong type passed, or null where it can't be: the
    /// expected type, the value's.
    ExpectedSubType(String, String),
    /// A condition that isn't a `Boolean`: the expected type, the value's.
    ExpectedValueOfType(String, String),
    /// A `Void` value stored or passed. Kotlin's `ExpectedNonVoid`.
    VoidAsValue,
    /// A call with too few or too many arguments: how many parameters.
    ArityMismatch(usize),
    /// A function without a return type where nothing gives it one.
    MissingReturnType,
    /// Arguments that don't fit the variant of a builtin that a call
    /// picks: the variant, the types it wants, and the arguments'. Kotlin's
    /// `SignatureInputMismatch`.
    SignatureMismatch(String, String, String),
    /// A method a class inherits from more than one supertype, none
    /// overriding the others: the inputs, `this` first, and each one's
    /// signature. Kotlin's `NoCalleeMatching`.
    NoCalleeMatches(String, String),
    /// A generic call's type parameter, written with its bound, solved to
    /// a type that doesn't fit the bound.
    TypeFormalBound(String, String, String),
    /// What may bubble, in a function whose return type doesn't say it
    /// may. Kotlin's `ExpectedNoBubble`.
    ExpectedNoBubble,
    /// `x is T` or `x as T` that can never succeed: `x`'s type, not
    /// null, and `T`. Kotlin's `ImpossibleRttiCheck`.
    ImpossibleRttiCheck(String, String),
    /// `x is T` or `x as T` that isn't allowed, and why: `T`, and `x`'s
    /// type.
    IllegalRttiCheck(RttiProblem, String, String),
    /// Information: `x is T` or `x as T` where `x` is a `T` already: `T`,
    /// and `x`'s type. Kotlin's `UnnecessaryRttiCheck`.
    UnnecessaryRttiCheck(String, String),
    /// Ours: a `@fun` interface's value where another `@fun` interface is
    /// wanted, whose function type it would fit, as named function types
    /// are nominal: its type, the type wanted, and a function literal
    /// that adapts it.
    FunNotOther(String, String, String),
    /// Ours: a function literal that initializes a `let` without a
    /// declared type, so nothing gives it its interface: the `let`'s name.
    UntypedFunLiteral(String),
    /// Ours: a `@fun` interface that breaks Kotlin Temper's documented
    /// rules for one: the interface, and how.
    FunInterfaceRule(String, FunRule),
    /// Ours: a regex literal that doesn't parse, and why.
    InvalidRegex(String),
    /// Ours: a feature the settle loop doesn't have yet.
    Unsupported(String),
    /// Ours: the item never settled; the chain of what it waited on.
    NeverSettled(String),
    /// Ours: the step budget ran out.
    OutOfSteps(u64),
}

/// Why a run-time type check, `x is T` or `x as T`, isn't allowed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RttiProblem {
    /// Neither type is a subtype of the other. Kotlin's
    /// `IllegalRttiCheckUpDown`.
    Unrelated,
    /// `T` is a type parameter. `IllegalRttiCheckFormals`.
    Formals,
    /// `T` has a type argument `x`'s type doesn't.
    /// `IllegalRttiCheckActuals`.
    Actuals,
    /// `T` is a final builtin type, which core marks
    /// `@mayDowncastTo(false)`. `IllegalRttiCheckMayDowncast`.
    MayDowncast,
    /// Ours: `T` is a `@fun` interface, which Kotlin Temper documents as
    /// not allowed but doesn't report.
    FunInterface,
}

/// How a `@fun` interface breaks the rules Kotlin Temper documents for
/// one (its `FunDecorator.kt`), which let backends make it either a named
/// interface or a plain function type. A type parameter of its `apply`
/// is Kotlin's error for any interface's method.
#[derive(Clone, Debug, PartialEq)]
pub enum FunRule {
    /// A supertype other than `AnyValue`, as written.
    Supertype(String),
    /// A member other than one method named `apply`, described, as
    /// `property n` or `static k`.
    Member(String),
    /// No `apply` method.
    NoApply,
    /// `@json`, which adds members.
    Json,
}

impl Message {
    pub fn text(&self) -> String {
        match self {
            Message::Syntax(m) => m.text(),
            Message::UndeclaredName(n) => format!("{n} has not been declared"),
            Message::MalformedSpecial(n) => format!("Use of builtin {n} is malformed"),
            Message::MalformedDeclaration => "Declaration is malformed".into(),
            Message::MalformedFunction => "Function is malformed".into(),
            Message::MalformedAssignment => "Malformed assignment".into(),
            Message::IllegalAssignment(n) => format!("Cannot assign to {n}, which is not a var"),
            Message::NoJumpTarget(what) => format!("No enclosing loop or label for {what}"),
            Message::ReturnOutsideFunction => "return outside a function".into(),
            Message::YieldingOutsideGeneratorFn(what) => {
                format!("{what} outside generator function body")
            }
            Message::ExpectedDeclarationForOf => "Declaration required for `of`".into(),
            Message::OfDeclarationInitializer => {
                "Initializer not allowed on `of` declaration".into()
            }
            Message::DeclaredAfterUse(n) => {
                format!("{n} is declared after a use that it would shadow")
            }
            Message::InvalidBlockContent => "Invalid block content".into(),
            Message::MissingCaseValue => "Missing case value".into(),
            Message::ElseMustBeLast => "Other cases are invalid after else".into(),
            Message::InvalidCaseCondition => "Invalid case condition".into(),
            Message::MultipleRenames => "Extra rename not allowed".into(),
            Message::WildcardWithoutImport => "Wildcard destructure allowed only for import".into(),
            Message::ImportFailed(path) => format!("Import of {path} failed"),
            Message::NotExported(module, name) => format!("{module} does not export symbol {name}"),
            Message::NoSignatureMatches => "No signature matches".into(),
            Message::MultipleConstructorsMatch(names) => {
                format!("Multiple types have matching constructors: {names}")
            }
            Message::NoValuePassed(name) => format!("No value passed for {name}"),
            Message::ObjectLiteralWantsOther(wanted, class) => {
                format!("{wanted} is wanted here, but these properties fit {class}'s constructor")
            }
            Message::CannotExtendConcrete(names) => {
                format!("Cannot extend concrete type(s) {names}")
            }
            Message::ConstructorArgumentInInterfaceType => {
                "Constructor parameters are not allowed on interface types".into()
            }
            Message::TypeParameterInInterfaceMethod(name) => {
                format!("Illegal type parameter {name}. Overridable methods don't allow generics")
            }
            Message::InterfaceBackedProperty(name) => {
                format!("Interfaces may not define backed properties like {name}")
            }
            Message::MissingMemberVisibility(class, members) => {
                format!("Members of class {class} require explicit visibility: {members}")
            }
            Message::MissingMethodDefinition(class, member, from, add) => {
                format!("Type {class} must implement {member} from {from}.  Maybe add `{add}`")
            }
            Message::IncompatibleVisibility(class, member, visibility, from) => format!(
                "Type {class} has method {member} but it's visibility is {visibility} which is \
                 narrower than the method it overrides in {from}"
            ),
            Message::IncompatibleSignature(class, member, sig, wanted, from) => format!(
                "Type {class} has method {member} with signature {sig}, but it should be \
                 {wanted} to correctly override from {from}"
            ),
            Message::ThisOutsideClassBody => {
                "`this` may only appear inside a type definition".into()
            }
            Message::CannotInstantiateAbstractType(name) => {
                format!("Cannot instantiate abstract type {name}")
            }
            Message::IncompatibleUsage(member, class) => {
                format!("Member {member} defined in {class} incompatible with usage")
            }
            Message::CannotAssign(to, from) => format!("Cannot assign to {to} from {from}"),
            Message::ExpectedSubType(expected, got) => {
                format!("Expected subtype of {expected}, but got {got}")
            }
            Message::ExpectedValueOfType(expected, got) => {
                format!("Expected value of type {expected} not {got}")
            }
            Message::VoidAsValue => "Void expressions cannot be used as values".into(),
            Message::ArityMismatch(n) => format!("Wrong number of arguments.  Expected {n}"),
            Message::MissingReturnType => "Explicit return type required".into(),
            Message::SignatureMismatch(sig, expected, got) => format!(
                "Actual arguments do not match signature: {sig} expected {expected}, but got {got}"
            ),
            Message::TypeFormalBound(formal, actual, bound) => format!(
                "Type formal <{formal}> cannot bind to {actual} which does not fit upper bounds [{bound}]"
            ),
            Message::NoCalleeMatches(inputs, among) => {
                format!("No callee matches inputs [{inputs}] among [{among}]")
            }
            Message::ExpectedNoBubble => {
                "Cannot bubble from a function without Bubble in its return type".into()
            }
            Message::ImpossibleRttiCheck(from, to) => {
                format!("Runtime type check from {from} to {to} can never succeed")
            }
            Message::IllegalRttiCheck(problem, target, from) => {
                const ERASED: &str = ", because a type argument's run-time type can't be checked";
                let (what, why) = match problem {
                    RttiProblem::Unrelated => ("Unrelated types cannot be targeted", ""),
                    RttiProblem::Actuals => ("Type arguments cannot be introduced", ERASED),
                    RttiProblem::Formals => ("Type parameters cannot be targeted", ERASED),
                    RttiProblem::MayDowncast => (
                        "Types marked @mayDowncastTo(false) cannot be targeted",
                        " because they may not be distinct on all backends",
                    ),
                    RttiProblem::FunInterface => (
                        "@fun interfaces cannot be targeted",
                        ", because backends may make them plain function types",
                    ),
                };
                format!("{what} with is or as runtime type checks{why}: <[{target}]> from {from}")
            }
            Message::UnnecessaryRttiCheck(to, from) => format!(
                "Unnecessary type check to {to} from expression with type {from} which is a subtype"
            ),
            Message::FunNotOther(from, to, adapter) => format!(
                "{} {from} isn't {} {to}; to adapt it, wrap it: `{adapter}`",
                article(from, true),
                article(to, false)
            ),
            Message::UntypedFunLiteral(name) => format!(
                "{name} needs a declared type, such as a @fun interface, since a function \
                 literal takes its type from the type wanted"
            ),
            Message::FunInterfaceRule(name, rule) => match rule {
                FunRule::Supertype(t) => {
                    format!("@fun interface {name} may extend only AnyValue, not {t}")
                }
                FunRule::Member(m) => {
                    format!("@fun interface {name} may declare only an apply method, not {m}")
                }
                FunRule::NoApply => format!("@fun interface {name} must declare an apply method"),
                FunRule::Json => {
                    format!("@fun interface {name} may not be @json, which adds members")
                }
            },
            Message::InvalidRegex(why) => format!("Invalid regex: {why}"),
            Message::Unsupported(what) => format!("Not supported yet: {what}"),
            Message::NeverSettled(chain) => format!("Never settled, waiting on {chain}"),
            Message::OutOfSteps(n) => format!("Gave up after {n} steps"),
        }
    }
}

/// "A" or "An" before a type's name, capitalized or not, by its first
/// letter, as "An IntPred" and "a Pred".
fn article(name: &str, capital: bool) -> &'static str {
    let vowel = name.starts_with(['A', 'E', 'I', 'O', 'U', 'a', 'e', 'i', 'o', 'u']);
    match (vowel, capital) {
        (true, true) => "An",
        (true, false) => "an",
        (false, true) => "A",
        (false, false) => "a",
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
    /// Declares `testServerPort` with this value, as Kotlin Temper does
    /// for its functional tests.
    pub test_server_port: Option<i32>,
    next_base: u32,
    next_origin: u32,
}

impl Library {
    pub fn new() -> Library {
        Library::default()
    }

    /// Parses a file and appends its tree, in the module for its directory.
    pub fn add_file(&mut self, path: &str, source: &str, kind: SourceKind) -> FileId {
        let dir = normalize(
            path.rsplit_once('/')
                .map_or(".", |(dir, _)| if dir.is_empty() { "/" } else { dir }),
        );
        let module = match self.modules.iter().position(|m| m.dir == dir) {
            Some(i) => ModuleId(i as u32),
            None => {
                self.modules.push(Module {
                    dir,
                    files: Vec::new(),
                });
                ModuleId(self.modules.len() as u32 - 1)
            }
        };
        self.modules[module.0 as usize]
            .files
            .push(FileId(self.files.len() as u32));
        let parsed = parse(source, kind);
        let ast = build_module(&parsed, Genre::Library);
        let base = self.next_base;
        let origin_base = self.next_origin;
        // Library-wide names for the file's.
        let names: Vec<NameId> = (0..ast.names().len() as u32)
            .map(|i| {
                let id = NameId(i);
                let text = ast.names().get(id);
                if ast.names().is_temporary(id) {
                    self.names.temporary(text)
                } else {
                    self.names.intern(text)
                }
            })
            .collect();
        let node_base = self.tree.len() as u32;
        let tree = ast.tree();
        let mut children = Vec::new();
        for i in 0..tree.len() as u32 {
            let id = NodeId(i);
            let kind = tree.kind(id);
            let data = match kind {
                NodeKind::LeftName | NodeKind::RightName | NodeKind::BuiltinName => {
                    names[tree.data(id) as usize].0
                }
                NodeKind::Value => {
                    let value = match ast.value(id).unwrap() {
                        Value::Symbol(name) => Value::Symbol(names[name.0 as usize]),
                        value => value.clone(),
                    };
                    self.values.push(value);
                    self.values.len() as u32 - 1
                }
                // Error nodes point at diagnostics, appended below.
                NodeKind::Error if tree.data(id) != u32::MAX => {
                    tree.data(id) + self.diagnostics.len() as u32
                }
                _ => tree.data(id),
            };
            children.clear();
            children.extend(tree.children(id).iter().map(|c| NodeId(c.0 + node_base)));
            let span = tree.span(id);
            let span = Span {
                start: span.start + base,
                end: span.end + base,
            };
            self.tree.add_node(kind, span, data, &children);
            self.origins.push(NodeId(ast.origin(id).0 + origin_base));
        }
        for d in ast.diagnostics() {
            let level = match d.level {
                ast::Level::Info => Level::Info,
                ast::Level::Warn => Level::Warn,
                ast::Level::Error => Level::Error,
            };
            let span = Span {
                start: d.span.start + base,
                end: d.span.end + base,
            };
            self.diagnostics.push(Diagnostic {
                level,
                message: Message::Syntax(d.message.clone()),
                span,
            });
        }
        let mut root = NodeId(ast.root().0 + node_base);
        // Code that's one labeled statement is that statement's block;
        // wrap it, so the module's block holds the labeled one.
        let labeled = self
            .tree
            .children(root)
            .first()
            .is_some_and(|&c| self.symbol(c).is_some_and(|s| self.names.get(s) == "label"));
        if labeled {
            root = self.add(NodeKind::Block, self.tree.span(root), 0, &[root], root);
        }
        // One past the end, so an empty span at a file's end is still its.
        self.next_base = base + source.len() as u32 + 1;
        self.next_origin = origin_base + parsed.tree().len() as u32;
        self.files.push(File {
            path: path.to_string(),
            module,
            parsed,
            base,
            origin_base,
            root,
        });
        FileId(self.files.len() as u32 - 1)
    }

    /// The directory an import path names from a module: a relative one,
    /// as `./b/` or `../c`, or one in the standard library, as
    /// `std/temporal`, under [`Library::std_root`]. `None` for others.
    pub fn import_dir(&self, from: ModuleId, path: &str) -> Option<String> {
        if let Some(rest) = path.strip_prefix("std/") {
            return Some(normalize(&format!("{}/{rest}", self.std_root())));
        }
        if !(path.starts_with("./") || path.starts_with("../")) {
            return None;
        }
        Some(normalize(&format!(
            "{}/{path}",
            self.modules[from.0 as usize].dir
        )))
    }

    /// The directory std modules' directories are in, [`stdlib::DIR`],
    /// whether built in or mounted from elsewhere
    /// ([`Sources`](crate::sources::Sources)).
    pub fn std_root(&self) -> &str {
        stdlib::DIR
    }

    /// The module for a directory, if it's loaded.
    pub fn module_at(&self, dir: &str) -> Option<ModuleId> {
        self.modules
            .iter()
            .position(|m| m.dir == dir)
            .map(|i| ModuleId(i as u32))
    }

    /// The directories that `import("…")` calls name ([`Library::import_dir`])
    /// and that aren't loaded yet, for a driver to load before settling.
    /// A file with a `test(name, body)` call imports `std/testing`, and one
    /// with a regex literal, `std/regex`.
    pub fn missing_imports(&self) -> Vec<String> {
        let mut missing = Vec::new();
        for i in 0..self.tree.len() as u32 {
            let node = NodeId(i);
            if self.tree.kind(node) != NodeKind::Call {
                continue;
            }
            let path = match *self.tree.children(node) {
                [callee, arg]
                    if self
                        .name(callee)
                        .is_some_and(|n| self.names.get(n) == "import") =>
                {
                    match self.value(arg) {
                        Some(Value::String(path)) => path,
                        _ => continue,
                    }
                }
                [callee, _, _]
                    if self
                        .name(callee)
                        .is_some_and(|n| self.names.get(n) == "test") =>
                {
                    "std/testing"
                }
                [callee, _, _]
                    if self
                        .name(callee)
                        .is_some_and(|n| self.names.get(n) == "rgx") =>
                {
                    "std/regex"
                }
                _ => continue,
            };
            let module = self.files[self.locate(self.tree.span(node).start).0.0 as usize].module;
            if let Some(dir) = self.import_dir(module, path) {
                if self.module_at(&dir).is_none() && !missing.contains(&dir) {
                    missing.push(dir);
                }
            }
        }
        missing
    }

    /// The source text a span covers.
    pub fn source_text(&self, span: Span) -> &str {
        let (file, start) = self.locate(span.start);
        let end = start as usize + span.end.saturating_sub(span.start) as usize;
        self.files
            .get(file.0 as usize)
            .and_then(|f| f.parsed.source().get(start as usize..end))
            .unwrap_or("")
    }

    /// The file a library-wide position is in, and the offset in it.
    pub fn locate(&self, pos: u32) -> (FileId, u32) {
        let i = self
            .files
            .partition_point(|f| f.base <= pos)
            .saturating_sub(1);
        (
            FileId(i as u32),
            pos - self.files.get(i).map_or(0, |f| f.base),
        )
    }

    /// The parse node a node came from, in its file's parse tree.
    pub fn origin(&self, id: NodeId) -> (FileId, NodeId) {
        let global = self.origins[id.0 as usize].0;
        let i = self
            .files
            .partition_point(|f| f.origin_base <= global)
            .saturating_sub(1);
        (FileId(i as u32), NodeId(global - self.files[i].origin_base))
    }

    /// A value node's value.
    pub fn value(&self, id: NodeId) -> Option<&Value> {
        (self.tree.kind(id) == NodeKind::Value).then(|| &self.values[self.tree.data(id) as usize])
    }

    /// A name node's name.
    pub fn name(&self, id: NodeId) -> Option<NameId> {
        matches!(
            self.tree.kind(id),
            NodeKind::LeftName | NodeKind::RightName | NodeKind::BuiltinName
        )
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
    pub fn add(
        &mut self,
        kind: NodeKind,
        span: Span,
        data: u32,
        children: &[NodeId],
        like: NodeId,
    ) -> NodeId {
        self.origins.push(self.origins[like.0 as usize]);
        self.tree.add_node(kind, span, data, children)
    }

    /// A copy of a subtree, with fresh node ids, sharing values.
    pub fn copy(&mut self, id: NodeId) -> NodeId {
        // A copy of the ids first, since copying each child borrows `self`.
        let kids = self.tree.children(id).to_vec();
        let children: Vec<NodeId> = kids.into_iter().map(|c| self.copy(c)).collect();
        self.add(
            self.tree.kind(id),
            self.tree.span(id),
            self.tree.data(id),
            &children,
            id,
        )
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
        self.diagnostics.push(Diagnostic {
            level,
            message,
            span,
        });
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
pub(crate) fn normalize(path: &str) -> String {
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
