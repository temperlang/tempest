//! The translation from tmpl to Python's output tree, for subset A, with
//! modules and imports: a module refers to another's functions and
//! `let`s by `from MODULE import NAME`, and to its `var`s through the
//! module (`import MODULE as ALIAS`, `ALIAS.NAME`), so a read is current
//! and a write lands there. Tests go in a file of their own
//! ([`Translator::tests`]), each a method of a `unittest.TestCase`.
//! Classes and interfaces, and their members' uses, are in
//! [`crate::classes`].

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

pub(crate) type R<T> = Result<T, NotYet>;

fn not_yet<T>(what: impl Into<String>) -> R<T> {
    Err(NotYet { what: what.into() })
}

pub struct Translator<'a> {
    pub(crate) p: &'a Program,
    pub(crate) names: &'a NameMap,
    qnames: QNames,
    /// Imported names, by module.
    imports: BTreeMap<String, BTreeSet<String>>,
    /// Modules imported whole, for their `var`s: each module's alias.
    modules: BTreeMap<String, String>,
    /// The module whose file this is; `NONE` for a tests file.
    pub(crate) here: DeclId,
    /// The class or interface whose member this is; `NONE` outside one.
    pub(crate) class: DeclId,
    /// This module's classes and interfaces defined so far, which
    /// annotations needn't quote.
    pub(crate) defined: BTreeSet<DeclId>,
    /// Each class's and interface's node, of any module, by declaration.
    pub(crate) class_nodes: BTreeMap<DeclId, NodeId>,
    /// The enclosing loops' labels, innermost last.
    loops: Vec<DeclId>,
    /// Statements that must run before the expression being translated,
    /// hoisted out of it (an `orelse`'s `try`, a block's statements), in
    /// order; the statement being translated takes them first.
    pre: Vec<Stmt>,
    /// Locals whose `let` a `??` became, to assign where first read, as
    /// `(subject := f())`.
    walrus: BTreeMap<DeclId, Expr>,
    /// Temporaries made so far, for their numbers.
    temps: usize,
    /// How many functions deep the translation is: 0 at module level.
    in_def: usize,
    /// Whether the statement being translated checks a `var` local for
    /// null.
    var_null_check: bool,
}

/// Where a value goes, for [`Translator::value_into`].
enum Sink {
    Return,
    /// Assigned to a name or attribute, with an annotation the first time.
    Assign(Expr, Option<String>),
    /// Dropped.
    Discard,
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap) -> Translator<'a> {
        let mut class_nodes = BTreeMap::new();
        for module in &p.modules {
            for &item in &module.items {
                if let Node::Class { decl, .. } | Node::Interface { decl, .. } = p.nodes.get(item) {
                    class_nodes.insert(decl, item);
                }
            }
        }
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            imports: BTreeMap::new(),
            modules: BTreeMap::new(),
            here: DeclId::NONE,
            class: DeclId::NONE,
            defined: BTreeSet::new(),
            class_nodes,
            loops: Vec::new(),
            pre: Vec::new(),
            walrus: BTreeMap::new(),
            temps: 0,
            in_def: 0,
            var_null_check: false,
        }
    }

    pub(crate) fn import(&mut self, from: &str, name: &str) {
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
            return crate::names::library_package(&library_name(p.name(module)));
        }
        if !d.owner.is_none() {
            let package = crate::names::library_package(&library_name(p.name(d.owner)));
            return dotted(&package, p.name(module));
        }
        let package = crate::names::library_package(&p.library.name);
        let path = p.modules.iter().find(|m| m.decl == module);
        match path.map(|m| m.path.as_str()) {
            None | Some("") => format!("{package}.{package}"),
            Some(path) => dotted(&package, path),
        }
    }

    /// A module-level declaration of this module's, or another's,
    /// imported: its name, or for a `var` of another module's, its
    /// module's alias and its name.
    pub(crate) fn top(&mut self, decl: DeclId) -> Expr {
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

    pub(crate) fn name(&self, decl: DeclId) -> String {
        self.names.get(decl).to_string()
    }

    pub(crate) fn qname(&self, decl: DeclId) -> String {
        self.qnames
            .get(decl)
            .map(|q| q.to_string())
            .unwrap_or_default()
    }

    /// A type as Python annotates it: the module's own classes and
    /// interfaces quoted until they're defined.
    pub(crate) fn ann(&mut self, ty: TypeId) -> R<String> {
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
            TypeData::Nominal { decl, .. } => match self.qname(decl).as_str() {
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
                _ if !is_external(self.p, decl) => match self.top(decl) {
                    Expr::Name(name)
                        if home(self.p, decl) == Some(self.here)
                            && !self.defined.contains(&decl) =>
                    {
                        tree::string_literal(&name)
                    }
                    Expr::Name(name) => name,
                    _ => return not_yet("a type annotation"),
                },
                q => return not_yet(format!("a type annotation for {q}")),
            },
            TypeData::Function { .. } => return not_yet("a function type"),
            TypeData::Nullable(inner) => {
                let inner = self.ann(inner)?;
                // A quoted name, not yet defined, is quoted as a whole.
                match inner.strip_prefix('\'').and_then(|i| i.strip_suffix('\'')) {
                    Some(name) => tree::string_literal(&format!("{name} | None")),
                    None => format!("{inner} | None"),
                }
            }
            TypeData::TypeParam { .. } => return not_yet("a type parameter"),
        })
    }

    pub fn module(&mut self, module: &Module) -> R<tree::Module> {
        self.here = module.decl;
        self.defined.clear();
        let mut body = Vec::new();
        let mut done = BTreeSet::new();
        for &item in &module.items {
            match self.p.nodes.get(item) {
                Node::Function { decl, .. } => {
                    let name = self.name(decl);
                    body.push(self.function(item, name, Vec::new())?);
                }
                Node::BodilessFunction { decl, .. } => {
                    return not_yet(format!("a connected function, {}", self.p.name(decl)));
                }
                Node::Class { .. } | Node::Interface { .. } => {
                    self.class_after_supers(item, &mut done, &mut body)?;
                }
                // Declared where its init block initializes it.
                _ => {}
            }
        }
        self.block(module.init, &mut body)?;
        tree::mark_unused(&mut body);
        Ok(self.finish(body))
    }

    /// A function, named, its parameters' names and annotations first,
    /// with `self` first for a method or constructor (`params` itself not
    /// including it); its body declares any module `var` it assigns
    /// `global`.
    pub(crate) fn def(
        &mut self,
        item: NodeId,
        decl: DeclId,
        params: temper_tmpl::Kids,
        body: Option<NodeId>,
        name: String,
        decorators: Vec<String>,
    ) -> R<Stmt> {
        let kind = self.p.decls.get(decl).kind;
        let is_method = matches!(
            kind,
            DeclKind::Method | DeclKind::Getter | DeclKind::Setter | DeclKind::Constructor
        );
        let mut ps = Vec::new();
        if is_method {
            ps.push(("self".to_string(), String::new()));
        }
        for &param in self.p.nodes.list(params) {
            if let Node::Param { decl } = self.p.nodes.get(param) {
                let ty = self.p.decls.get(decl).ty;
                ps.push((self.name(decl), self.ann(ty)?));
            }
        }
        let result = match self.p.signature(decl) {
            _ if kind == DeclKind::Constructor => "None".into(),
            Some(sig) => self.ann(sig.result)?,
            None => "None".into(),
        };
        // A constructor's parameters may be passed by name, as Kotlin's.
        let positional = if kind == DeclKind::Constructor {
            1
        } else {
            ps.len()
        };
        let mut stmts = Vec::new();
        match body {
            Some(block) => {
                let globals: Vec<String> = module_writes(self.p, item)
                    .into_iter()
                    .map(|d| self.name(d))
                    .collect();
                if !globals.is_empty() {
                    stmts.push(Stmt::Global(globals));
                }
                self.in_def += 1;
                let temps = std::mem::take(&mut self.temps);
                let body = self.block(block, &mut stmts);
                self.temps = temps;
                self.in_def -= 1;
                body?;
            }
            None => stmts.push(Stmt::Expr(Expr::Ellipsis)),
        }
        Ok(Stmt::Def {
            decorators,
            name,
            params: ps,
            positional,
            result,
            body: stmts,
        })
    }

    /// A function node: top-level, or a class's static one.
    pub(crate) fn function(
        &mut self,
        item: NodeId,
        name: String,
        decorators: Vec<String>,
    ) -> R<Stmt> {
        let Node::Function {
            decl, params, body, ..
        } = self.p.nodes.get(item)
        else {
            return not_yet("a function that isn't one");
        };
        self.def(item, decl, params, Some(body), name, decorators)
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
            self.in_def += 1;
            let temps = std::mem::take(&mut self.temps);
            let translated = self.block(body, &mut stmts);
            self.temps = temps;
            self.in_def -= 1;
            translated?;
            let test_var = self.name(param);
            let soft_fail = Expr::name(&test_var).attr("soft_fail_to_hard").call(vec![]);
            let method = test_method_name(&text, &mut used);
            methods.push(Stmt::Def {
                decorators: Vec::new(),
                name: method,
                params: vec![("self".into(), String::new())],
                positional: 1,
                result: "None".into(),
                body: vec![
                    Stmt::Expr(Expr::Str(text)),
                    Stmt::Assign {
                        target: Expr::name(test_var),
                        ann: Some(test_class.clone()),
                        value: Expr::name(test_class).call(vec![]),
                    },
                    Stmt::Try {
                        body: stmts,
                        except: None,
                        finally: vec![Stmt::Expr(soft_fail)],
                    },
                ],
            });
        }
        self.import("unittest", "TestCase");
        let body = vec![Stmt::Class {
            name: class.to_string(),
            bases: vec!["TestCase".into()],
            body: methods,
        }];
        Ok(Some(self.finish(body)))
    }

    pub(crate) fn block(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
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

    /// A nested block's statements, apart from any statements hoisted
    /// for the statement around it.
    fn stmts(&mut self, id: NodeId) -> R<Vec<Stmt>> {
        let saved = std::mem::take(&mut self.pre);
        let mut out = Vec::new();
        let done = self.block(id, &mut out);
        self.pre = saved;
        done?;
        Ok(out)
    }

    /// Takes the hoisted statements into `out`, before the statement
    /// that needs them.
    fn flush(&mut self, out: &mut Vec<Stmt>) {
        out.append(&mut self.pre);
    }

    /// An expression translated apart: the statements it needs first, and
    /// it.
    fn apart(&mut self, id: NodeId) -> R<(Vec<Stmt>, Expr)> {
        let saved = std::mem::take(&mut self.pre);
        let e = self.expr(id);
        let pre = std::mem::replace(&mut self.pre, saved);
        Ok((pre, e?))
    }

    /// A fresh temporary's name: `t_N` in a function, `_t_N` at module
    /// level (private, as Kotlin's), past any name in use.
    fn temp(&mut self) -> String {
        loop {
            let n = self.temps;
            self.temps += 1;
            let name = if self.in_def > 0 {
                format!("t_{n}")
            } else {
                format!("_t_{n}")
            };
            let taken = (self.p.decls.ids()).any(|d| self.names.get(d) == name)
                || self.modules.values().any(|a| *a == name);
            if !taken {
                return name;
            }
        }
    }

    /// A value put where `sink` says, as statements: an `orelse` as
    /// `try`, `bubble()` and `panic()` as `raise`, else the value's
    /// statements and then the value.
    fn value_into(&mut self, id: NodeId, sink: Sink, out: &mut Vec<Stmt>) -> R<()> {
        let saved = std::mem::take(&mut self.pre);
        let done = self.value_into_apart(id, sink, out);
        self.pre = saved;
        done
    }

    fn value_into_apart(&mut self, id: NodeId, sink: Sink, out: &mut Vec<Stmt>) -> R<()> {
        match self.p.nodes.get(id) {
            Node::OrElse { body, fallback } => {
                let sink = match sink {
                    Sink::Assign(target, Some(ann)) => {
                        if let Expr::Name(name) = &target {
                            out.push(Stmt::Declare {
                                target: name.clone(),
                                ann,
                            });
                        }
                        Sink::Assign(target, None)
                    }
                    sink => sink,
                };
                let mut tried = Vec::new();
                let mut caught = Vec::new();
                let again = match &sink {
                    Sink::Return => Sink::Return,
                    Sink::Assign(t, ann) => Sink::Assign(t.clone(), ann.clone()),
                    Sink::Discard => Sink::Discard,
                };
                self.value_into(body, sink, &mut tried)?;
                self.value_into(fallback, again, &mut caught)?;
                out.push(Stmt::Try {
                    body: tried,
                    except: Some(caught),
                    finally: Vec::new(),
                });
            }
            Node::Bubble { .. } => out.push(Stmt::Raise(Expr::name("RuntimeError").call(vec![]))),
            Node::Panic {} => out.push(panic()),
            _ => {
                let value = self.expr(id)?;
                self.flush(out);
                match sink {
                    Sink::Return => out.push(Stmt::Return(Some(value))),
                    Sink::Assign(target, ann) => out.push(Stmt::Assign { target, ann, value }),
                    Sink::Discard if is_simple(&value) => {}
                    Sink::Discard => out.push(Stmt::Expr(value)),
                }
            }
        }
        Ok(())
    }

    /// Expressions in evaluation order, `None` for each `Omitted`: where a
    /// later one hoists statements, earlier ones that aren't simple are
    /// kept in temporaries first, so they still run first.
    pub(crate) fn ordered(&mut self, ids: &[NodeId]) -> R<Vec<Option<Expr>>> {
        let mut out: Vec<(Option<Expr>, usize)> = Vec::new();
        for &a in ids {
            let e = match self.p.nodes.get(a) {
                Node::Omitted {} => None,
                _ => Some(self.expr(a)?),
            };
            out.push((e, self.pre.len()));
        }
        let end = self.pre.len();
        for j in (0..out.len()).rev() {
            let at = out[j].1;
            let simple = out[j].0.as_ref().is_none_or(is_simple);
            if at == end || simple || out[j + 1..].iter().all(|(_, m)| *m == at) {
                continue;
            }
            let t = self.temp();
            let value = out[j].0.take().unwrap();
            self.pre.insert(
                at,
                Stmt::Assign {
                    target: Expr::name(&t),
                    ann: None,
                    value,
                },
            );
            out[j].0 = Some(Expr::name(t));
        }
        Ok(out.into_iter().map(|(e, _)| e).collect())
    }

    /// A temporary for a value that needs statements: declared here, then
    /// set by `set`.
    fn temp_of(&mut self, ty: TypeId) -> R<String> {
        let t = self.temp();
        let ann = self.ann(ty)?;
        self.pre.push(Stmt::Declare {
            target: t.clone(),
            ann,
        });
        Ok(t)
    }

    fn stmt(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        let outer = std::mem::take(&mut self.var_null_check);
        let start = out.len();
        let done = self.stmt_inner(id, out);
        // pyright narrows a `var` by what's assigned to it, so a later
        // null check may look unneeded to it, though another path may
        // assign null.
        if std::mem::replace(&mut self.var_null_check, outer) && out.len() > start {
            let last = out.pop().unwrap();
            let rule = "pyright: ignore[reportUnnecessaryComparison]".to_string();
            out.push(Stmt::Commented(Box::new(last), rule));
        }
        done
    }

    fn stmt_inner(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Block { .. } => self.block(id, out)?,
            Node::Let { local, init } => {
                let target = self.name(local);
                match init {
                    Some(init) => {
                        // Nullable ones are annotated, so a later value
                        // fits.
                        let ty = p.decls.get(local).ty;
                        let ann = if p.types.is_nullable(ty) {
                            Some(self.ann(ty)?)
                        } else {
                            None
                        };
                        self.value_into(init, Sink::Assign(Expr::name(target), ann), out)?;
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
                let target = Expr::name(self.name(target));
                self.value_into(value, Sink::Assign(target, None), out)?;
            }
            Node::SetModuleLet { target, value } => {
                let value = self.expr(value)?;
                let target = self.top(target);
                self.flush(out);
                out.push(Stmt::Assign {
                    target,
                    ann: None,
                    value,
                });
            }
            Node::InitLet { target, value } => {
                let ann = Some(self.ann(p.decls.get(target).ty)?);
                let target = Expr::name(self.name(target));
                self.value_into(value, Sink::Assign(target, ann), out)?;
            }
            Node::SetField {
                field,
                object,
                value,
            } => {
                let [object, value] = self.exprs(&[object, value])?;
                self.flush(out);
                out.push(Stmt::Assign {
                    target: self.field(object, field),
                    ann: None,
                    value,
                });
            }
            Node::InitStatic { target, value } => {
                let value = self.expr(value)?;
                self.flush(out);
                let stmt = Stmt::Assign {
                    target: self.static_ref(target),
                    ann: None,
                    value,
                };
                out.push(self.private_use(target, stmt));
            }
            Node::ExprStatement { expr } if self.is_setter_call(expr) => {
                let stmt = self.setter_call(expr)?;
                self.flush(out);
                out.push(stmt);
            }
            Node::If { cond, then, els } => {
                let cond = self.expr(cond)?;
                let then = self.stmts(then)?;
                let els = match els {
                    Some(els) => self.stmts(els)?,
                    None => Vec::new(),
                };
                self.flush(out);
                out.push(Stmt::If { cond, then, els });
            }
            Node::While { label, cond, body } => {
                let (before, cond) = self.apart(cond)?;
                self.loops.push(label);
                let body = self.stmts(body);
                self.loops.pop();
                let mut body = body?;
                if before.is_empty() {
                    out.push(Stmt::While { cond, body });
                } else {
                    // The condition's statements run before each test.
                    let mut first = before;
                    first.push(Stmt::If {
                        cond: not(cond),
                        then: vec![Stmt::Break],
                        els: Vec::new(),
                    });
                    first.append(&mut body);
                    out.push(Stmt::While {
                        cond: Expr::Bool(true),
                        body: first,
                    });
                }
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
                let (mut before, cond) = self.apart(cond)?;
                body.append(&mut before);
                body.push(Stmt::If {
                    cond: not(cond),
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
            Node::Return { value: Some(value) } => self.value_into(value, Sink::Return, out)?,
            Node::Return { value: None } => out.push(Stmt::Return(None)),
            Node::ExprStatement { expr } => self.value_into(expr, Sink::Discard, out)?,
            other => return not_yet(format!("a {} statement", other.kind().name())),
        }
        Ok(())
    }

    /// Arguments, with `None` for each left out.
    pub(crate) fn args(&mut self, args: &[NodeId]) -> R<Vec<Option<Expr>>> {
        self.ordered(args)
    }

    /// Expressions in evaluation order, none `Omitted`.
    pub(crate) fn exprs<const N: usize>(&mut self, ids: &[NodeId; N]) -> R<[Expr; N]> {
        let out = present(self.ordered(ids)?)?;
        Ok(out.try_into().unwrap_or_else(|_| unreachable!()))
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
            Connection::Runtime(name) => {
                self.import("temper_core", name);
                Expr::name(name).call(present(args)?)
            }
            Connection::Identity => {
                let mut args = present(args)?;
                if args.is_empty() {
                    return not_yet(format!("a receiver for {qname}"));
                }
                args.remove(0)
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

    pub(crate) fn expr(&mut self, id: NodeId) -> R<Expr> {
        let p = self.p;
        Ok(match p.nodes.get(id) {
            Node::BoolLit { value } => Expr::Bool(value),
            Node::Int32Lit { value } => Expr::Int(value.into()),
            Node::Int64Lit { value } => Expr::Int(value),
            Node::Float64Lit { value } => Expr::Float(value),
            Node::StringLit { value } => Expr::Str(p.strings.get(value).to_string()),
            Node::VoidLit {} => Expr::None,
            Node::Local { local: decl } => match self.walrus.remove(&decl) {
                Some(value) => Expr::Walrus(self.name(decl), Box::new(value)),
                None => Expr::name(self.name(decl)),
            },
            Node::GetModuleLet { target: decl } => self.top(decl),
            Node::CallFunction { callee, args, .. } => {
                let args = self.args(p.nodes.list(args))?;
                if is_external(p, callee) && library_of(p, callee) == "core" {
                    return self.connected(callee, args);
                }
                let present = present(args)?;
                if self.is_member(callee) {
                    let class = self.p.decls.get(callee).owner;
                    let name = self.member_name(callee);
                    return Ok(self.class_ref(class).attr(name).call(present));
                }
                self.top(callee).call(present)
            }
            Node::CallMethod {
                method,
                receiver,
                args,
                ..
            } => {
                if !is_external(p, method) || library_of(p, method) != "core" {
                    return self.method_call(id);
                }
                let mut ids = vec![receiver];
                ids.extend_from_slice(p.nodes.list(args));
                let all = self.ordered(&ids)?;
                self.connected(method, all)?
            }
            Node::Op { op, args, .. } => {
                let Some(spelling) = ops::spelling(op) else {
                    return not_yet("an operator without a spelling");
                };
                if spelling == Spelling::Cat {
                    return self.cat(id);
                }
                let ids = p.nodes.list(args);
                // `x is not None` for `not (x is None)`.
                if let (Spelling::Unary(tree::Unary::Not), [only]) = (spelling, ids) {
                    if let Node::IsNull { .. } = p.nodes.get(*only) {
                        let Expr::Bin(_, value, none) = self.expr(*only)? else {
                            unreachable!("a null check is `is None`")
                        };
                        return Ok(Expr::Bin(Bin::IsNot, value, none));
                    }
                }
                let args = present(self.ordered(ids)?)?;
                // A negated literal that can't overflow, as `-1`.
                if let (Spelling::Helper(_, "int_negate" | "int64_negate"), [Expr::Int(n)]) =
                    (spelling, &args[..])
                {
                    if *n != i64::from(i32::MIN) && *n != i64::MIN {
                        return Ok(Expr::Int(-n));
                    }
                }
                if let Spelling::FloatCmp(_) = spelling {
                    self.import("temper_core", "float_cmp");
                }
                if let Spelling::Helper(from, name) = spelling {
                    self.import(from, name);
                }
                match ops::apply(spelling, args) {
                    Some(e) => e,
                    None => return not_yet("a comparison of strings that aren't simple"),
                }
            }
            Node::And { left, right } | Node::Or { left, right } => {
                let and = matches!(p.nodes.get(id), Node::And { .. });
                let left = self.expr(left)?;
                let (before, right) = self.apart(right)?;
                let op = if and { Bin::And } else { Bin::Or };
                if before.is_empty() {
                    return Ok(Expr::bin(op, left, right));
                }
                // The right side's statements run only where it's needed.
                let t = self.temp_of(TypeId::BOOLEAN)?;
                self.pre.push(Stmt::Assign {
                    target: Expr::name(&t),
                    ann: None,
                    value: left,
                });
                let mut then = before;
                set_unless_raised(&mut then, &t, right);
                let cond = if and {
                    Expr::name(&t)
                } else {
                    not(Expr::name(&t))
                };
                self.pre.push(Stmt::If {
                    cond,
                    then,
                    els: Vec::new(),
                });
                Expr::name(t)
            }
            Node::IfValue { cond, then, els } => {
                let cond = self.expr(cond)?;
                let (then_pre, then) = self.apart(then)?;
                let (els_pre, els) = self.apart(els)?;
                if then_pre.is_empty() && els_pre.is_empty() {
                    return Ok(Expr::IfElse {
                        cond: Box::new(cond),
                        then: Box::new(then),
                        els: Box::new(els),
                    });
                }
                let t = self.temp_of(p.nodes.ty(id))?;
                let (mut a, mut b) = (then_pre, els_pre);
                set_unless_raised(&mut a, &t, then);
                set_unless_raised(&mut b, &t, els);
                self.pre.push(Stmt::If {
                    cond,
                    then: a,
                    els: b,
                });
                Expr::name(t)
            }
            Node::BlockValue { stmts, value } => {
                let list = p.nodes.list(stmts);
                // `??`: `d if (subject := e) is None else subject`.
                if let ([only], Node::IfValue { cond, .. }) = (list, p.nodes.get(value)) {
                    if let (
                        Node::Let {
                            local,
                            init: Some(init),
                        },
                        Node::IsNull { value: read },
                    ) = (p.nodes.get(*only), p.nodes.get(cond))
                    {
                        if matches!(p.nodes.get(read), Node::Local { local: l } if l == local) {
                            let init = self.expr(init)?;
                            self.walrus.insert(local, init);
                            return self.expr(value);
                        }
                    }
                }
                let mut stmts = Vec::new();
                for &s in list {
                    self.stmt(s, &mut stmts)?;
                }
                self.pre.append(&mut stmts);
                self.expr(value)?
            }
            Node::Null {} => Expr::None,
            Node::IsNull { value } => {
                if let Node::Local { local } = p.nodes.get(value) {
                    self.var_null_check |= p.decls.get(local).facts.contains(Facts::VAR);
                }
                Expr::bin(Bin::Is, self.expr(value)?, Expr::None)
            }
            Node::NonNull { value } | Node::AsNullable { value } => self.expr(value)?,
            Node::EqNullable { op, left, right } => {
                let [left, right] = self.exprs(&[left, right])?;
                match ops::spelling(op) {
                    Some(Spelling::Bin(Bin::Eq)) => Expr::bin(Bin::Eq, left, right),
                    Some(Spelling::Helper(from, name)) => {
                        // `a is b if a is None or b is None else eq(a, b)`.
                        self.import(from, name);
                        let left = self.simple(left);
                        let right = self.simple(right);
                        let either = Expr::bin(
                            Bin::Or,
                            Expr::bin(Bin::Is, left.clone(), Expr::None),
                            Expr::bin(Bin::Is, right.clone(), Expr::None),
                        );
                        Expr::IfElse {
                            cond: Box::new(either),
                            then: Box::new(Expr::bin(Bin::Is, left.clone(), right.clone())),
                            els: Box::new(Expr::name(name).call(vec![left, right])),
                        }
                    }
                    _ => return not_yet("a nullable equality without a spelling"),
                }
            }
            Node::OrElse { .. } => {
                let ty = p.nodes.ty(id);
                if ty == TypeId::VOID {
                    let mut stmts = Vec::new();
                    self.value_into(id, Sink::Discard, &mut stmts)?;
                    self.pre.append(&mut stmts);
                    return Ok(Expr::None);
                }
                let t = self.temp_of(ty)?;
                let mut stmts = Vec::new();
                self.value_into(id, Sink::Assign(Expr::name(&t), None), &mut stmts)?;
                self.pre.append(&mut stmts);
                Expr::name(t)
            }
            Node::Bubble { .. } => {
                self.import("temper_core", "bubble");
                Expr::name("bubble").call(Vec::new())
            }
            Node::Panic {} => {
                self.pre.push(panic());
                Expr::None
            }
            Node::Error { .. } => {
                self.import("temper_core", "bubble");
                Expr::name("bubble").call(Vec::new())
            }
            Node::This {} => Expr::name("self"),
            Node::GetField { field, object } => {
                let object = self.expr(object)?;
                self.field(object, field)
            }
            Node::GetStatic { target } => self.static_ref(target),
            Node::New { ctor, args, .. } => {
                let args = present(self.args(p.nodes.list(args))?)?;

                let class = self.p.decls.get(ctor).owner;
                self.class_ref(class).call(args)
            }
            // Python's checkers narrow by `isinstance` as Temper does.
            Node::Upcast { value } | Node::Downcast { value } => self.expr(value)?,
            Node::As { value, .. } => {
                let value = self.expr(value)?;
                let class = match p.types.get(p.nodes.ty(id)) {
                    // `x as T` for `x: T?`: a check that it isn't null.
                    TypeData::Scalar(s) => Expr::name(match s {
                        Scalar::Boolean => "bool",
                        Scalar::Int32 | Scalar::Int64 => "int",
                        Scalar::Float64 => "float",
                        Scalar::String => "str",
                        _ => return not_yet("an as cast to Void or AnyValue"),
                    }),
                    _ => self.type_ref(p.nodes.ty(id))?,
                };
                self.import("temper_core", "cast_by_type");
                Expr::name("cast_by_type").call(vec![value, class])
            }
            Node::Is { ty, value } => {
                let value = self.expr(value)?;
                let class = self.type_ref(ty)?;
                Expr::name("isinstance").call(vec![value, class])
            }
            Node::Identical { left, right } => {
                Expr::bin(Bin::Is, self.expr(left)?, self.expr(right)?)
            }
            other => return not_yet(format!("a {} expression", other.kind().name())),
        })
    }

    /// A chain of `String.cat` as one f-string: its literal parts as they
    /// are, and an `Int32.toString()` without a radix as the int itself.
    fn cat(&mut self, id: NodeId) -> R<Expr> {
        let mut parts = Vec::new();
        let mut marks = Vec::new();
        self.cat_parts(id, &mut parts, &mut marks)?;
        // Where a later part hoists statements, earlier parts that aren't
        // simple are kept in temporaries first, so they still run first.
        let end = self.pre.len();
        for j in (0..marks.len()).rev() {
            let (i, at) = marks[j];
            let FPart::Expr { expr, .. } = &mut parts[i] else {
                continue;
            };
            if at == end || is_simple(expr) || marks[j + 1..].iter().all(|&(_, m)| m == at) {
                continue;
            }
            let t = self.temp();
            let value = std::mem::replace(expr, Expr::name(&t));
            self.pre.insert(
                at,
                Stmt::Assign {
                    target: Expr::name(t),
                    ann: None,
                    value,
                },
            );
        }
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

    fn cat_parts(
        &mut self,
        id: NodeId,
        parts: &mut Vec<FPart>,
        marks: &mut Vec<(usize, usize)>,
    ) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Op { op, args, .. } if ops::spelling(op) == Some(Spelling::Cat) => {
                for &a in p.nodes.list(args) {
                    self.cat_parts(a, parts, marks)?;
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
                marks.push((parts.len(), self.pre.len()));
                parts.push(FPart::Expr { expr, str: true });
            }
            _ => {
                let expr = self.expr(id)?;
                marks.push((parts.len(), self.pre.len()));
                parts.push(FPart::Expr { expr, str: false });
            }
        }
        Ok(())
    }
}

/// Arguments, each present: one left out needs its default, which waits
/// for default parameters.
pub(crate) fn present(args: Vec<Option<Expr>>) -> R<Vec<Expr>> {
    args.into_iter()
        .map(|a| a.map_or_else(|| not_yet("a default parameter"), Ok))
        .collect()
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

/// Whether an expression may be repeated or moved freely: a name or a
/// literal.
fn is_simple(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Name(_) | Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Bool(_) | Expr::None
    )
}

fn not(e: Expr) -> Expr {
    Expr::unary(tree::Unary::Not, e)
}

/// `panic()`: raised past any `orelse`, which catches only `Exception`s.
/// Kotlin's raises `RuntimeError`, as `bubble()` does, so an `orelse`
/// caught it (notes/kotlin-differences.md).
fn panic() -> Stmt {
    Stmt::Raise(Expr::name("BaseException").call(vec![Expr::Str("panic".into())]))
}

/// `t = value` after `stmts`, unless they end by raising.
fn set_unless_raised(stmts: &mut Vec<Stmt>, t: &str, value: Expr) {
    if !matches!(stmts.last(), Some(Stmt::Raise(_))) {
        stmts.push(Stmt::Assign {
            target: Expr::name(t),
            ann: None,
            value,
        });
    }
}

impl Translator<'_> {
    /// `e` itself if it's simple, else a temporary holding it.
    fn simple(&mut self, e: Expr) -> Expr {
        if is_simple(&e) {
            return e;
        }
        let t = self.temp();
        self.pre.push(Stmt::Assign {
            target: Expr::name(&t),
            ann: None,
            value: e,
        });
        Expr::name(t)
    }
}
