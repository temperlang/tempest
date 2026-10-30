//! The frontend's translation into tmpl, the backend IR
//! (notes/tmpl/README.md): a settled, typed library as a
//! [`temper_tmpl::Program`], for subset A so far (notes/tmpl/roadmap.md):
//! functions, parameters (`var` ones too), locals, module `let`s and
//! `var`s, `if`, `while` and `do … while` loops, `for` loops without
//! `continue`, `break`, `continue`, returns, calls of the library's
//! functions, the builtin types' operators, core's members as external
//! declarations, string templates, and `console.log` through a
//! compiler-made `~console`, as Kotlin does.
//!
//! Types come from [`Typing::expr`](crate::typing::Typing::expr) and
//! [`Typing::decls`](crate::typing::Typing), and what each call calls from
//! [`Typing::resolved`](crate::typing::Typing::resolved); nothing is
//! resolved again here. Core's members are copied, as used, from
//! temper-tmpl's core fixture (`tests/fixtures/core.tmpl`, which its tests
//! check against `core.temper`), so their signatures, connected marks, and
//! parameter names have one source.
//!
//! A library's own modules translate in the order they run, each after
//! those it imports, named by their paths from the library's root. An
//! import is a reference: `let { f } = import("./sub")` makes a
//! module-level `let f = f'`, which this follows to `f'`, so a call of
//! `f` calls the other module's function, and backends derive what each
//! module imports from what it refers to
//! ([`temper_tmpl::analysis::imports`]). What's used of std or of another
//! library under the work root ([`Library::libraries`](crate::library::Library))
//! becomes external declarations in a `uses` section for that library, in
//! the module of its path there, as `uses std 0.6.0 { module testing {
//! class Test {} } }`.
//!
//! A module's `test` blocks ([`crate::testing`]) become its `Test`s: each
//! a function taking std's `Test`, named in reports by the test's name,
//! which must be a constant string.
//!
//! Anything subset A doesn't cover gives [`NotYet`], saying what, so the
//! coverage count (`compat/count_tmpl.py`) can tally it; nothing here
//! panics on a construct it doesn't know.

use crate::builtins::{Builtin, Constant, Function};
use crate::core::{self, LET_VAR, LOOP_EACH, LOOP_NEXT};
use crate::library::{Level, ModuleId};
use crate::scope::{ScopeId, SymbolId};
use crate::settle::Settled;
use crate::types::{self as ft, TypeData as FTypeData};
use crate::typing::Resolved;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use temper_syntax::ast::Value;
use temper_syntax::tree::{NodeId as FNode, NodeKind};
use temper_tmpl::ops::Op;
use temper_tmpl::{
    Builder, DeclId, DeclKind, Facts, FileId, NodeId, Program, Span, TypeData, TypeId,
};

/// Why a library isn't translated: what it uses that tmpl's translation
/// doesn't cover yet, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    /// What isn't covered, short and general, as "a class" or "a nullable
    /// type", so the coverage count can group by it.
    pub what: String,
    /// Where, as `path:line:column`, if somewhere in particular.
    pub at: Option<String>,
}

impl std::fmt::Display for NotYet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.at {
            Some(at) => write!(f, "not yet in tmpl: {} at {at}", self.what),
            None => write!(f, "not yet in tmpl: {}", self.what),
        }
    }
}

type Tx<T> = Result<T, NotYet>;

#[path = "to_tmpl_classes.rs"]
mod classes;
use classes::Member;

/// Core's members that tmpl knows, as external declarations.
const CORE_FIXTURE: &str = include_str!("../../temper-tmpl/tests/fixtures/core.tmpl");

/// Core's declarations that tmpl knows, and each by its key: `Type.member`
/// for a member, as typing's resolutions name builtins, or a top-level
/// function's name.
struct Catalog {
    program: Program,
    keys: HashMap<String, DeclId>,
    version: String,
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let program = temper_tmpl::text::read(CORE_FIXTURE).expect("core.tmpl reads");
        let mut keys = HashMap::new();
        for d in program.decls.ids() {
            if !program.decls.contains(d) {
                continue;
            }
            let decl = program.decls.get(d);
            let owner = decl.owner;
            if owner.is_none() || !program.decls.contains(owner) {
                continue;
            }
            let key = match program.decls.get(owner).kind {
                DeclKind::Library => program.name(d).to_string(),
                DeclKind::Class | DeclKind::Interface => {
                    format!("{}.{}", program.name(owner), program.name(d))
                }
                _ => continue,
            };
            keys.insert(key, d);
        }
        let version = program.uses.first().map_or("0.1.0", |u| &u.version);
        let version = version.to_string();
        Catalog {
            program,
            keys,
            version,
        }
    })
}

/// std's version, as its config gives it, for `uses std` sections.
const STD_VERSION: &str = "0.6.0";

/// Where a settled module comes from: the library being translated, std,
/// or another library under the work root, with its path from that
/// library's root (empty for the root).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Home {
    Own(String),
    Other { library: String, path: String },
}

/// Each module's home, by [`ModuleId`]: std's under [`stdlib::DIR`], other
/// libraries' under their roots, and the rest the library's own, whose
/// root is the directory of its first file's module (or the library root
/// holding it).
fn homes(s: &Settled) -> Vec<Home> {
    let lib = &s.library;
    let std_root = lib.std_root();
    let main = lib
        .files
        .first()
        .map(|f| lib.modules[f.module.0 as usize].dir.as_str());
    let main = main.unwrap_or(".");
    let under = |dir: &str, root: &str| -> Option<String> {
        if dir == root {
            return Some(String::new());
        }
        let rest = dir.strip_prefix(root)?.strip_prefix('/')?;
        Some(rest.trim_end_matches('/').to_string())
    };
    let own_root = (lib.libraries.iter())
        .map(|(_, root)| root.as_str())
        .filter(|root| under(main, root).is_some())
        .max_by_key(|root| root.len())
        .unwrap_or(main)
        .to_string();
    (lib.modules.iter())
        .map(|m| {
            if let Some(path) = under(&m.dir, std_root) {
                return Home::Other {
                    library: "std".into(),
                    path,
                };
            }
            let other = (lib.libraries.iter())
                .filter(|(_, root)| *root != own_root)
                .filter_map(|(name, root)| Some((name, root, under(&m.dir, root)?)))
                .max_by_key(|(_, root, _)| root.len());
            match (other, under(&m.dir, &own_root)) {
                (Some((_, root, _)), Some(_)) if root.len() <= own_root.len() => {
                    Home::Own(under(&m.dir, &own_root).unwrap_or_default())
                }
                (Some((name, _, path)), _) => Home::Other {
                    library: name.clone(),
                    path,
                },
                (None, own) => Home::Own(own.unwrap_or_else(|| m.dir.trim_matches('/').into())),
            }
        })
        .collect()
}

/// Translates a settled library into tmpl, named `library`: its own
/// modules, with what it uses of core, std, and other libraries. Fails
/// with what isn't covered yet, or if the library has errors.
pub fn to_tmpl(settled: &Settled, library: &str) -> Result<Program, NotYet> {
    let lib = &settled.library;
    if let Some(d) = (lib.diagnostics.iter()).find(|d| d.level == Level::Error) {
        return Err(NotYet {
            what: "a library with errors".into(),
            at: Some(position(settled, d.span.start)),
        });
    }
    let mut t = Translator {
        s: settled,
        b: Builder::new(library, "0.0.0"),
        decls: HashMap::new(),
        functions: HashSet::new(),
        loops: HashMap::new(),
        module: DeclId::NONE,
        owner: DeclId::NONE,
        result: TypeId::NONE,
        files: HashMap::new(),
        core: None,
        externals: HashMap::new(),
        console: None,
        homes: homes(settled),
        scopes: (settled.modules.iter()).map(|m| (m.scope, m.id)).collect(),
        libraries: HashMap::new(),
        external_modules: HashMap::new(),
        external_lets: HashMap::new(),
        external_types: HashMap::new(),
        classes: HashMap::new(),
        class_order: Vec::new(),
        members: HashMap::new(),
        ctors: HashMap::new(),
        this_class: None,
    };
    // Modules first, then their classes, so that any module may name any
    // class, then the classes' members, then bodies.
    let mut own = Vec::new();
    for module in &settled.modules {
        if let Home::Own(path) = &t.homes[module.id.0 as usize] {
            let decl = if path.is_empty() {
                let name = t.b.program().library.name.clone();
                t.b.module(&name)
            } else {
                let decl = t.b.module(path);
                t.b.set_module_path(decl, path);
                decl
            };
            t.module_files(module, decl);
            own.push((module, decl));
        }
    }
    for &(module, decl) in &own {
        t.declare_classes(decl, &module.top_levels)?;
    }
    for class in t.class_order.clone() {
        t.declare_members(class)?;
    }
    for (module, decl) in own {
        t.module(module, decl)?;
    }
    Ok(t.b.finish())
}

/// Where a library-wide position is, as `path:line:column`.
fn position(s: &Settled, pos: u32) -> String {
    let lib = &s.library;
    let (file, offset) = lib.locate(pos);
    let Some(f) = lib.files.get(file.0 as usize) else {
        return "?".into();
    };
    let source = f.parsed.source();
    let before = source.get(..offset as usize).unwrap_or(source);
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    format!("{}:{line}:{column}", f.path)
}

/// A 128-bit FNV-1a hash of a file's text, to match it later.
fn hash(text: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (half, seed) in [0xcbf29ce484222325u64, 0x84222325cbf29ce4]
        .iter()
        .enumerate()
    {
        let mut h = *seed;
        for &byte in text.as_bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        out[half * 8..half * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}

struct Translator<'a> {
    s: &'a Settled,
    b: Builder,
    /// The library's declarations, by their `Let`s.
    decls: HashMap<FNode, DeclId>,
    /// The `Let`s that declare functions.
    functions: HashSet<FNode>,
    /// Each `Loop`'s label.
    loops: HashMap<FNode, DeclId>,
    module: DeclId,
    /// What owns locals and labels: the function being translated, or the
    /// module in its init block.
    owner: DeclId,
    /// The function's result, or `NONE` in the init block.
    result: TypeId,
    /// tmpl's files, by the library's.
    files: HashMap<u32, FileId>,
    /// `temper-core`'s declaration, once something of it is used.
    core: Option<DeclId>,
    /// Core's declarations used, by the catalog's.
    externals: HashMap<DeclId, DeclId>,
    /// The module's `~console`, and the call that first needed it.
    console: Option<(DeclId, FNode)>,
    /// Each settled module's home, by [`ModuleId`].
    homes: Vec<Home>,
    /// Modules by their scopes.
    scopes: HashMap<ScopeId, ModuleId>,
    /// Other libraries' `uses` declarations, by name.
    libraries: HashMap<String, DeclId>,
    /// Other libraries' modules, by library and path.
    external_modules: HashMap<(String, String), DeclId>,
    /// Other libraries' functions and module `let`s, by their `Let`s.
    external_lets: HashMap<FNode, DeclId>,
    /// Other libraries' classes and interfaces, by their symbols.
    external_types: HashMap<SymbolId, DeclId>,
    /// The library's classes and interfaces, by their nodes.
    classes: HashMap<FNode, DeclId>,
    /// Their nodes, in the order declared.
    class_order: Vec<FNode>,
    /// Their members, by their `Let`s.
    members: HashMap<FNode, Member>,
    /// Each class's constructor, the `Let` of its own if it has one, and
    /// its parameters' `Let`s: its own's, or its header's fields.
    ctors: HashMap<DeclId, (DeclId, Option<FNode>, Vec<FNode>)>,
    /// The class or interface whose member is being translated.
    this_class: Option<(FNode, DeclId)>,
}

impl Translator<'_> {
    fn not_yet<T>(&self, node: FNode, what: impl Into<String>) -> Tx<T> {
        let start = self.s.library.tree.span(node).start;
        Err(NotYet {
            what: what.into(),
            at: Some(position(self.s, start)),
        })
    }

    fn kind(&self, node: FNode) -> NodeKind {
        self.s.library.tree.kind(node)
    }

    fn children(&self, node: FNode) -> &[FNode] {
        self.s.library.tree.children(node)
    }

    fn data(&self, node: FNode) -> u32 {
        self.s.library.tree.data(node)
    }

    fn name_of(&self, name: FNode) -> &str {
        let lib = &self.s.library;
        lib.name(name).map_or("", |n| lib.names.get(n))
    }

    fn function_of(&self, node: FNode) -> Option<Function> {
        match self.s.scopes.resolved_builtin(node) {
            Some(Builtin::Function(f)) => Some(f),
            _ => None,
        }
    }

    /// The `Let` a name resolved to.
    fn declaration(&self, name: FNode) -> Option<FNode> {
        let symbol = self.s.scopes.resolved(name)?;
        self.s.scopes.symbol(symbol).decl
    }

    fn is_exported(&self, decl: FNode) -> bool {
        let name = self.children(decl)[0];
        (self.s.scopes.resolved(name)).is_some_and(|s| self.s.scopes.symbol(s).exported)
    }

    // Spans.

    fn span(&self, node: FNode) -> Span {
        let lib = &self.s.library;
        let span = lib.tree.span(node);
        let (file, start) = lib.locate(span.start);
        match self.files.get(&file.0) {
            Some(&file) => Span {
                file,
                start,
                end: start + span.end.saturating_sub(span.start),
            },
            None => Span::UNKNOWN,
        }
    }

    /// Gives a node the span of what it translates.
    fn own(&mut self, node: NodeId, from: FNode) -> NodeId {
        let span = self.span(from);
        self.b.set_span(node, span, false);
        node
    }

    /// Gives a node made for `from` its span, marked derived.
    fn made(&mut self, node: NodeId, from: FNode) -> NodeId {
        let span = self.span(from);
        self.b.set_span(node, span, true);
        node
    }

    /// Gives a declaration its `Let`'s spans: the whole, and its name.
    fn decl_spans(&mut self, decl: DeclId, from: FNode) {
        let name = self.children(from).first().copied().unwrap_or(from);
        let (span, name_span) = (self.span(from), self.span(name));
        self.b.set_decl_spans(decl, span, name_span);
    }

    // Types.

    fn ty(&mut self, t: ft::TypeId, at: FNode) -> Tx<TypeId> {
        if let FTypeData::Class(symbol) = *self.s.typing.types.get(t) {
            let own = self
                .class_node(symbol)
                .and_then(|c| self.classes.get(&c).copied());
            if let Some(decl) = own {
                return Ok(self.b.nominal(decl));
            }
            if let Some(decl) = self.external_type(symbol, at)? {
                return Ok(self.b.nominal(decl));
            }
        }
        Ok(match t {
            ft::VOID => TypeId::VOID,
            ft::BOOLEAN => TypeId::BOOLEAN,
            ft::INT32 => TypeId::INT32,
            ft::INT64 => TypeId::INT64,
            ft::FLOAT64 => TypeId::FLOAT64,
            ft::STRING => TypeId::STRING,
            ft::ANY_VALUE => return self.not_yet(at, "type AnyValue"),
            _ => {
                let what = match self.s.typing.types.get(t) {
                    FTypeData::Unknown => "an expression of unknown type",
                    FTypeData::Never => "type Never",
                    FTypeData::Null => "null",
                    FTypeData::Builtin(_) => "a builtin generic type",
                    FTypeData::Class(_) => "a class or interface type",
                    FTypeData::Param(_) => "a type parameter",
                    FTypeData::Applied { .. } => "a generic type",
                    FTypeData::Nullable(_) => "a nullable type",
                    FTypeData::Fn { .. } => "a function type",
                };
                return self.not_yet(at, what);
            }
        })
    }

    /// An expression's type.
    fn expr_ty(&mut self, node: FNode) -> Tx<TypeId> {
        match self.s.typing.expr(node) {
            Some(t) => self.ty(t, node),
            None => self.not_yet(node, "an expression of unknown type"),
        }
    }

    /// A declaration's type, by its `Let`.
    fn decl_ty(&mut self, decl: FNode) -> Tx<TypeId> {
        let typing = &self.s.typing;
        let init = core::let_parts(&self.s.library.tree, decl).2;
        let t = (typing.decls.get(&decl).copied()).or_else(|| init.and_then(|i| typing.expr(i)));
        match t {
            Some(t) => self.ty(t, decl),
            None => self.not_yet(decl, "an expression of unknown type"),
        }
    }

    /// A function's signature, from its `Fn`'s type.
    fn signature(&mut self, f: FNode) -> Tx<TypeId> {
        let Some(t) = self.s.typing.expr(f) else {
            return self.not_yet(f, "an expression of unknown type");
        };
        let FTypeData::Fn {
            params,
            ret,
            throws,
        } = self.s.typing.types.get(t)
        else {
            return self.not_yet(f, "a function of unknown type");
        };
        let (params, ret, throws) = (params.clone(), *ret, *throws);
        let mut types = Vec::new();
        for p in &params {
            if p.optional {
                return self.not_yet(f, "a parameter's default");
            }
            types.push(self.ty(p.ty, f)?);
        }
        let result = self.ty(ret, f)?;
        Ok(self.b.function_type(&types, result, throws))
    }

    // Core's declarations.

    /// Core's declaration with this key, as `Int32.toString`, copied into
    /// the program's `uses` section the first time.
    fn core_decl(&mut self, key: &str) -> Option<DeclId> {
        let &c = catalog().keys.get(key)?;
        Some(self.external(c))
    }

    fn external(&mut self, c: DeclId) -> DeclId {
        if let Some(&d) = self.externals.get(&c) {
            return d;
        }
        let cat = &catalog().program;
        let decl = *cat.decls.get(c);
        let owner = if cat.decls.get(decl.owner).kind == DeclKind::Library {
            match self.core {
                Some(core) => core,
                None => {
                    let core = self.b.uses("temper-core", &catalog().version);
                    self.core = Some(core);
                    core
                }
            }
        } else {
            self.external(decl.owner)
        };
        let name = cat.name(c);
        let d = match decl.kind {
            DeclKind::Class | DeclKind::Interface => self.b.declare_type(decl.kind, name, owner),
            kind => {
                let ty = self.core_type(decl.ty);
                let d = self.b.declare(kind, name, owner, ty);
                let names: Vec<&str> = (cat.decls.param_names(c).iter())
                    .map(|&n| cat.strings.get(n))
                    .collect();
                self.b.set_param_names(d, &names);
                d
            }
        };
        self.b.add_facts(d, decl.facts);
        let backends: Vec<&str> = (cat.decls.backends(c).iter())
            .map(|&n| cat.strings.get(n))
            .collect();
        if !backends.is_empty() {
            self.b.set_connected(d, &backends);
        }
        self.externals.insert(c, d);
        d
    }

    /// A catalog's type as this program's.
    fn core_type(&mut self, t: TypeId) -> TypeId {
        let cat = &catalog().program;
        match cat.types.get(t) {
            TypeData::Scalar(_) => t,
            TypeData::Nominal { decl } => {
                let d = self.external(decl);
                self.b.nominal(d)
            }
            TypeData::Function { .. } => {
                let sig = cat.types.signature(t).expect("a signature");
                let params: Vec<TypeId> = sig.params.iter().map(|&p| self.core_type(p)).collect();
                let result = self.core_type(sig.result);
                (self.b).function_type_with(&params, sig.optional, result, sig.bubbles)
            }
        }
    }

    // Other modules' declarations.

    /// The module a top-level `Let`'s symbol is declared in, if it's
    /// top-level.
    fn module_of(&self, symbol: SymbolId) -> Option<ModuleId> {
        self.scopes
            .get(&self.s.scopes.symbol(symbol).owner)
            .copied()
    }

    /// The symbol a `Let` declares.
    fn let_symbol(&self, decl: FNode) -> Option<SymbolId> {
        let name = *self.children(decl).first()?;
        self.s.scopes.resolved(name)
    }

    /// What a top-level `Let` stands for, following imports: an import is
    /// a module-level `let f = f'`, `f'` another module's declaration. Not
    /// followed to a `var`: its import is a `let` initialized from it.
    fn chase(&self, decl: FNode) -> FNode {
        let mut d = decl;
        for _ in 0..64 {
            if self.kind(d) != NodeKind::Let {
                break;
            }
            let Some(symbol) = self.let_symbol(d) else {
                break;
            };
            let Some(here) = self.module_of(symbol) else {
                break;
            };
            let Some(init) = core::let_parts(&self.s.library.tree, d).2 else {
                break;
            };
            let Some(target) = self.s.scopes.resolved(init) else {
                break;
            };
            let target_symbol = self.s.scopes.symbol(target);
            // An imported `var` is copied when the import runs, as the
            // interpreter and Kotlin's backends read it, so it stays a `let`
            // of the importer's.
            if target_symbol.mutable {
                break;
            }
            match (self.module_of(target), target_symbol.decl) {
                (Some(there), Some(next)) if there != here => d = next,
                _ => break,
            }
        }
        d
    }

    /// Whether a top-level `Let` is an import's, standing for another
    /// module's declaration.
    fn is_import(&self, decl: FNode) -> bool {
        self.chase(decl) != decl
    }

    /// A `Let`'s declaration in tmpl, following imports: the library's
    /// own, or another library's, as an external declaration the first
    /// time it's used. `None` for what isn't top-level.
    fn top_decl(&mut self, decl: FNode, at: FNode) -> Tx<Option<DeclId>> {
        let decl = self.chase(decl);
        if let Some(&d) = self.decls.get(&decl) {
            return Ok(Some(d));
        }
        if let Some(&d) = self.external_lets.get(&decl) {
            return Ok(Some(d));
        }
        let Some(symbol) = self.let_symbol(decl) else {
            return Ok(None);
        };
        let Some(module) = self.module_of(symbol) else {
            return Ok(None);
        };
        let Home::Other { library, path } = self.homes[module.0 as usize].clone() else {
            return Ok(None);
        };
        let owner = self.external_module(&library, &path);
        let (name, _, init) = core::let_parts(&self.s.library.tree, decl);
        let name = self.name_of(name).to_string();
        let var = self.data(decl) & LET_VAR != 0;
        let d = match init.filter(|&i| self.kind(i) == NodeKind::Fn) {
            Some(f) => {
                if !core::type_params(&self.s.library.tree, f).is_empty() {
                    return self.not_yet(at, "a generic function");
                }
                let sig = self.signature(f)?;
                let d = self.b.declare(DeclKind::Function, &name, owner, sig);
                let (params, ..) = core::fn_parts(&self.s.library.tree, f);
                let names: Vec<String> = (params.iter())
                    .map(|&p| {
                        let name = core::let_parts(&self.s.library.tree, p).0;
                        self.name_of(name).to_string()
                    })
                    .collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                self.b.set_param_names(d, &names);
                d
            }
            None => {
                let ty = self.decl_ty(decl)?;
                let d = self.b.declare(DeclKind::ModuleLet, &name, owner, ty);
                if var {
                    self.b.add_facts(d, Facts::VAR);
                }
                d
            }
        };
        if self.s.scopes.symbol(symbol).exported {
            self.external_lets.insert(decl, d);
            Ok(Some(d))
        } else {
            self.not_yet(at, "another library's declaration that isn't exported")
        }
    }

    /// Another library's module, in its `uses` section, declared the first
    /// time.
    fn external_module(&mut self, library: &str, path: &str) -> DeclId {
        let key = (library.to_string(), path.to_string());
        if let Some(&m) = self.external_modules.get(&key) {
            return m;
        }
        let uses = match self.libraries.get(library) {
            Some(&u) => u,
            None => {
                let version = if library == "std" {
                    STD_VERSION
                } else {
                    "0.0.0"
                };
                let u = self.b.uses(library, version);
                self.libraries.insert(library.to_string(), u);
                u
            }
        };
        let m = self.b.external_module(uses, path);
        self.external_modules.insert(key, m);
        m
    }

    /// Another library's class or interface, by its symbol, declared the
    /// first time; `None` for the library's own.
    fn external_type(&mut self, symbol: SymbolId, at: FNode) -> Tx<Option<DeclId>> {
        if let Some(&d) = self.external_types.get(&symbol) {
            return Ok(Some(d));
        }
        let Some(module) = self.module_of(symbol) else {
            return Ok(None);
        };
        let Home::Other { library, path } = self.homes[module.0 as usize].clone() else {
            return Ok(None);
        };
        let s = self.s.scopes.symbol(symbol);
        let Some(class) = s.decl else {
            return self.not_yet(at, "a class or interface type");
        };
        let kind = if self.data(class) & core::CLASS_INTERFACE != 0 {
            DeclKind::Interface
        } else {
            DeclKind::Class
        };
        let name = self.s.library.names.get(s.name).to_string();
        let owner = self.external_module(&library, &path);
        let d = self.b.declare_type(kind, &name, owner);
        self.external_types.insert(symbol, d);
        Ok(Some(d))
    }

    /// The module's `~console`, read: declared the first time.
    fn console(&mut self, call: FNode, at: FNode) -> Tx<NodeId> {
        let console = match self.console {
            Some((console, _)) => console,
            None => {
                let Some(interface) = self.core_decl("Console") else {
                    return self.not_yet(call, "console");
                };
                let ty = self.b.program().decls.get(interface).ty;
                let hints = ["console"];
                let console = self
                    .b
                    .declare_made(DeclKind::ModuleLet, &hints, self.module, ty);
                let span = self.span(call);
                self.b.set_decl_spans(console, span, span);
                self.console = Some((console, call));
                console
            }
        };
        let read = self.b.get_module_let(console);
        Ok(self.own(read, at))
    }

    // Modules.

    /// Adds a module's files, and gives the module its first file's span.
    fn module_files(&mut self, m: &crate::settle::SettledModule, module: DeclId) {
        let lib = &self.s.library;
        let file_ids = lib.modules[m.id.0 as usize].files.clone();
        for f in &file_ids {
            let file = &lib.files[f.0 as usize];
            let path = file.path.rsplit(['/', '\\']).next().unwrap_or(&file.path);
            let source = file.parsed.source();
            let fid = self.b.add_file(path, hash(source));
            self.b.set_file_text(fid, source);
            self.files.insert(f.0, fid);
        }
        if let Some(f) = file_ids.first() {
            let file = &lib.files[f.0 as usize];
            let fid = self.files[&f.0];
            let len = file.parsed.source().len() as u32;
            let whole = Span {
                file: fid,
                start: 0,
                end: len,
            };
            self.b.set_decl_spans(module, whole, whole);
        }
    }

    fn module(&mut self, m: &crate::settle::SettledModule, module: DeclId) -> Tx<()> {
        let (id, top_levels) = (m.id, &m.top_levels);
        let lib = &self.s.library;
        self.module = module;
        self.owner = module;
        self.console = None;
        let file_ids = lib.modules[id.0 as usize].files.clone();
        // Imports and tests aren't items.
        let top_levels: Vec<FNode> = (top_levels.iter().copied())
            .filter(|&n| {
                self.kind(n) != NodeKind::Let || !(self.is_import(n) || m.test_cases.contains(&n))
            })
            .collect();
        // Declare first, so a body may use what's declared after it.
        for &node in &top_levels {
            if self.kind(node) == NodeKind::Let {
                self.declare_top_level(node)?;
            }
        }
        let mut init = Vec::new();
        let mut items = Vec::new();
        for &node in &top_levels {
            if self.kind(node) == NodeKind::Class {
                items.push(self.class_item(node, &mut init)?);
                continue;
            }
            if self.kind(node) != NodeKind::Let {
                self.stmt(node, &mut init)?;
                continue;
            }
            let decl = self.decls[&node];
            if self.functions.contains(&node) {
                items.push(self.function(node, decl)?);
                continue;
            }
            let item = self.b.module_let(decl);
            items.push(self.own(item, node));
            if let Some(value) = core::let_parts(&self.s.library.tree, node).2 {
                let value = self.expr(value)?;
                let value = self.fit(value, self.b.program().decls.get(decl).ty);
                let stmt = self.b.init_let(decl, value);
                init.push(self.own(stmt, node));
            }
        }
        if let Some((console, call)) = self.console {
            let Some(get) = self.core_decl("getConsole") else {
                return self.not_yet(call, "console");
            };
            let omitted = self.b.omitted(TypeId::STRING);
            let omitted = self.made(omitted, call);
            let value = self.b.call(get, &[omitted]);
            let value = self.made(value, call);
            let stmt = self.b.init_let(console, value);
            init.insert(0, self.made(stmt, call));
            let item = self.b.module_let(console);
            items.insert(0, self.made(item, call));
        }
        for item in items {
            self.b.add_item(module, item);
        }
        let block = self.b.block(&init);
        let block = match file_ids.first() {
            Some(f) => {
                let root = lib.files[f.0 as usize].root;
                self.made(block, root)
            }
            None => block,
        };
        self.b.set_init(module, block);
        for &case in &m.test_cases {
            let test = self.test(case)?;
            self.b.add_test(module, test);
        }
        Ok(())
    }

    /// A test's parameter's type, which typing doesn't record: the class
    /// its written type names, std's `Test`.
    fn test_param_type(&mut self, param: FNode) -> Tx<TypeId> {
        let written = core::let_parts(&self.s.library.tree, param).1;
        let class = written
            .and_then(|w| self.s.scopes.resolved(w))
            .and_then(|s| self.s.scopes.symbol(s).decl)
            .map(|d| self.chase(d));
        let symbol = class.and_then(|c| {
            let name = *self.children(c).first()?;
            self.s.scopes.resolved(name)
        });
        match symbol {
            Some(symbol) => match self.external_type(symbol, param)? {
                Some(decl) => Ok(self.b.nominal(decl)),
                None => self.not_yet(param, "a test's parameter type"),
            },
            None => self.not_yet(param, "a test's parameter type"),
        }
    }

    /// A test, from its `let t# = new Pair(name, fn (test: Test): Void
    /// throws Bubble { … })`.
    fn test(&mut self, case: FNode) -> Tx<NodeId> {
        let init = core::let_parts(&self.s.library.tree, case).2;
        let parts = init.map(|i| self.children(i).to_vec()).unwrap_or_default();
        let [_, _, name, f] = parts[..] else {
            return self.not_yet(case, "a test's form");
        };
        let Some(Value::String(text)) = self.s.library.value(name) else {
            return self.not_yet(name, "a test's name that isn't a constant string");
        };
        let text = text.clone();
        if self.kind(f) != NodeKind::Fn {
            return self.not_yet(f, "a test's form");
        }
        // `fn (test: Test): Void throws Bubble`, as `testing` makes it.
        let mut types = Vec::new();
        for p in core::fn_parts(&self.s.library.tree, f).0 {
            let ty = match self.s.typing.decls.get(&p) {
                Some(_) => self.decl_ty(p)?,
                None => self.test_param_type(p)?,
            };
            types.push(ty);
        }
        let sig = self.b.function_type(&types, TypeId::VOID, true);
        let decl = self
            .b
            .declare_made(DeclKind::Function, &["test"], self.module, sig);
        self.decl_spans(decl, case);
        let (params, body) = self.function_parts(f, decl)?;
        let test = self.b.test(decl, &text, &params, body);
        Ok(self.own(test, case))
    }

    /// Declares a top-level `let`: a function, or a module `let` or `var`.
    fn declare_top_level(&mut self, node: FNode) -> Tx<()> {
        let (name, _, init) = core::let_parts(&self.s.library.tree, node);
        let var = self.data(node) & LET_VAR != 0;
        let is_fn = init.is_some_and(|i| self.kind(i) == NodeKind::Fn);
        let name = self.name_of(name).to_string();
        let decl = if is_fn {
            if var {
                return self.not_yet(node, "a function in a var");
            }
            let f = init.unwrap();
            if !core::type_params(&self.s.library.tree, f).is_empty() {
                return self.not_yet(node, "a generic function");
            }
            let sig = self.signature(f)?;
            self.functions.insert(node);
            self.b.declare(DeclKind::Function, &name, self.module, sig)
        } else {
            let ty = self.decl_ty(node)?;
            let decl = self.b.declare(DeclKind::ModuleLet, &name, self.module, ty);
            if var {
                self.b.add_facts(decl, Facts::VAR);
            }
            decl
        };
        if self.is_exported(node) {
            self.b.add_facts(decl, Facts::EXPORTED);
        }
        self.decl_spans(decl, node);
        self.decls.insert(node, decl);
        Ok(())
    }

    fn function(&mut self, node: FNode, decl: DeclId) -> Tx<NodeId> {
        let f = core::let_parts(&self.s.library.tree, node).2.unwrap();
        let (params, block) = self.function_parts(f, decl)?;
        let function = self.b.function(decl, &params, block);
        Ok(self.own(function, node))
    }

    /// A function's parameters and body.
    fn function_parts(&mut self, f: FNode, decl: DeclId) -> Tx<(Vec<NodeId>, NodeId)> {
        let (params, _, body) = core::fn_parts(&self.s.library.tree, f);
        let Some(body) = body else {
            return self.not_yet(f, "a connected function");
        };
        let result = self
            .b
            .program()
            .signature(decl)
            .map_or(TypeId::NONE, |s| s.result);
        let (owner, outer) = (
            std::mem::replace(&mut self.owner, decl),
            std::mem::replace(&mut self.result, result),
        );
        let types: Vec<TypeId> =
            (self.b.program().signature(decl)).map_or_else(Vec::new, |s| s.params.to_vec());
        let mut param_nodes = Vec::new();
        for (i, p) in params.into_iter().enumerate() {
            if self.kind(p) != NodeKind::Let {
                return self.not_yet(p, "a parameter that isn't a name");
            }
            let (name, _, default) = core::let_parts(&self.s.library.tree, p);
            if default.is_some() {
                return self.not_yet(p, "a parameter's default");
            }
            let ty = match types.get(i) {
                Some(&ty) => ty,
                None => self.decl_ty(p)?,
            };
            let name = self.name_of(name).to_string();
            let d = self.b.declare(DeclKind::Param, &name, decl, ty);
            if self.data(p) & LET_VAR != 0 {
                self.b.add_facts(d, Facts::VAR);
            }
            self.decl_spans(d, p);
            self.decls.insert(p, d);
            let param = self.b.param(d);
            param_nodes.push(self.own(param, p));
        }
        let mut stmts = Vec::new();
        if result == TypeId::VOID {
            self.stmts(body, &mut stmts)?;
        } else {
            self.tail_stmts(body, &mut stmts)?;
        }
        let block = self.b.block(&stmts);
        let block = self.own(block, body);
        self.owner = owner;
        self.result = outer;
        Ok((param_nodes, block))
    }

    // Statements.

    /// A block's statements, or a statement's, without a block around them.
    fn stmts(&mut self, node: FNode, out: &mut Vec<NodeId>) -> Tx<()> {
        if self.kind(node) == NodeKind::Block {
            for &child in self.children(node).to_vec().iter() {
                self.stmt(child, out)?;
            }
            Ok(())
        } else {
            self.stmt(node, out)
        }
    }

    /// A block, as a statement's part: a branch or a loop's body.
    fn block(&mut self, node: FNode) -> Tx<NodeId> {
        let mut stmts = Vec::new();
        self.stmts(node, &mut stmts)?;
        let block = self.b.block(&stmts);
        Ok(self.own(block, node))
    }

    fn stmt(&mut self, node: FNode, out: &mut Vec<NodeId>) -> Tx<()> {
        let children = self.children(node).to_vec();
        let stmt = match self.kind(node) {
            // A value or a name read alone does nothing.
            NodeKind::Value | NodeKind::RightName => return Ok(()),
            NodeKind::Block => self.block(node)?,
            NodeKind::Let => self.let_stmt(node)?,
            NodeKind::Assign => {
                let [target, value] = children[..] else {
                    return self.not_yet(node, "an assignment's form");
                };
                if self.kind(target) != NodeKind::RightName {
                    let Some((object, m)) = self.assigned_member(target) else {
                        return self.not_yet(node, "an assignment to a property");
                    };
                    let object = self.static_object(object);
                    let stmt = self.assign_member(m, object, value, node)?;
                    out.push(self.own(stmt, node));
                    return Ok(());
                }
                let member = self.s.scopes.resolved(target).and_then(|s| {
                    let s = self.s.scopes.symbol(s);
                    (s.kind == crate::scope::SymbolKind::Member)
                        .then_some(s.decl)
                        .flatten()
                });
                if let Some(m) = member {
                    let stmt = self.assign_member(m, None, value, node)?;
                    out.push(self.own(stmt, node));
                    return Ok(());
                }
                let decl = self.declaration(target).and_then(|d| self.decls.get(&d));
                let Some(&decl) = decl else {
                    return self.not_yet(node, "an assignment to something not declared here");
                };
                let value = self.expr(value)?;
                let value = self.fit(value, self.b.program().decls.get(decl).ty);
                if self.b.program().decls.get(decl).kind == DeclKind::ModuleLet {
                    self.b.set_module_let(decl, value)
                } else {
                    self.b.assign(decl, value)
                }
            }
            NodeKind::If => {
                let [cond, then, els] = children[..] else {
                    return self.not_yet(node, "an if's form");
                };
                let cond = self.expr(cond)?;
                let then = self.block(then)?;
                let empty = self.kind(els) == NodeKind::Block && self.children(els).is_empty();
                let els = if empty { None } else { Some(self.block(els)?) };
                self.b.if_stmt(cond, then, els)
            }
            NodeKind::Loop => self.loop_stmt(node)?,
            NodeKind::Break | NodeKind::Continue => {
                let target = FNode(self.data(node));
                let Some(&label) = self.loops.get(&target) else {
                    return self.not_yet(node, "a jump out of a labeled block");
                };
                if self.kind(node) == NodeKind::Break {
                    self.b.break_loop(label)
                } else {
                    self.b.continue_loop(label)
                }
            }
            NodeKind::Return => {
                if self.result.is_none() {
                    return self.not_yet(node, "a return outside a function");
                }
                let value = children.first().copied();
                let value = match value {
                    Some(v) if self.result == TypeId::VOID => {
                        if !matches!(self.s.library.value(v), Some(Value::Void)) {
                            self.stmt(v, out)?;
                        }
                        None
                    }
                    Some(v) => {
                        let v = self.expr(v)?;
                        Some(self.fit(v, self.result))
                    }
                    None => None,
                };
                self.b.return_stmt(value)
            }
            NodeKind::Call => {
                let call = self.expr(node)?;
                self.b.expr_statement(call)
            }
            NodeKind::Orelse | NodeKind::Fail => return self.not_yet(node, "orelse and bubble()"),
            NodeKind::Yield | NodeKind::Await => return self.not_yet(node, "yield and await"),
            NodeKind::Class => return self.not_yet(node, "a class"),
            NodeKind::Fn => return self.not_yet(node, "a function value"),
            kind => return self.not_yet(node, format!("{kind:?}")),
        };
        out.push(self.own(stmt, node));
        Ok(())
    }

    fn let_stmt(&mut self, node: FNode) -> Tx<NodeId> {
        let (name, _, init) = core::let_parts(&self.s.library.tree, node);
        if init.is_some_and(|i| self.kind(i) == NodeKind::Fn) {
            return self.not_yet(node, "a local function");
        }
        let ty = self.decl_ty(node)?;
        let name = self.name_of(name).to_string();
        let local = self.b.declare(DeclKind::Local, &name, self.owner, ty);
        if self.data(node) & LET_VAR != 0 {
            self.b.add_facts(local, Facts::VAR);
        }
        self.decl_spans(local, node);
        self.decls.insert(node, local);
        let init = match init {
            Some(init) => {
                let init = self.expr(init)?;
                Some(self.fit(init, ty))
            }
            None => None,
        };
        Ok(self.b.let_stmt(local, init))
    }

    /// Whether a node is `break` to `target`, maybe alone in a block.
    fn breaks_to(&self, node: FNode, target: FNode) -> bool {
        match self.kind(node) {
            NodeKind::Break => FNode(self.data(node)) == target,
            NodeKind::Block => match self.children(node) {
                &[only] => self.breaks_to(only, target),
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether anything under a node continues `target`.
    fn continues_to(&self, node: FNode, target: FNode) -> bool {
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if self.kind(n) == NodeKind::Continue && FNode(self.data(n)) == target {
                return true;
            }
            stack.extend_from_slice(self.children(n));
        }
        false
    }

    /// `if (cond) { body } else { break }`, the shape `while` takes: its
    /// condition and body.
    fn while_shape(&self, node: FNode, target: FNode) -> Option<(FNode, FNode)> {
        if self.kind(node) != NodeKind::If {
            return None;
        }
        match *self.children(node) {
            [cond, body, els] if self.breaks_to(els, target) => Some((cond, body)),
            _ => None,
        }
    }

    /// A loop: `while`, `do … while`, or `for` without `continue`; any
    /// other as `while (true)`.
    fn loop_stmt(&mut self, node: FNode) -> Tx<NodeId> {
        let data = self.data(node);
        if data & LOOP_EACH != 0 {
            return self.not_yet(node, "a for … of loop");
        }
        let label = self
            .b
            .declare_made(DeclKind::Label, &["loop"], self.owner, TypeId::NONE);
        let span = self.span(node);
        self.b.set_decl_spans(label, span, span);
        self.loops.insert(node, label);
        let children = self.children(node).to_vec();
        if data & LOOP_NEXT != 0 {
            let [body, next] = children[..] else {
                return self.not_yet(node, "a loop's form");
            };
            if self.continues_to(body, node) {
                return self.not_yet(node, "continue in a for loop");
            }
            // `do { body } while (cond)`: `next` checks.
            if let Some((cond, then)) = self.while_shape(next, node) {
                if self.kind(then) == NodeKind::Block && self.children(then).is_empty() {
                    let body = self.block(body)?;
                    let cond = self.expr(cond)?;
                    return Ok(self.b.do_while(label, body, cond));
                }
            }
            // `for (…; cond; next) { body }`.
            let (cond, body) = match self.while_shape(body, node) {
                Some((cond, body)) => (Some(cond), body),
                None => (None, body),
            };
            let cond = match cond {
                Some(cond) => self.expr(cond)?,
                None => {
                    let yes = self.b.bool_lit(true);
                    self.made(yes, node)
                }
            };
            let mut stmts = Vec::new();
            self.stmts(body, &mut stmts)?;
            self.stmts(next, &mut stmts)?;
            let block = self.b.block(&stmts);
            let block = self.own(block, body);
            return Ok(self.b.while_loop(label, cond, block));
        }
        let [body] = children[..] else {
            return self.not_yet(node, "a loop's form");
        };
        let (cond, body) = match self.while_shape(body, node) {
            Some((cond, body)) => (self.expr(cond)?, body),
            None => {
                let yes = self.b.bool_lit(true);
                (self.made(yes, node), body)
            }
        };
        let body = self.block(body)?;
        Ok(self.b.while_loop(label, cond, body))
    }

    /// A function's body, whose last value it returns: `return`s pushed
    /// into the branches of a trailing `if`.
    fn tail_stmts(&mut self, node: FNode, out: &mut Vec<NodeId>) -> Tx<()> {
        if self.kind(node) != NodeKind::Block {
            return self.tail(node, out);
        }
        let children = self.children(node).to_vec();
        if let Some((&last, rest)) = children.split_last() {
            for &child in rest {
                self.stmt(child, out)?;
            }
            // `{ return x; }` ends with a `void` that's never reached.
            let after_return = rest
                .last()
                .is_some_and(|&r| self.kind(r) == NodeKind::Return);
            if after_return && matches!(self.s.library.value(last), Some(Value::Void)) {
                return Ok(());
            }
            self.tail(last, out)?;
        }
        Ok(())
    }

    fn tail(&mut self, node: FNode, out: &mut Vec<NodeId>) -> Tx<()> {
        match self.kind(node) {
            NodeKind::Block => {
                let mut stmts = Vec::new();
                self.tail_stmts(node, &mut stmts)?;
                let block = self.b.block(&stmts);
                out.push(self.own(block, node));
                Ok(())
            }
            NodeKind::If if self.and_or(node).is_none() => {
                let [cond, then, els] = self.children(node).to_vec()[..] else {
                    return self.not_yet(node, "an if's form");
                };
                let cond = self.expr(cond)?;
                let mut branches = Vec::new();
                for branch in [then, els] {
                    let mut stmts = Vec::new();
                    self.tail_stmts(branch, &mut stmts)?;
                    let block = self.b.block(&stmts);
                    branches.push(self.own(block, branch));
                }
                let stmt = self.b.if_stmt(cond, branches[0], Some(branches[1]));
                out.push(self.own(stmt, node));
                Ok(())
            }
            NodeKind::Loop
            | NodeKind::Let
            | NodeKind::Assign
            | NodeKind::Return
            | NodeKind::Break
            | NodeKind::Continue => self.stmt(node, out),
            _ => {
                let value = self.expr(node)?;
                let value = self.fit(value, self.result);
                let stmt = self.b.return_stmt(Some(value));
                out.push(self.made(stmt, node));
                Ok(())
            }
        }
    }

    // Expressions.

    fn expr(&mut self, node: FNode) -> Tx<NodeId> {
        let lib = &self.s.library;
        match self.kind(node) {
            NodeKind::Value => {
                let made = match lib.value(node) {
                    Some(Value::Int32(v)) => self.b.int32(*v),
                    Some(Value::Int64(v)) => self.b.int64(*v),
                    Some(Value::Float64(v)) => self.b.float64(*v),
                    Some(Value::String(v)) => self.b.string(v),
                    Some(Value::Bool(v)) => self.b.bool_lit(*v),
                    Some(Value::Void) => self.b.void(),
                    Some(Value::Null) => return self.not_yet(node, "null"),
                    _ => return self.not_yet(node, "a value of another kind"),
                };
                Ok(self.own(made, node))
            }
            NodeKind::RightName => {
                if let Some(Builtin::Value(c)) = self.s.scopes.resolved_builtin(node) {
                    let made = match c {
                        Constant::True => self.b.bool_lit(true),
                        Constant::False => self.b.bool_lit(false),
                        Constant::Void => self.b.void(),
                        Constant::NaN => self.b.float64(f64::NAN),
                        Constant::Infinity => self.b.float64(f64::INFINITY),
                        Constant::TestServerPort(port) => self.b.int32(port),
                        Constant::Null => return self.not_yet(node, "null"),
                        Constant::Console => return self.not_yet(node, "console as a value"),
                    };
                    return Ok(self.own(made, node));
                }
                if self.is_this(node) {
                    return self.this(node);
                }
                let member = self.s.scopes.resolved(node).and_then(|s| {
                    let s = self.s.scopes.symbol(s);
                    (s.kind == crate::scope::SymbolKind::Member)
                        .then_some(s.decl)
                        .flatten()
                });
                if let Some(m) = member {
                    return self.read_member(m, None, node);
                }
                let decl = match self.declaration(node) {
                    Some(d) if self.decls.contains_key(&d) => Some(self.decls[&d]),
                    Some(d) => self.top_decl(d, node)?,
                    None => None,
                };
                let Some(decl) = decl else {
                    return self.not_yet(node, "a name not declared here");
                };
                let made = match self.b.program().decls.get(decl).kind {
                    DeclKind::Local | DeclKind::Param => {
                        let read = self.b.local(decl);
                        let read = self.own(read, node);
                        return self.narrowed(read, node);
                    }
                    DeclKind::ModuleLet => self.b.get_module_let(decl),
                    _ => return self.not_yet(node, "a function as a value"),
                };
                Ok(self.own(made, node))
            }
            NodeKind::Call => self.call(node),
            NodeKind::If => self.if_value(node),
            NodeKind::Block => {
                let ty = self.expr_ty(node)?;
                self.block_value(node, ty)
            }
            NodeKind::Fn => self.not_yet(node, "a function value"),
            NodeKind::Orelse => self.not_yet(node, "orelse and bubble()"),
            kind => self.not_yet(node, format!("{kind:?}")),
        }
    }

    /// `a && b`, as `if (a) { b } else { false }`, or `a || b`, as `if (a)
    /// { true } else { b }`: the operands and whether it's `&&`.
    fn and_or(&self, node: FNode) -> Option<(FNode, FNode, bool)> {
        let &[cond, then, els] = self.children(node) else {
            return None;
        };
        if self.s.typing.expr(node) != Some(ft::BOOLEAN) {
            return None;
        }
        match (self.bool_const(then), self.bool_const(els)) {
            (_, Some(false)) => Some((cond, then, true)),
            (Some(true), _) => Some((cond, els, false)),
            _ => None,
        }
    }

    /// `true` or `false`, as a value or a name.
    fn bool_const(&self, node: FNode) -> Option<bool> {
        match self.s.library.value(node) {
            Some(Value::Bool(b)) => return Some(*b),
            Some(_) => return None,
            None => {}
        }
        match self.s.scopes.resolved_builtin(node) {
            Some(Builtin::Value(Constant::True)) => Some(true),
            Some(Builtin::Value(Constant::False)) => Some(false),
            _ => None,
        }
    }

    fn if_value(&mut self, node: FNode) -> Tx<NodeId> {
        if let Some((left, right, and)) = self.and_or(node) {
            let (left, right) = (self.expr(left)?, self.expr(right)?);
            let made = if and {
                self.b.and(left, right)
            } else {
                self.b.or(left, right)
            };
            return Ok(self.own(made, node));
        }
        let [cond, then, els] = self.children(node).to_vec()[..] else {
            return self.not_yet(node, "an if's form");
        };
        let ty = self.expr_ty(node)?;
        let cond = self.expr(cond)?;
        let then = self.block_value(then, ty)?;
        let els = self.block_value(els, ty)?;
        let made = self.b.if_value(ty, cond, then, els);
        Ok(self.own(made, node))
    }

    /// A block's value: its last part's, after its statements.
    fn block_value(&mut self, node: FNode, ty: TypeId) -> Tx<NodeId> {
        if self.kind(node) != NodeKind::Block {
            return self.expr(node);
        }
        let children = self.children(node).to_vec();
        let Some((&last, rest)) = children.split_last() else {
            return self.not_yet(node, "an empty block's value");
        };
        if rest.is_empty() {
            return self.expr(last);
        }
        let mut stmts = Vec::new();
        for &child in rest {
            self.stmt(child, &mut stmts)?;
        }
        let value = self.expr(last)?;
        let made = self.b.block_value(ty, &stmts, value);
        Ok(self.own(made, node))
    }

    /// A call's arguments, if they're all plain values.
    fn plain_args(&self, call: FNode, args: &[FNode]) -> Tx<Vec<FNode>> {
        for &arg in args {
            if let Some(name) = self.s.library.symbol(arg) {
                let what = if self.s.library.names.get(name) == "typeArg" {
                    "type arguments"
                } else {
                    "a named argument"
                };
                return self.not_yet(call, what);
            }
        }
        Ok(args.to_vec())
    }

    fn call(&mut self, node: FNode) -> Tx<NodeId> {
        let children = self.children(node).to_vec();
        let (callee, args) = (children[0], &children[1..]);
        if self.s.typing.trailing_blocks.contains_key(&node) {
            return self.not_yet(node, "a trailing block");
        }
        let f = self.function_of(callee);
        if f == Some(Function::Is) {
            return self.is_check(node, args);
        }
        if f == Some(Function::As) {
            return self.as_cast(node, args);
        }
        match self.s.typing.resolved(node) {
            Some(&Resolved::Decl(d)) if self.is_member(d) || self.kind(d) == NodeKind::Class => {
                let d = self.chase(d);
                match f {
                    Some(Function::New) => {
                        let class = if self.kind(d) == NodeKind::Class {
                            Some(d)
                        } else {
                            self.member_class(d)
                        };
                        match class {
                            Some(class) => self.new_object(class, &args[1..], node),
                            None => self.not_yet(node, "a constructor"),
                        }
                    }
                    Some(Function::Dot) => {
                        let object = self.children(node)[1];
                        let object = self.static_object(object);
                        self.read_member(d, object, node)
                    }
                    Some(
                        f @ (Function::Eq
                        | Function::Ne
                        | Function::Lt
                        | Function::Le
                        | Function::Gt
                        | Function::Ge
                        | Function::Cmp),
                    ) => self.operator_call(node, f, d, args),
                    Some(_) => self.not_yet(node, "a call of a member or constructor"),
                    None => {
                        let args = self.plain_args(node, args)?;
                        match self.dot_object(callee) {
                            Some(object) => {
                                let object = self.static_object(object);
                                self.call_member(d, object, &args, node)
                            }
                            None if self.kind(callee) == NodeKind::RightName => {
                                self.call_member(d, None, &args, node)
                            }
                            None => self.not_yet(node, "a call of a member or constructor"),
                        }
                    }
                }
            }
            None if self.s.typing.identity_eq(node).is_some() => {
                self.identity(node, f.unwrap_or(Function::Eq), args)
            }
            None => self.not_yet(node, "a call typing didn't resolve"),
            Some(Resolved::Builtin(key)) => {
                let key = key.clone();
                self.builtin_call(node, f, &key, args)
            }
            Some(&Resolved::Decl(d)) if self.is_function(d) => {
                let decl = self.top_decl(d, node)?.unwrap();
                let args = self.plain_args(node, args)?;
                let count = self
                    .b
                    .program()
                    .signature(decl)
                    .map_or(0, |s| s.params.len());
                if args.len() != count {
                    return self.not_yet(node, "an argument left out");
                }
                let mut values = Vec::new();
                for arg in args {
                    values.push(self.expr(arg)?);
                }
                let values = self.fit_args(decl, values);
                let made = self.b.call(decl, &values);
                Ok(self.own(made, node))
            }
            Some(Resolved::Decl(_)) => self.not_yet(node, "a call of a member or constructor"),
            Some(Resolved::NullCheck) => self.not_yet(node, "a null check"),
            Some(Resolved::Value) => self.not_yet(node, "a call of a function value"),
        }
    }

    /// Whether a `Let` declares a function of the library's, or of another
    /// module or library, following imports.
    fn is_function(&self, decl: FNode) -> bool {
        let decl = self.chase(decl);
        if self.functions.contains(&decl) {
            return true;
        }
        let top = self
            .let_symbol(decl)
            .is_some_and(|s| self.module_of(s).is_some());
        let init = core::let_parts(&self.s.library.tree, decl).2;
        top && self.kind(decl) == NodeKind::Let
            && init.is_some_and(|i| self.kind(i) == NodeKind::Fn)
    }

    /// `object` in a callee `object.name`, if it's one.
    fn dot_object(&self, callee: FNode) -> Option<FNode> {
        if self.kind(callee) != NodeKind::Call {
            return None;
        }
        match *self.children(callee) {
            [dot, object, _] if self.function_of(dot) == Some(Function::Dot) => Some(object),
            _ => None,
        }
    }

    fn builtin_call(
        &mut self,
        node: FNode,
        f: Option<Function>,
        key: &str,
        args: &[FNode],
    ) -> Tx<NodeId> {
        // `object.name`'s parts aren't arguments.
        let args = match f {
            Some(Function::Dot) => Vec::new(),
            _ => self.plain_args(node, args)?,
        };
        if key == "String.cat" {
            return self.cat(node, &args);
        }
        if key == "console.log" {
            let [arg] = args[..] else {
                return self.not_yet(node, "console.log's arguments");
            };
            let Some(log) = self.core_decl("Console.log") else {
                return self.not_yet(node, "console");
            };
            let callee = self.children(node)[0];
            let at = self.children(callee).get(1).copied().unwrap_or(callee);
            let receiver = self.console(node, at)?;
            let arg = self.expr(arg)?;
            let made = self.b.call_method(log, receiver, &[arg]);
            return Ok(self.own(made, node));
        }
        // An operator.
        if f.is_some_and(|f| f != Function::Dot) {
            let Some(op) = Op::named(key) else {
                return self.not_yet(node, format!("builtin {key}"));
            };
            let made = self.op(node, op, None, &args)?;
            if f == Some(Function::Ne) {
                let not = Op::named("Boolean.not").unwrap();
                let made = self.b.op(not, &[made]);
                return Ok(self.made(made, node));
            }
            return Ok(made);
        }
        // `object.name`, a getter's or field's value.
        let callee = self.children(node)[0];
        let (object, method) = match f {
            Some(Function::Dot) => (self.children(node)[1], false),
            _ => match self.dot_object(callee) {
                Some(object) => (object, true),
                None => return self.not_yet(node, format!("builtin {key}")),
            },
        };
        if method {
            if let Some(op) = Op::named(key) {
                return self.op(node, op, Some(object), &args);
            }
        }
        let Some(member) = self.core_decl(key) else {
            return self.not_yet(node, format!("core's {key}"));
        };
        let kind = self.b.program().decls.get(member).kind;
        match (kind, method) {
            (DeclKind::Getter, false) => {
                let receiver = self.expr(object)?;
                let made = self.b.call_method(member, receiver, &[]);
                Ok(self.own(made, node))
            }
            (DeclKind::Method, true) => {
                let receiver = self.expr(object)?;
                let values = self.args_for(node, member, &args)?;
                let made = self.b.call_method(member, receiver, &values);
                Ok(self.own(made, node))
            }
            (DeclKind::Function, true) => {
                let values = self.args_for(node, member, &args)?;
                let made = self.b.call(member, &values);
                Ok(self.own(made, node))
            }
            _ => self.not_yet(node, format!("core's {key} used so")),
        }
    }

    /// An operator applied to its operands, a receiver first if any.
    fn op(&mut self, node: FNode, op: Op, receiver: Option<FNode>, args: &[FNode]) -> Tx<NodeId> {
        let arity = op.info().map_or(0, |i| i.params.len());
        if receiver.iter().count() + args.len() != arity {
            return self.not_yet(node, "an operator's arguments");
        }
        let mut values = Vec::new();
        for &arg in receiver.iter().chain(args) {
            values.push(self.expr(arg)?);
        }
        let made = self.b.op(op, &values);
        Ok(self.own(made, node))
    }

    /// The arguments for a call of `callee`, with `Omitted` for optional
    /// parameters left out.
    fn args_for(&mut self, node: FNode, callee: DeclId, args: &[FNode]) -> Tx<Vec<NodeId>> {
        let Some(sig) = self.b.program().signature(callee) else {
            return self.not_yet(node, "a call without a signature");
        };
        let (params, optional) = (sig.params.to_vec(), sig.optional.to_vec());
        if args.len() > params.len() || optional[args.len()..].contains(&false) {
            return self.not_yet(node, "an argument left out");
        }
        let mut values = Vec::new();
        for &arg in args {
            values.push(self.expr(arg)?);
        }
        for &ty in &params[args.len()..] {
            let omitted = self.b.omitted(ty);
            values.push(self.made(omitted, node));
        }
        Ok(values)
    }

    /// A string template, `"a${b}"`: each part as a `String`, by its
    /// `toString` if it's another builtin type, joined by `String.cat`.
    fn cat(&mut self, node: FNode, parts: &[FNode]) -> Tx<NodeId> {
        let mut joined: Option<NodeId> = None;
        for &part in parts {
            let value = self.expr(part)?;
            let ty = self.b.program().nodes.ty(value);
            let value = if ty == TypeId::STRING {
                value
            } else {
                let name = match ty {
                    TypeId::BOOLEAN => "Boolean",
                    TypeId::INT32 => "Int32",
                    TypeId::INT64 => "Int64",
                    TypeId::FLOAT64 => "Float64",
                    _ => return self.not_yet(part, "a template's part of another type"),
                };
                let Some(to_string) = self.core_decl(&format!("{name}.toString")) else {
                    return self.not_yet(part, "a template's part of another type");
                };
                let sig = self.b.program().signature(to_string).unwrap();
                let params = sig.params.to_vec();
                let mut args = Vec::new();
                for ty in params {
                    let omitted = self.b.omitted(ty);
                    args.push(self.made(omitted, part));
                }
                let made = self.b.call_method(to_string, value, &args);
                self.made(made, part)
            };
            joined = Some(match joined {
                None => value,
                Some(left) => {
                    let cat = Op::named("String.cat").unwrap();
                    let made = self.b.op(cat, &[left, value]);
                    self.made(made, node)
                }
            });
        }
        match joined {
            Some(joined) => Ok(joined),
            None => {
                let empty = self.b.string("");
                Ok(self.own(empty, node))
            }
        }
    }
}
