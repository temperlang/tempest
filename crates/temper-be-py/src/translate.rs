//! The translation from tmpl to Python's output tree, for subset A, with
//! modules and imports: a module refers to another's functions and
//! `let`s by `from MODULE import NAME`, and to its `var`s through the
//! module (`import MODULE as ALIAS`, `ALIAS.NAME`), so a read is current
//! and a write lands there. Tests go in a file of their own
//! ([`Translator::tests`]), each a method of a `unittest.TestCase`.

use crate::NotYet;
use crate::connected::{self, Connection};
use crate::ops::{self, Spelling};
use crate::tree::{self, Bin, Expr, FPart, Stmt};
use std::collections::{BTreeMap, BTreeSet};
use temper_tmpl::analysis::{home, module_writes};
use temper_tmpl::naming::NameMap;
use temper_tmpl::qname::{is_external, library_of};
use temper_tmpl::{
    DeclId, DeclKind, Facts, Module, Node, NodeId, Program, QNames, Scalar, TypeData, TypeId,
};

type R<T> = Result<T, NotYet>;

fn not_yet<T>(what: impl Into<String>) -> R<T> {
    Err(NotYet { what: what.into() })
}

pub struct Translator<'a> {
    p: &'a Program,
    names: &'a NameMap,
    qnames: QNames,
    /// Imported names, by module.
    imports: BTreeMap<String, BTreeSet<String>>,
    /// Modules imported whole, for their `var`s: each module's alias.
    modules: BTreeMap<String, String>,
    /// The module whose file this is; `NONE` for a tests file.
    here: DeclId,
    /// The enclosing loops' labels, innermost last.
    loops: Vec<DeclId>,
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap) -> Translator<'a> {
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            imports: BTreeMap::new(),
            modules: BTreeMap::new(),
            here: DeclId::NONE,
            loops: Vec::new(),
        }
    }

    fn import(&mut self, from: &str, name: &str) {
        let names = self.imports.entry(from.to_string()).or_default();
        names.insert(name.to_string());
    }

    /// The Python module a module of tmpl's is, as `pkg.helpers` (the
    /// root module, `pkg.pkg`), or another library's, as
    /// `other.helpers` (its root, `other`).
    pub fn py_module(&self, module: DeclId) -> String {
        let p = self.p;
        let d = p.decls.get(module);
        if d.kind == DeclKind::Library {
            return crate::names::package_name(&library_name(p.name(module)));
        }
        if !d.owner.is_none() {
            let package = crate::names::package_name(&library_name(p.name(d.owner)));
            return dotted(&package, p.name(module));
        }
        let package = crate::names::package_name(&p.library.name);
        let path = p.modules.iter().find(|m| m.decl == module);
        match path.map(|m| m.path.as_str()) {
            None | Some("") => format!("{package}.{package}"),
            Some(path) => dotted(&package, path),
        }
    }

    /// A module-level declaration of this module's, or another's,
    /// imported: its name, or for a `var` of another module's, its
    /// module's alias and its name.
    fn top(&mut self, decl: DeclId) -> Expr {
        let name = self.name(decl);
        let from = home(self.p, decl).unwrap_or(DeclId::NONE);
        if from == self.here {
            return Expr::name(name);
        }
        let module = self.py_module(from);
        let d = self.p.decls.get(decl);
        if d.kind == DeclKind::ModuleLet && d.facts.contains(Facts::VAR) {
            let alias = self.module_alias(&module);
            return Expr::name(alias).attr(&name);
        }
        self.import(&module, &name);
        Expr::name(name)
    }

    /// The alias a module imported whole goes by: `_m_` and its dotted
    /// name, with `_` for the dots, numbered past any name in use.
    fn module_alias(&mut self, module: &str) -> String {
        if let Some(alias) = self.modules.get(module) {
            return alias.clone();
        }
        let base = format!("_m_{}", module.replace('.', "_"));
        let taken = |n: &str| {
            (self.p.decls.ids()).any(|d| self.names.get(d) == n)
                || self.modules.values().any(|a| a == n)
        };
        let mut alias = base.clone();
        let mut n = 2;
        while taken(&alias) {
            alias = format!("{base}_{n}");
            n += 1;
        }
        self.modules.insert(module.to_string(), alias.clone());
        alias
    }

    fn name(&self, decl: DeclId) -> String {
        self.names.get(decl).to_string()
    }

    fn qname(&self, decl: DeclId) -> String {
        self.qnames
            .get(decl)
            .map(|q| q.to_string())
            .unwrap_or_default()
    }

    /// A type as Python annotates it.
    fn ann(&mut self, ty: TypeId) -> R<String> {
        Ok(match self.p.types.get(ty) {
            TypeData::Scalar(s) => match s {
                Scalar::Void => "None",
                Scalar::Boolean => "bool",
                Scalar::Int32 | Scalar::Int64 => "int",
                Scalar::Float64 => "float",
                Scalar::String => "str",
                Scalar::AnyValue => "object",
            }
            .to_string(),
            TypeData::Nominal { decl } => match self.qname(decl).as_str() {
                "core.type Console" => {
                    self.import("temper_core", "LoggingConsole");
                    "LoggingConsole".to_string()
                }
                _ if is_external(self.p, decl) && library_of(self.p, decl) != "core" => {
                    match self.top(decl) {
                        Expr::Name(name) => name,
                        _ => return not_yet("a type annotation"),
                    }
                }
                q => return not_yet(format!("a type annotation for {q}")),
            },
            TypeData::Function { .. } => return not_yet("a function type"),
        })
    }

    pub fn module(&mut self, module: &Module) -> R<tree::Module> {
        self.here = module.decl;
        let mut body = Vec::new();
        for &item in &module.items {
            match self.p.nodes.get(item) {
                Node::Function {
                    decl,
                    params,
                    body: block,
                    ..
                } => {
                    let mut ps = Vec::new();
                    for &param in self.p.nodes.list(params) {
                        if let Node::Param { decl } = self.p.nodes.get(param) {
                            let ty = self.p.decls.get(decl).ty;
                            ps.push((self.name(decl), self.ann(ty)?));
                        }
                    }
                    let result = match self.p.signature(decl) {
                        Some(sig) => self.ann(sig.result)?,
                        None => "None".into(),
                    };
                    let mut stmts = Vec::new();
                    let globals: Vec<String> = module_writes(self.p, item)
                        .into_iter()
                        .map(|d| self.name(d))
                        .collect();
                    if !globals.is_empty() {
                        stmts.push(Stmt::Global(globals));
                    }
                    self.block(block, &mut stmts)?;
                    body.push(Stmt::Def {
                        name: self.name(decl),
                        params: ps,
                        result,
                        body: stmts,
                    });
                }
                Node::BodilessFunction { decl, .. } => {
                    return not_yet(format!("a connected function, {}", self.p.name(decl)));
                }
                // Declared where its init block initializes it.
                _ => {}
            }
        }
        self.block(module.init, &mut body)?;
        Ok(self.finish(body))
    }

    /// The file's imports, taken, with its body.
    fn finish(&mut self, body: Vec<Stmt>) -> tree::Module {
        let imports = std::mem::take(&mut self.imports);
        let imports = (imports.into_iter())
            .map(|(from, names)| (from, names.into_iter().collect()))
            .collect();
        let modules = std::mem::take(&mut self.modules).into_iter().collect();
        tree::Module {
            modules,
            imports,
            body,
        }
    }

    /// A module's tests, as a `unittest` file: one `TestCase` class, named
    /// for the module, with a method per test, as `test_adds` for "adds",
    /// whose docstring is the test's name. Each makes std's `Test`, runs
    /// the test's body, and then, even if the body raised, fails if the
    /// `Test` has failures (`softFailToHard`), as Kotlin's do. `None` if
    /// the module has no tests.
    pub fn tests(&mut self, module: &Module, class: &str) -> R<Option<tree::Module>> {
        if module.tests.is_empty() {
            return Ok(None);
        }
        self.here = DeclId::NONE;
        let mut methods = Vec::new();
        let mut used = BTreeSet::new();
        for &test in &module.tests {
            let Node::Test {
                name, params, body, ..
            } = self.p.nodes.get(test)
            else {
                continue;
            };
            let text = self.p.strings.get(name).to_string();
            let [param] = self.p.nodes.list(params)[..] else {
                return not_yet("a test without one parameter");
            };
            let Node::Param { decl: param } = self.p.nodes.get(param) else {
                return not_yet("a test's parameter");
            };
            let test_class = match self.ann(self.p.decls.get(param).ty)? {
                ann if ann == "Test" => ann,
                _ => return not_yet("a test's parameter that isn't std's Test"),
            };
            let mut stmts = Vec::new();
            self.block(body, &mut stmts)?;
            let test_var = self.name(param);
            let soft_fail = Expr::name(&test_var).attr("soft_fail_to_hard").call(vec![]);
            let method = test_method_name(&text, &mut used);
            methods.push(Stmt::Def {
                name: method,
                params: vec![("self".into(), String::new())],
                result: "None".into(),
                body: vec![
                    Stmt::Expr(Expr::Str(text)),
                    Stmt::Assign {
                        target: test_var,
                        ann: Some(test_class.clone()),
                        value: Expr::name(test_class).call(vec![]),
                    },
                    Stmt::Try {
                        body: stmts,
                        finally: vec![Stmt::Expr(soft_fail)],
                    },
                ],
            });
        }
        self.import("unittest", "TestCase");
        let body = vec![Stmt::Class {
            name: class.to_string(),
            base: "TestCase".into(),
            body: methods,
        }];
        Ok(Some(self.finish(body)))
    }

    fn block(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        match self.p.nodes.get(id) {
            Node::Block { stmts } => {
                for &s in self.p.nodes.list(stmts) {
                    self.stmt(s, out)?;
                }
                Ok(())
            }
            _ => self.stmt(id, out),
        }
    }

    fn stmts(&mut self, id: NodeId) -> R<Vec<Stmt>> {
        let mut out = Vec::new();
        self.block(id, &mut out)?;
        Ok(out)
    }

    fn stmt(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Block { .. } => self.block(id, out)?,
            Node::Let { local, init } => {
                let target = self.name(local);
                match init {
                    Some(init) => {
                        let value = self.expr(init)?;
                        out.push(Stmt::Assign {
                            target,
                            ann: None,
                            value,
                        });
                    }
                    None => {
                        let ann = self.ann(p.decls.get(local).ty)?;
                        out.push(Stmt::Declare { target, ann });
                    }
                }
            }
            Node::Assign {
                local: target,
                value,
            } => {
                let value = self.expr(value)?;
                out.push(Stmt::Assign {
                    target: self.name(target),
                    ann: None,
                    value,
                });
            }
            Node::SetModuleLet { target, value } => {
                let value = self.expr(value)?;
                let target = match self.top(target) {
                    Expr::Name(name) => name,
                    Expr::Attr(module, name) => match *module {
                        Expr::Name(module) => format!("{module}.{name}"),
                        _ => return not_yet("an assignment's target"),
                    },
                    _ => return not_yet("an assignment's target"),
                };
                out.push(Stmt::Assign {
                    target,
                    ann: None,
                    value,
                });
            }
            Node::InitLet { target, value } => {
                let ann = Some(self.ann(p.decls.get(target).ty)?);
                let value = self.expr(value)?;
                out.push(Stmt::Assign {
                    target: self.name(target),
                    ann,
                    value,
                });
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
                self.loops.push(label);
                let body = self.stmts(body);
                self.loops.pop();
                out.push(Stmt::While { cond, body: body? });
            }
            Node::DoWhile { label, body, cond } => {
                let mut continues = false;
                temper_tmpl::analysis::walk(p, body, &mut |n| {
                    continues |=
                        matches!(p.nodes.get(n), Node::Continue { label: l } if l == label);
                });
                if continues {
                    return not_yet("`continue` in a `do … while` loop");
                }
                self.loops.push(label);
                let body = self.stmts(body);
                self.loops.pop();
                let mut body = body?;
                let cond = self.expr(cond)?;
                body.push(Stmt::If {
                    cond: Expr::unary(tree::Unary::Not, cond),
                    then: vec![Stmt::Break],
                    els: Vec::new(),
                });
                out.push(Stmt::While {
                    cond: Expr::Bool(true),
                    body,
                });
            }
            Node::Break { label } | Node::Continue { label } => {
                if self.loops.last() != Some(&label) {
                    return not_yet("a jump out of a loop that isn't the innermost");
                }
                let jump = if matches!(p.nodes.get(id), Node::Break { .. }) {
                    Stmt::Break
                } else {
                    Stmt::Continue
                };
                out.push(jump);
            }
            Node::Return { value } => {
                let value = value.map(|v| self.expr(v)).transpose()?;
                out.push(Stmt::Return(value));
            }
            Node::ExprStatement { expr } => {
                let e = self.expr(expr)?;
                out.push(Stmt::Expr(e));
            }
            other => return not_yet(format!("a {} statement", other.kind().name())),
        }
        Ok(())
    }

    /// Arguments, with `None` for each left out.
    fn args(&mut self, args: &[NodeId]) -> R<Vec<Option<Expr>>> {
        args.iter()
            .map(|&a| match self.p.nodes.get(a) {
                Node::Omitted {} => Ok(None),
                _ => self.expr(a).map(Some),
            })
            .collect()
    }

    /// A call of a connected declaration, by the connection table.
    fn connected(&mut self, decl: DeclId, args: Vec<Option<Expr>>) -> R<Expr> {
        let qname = self.qname(decl);
        let Some(connection) = connected::lookup(&qname) else {
            return not_yet(format!("a connection for {qname}"));
        };
        let present = |args: Vec<Option<Expr>>| -> R<Vec<Expr>> {
            let mut out = Vec::new();
            let mut gap = false;
            for a in args {
                match a {
                    Some(_) if gap => return not_yet("an argument after one left out"),
                    Some(a) => out.push(a),
                    None => gap = true,
                }
            }
            Ok(out)
        };
        Ok(match connection {
            Connection::Builtin(name) => Expr::name(name).call(present(args)?),
            Connection::Method(name) => {
                let mut args = present(args)?;
                if args.is_empty() {
                    return not_yet(format!("a receiver for {qname}"));
                }
                let receiver = args.remove(0);
                receiver.attr(name).call(args)
            }
            Connection::Translate(f) => {
                let (e, imports) = f(args);
                for &name in imports {
                    self.import("temper_core", name);
                }
                e
            }
        })
    }

    fn expr(&mut self, id: NodeId) -> R<Expr> {
        let p = self.p;
        Ok(match p.nodes.get(id) {
            Node::BoolLit { value } => Expr::Bool(value),
            Node::Int32Lit { value } => Expr::Int(value.into()),
            Node::Int64Lit { value } => Expr::Int(value),
            Node::Float64Lit { value } => Expr::Float(value),
            Node::StringLit { value } => Expr::Str(p.strings.get(value).to_string()),
            Node::VoidLit {} => Expr::None,
            Node::Local { local: decl } => Expr::name(self.name(decl)),
            Node::GetModuleLet { target: decl } => self.top(decl),
            Node::CallFunction { callee, args, .. } => {
                let args = self.args(p.nodes.list(args))?;
                if is_external(p, callee) && library_of(p, callee) == "core" {
                    return self.connected(callee, args);
                }
                let mut present = Vec::new();
                for a in args {
                    match a {
                        Some(a) => present.push(a),
                        None => return not_yet("a default parameter"),
                    }
                }
                self.top(callee).call(present)
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
                let mut all = vec![Some(self.expr(receiver)?)];
                all.extend(self.args(p.nodes.list(args))?);
                self.connected(method, all)?
            }
            Node::Op { op, args, .. } => {
                let Some(spelling) = ops::spelling(op) else {
                    return not_yet("an operator without a spelling");
                };
                if spelling == Spelling::Cat {
                    return self.cat(id);
                }
                let args: Vec<Expr> = p
                    .nodes
                    .list(args)
                    .iter()
                    .map(|&a| self.expr(a))
                    .collect::<R<_>>()?;
                if let Spelling::Helper(from, name) = spelling {
                    self.import(from, name);
                }
                match ops::apply(spelling, args) {
                    Some(e) => e,
                    None => return not_yet("a comparison of strings that aren't simple"),
                }
            }
            Node::And { left, right } => Expr::bin(Bin::And, self.expr(left)?, self.expr(right)?),
            Node::Or { left, right } => Expr::bin(Bin::Or, self.expr(left)?, self.expr(right)?),
            Node::IfValue { cond, then, els } => Expr::IfElse {
                cond: Box::new(self.expr(cond)?),
                then: Box::new(self.expr(then)?),
                els: Box::new(self.expr(els)?),
            },
            Node::Error { .. } => {
                self.import("temper_core", "bubble");
                Expr::name("bubble").call(Vec::new())
            }
            other => return not_yet(format!("a {} expression", other.kind().name())),
        })
    }

    /// A chain of `String.cat` as one f-string: its literal parts as they
    /// are, and an `Int32.toString()` without a radix as the int itself.
    fn cat(&mut self, id: NodeId) -> R<Expr> {
        let mut parts = Vec::new();
        self.cat_parts(id, &mut parts)?;
        let lits = parts.iter().filter(|p| matches!(p, FPart::Lit(_))).count();
        if lits == parts.len() {
            let text: String = parts
                .iter()
                .filter_map(|p| match p {
                    FPart::Lit(s) => Some(s.as_str()),
                    FPart::Expr { .. } => None,
                })
                .collect();
            return Ok(Expr::Str(text));
        }
        if lits == 0 {
            let terms = parts.into_iter().map(|p| match p {
                FPart::Expr { expr, str: true } => Expr::name("str").call(vec![expr]),
                FPart::Expr { expr, .. } => expr,
                FPart::Lit(s) => Expr::Str(s),
            });
            return Ok(terms
                .reduce(|a, b| Expr::bin(Bin::Add, a, b))
                .unwrap_or(Expr::Str(String::new())));
        }
        Ok(Expr::FString(parts))
    }

    fn cat_parts(&mut self, id: NodeId, parts: &mut Vec<FPart>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Op { op, args, .. } if ops::spelling(op) == Some(Spelling::Cat) => {
                for &a in p.nodes.list(args) {
                    self.cat_parts(a, parts)?;
                }
            }
            Node::StringLit { value } => {
                let text = p.strings.get(value);
                if let Some(FPart::Lit(last)) = parts.last_mut() {
                    last.push_str(text);
                } else {
                    parts.push(FPart::Lit(text.to_string()));
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
                let expr = self.expr(receiver)?;
                parts.push(FPart::Expr { expr, str: true });
            }
            _ => {
                let expr = self.expr(id)?;
                parts.push(FPart::Expr { expr, str: false });
            }
        }
        Ok(())
    }
}

/// A library's name as its Python package is named: std's is
/// `temper-std`, as Kotlin's.
pub fn library_name(name: &str) -> String {
    match name {
        "std" => "temper-std".into(),
        _ => name.into(),
    }
}

/// `package.a.b` for a module's path `a/b`.
fn dotted(package: &str, path: &str) -> String {
    let mut out = package.to_string();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        out.push('.');
        out.push_str(&crate::names::package_name(part));
    }
    out
}

/// A test's method name: `test_` and its name in snake case, letters,
/// digits, and `_` only (Python's identifiers take Unicode letters),
/// numbered past those already used.
fn test_method_name(text: &str, used: &mut BTreeSet<String>) -> String {
    let mut name = String::from("test_");
    for c in temper_tmpl::naming::snake_case(text).chars() {
        if c.is_alphanumeric() {
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
    while !used.insert(unique.clone()) {
        unique = format!("{name}_{n}");
        n += 1;
    }
    unique
}
