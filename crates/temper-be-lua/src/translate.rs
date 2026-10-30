//! The translation from tmpl to Lua's output tree, for subset A.
//!
//! A module is a chunk that runs once, when it's first `require`d, as
//! Kotlin's, and returns its exports in a table. Its functions are local
//! functions, and its module `let`s and `var`s locals of the chunk, which
//! functions refer to as upvalues. Lua resolves a name where it's read, to
//! the innermost local declared before it in the text, so the chunk is
//! laid out as:
//!
//! 1. `local temper = require("temper-core")`, if it uses the runtime;
//! 2. `local exports = {}`, if an exported `var` is written by functions
//!    (below);
//! 3. module `let`s and `var`s that functions use, or that the init block
//!    doesn't initialize at its top level, declared with their type
//!    (`---@type integer`) and no value
//!    ([`module_state`]);
//! 4. functions referred to before they're defined, declared together
//!    (`local f, g`), and then defined without `local`;
//! 5. the functions, in source order;
//! 6. the init block, where a module `let` or `var` initialized at its top
//!    level, and not declared before, is declared with its value;
//! 7. the exports, `return { … }`.
//!
//! **Exported `var`s that functions write** live in the exports table
//! itself (`exports.total`), so a library's users see their current
//! value. Kotlin's copies every export into the table once, after the
//! init block, so such a `var` stays at its first value for users.
//!
//! **Modules and imports:** each module is a chunk of its own, required
//! by its path (`lib/helpers/deep`, the root module by the library's
//! name), as Kotlin's. A module's functions and `let`s used from another
//! are locals there, `local greet = require('lib/helpers/deep').greet`;
//! its `var`s are read where they're used, from its exports table, so
//! the read is current. Tests go in a file of their own
//! ([`Translator::tests`]), which reaches what its module doesn't export,
//! and the `var`s its tests assign, through a table its module adds to
//! its exports for them, under `'$test'`. The root module requires the
//! library's other modules that it doesn't import, so requiring the
//! library runs them all, as Kotlin's Python and JS do.
//!
//! **Lua's limit on locals:** a function, the chunk included, may have at
//! most 200 locals alive at once. Past 128 module-level locals (Kotlin's
//! threshold), the module's functions and module `let`s after its first
//! 100 are fields of a `module` table instead, as Kotlin's keeps its first
//! 100 and puts the rest in a table (`env_t1`); a function with more than
//! 200 locals alive at once is not yet.
//!
//! **Jumps:** `break` is Lua's own for the innermost loop. A `continue` is
//! `goto continue`, to a label ending the loop's body (`::continue::`),
//! which runs a `repeat`'s condition, as Temper's `do … while` does; a
//! jump from an inner loop to an outer one is a `goto` to a label named
//! for the loop, ending its body (`continue_outer`) or after it
//! (`break_outer`). Labels are printed only where a jump needs one, and
//! `goto` needs Lua 5.2 or later, or LuaJIT, as Kotlin's output does.
//!
//! **Order of evaluation:** Lua evaluates operands and arguments left to
//! right (its reference implementation and LuaJIT; the manual leaves only
//! multiple assignment's order open, which this never prints), so Temper's
//! order needs no temporaries. Where Lua's `and` and `or` would skip an
//! operand that Temper evaluates (an `if` value's spelling, `c and a or
//! b`, and `Boolean`'s order, `not a and b`), the translator checks with
//! [`Effects`] that skipping or repeating it can't show, and otherwise
//! spells it another way.

use crate::NotYet;
use crate::connected::{self, Connection, TypeConnection};
use crate::ops::{self, Spelling};
use crate::tree::{self, Bin, Expr, Signature, Stmt};
use std::collections::{HashMap, HashSet};
use temper_tmpl::analysis::{Effects, home, module_imports, module_refs, module_state, walk};
use temper_tmpl::naming::NameMap;
use temper_tmpl::qname::{is_external, library_of};
use temper_tmpl::{
    DeclId, DeclKind, Facts, Module, Node, NodeId, Program, QNames, Scalar, TypeData, TypeId,
};

type R<T> = Result<T, NotYet>;

fn not_yet<T>(what: impl Into<String>) -> R<T> {
    Err(NotYet { what: what.into() })
}

/// The most locals Lua lets a function have alive at once
/// (`LUAI_MAXVARS`, 200 in every Lua since 5.1, and in LuaJIT).
pub const MAX_LOCALS: usize = 200;

/// Past this many module-level locals, module-level declarations after
/// the first [`KEPT_LOCALS`] go into a table, as Kotlin's do
/// (`MAX_ALLOWABLE_LOCALS`), leaving room in the chunk for the init
/// block's own locals, and for functions' upvalues.
pub const MODULE_LOCALS: usize = 128;

/// How many module-level declarations stay locals past [`MODULE_LOCALS`]
/// (Kotlin's `LOCALS_TO_KEEP`).
pub const KEPT_LOCALS: usize = 100;

/// The table that holds module-level declarations past [`MODULE_LOCALS`].
pub const MODULE_TABLE: &str = "module";

/// The table of exports, declared first when it holds live `var`s.
pub const EXPORTS: &str = "exports";

/// Labels a loop needs, by what jumps to it.
#[derive(Clone, Debug, Default)]
struct Labels {
    /// Ending its body, for a `continue`.
    next: Option<String>,
    /// After it, for a `break` from an inner loop.
    end: Option<String>,
}

pub struct Translator<'a> {
    p: &'a Program,
    names: &'a NameMap,
    qnames: QNames,
    effects: Effects,
    /// Module `let`s of an erased type, which aren't declared.
    erased: HashSet<DeclId>,
    /// Module-level declarations that are fields of a table, by the
    /// table's name.
    fields: HashMap<DeclId, &'static str>,
    /// Whether the module calls the runtime.
    uses_temper: bool,
    /// The enclosing loops, innermost last.
    loops: Vec<DeclId>,
    /// Labels printed, by loop.
    labels: HashMap<DeclId, Labels>,
    /// Whether translating a function with a result, where a bubble is
    /// `return temper.bubble()`, so LuaLS sees every path return.
    returns: bool,
    /// The module whose chunk this is; `NONE` for a tests file.
    here: DeclId,
    /// In a tests file, the module tested, reached through `tested`.
    tested: DeclId,
}

/// The key of the table a module with tests adds to its exports for them.
pub const TEST_KEY: &str = "$test";

/// What a module's tests use of it: its declarations, which the table
/// under [`TEST_KEY`] holds, and of those, the `var`s, which it reads
/// and assigns through functions there, `NAME__get` and `NAME__set`.
#[derive(Clone, Debug, Default)]
pub struct TestAccess {
    pub used: Vec<DeclId>,
    pub vars: Vec<DeclId>,
}

impl TestAccess {
    pub fn of(p: &Program, module: &Module) -> TestAccess {
        let mut access = TestAccess::default();
        for &test in &module.tests {
            walk(p, test, &mut |id| {
                let decl = match p.nodes.get(id) {
                    Node::CallFunction { callee, .. } => callee,
                    Node::GetModuleLet { target } | Node::SetModuleLet { target, .. } => target,
                    _ => return,
                };
                if home(p, decl) == Some(module.decl) && !access.used.contains(&decl) {
                    access.used.push(decl);
                }
            });
        }
        access.used.sort();
        access.vars = (access.used.iter().copied())
            .filter(|&d| is_var(p, d))
            .collect();
        access
    }
}

fn is_var(p: &Program, decl: DeclId) -> bool {
    let d = p.decls.get(decl);
    d.kind == DeclKind::ModuleLet && d.facts.contains(Facts::VAR)
}

/// What `require` names a module by: the library's name for its root
/// module, and its path after it for another, as `lib/helpers/deep`; for
/// another library's module, that library's.
pub fn require_path(p: &Program, module: DeclId) -> String {
    let d = p.decls.get(module);
    if d.kind == DeclKind::Library {
        return p.name(module).to_string();
    }
    let (library, path) = if d.owner.is_none() {
        let path = p.modules.iter().find(|m| m.decl == module);
        (
            p.library.name.as_str(),
            path.map_or("", |m| m.path.as_str()),
        )
    } else {
        (p.name(d.owner), p.name(module))
    };
    match path {
        "" => library.to_string(),
        path => format!("{library}/{path}"),
    }
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap, module: &Module) -> Translator<'a> {
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            effects: Effects::of(p, module),
            erased: HashSet::new(),
            fields: HashMap::new(),
            uses_temper: false,
            loops: Vec::new(),
            labels: HashMap::new(),
            returns: false,
            here: module.decl,
            tested: DeclId::NONE,
        }
    }

    /// A module-level declaration of another module's: in a tests file,
    /// the tested module's through `tested`; a `var`, read from its
    /// module's exports (`require('lib/helpers').calls`); else the local
    /// it's imported as, by its name. `None` for this module's own.
    fn other(&self, decl: DeclId) -> Option<Expr> {
        let p = self.p;
        // Only module-level declarations: what a module or library owns.
        let owner = p.decls.get(decl).owner;
        if owner.is_none()
            || !matches!(
                p.decls.get(owner).kind,
                DeclKind::Module | DeclKind::Library
            )
        {
            return None;
        }
        let module = home(p, decl).filter(|&m| m != self.here)?;
        let name = self.name(decl);
        if module == self.tested {
            return Some(if is_var(p, decl) {
                Expr::name("tested").method(format!("{name}__get"), vec![])
            } else {
                Expr::name("tested").member(name)
            });
        }
        if is_var(p, decl) {
            let path = Expr::Str(require_path(p, module));
            return Some(Expr::name("require").call(vec![path]).member(name));
        }
        Some(Expr::name(name))
    }

    pub(crate) fn node(&self, id: NodeId) -> Node {
        self.p.nodes.get(id)
    }

    fn name(&self, decl: DeclId) -> String {
        self.names.get(decl).to_string()
    }

    /// A module-level declaration, or a local, as code refers to it: its
    /// name, or a table's field.
    fn access(&self, decl: DeclId) -> Expr {
        if let Some(e) = self.other(decl) {
            return e;
        }
        match self.fields.get(&decl) {
            Some(table) => Expr::name(*table).member(self.name(decl)),
            None => Expr::name(self.name(decl)),
        }
    }

    fn qname(&self, decl: DeclId) -> String {
        self.qnames
            .get(decl)
            .map(|q| q.to_string())
            .unwrap_or_default()
    }

    fn type_connection(&self, ty: TypeId) -> Option<TypeConnection> {
        match self.p.types.get(ty) {
            TypeData::Nominal { decl } => connected::lookup_type(&self.qname(decl)),
            _ => None,
        }
    }

    /// Whether a node reads a declaration that isn't kept, or calls a
    /// function whose result isn't.
    pub(crate) fn is_erased(&self, id: NodeId) -> bool {
        match self.node(id) {
            Node::GetModuleLet { target } => self.erased.contains(&target),
            Node::CallFunction { callee, .. } => {
                matches!(
                    connected::lookup(&self.qname(callee)),
                    Some(Connection::Erased)
                )
            }
            _ => false,
        }
    }

    /// A type as LuaCATS writes it, for LuaLS's checker.
    fn ty(&self, ty: TypeId) -> R<String> {
        Ok(match self.p.types.get(ty) {
            TypeData::Scalar(s) => match s {
                Scalar::Void => "nil",
                Scalar::Boolean => "boolean",
                Scalar::Int32 | Scalar::Int64 => "integer",
                Scalar::Float64 => "number",
                Scalar::String => "string",
                Scalar::AnyValue => "any",
            }
            .to_string(),
            TypeData::Nominal { decl } => {
                return not_yet(format!("a type annotation for {}", self.qname(decl)));
            }
            TypeData::Function { .. } => return not_yet("a function type"),
        })
    }

    fn exported(&self, decl: DeclId) -> bool {
        self.p.decls.get(decl).facts.contains(Facts::EXPORTED)
    }

    /// Translates the module into a chunk; `access` is what its tests use
    /// of it.
    pub fn module(&mut self, module: &Module, access: &TestAccess) -> R<tree::Chunk> {
        let p = self.p;
        self.here = module.decl;
        // Other modules' functions and `let`s, as locals; and in the root
        // module, the library's other modules it doesn't import, so
        // requiring the library runs every module, as in JS and Python.
        let mut imports = Vec::new();
        let imported = module_imports(p, module);
        if module.path.is_empty() {
            for other in &p.modules {
                if other.decl != module.decl && !imported.iter().any(|i| i.from == other.decl) {
                    let path = Expr::Str(require_path(p, other.decl));
                    imports.push(Stmt::Expr(Expr::name("require").call(vec![path])));
                }
            }
        }
        for import in imported {
            let path = require_path(p, import.from);
            for d in import.decls {
                if is_var(p, d)
                    || !matches!(
                        p.decls.get(d).kind,
                        DeclKind::Function | DeclKind::ModuleLet
                    )
                {
                    continue;
                }
                let required = Expr::name("require").call(vec![Expr::Str(path.clone())]);
                imports.push(Stmt::Local {
                    name: self.name(d),
                    ty: None,
                    value: Some(required.member(self.name(d))),
                });
            }
        }
        let state = module_state(p, module);
        for &item in module.items.iter().chain([&module.init]) {
            if let Node::ModuleLet { decl } = p.nodes.get(item) {
                if self.type_connection(p.decls.get(decl).ty) == Some(TypeConnection::Erased) {
                    self.erased.insert(decl);
                }
            }
        }
        // The module's own declarations, in source order.
        let declared: Vec<(NodeId, DeclId)> = (module.items.iter())
            .filter_map(|&item| match p.nodes.get(item) {
                Node::Function { decl, .. } => Some((item, decl)),
                Node::ModuleLet { decl } if !self.erased.contains(&decl) => Some((item, decl)),
                Node::BodilessFunction { .. } => None,
                _ => None,
            })
            .collect();
        for &item in &module.items {
            if let Node::BodilessFunction { decl, .. } = p.nodes.get(item) {
                return not_yet(format!("a connected function, {}", p.name(decl)));
            }
        }
        // Exported `var`s that functions write live in the exports table.
        for &(_, decl) in &declared {
            let d = p.decls.get(decl);
            if d.facts.contains(Facts::VAR)
                && self.exported(decl)
                && self.effects.mutable.contains(&decl)
            {
                self.fields.insert(decl, EXPORTS);
            }
        }
        let live = !self.fields.is_empty();
        // Past the threshold, those after the first few are a table's.
        let init_locals = max_locals(p, module.init) + imports.len() + 2 * access.vars.len();
        let locals = declared.len() + init_locals + 3;
        let overflow = locals > MODULE_LOCALS;
        if overflow {
            for &(_, decl) in declared.iter().skip(KEPT_LOCALS) {
                self.fields.entry(decl).or_insert(MODULE_TABLE);
            }
        }
        let kept = declared
            .iter()
            .filter(|(_, d)| !self.fields.contains_key(d))
            .count();
        if kept + init_locals + 3 > MAX_LOCALS {
            return not_yet(format!(
                "a module whose init block has {init_locals} locals alive at once"
            ));
        }

        let mut head = Vec::new();
        if live {
            head.push(Stmt::Local {
                name: EXPORTS.into(),
                ty: None,
                value: Some(Expr::Table(Vec::new())),
            });
        }
        if overflow {
            head.push(Stmt::Local {
                name: MODULE_TABLE.into(),
                ty: None,
                value: Some(Expr::Table(Vec::new())),
            });
        }
        // Module `let`s and `var`s declared before the functions.
        let mut first: HashSet<DeclId> = HashSet::new();
        let mut lets = Vec::new();
        for &(item, decl) in &declared {
            if !matches!(p.nodes.get(item), Node::ModuleLet { .. })
                || self.fields.contains_key(&decl)
            {
                continue;
            }
            if state.used_by_functions.contains(&decl) || !state.initialized_at_top.contains(&decl)
            {
                first.insert(decl);
                lets.push(Stmt::Local {
                    name: self.name(decl),
                    ty: Some(self.ty(p.decls.get(decl).ty)?),
                    value: None,
                });
            }
        }
        // Functions referred to by a function before them.
        let functions: Vec<(NodeId, DeclId)> = (declared.iter().copied())
            .filter(|&(item, _)| matches!(p.nodes.get(item), Node::Function { .. }))
            .collect();
        let mut forward = Vec::new();
        let mut seen: HashSet<DeclId> = HashSet::new();
        for &(item, decl) in &functions {
            seen.insert(decl);
            for d in module_refs(p, item) {
                if !seen.contains(&d)
                    && !self.fields.contains_key(&d)
                    && functions.iter().any(|&(_, f)| f == d)
                    && !forward.contains(&d)
                {
                    forward.push(d);
                }
            }
        }
        let mut defs = Vec::new();
        for &(item, decl) in &functions {
            let local = !forward.contains(&decl) && !self.fields.contains_key(&decl);
            defs.push(self.function(item, local)?);
        }

        // The init block.
        let mut init = Vec::new();
        self.begin(module.init);
        let stmts: Vec<NodeId> = match p.nodes.get(module.init) {
            Node::Block { stmts } => p.nodes.list(stmts).to_vec(),
            _ => vec![module.init],
        };
        for s in stmts {
            match p.nodes.get(s) {
                Node::InitLet { target, value }
                    if !self.erased.contains(&target)
                        && !first.contains(&target)
                        && !self.fields.contains_key(&target) =>
                {
                    let value = self.expr(value)?;
                    init.push(Stmt::Local {
                        name: self.name(target),
                        ty: None,
                        value: Some(value),
                    });
                }
                _ => self.stmt(s, &mut init)?,
            }
        }
        let init = reachable(init);

        // The exports, in source order, as Kotlin lists them.
        let exports: Vec<DeclId> = (declared.iter())
            .map(|&(_, d)| d)
            .filter(|&d| self.exported(d))
            .collect();
        let mut tail = Vec::new();
        // What its tests use, under `'$test'`, with a getter and a setter
        // for each `var`.
        let mut test_entries = Vec::new();
        for &d in access.used.iter().filter(|d| !access.vars.contains(d)) {
            test_entries.push((self.name(d), self.access(d)));
        }
        for &d in &access.vars {
            let name = self.name(d);
            let ty = self.ty(p.decls.get(d).ty)?;
            for suffix in ["__get", "__set"] {
                let f = format!("{name}{suffix}");
                if p.decls.ids().any(|o| self.names.get(o) == f) {
                    return not_yet(format!("a test helper named as a declaration, {f}"));
                }
                test_entries.push((f, Expr::name(format!("{name}{suffix}"))));
            }
            tail.push(Stmt::Function {
                local: true,
                name: format!("{name}__get"),
                signature: Signature {
                    params: Vec::new(),
                    result: Some(ty.clone()),
                },
                body: vec![Stmt::Return(Some(self.access(d)))],
            });
            tail.push(Stmt::Function {
                local: true,
                name: format!("{name}__set"),
                signature: Signature {
                    params: vec![("value".into(), ty)],
                    result: None,
                },
                body: vec![Stmt::Assign {
                    target: self.access(d),
                    value: Expr::name("value"),
                }],
            });
        }
        if !tail.is_empty() {
            tail.push(Stmt::Blank);
        }
        let test_table = (!test_entries.is_empty()).then_some(Expr::Table(test_entries));
        if live {
            if let Some(table) = test_table.clone() {
                tail.push(Stmt::Assign {
                    target: Expr::name(EXPORTS).member(TEST_KEY),
                    value: table,
                });
            }
            for &d in &exports {
                if self.fields.get(&d) != Some(&EXPORTS) {
                    tail.push(Stmt::Assign {
                        target: Expr::name(EXPORTS).member(self.name(d)),
                        value: self.access(d),
                    });
                }
            }
            tail.push(Stmt::Return(Some(Expr::name(EXPORTS))));
        } else {
            let mut entries: Vec<(String, Expr)> = (exports.iter())
                .map(|&d| (self.name(d), self.access(d)))
                .collect();
            if let Some(table) = test_table {
                entries.push((TEST_KEY.into(), table));
            }
            tail.push(Stmt::Return(Some(Expr::Table(entries))));
        }

        let mut body = Vec::new();
        if self.uses_temper {
            body.push(Stmt::Local {
                name: "temper".into(),
                ty: None,
                value: Some(Expr::name("require").call(vec![Expr::Str("temper-core".into())])),
            });
        }
        body.extend(imports);
        body.extend(head);
        if !lets.is_empty() || !forward.is_empty() {
            body.push(Stmt::Blank);
        }
        body.extend(lets);
        if !forward.is_empty() {
            body.push(Stmt::Locals(
                forward.iter().map(|&d| self.name(d)).collect(),
            ));
        }
        body.extend(defs);
        if !body.is_empty() {
            body.push(Stmt::Blank);
        }
        body.extend(init);
        if !body.is_empty() && !matches!(body.last(), Some(Stmt::Blank)) {
            body.push(Stmt::Blank);
        }
        body.extend(tail);
        Ok(tree::Chunk { body })
    }

    /// A module's tests, as a LuaUnit file, as Kotlin's: a function of
    /// the global `Test_` per test, each running `temper.test`, then
    /// LuaUnit run on them. Lua's std `Test` is the runtime's, which
    /// `temper.test` makes, so the tests need nothing of std's. `None` if
    /// it has none.
    pub fn tests(&mut self, module: &Module) -> R<Option<tree::Chunk>> {
        if module.tests.is_empty() {
            return Ok(None);
        }
        let p = self.p;
        self.here = DeclId::NONE;
        self.tested = module.decl;
        self.uses_temper = true;
        let mut used = Vec::new();
        let mut tests = Vec::new();
        for &test in &module.tests {
            let Node::Test {
                name, params, body, ..
            } = p.nodes.get(test)
            else {
                continue;
            };
            let [param] = p.nodes.list(params)[..] else {
                return not_yet("a test without one parameter");
            };
            let Node::Param { decl: param } = p.nodes.get(param) else {
                return not_yet("a test's parameter");
            };
            let text = p.strings.get(name).to_string();
            self.begin(test);
            let mut stmts = self.stmts(body)?;
            if stmts.last() == Some(&Stmt::Return(None)) {
                stmts.pop();
            }
            tests.push(Stmt::Test {
                method: format!("Test_.{}", test_method_name(&text, &mut used)),
                name: text,
                test: self.name(param),
                body: stmts,
            });
        }
        let require = |path: String| Expr::name("require").call(vec![Expr::Str(path)]);
        let mut body = vec![Stmt::Local {
            name: "temper".into(),
            ty: None,
            value: Some(require("temper-core".into())),
        }];
        body.push(Stmt::Local {
            name: "tested".into(),
            ty: None,
            value: Some(require(require_path(p, module.decl)).member(TEST_KEY)),
        });
        for import in temper_tmpl::analysis::test_imports(p, module) {
            let path = require_path(p, import.from);
            for d in import.decls {
                if is_var(p, d)
                    || !matches!(
                        p.decls.get(d).kind,
                        DeclKind::Function | DeclKind::ModuleLet
                    )
                {
                    continue;
                }
                body.push(Stmt::Local {
                    name: self.name(d),
                    ty: None,
                    value: Some(require(path.clone()).member(self.name(d))),
                });
            }
        }
        body.push(Stmt::Local {
            name: "luaunit".into(),
            ty: None,
            value: Some(require("luaunit".into())),
        });
        body.push(Stmt::Assign {
            target: Expr::name("luaunit").member("FAILURE_PREFIX"),
            value: Expr::name("temper").member("test_failure_prefix"),
        });
        body.push(Stmt::Blank);
        body.push(Stmt::Assign {
            target: Expr::name("Test_"),
            value: Expr::Table(Vec::new()),
        });
        body.extend(tests);
        // `luaunit.LuaUnit.run('--pattern', '^Test_%.', ...)`: the
        // tests file's own arguments after LuaUnit's pattern, as Kotlin's.
        let args = vec![
            Expr::Str("--pattern".into()),
            Expr::Str("^Test_%.".into()),
            Expr::name("..."),
        ];
        body.push(Stmt::Expr(
            Expr::name("luaunit")
                .member("LuaUnit")
                .member("run")
                .call(args),
        ));
        Ok(Some(tree::Chunk { body }))
    }

    fn function(&mut self, item: NodeId, local: bool) -> R<Stmt> {
        let p = self.p;
        let Node::Function { decl, params, body } = p.nodes.get(item) else {
            return not_yet("a function that isn't one");
        };
        let mut ps = Vec::new();
        for &param in p.nodes.list(params) {
            if let Node::Param { decl } = p.nodes.get(param) {
                ps.push((self.name(decl), self.ty(p.decls.get(decl).ty)?));
            }
        }
        let alive = ps.len() + max_locals(p, body);
        if alive > MAX_LOCALS {
            return not_yet(format!(
                "a function with {alive} locals alive at once, past Lua's {MAX_LOCALS}"
            ));
        }
        let result = match p.signature(decl) {
            Some(sig) if sig.result != TypeId::VOID => Some(self.ty(sig.result)?),
            _ => None,
        };
        self.begin(item);
        self.returns = result.is_some();
        let stmts = self.stmts(body);
        self.returns = false;
        let mut stmts = stmts?;
        // A `return` without a value that ends a function says nothing.
        if result.is_none() && stmts.last() == Some(&Stmt::Return(None)) {
            stmts.pop();
        }
        let name = match self.access(decl) {
            Expr::Member(table, field) => format!("{}.{field}", tree::expr(&table, 0)),
            e => tree::expr(&e, 0),
        };
        Ok(Stmt::Function {
            local,
            name,
            signature: Signature { params: ps, result },
            body: stmts,
        })
    }

    /// Starts translating a function or the init block: plans which loops
    /// need labels. A `continue` of the innermost loop goes to
    /// `continue`, ending its body; a jump from an inner loop, to a label
    /// named for the loop it leaves or continues: its source name, or for
    /// a compiler-made one, `outer`, numbered only if another label in the
    /// function has it.
    fn begin(&mut self, scope: NodeId) {
        /// Jumps to each loop: (loop, whether a `break`, whether from an
        /// inner loop).
        fn visit(
            p: &Program,
            id: NodeId,
            loops: &mut Vec<DeclId>,
            jumps: &mut Vec<(DeclId, bool, bool)>,
        ) {
            match p.nodes.get(id) {
                Node::While { label, .. } | Node::DoWhile { label, .. } => {
                    loops.push(label);
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, jumps);
                    }
                    loops.pop();
                }
                Node::Break { label } | Node::Continue { label } => {
                    let brk = matches!(p.nodes.get(id), Node::Break { .. });
                    let inner = loops.last() != Some(&label);
                    if (inner || !brk) && !jumps.contains(&(label, brk, inner)) {
                        jumps.push((label, brk, inner));
                    }
                }
                _ => {
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, jumps);
                    }
                }
            }
        }
        let p = self.p;
        let mut jumps = Vec::new();
        visit(p, scope, &mut Vec::new(), &mut jumps);
        self.loops.clear();
        self.labels.clear();
        let mut taken: HashSet<String> = HashSet::new();
        for &(l, brk, inner) in &jumps {
            let labels = self.labels.entry(l).or_default();
            if brk || inner || labels.next.is_some() {
                continue;
            }
            // A `continue` only from the loop itself.
            if !jumps.contains(&(l, false, true)) {
                labels.next = Some("continue".into());
            }
        }
        for (l, brk, inner) in jumps {
            if !inner {
                continue;
            }
            let d = p.decls.get(l);
            let base = if d.facts.contains(Facts::COMPILER_MADE) || p.name(l).is_empty() {
                "outer".to_string()
            } else {
                p.name(l)
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect()
            };
            let base = format!("{}_{base}", if brk { "break" } else { "continue" });
            let name = (1..)
                .map(|n| match n {
                    1 => base.clone(),
                    n => format!("{base}{n}"),
                })
                .find(|n| !taken.contains(n))
                .unwrap_or_default();
            taken.insert(name.clone());
            let labels = self.labels.entry(l).or_default();
            if brk {
                labels.end = Some(name);
            } else {
                labels.next = Some(name);
            }
        }
    }

    fn stmts(&mut self, id: NodeId) -> R<Vec<Stmt>> {
        let mut out = Vec::new();
        match self.p.nodes.get(id) {
            Node::Block { stmts } => {
                for &s in self.p.nodes.list(stmts) {
                    self.stmt(s, &mut out)?;
                }
            }
            _ => self.stmt(id, &mut out)?,
        }
        Ok(reachable(out))
    }

    fn loop_body(&mut self, label: DeclId, body: NodeId) -> R<Vec<Stmt>> {
        self.loops.push(label);
        let body = self.stmts(body);
        self.loops.pop();
        body
    }

    fn stmt(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            // A block inside a block keeps its scope.
            Node::Block { .. } => out.push(Stmt::Do(self.stmts(id)?)),
            Node::Let { local, init } => {
                let name = self.name(local);
                out.push(match init {
                    Some(init) => Stmt::Local {
                        name,
                        ty: None,
                        value: Some(self.expr(init)?),
                    },
                    None => Stmt::Local {
                        name,
                        ty: Some(self.ty(p.decls.get(local).ty)?),
                        value: None,
                    },
                });
            }
            Node::SetModuleLet { target, value } if home(p, target) == Some(self.tested) => {
                let value = self.expr(value)?;
                let setter = format!("{}__set", self.name(target));
                out.push(Stmt::Expr(Expr::name("tested").method(setter, vec![value])));
            }
            Node::Assign {
                local: target,
                value,
            }
            | Node::SetModuleLet { target, value }
            | Node::InitLet { target, value } => {
                if self.erased.contains(&target) {
                    return Ok(());
                }
                let value = self.expr(value)?;
                let target = self.access(target);
                // `x = x` does nothing.
                if value != target {
                    out.push(Stmt::Assign { target, value });
                }
            }
            Node::If { cond, then, els } => {
                let cond = self.expr(cond)?;
                let then = self.stmts(then)?;
                let els = match els {
                    Some(els) => self.stmts(els)?,
                    None => Vec::new(),
                };
                out.push(Stmt::If { cond, then, els });
            }
            Node::While { label, cond, body } => {
                let cond = self.expr(cond)?;
                let body = self.loop_body(label, body)?;
                let labels = self.labels.get(&label).cloned().unwrap_or_default();
                out.push(Stmt::While {
                    cond,
                    body,
                    next: labels.next,
                });
                out.extend(labels.end.map(Stmt::Label));
            }
            Node::DoWhile { label, body, cond } => {
                let body = self.loop_body(label, body)?;
                let cond = self.expr(cond)?.negate();
                let labels = self.labels.get(&label).cloned().unwrap_or_default();
                out.push(Stmt::Repeat {
                    body,
                    cond,
                    next: labels.next,
                });
                out.extend(labels.end.map(Stmt::Label));
            }
            Node::Break { label } | Node::Continue { label } => {
                let brk = matches!(p.nodes.get(id), Node::Break { .. });
                if brk && self.loops.last() == Some(&label) {
                    out.push(Stmt::Break);
                    return Ok(());
                }
                let labels = self.labels.get(&label);
                let target = if brk {
                    labels.and_then(|l| l.end.clone())
                } else {
                    labels.and_then(|l| l.next.clone())
                };
                match target {
                    Some(l) => out.push(Stmt::Goto(l)),
                    None => return not_yet("a jump to a loop that doesn't enclose it"),
                }
            }
            Node::Return { value } => match value.map(|v| (v, p.nodes.get(v))) {
                None | Some((_, Node::VoidLit {})) => out.push(Stmt::Return(None)),
                Some((_, Node::Error { .. })) => out.push(self.bubble_stmt()),
                Some((v, _)) => out.push(Stmt::Return(Some(self.expr(v)?))),
            },
            Node::ExprStatement { expr } if matches!(p.nodes.get(expr), Node::Error { .. }) => {
                out.push(self.bubble_stmt());
            }
            Node::ExprStatement { expr } => {
                let e = self.expr(expr)?;
                if e.is_statement() {
                    out.push(Stmt::Expr(e));
                } else {
                    // Lua takes only calls as statements.
                    out.push(Stmt::Local {
                        name: "_".into(),
                        ty: None,
                        value: Some(e),
                    });
                }
            }
            other => return not_yet(format!("a {} statement", other.kind().name())),
        }
        Ok(())
    }

    fn bubble(&mut self) -> Expr {
        self.uses_temper = true;
        ops::temper().method("bubble", Vec::new())
    }

    /// A bubble as a statement: `return temper.bubble()` in a function
    /// with a result, which never returns, but LuaLS then sees every path
    /// end in a `return`; elsewhere, the call.
    fn bubble_stmt(&mut self) -> Stmt {
        let bubble = self.bubble();
        if self.returns {
            Stmt::Return(Some(bubble))
        } else {
            Stmt::Expr(bubble)
        }
    }

    /// A call of a connected declaration, by the connection table; `args`
    /// has a method's receiver first.
    fn connected(&mut self, decl: DeclId, args: &[NodeId]) -> R<Expr> {
        let qname = self.qname(decl);
        let Some(connection) = connected::lookup(&qname) else {
            return not_yet(format!("a connection for {qname}"));
        };
        match connection {
            Connection::Math(name) => {
                let args = self.args(args)?;
                Ok(Expr::name("math").method(name, args))
            }
            Connection::Translate(f) => f(self, args),
            Connection::Erased => not_yet(format!("a value of {qname}, whose type is erased")),
        }
    }

    /// Arguments, none left out.
    fn args(&mut self, args: &[NodeId]) -> R<Vec<Expr>> {
        args.iter()
            .map(|&a| match self.p.nodes.get(a) {
                Node::Omitted {} => not_yet("a default parameter"),
                _ => self.expr(a),
            })
            .collect()
    }

    /// Whether evaluating an expression does anything its order, or its
    /// being skipped or repeated, could show.
    fn has_effects(&self, id: NodeId) -> bool {
        self.effects.node(self.p, id).any()
    }

    pub(crate) fn expr(&mut self, id: NodeId) -> R<Expr> {
        let p = self.p;
        Ok(match p.nodes.get(id) {
            Node::BoolLit { value } => Expr::Bool(value),
            Node::Int32Lit { value } => Expr::Int(value.into()),
            Node::Int64Lit { value } => Expr::Int(value),
            Node::Float64Lit { value } => Expr::Float(value),
            Node::StringLit { value } => Expr::Str(p.strings.get(value).to_string()),
            Node::VoidLit {} => Expr::Nil,
            Node::Local { local: decl } => self.access(decl),
            Node::GetModuleLet { target } => {
                if self.erased.contains(&target) {
                    return not_yet("a value whose type is erased");
                }
                self.access(target)
            }
            Node::CallFunction { callee, args, .. } => {
                let args = p.nodes.list(args);
                if is_external(p, callee) && library_of(p, callee) == "core" {
                    return self.connected(callee, args);
                }
                let args = self.args(args)?;
                self.access(callee).call(args)
            }
            Node::CallMethod {
                method,
                receiver,
                args,
                ..
            } => {
                if !is_external(p, method) {
                    return not_yet("a method of the program's own");
                }
                let mut all = vec![receiver];
                all.extend_from_slice(p.nodes.list(args));
                self.connected(method, &all)?
            }
            Node::Op { op, args, .. } => {
                let Some(mut spelling) = ops::spelling(op) else {
                    return not_yet("an operator without a spelling");
                };
                if spelling == Spelling::Cat {
                    return self.cat(id);
                }
                let list = p.nodes.list(args);
                // A negated literal is a literal, wrapped as the type
                // wraps (`-(-2147483648)` is itself).
                if let [arg] = list {
                    match (spelling, p.nodes.get(*arg)) {
                        (Spelling::Helper("int32_unm"), Node::Int32Lit { value }) => {
                            return Ok(Expr::Int(value.wrapping_neg().into()));
                        }
                        (Spelling::Helper("int64_unm"), Node::Int64Lit { value }) => {
                            return Ok(Expr::Int(value.wrapping_neg()));
                        }
                        (Spelling::Unary(tree::Unary::Neg), Node::Float64Lit { value }) => {
                            return Ok(Expr::Float(-value));
                        }
                        _ => {}
                    }
                }
                // Lua's `and` and `or` skip their right side when the left
                // decides, so `Boolean`'s order is the runtime's where the
                // right side does something.
                if let (Spelling::BoolOrder(cmp), [_, right]) = (spelling, list) {
                    if self.has_effects(*right) {
                        spelling = Spelling::CmpWith("bool_cmp", cmp);
                    }
                }
                let args: Vec<Expr> = list.iter().map(|&a| self.expr(a)).collect::<R<_>>()?;
                if ops::helper(spelling).is_some() {
                    self.uses_temper = true;
                }
                ops::apply(spelling, args)
            }
            Node::And { left, right } => Expr::bin(Bin::And, self.expr(left)?, self.expr(right)?),
            Node::Or { left, right } => Expr::bin(Bin::Or, self.expr(left)?, self.expr(right)?),
            Node::IfValue { cond, then, els } => self.if_value(cond, then, els)?,
            Node::Error { .. } => self.bubble(),
            other => return not_yet(format!("a {} expression", other.kind().name())),
        })
    }

    /// An `if` value, as `c and a or b`, which is right when `a` can't be
    /// `false` or `nil`: when it isn't a `Boolean` (or `Void`, or
    /// `AnyValue`). A `Boolean` literal there gives `c or b`, or `not c
    /// and b`; another `Boolean`, `(c and a) or (not c and b)`, when
    /// evaluating `c` twice can't show. Otherwise it waits for the hoisting
    /// pass, which will give it a temporary.
    fn if_value(&mut self, cond: NodeId, then: NodeId, els: NodeId) -> R<Expr> {
        let p = self.p;
        let falsy = matches!(
            p.types.get(p.nodes.ty(then)),
            TypeData::Scalar(Scalar::Boolean | Scalar::Void | Scalar::AnyValue)
        );
        let c = self.expr(cond)?;
        if !falsy {
            let a = self.expr(then)?;
            let b = self.expr(els)?;
            return Ok(Expr::bin(Bin::Or, Expr::bin(Bin::And, c, a), b));
        }
        match p.nodes.get(then) {
            Node::BoolLit { value: true } => Ok(Expr::bin(Bin::Or, c, self.expr(els)?)),
            Node::BoolLit { value: false } => Ok(Expr::bin(Bin::And, c.negate(), self.expr(els)?)),
            _ if !self.has_effects(cond)
                && matches!(
                    p.types.get(p.nodes.ty(then)),
                    TypeData::Scalar(Scalar::Boolean)
                ) =>
            {
                let a = self.expr(then)?;
                let b = self.expr(els)?;
                Ok(Expr::bin(
                    Bin::Or,
                    Expr::bin(Bin::And, c.clone(), a),
                    Expr::bin(Bin::And, c.negate(), b),
                ))
            }
            _ => not_yet("an `if` value of a type Lua's `and` and `or` can't choose"),
        }
    }

    /// A chain of `String.cat` as one `..`: its literal parts as they
    /// are, an `Int32.toString()` without a radix as the integer itself,
    /// which `..` writes the same way, and a `Boolean.toString()` as
    /// `tostring(b)`. All literals give one string.
    fn cat(&mut self, id: NodeId) -> R<Expr> {
        let mut parts = Vec::new();
        self.cat_parts(id, &mut parts)?;
        if parts.iter().all(|(e, _)| matches!(e, Expr::Str(_))) {
            let text: String = parts
                .iter()
                .filter_map(|(e, _)| match e {
                    Expr::Str(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect();
            return Ok(Expr::Str(text));
        }
        // Lua's `..` needs two operands; a number alone needs `tostring`.
        if let [(e, number)] = parts.as_slice() {
            return Ok(if *number {
                Expr::name("tostring").call(vec![e.clone()])
            } else {
                e.clone()
            });
        }
        Ok(Expr::Concat(parts.into_iter().map(|(e, _)| e).collect()))
    }

    /// Each part, and whether it's a number that `..` converts.
    fn cat_parts(&mut self, id: NodeId, parts: &mut Vec<(Expr, bool)>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Op { op, args, .. } if ops::spelling(op) == Some(Spelling::Cat) => {
                for &a in p.nodes.list(args) {
                    self.cat_parts(a, parts)?;
                }
            }
            Node::StringLit { value } => {
                let text = p.strings.get(value);
                if text.is_empty() {
                    return Ok(());
                }
                if let Some((Expr::Str(last), _)) = parts.last_mut() {
                    last.push_str(text);
                } else {
                    parts.push((Expr::Str(text.to_string()), false));
                }
            }
            Node::CallMethod {
                method,
                receiver,
                args,
                ..
            } if self.qname(method) == "core.type Int32.toString()"
                && p.nodes
                    .list(args)
                    .iter()
                    .all(|&a| matches!(p.nodes.get(a), Node::Omitted {})) =>
            {
                let e = self.expr(receiver)?;
                parts.push((e, true));
            }
            _ => {
                let e = self.expr(id)?;
                parts.push((e, false));
            }
        }
        Ok(())
    }
}

/// The statements that can run: none after a `return`, `break`, or
/// `goto` in the same block, which Lua requires of `return`.
fn reachable(mut stmts: Vec<Stmt>) -> Vec<Stmt> {
    if let Some(i) = stmts.iter().position(Stmt::ends) {
        stmts.truncate(i + 1);
    }
    stmts
}

/// The most locals alive at once in a block or statement: its own, and
/// those of the blocks nested in it.
fn max_locals(p: &Program, id: NodeId) -> usize {
    match p.nodes.get(id) {
        Node::Block { stmts } => {
            let (mut alive, mut most) = (0, 0);
            for &s in p.nodes.list(stmts) {
                if let Node::Let { .. } = p.nodes.get(s) {
                    alive += 1;
                }
                most = most.max(alive + nested(p, s));
            }
            most
        }
        Node::Let { .. } => 1,
        _ => nested(p, id),
    }
}

fn nested(p: &Program, id: NodeId) -> usize {
    match p.nodes.get(id) {
        Node::Block { .. } => max_locals(p, id),
        Node::If { then, els, .. } => max_locals(p, then).max(els.map_or(0, |e| max_locals(p, e))),
        Node::While { body, .. } | Node::DoWhile { body, .. } => max_locals(p, body),
        Node::Function { body, .. } => max_locals(p, body),
        _ => 0,
    }
}

/// A test's function name: `test_` and its name in snake case, ASCII
/// letters, digits, and `_` only, numbered past those already used.
fn test_method_name(text: &str, used: &mut Vec<String>) -> String {
    let mut name = String::from("test_");
    for c in temper_tmpl::naming::snake_case(text).chars() {
        if c.is_ascii_alphanumeric() {
            name.push(c);
        } else if !name.ends_with('_') {
            name.push('_');
        }
    }
    let name = name.trim_end_matches('_').to_string();
    let name = if name == "test" {
        "test_".to_string()
    } else {
        name
    };
    let mut unique = name.clone();
    let mut n = 2;
    while used.contains(&unique) {
        unique = format!("{name}_{n}");
        n += 1;
    }
    used.push(unique.clone());
    unique
}
