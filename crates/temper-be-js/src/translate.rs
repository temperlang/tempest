//! The translation from tmpl to JS's output tree, for subset A.
//!
//! Module state is plain, as Kotlin's: module `let`s and `var`s are a
//! module's own `const`s and `let`s, initialized as the module's code
//! runs, once, when it's first imported. One the init block initializes
//! at its top level is declared there, with its value; others are
//! declared first, typed but without a value. A `Console` is erased
//! ([`connected::TYPES`]): logging is the global `console.log`.

use crate::NotYet;
use crate::connected::{self, Connection, TypeConnection};
use crate::names::{KEYWORDS, SPECIAL};
use crate::ops::{self, Spelling};
use crate::tree::{self, Bin, Expr, Keyword, Stmt, TPart};
use std::collections::{BTreeSet, HashMap, HashSet};
use temper_tmpl::analysis::{module_state, walk};
use temper_tmpl::naming::NameMap;
use temper_tmpl::qname::is_external;
use temper_tmpl::{DeclId, Facts, Module, Node, NodeId, Program, QNames, Scalar, TypeData, TypeId};

type R<T> = Result<T, NotYet>;

fn not_yet<T>(what: impl Into<String>) -> R<T> {
    Err(NotYet { what: what.into() })
}

pub struct Translator<'a> {
    p: &'a Program,
    names: &'a NameMap,
    qnames: QNames,
    /// `@temperlang/core` names imported.
    imports: BTreeSet<&'static str>,
    /// Module `let`s of an erased type, which aren't declared.
    erased: HashSet<DeclId>,
    /// Locals assigned after they're declared, which are `let`, not
    /// `const`.
    assigned: HashSet<DeclId>,
    /// The enclosing loops, innermost last.
    loops: Vec<DeclId>,
    /// Labels printed, by loop: only those a jump from an inner loop
    /// needs.
    labels: HashMap<DeclId, String>,
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap) -> Translator<'a> {
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            imports: BTreeSet::new(),
            erased: HashSet::new(),
            assigned: HashSet::new(),
            loops: Vec::new(),
            labels: HashMap::new(),
        }
    }

    pub(crate) fn node(&self, id: NodeId) -> Node {
        self.p.nodes.get(id)
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

    /// A type as JSDoc writes it, for TypeScript's checker.
    fn ty(&self, ty: TypeId) -> R<String> {
        Ok(match self.p.types.get(ty) {
            TypeData::Scalar(s) => match s {
                Scalar::Void => "void",
                Scalar::Boolean => "boolean",
                Scalar::Int32 | Scalar::Float64 => "number",
                Scalar::Int64 => "bigint",
                Scalar::String => "string",
                Scalar::AnyValue => "unknown",
            }
            .to_string(),
            TypeData::Nominal { decl } => match self.qname(decl).as_str() {
                "core.type Console" => "Console".to_string(),
                q => return not_yet(format!("a type annotation for {q}")),
            },
            TypeData::Function { .. } => return not_yet("a function type"),
        })
    }

    fn exported(&self, decl: DeclId) -> bool {
        self.p.decls.get(decl).facts.contains(Facts::EXPORTED)
    }

    pub fn module(&mut self, module: &Module) -> R<tree::Module> {
        let p = self.p;
        let state = module_state(p, module);
        for &item in module.items.iter().chain([&module.init]) {
            if let Node::ModuleLet { decl } = p.nodes.get(item) {
                if self.type_connection(p.decls.get(decl).ty) == Some(TypeConnection::Erased) {
                    self.erased.insert(decl);
                }
            }
            walk(p, item, &mut |id| {
                if let Node::Assign { local, .. } = p.nodes.get(id) {
                    self.assigned.insert(local);
                }
            });
        }

        let mut body = Vec::new();
        for &item in &module.items {
            match p.nodes.get(item) {
                Node::Function { .. } => body.push(self.function(item)?),
                Node::BodilessFunction { decl, .. } => {
                    return not_yet(format!("a connected function, {}", p.name(decl)));
                }
                _ => {}
            }
        }
        // Module `let`s and `var`s that the init block doesn't initialize
        // at its top level are declared first.
        for &item in &module.items {
            if let Node::ModuleLet { decl } = p.nodes.get(item) {
                if !self.erased.contains(&decl) && !state.initialized_at_top.contains(&decl) {
                    body.push(Stmt::Decl {
                        export: self.exported(decl),
                        keyword: Keyword::Let,
                        name: self.name(decl),
                        ty: Some(self.ty(p.decls.get(decl).ty)?),
                        value: None,
                    });
                }
            }
        }
        self.begin(module.init);
        match p.nodes.get(module.init) {
            Node::Block { stmts } => {
                for &s in p.nodes.list(stmts) {
                    match p.nodes.get(s) {
                        Node::InitLet { target, value } if !self.erased.contains(&target) => {
                            let var = p.decls.get(target).facts.contains(Facts::VAR);
                            body.push(Stmt::Decl {
                                export: self.exported(target),
                                keyword: if var { Keyword::Let } else { Keyword::Const },
                                name: self.name(target),
                                ty: None,
                                value: Some(self.expr(value)?),
                            });
                        }
                        _ => self.stmt(s, &mut body)?,
                    }
                }
            }
            _ => self.stmt(module.init, &mut body)?,
        }
        Ok(tree::Module {
            imports: self.imports.iter().map(|s| s.to_string()).collect(),
            body,
        })
    }

    fn function(&mut self, item: NodeId) -> R<Stmt> {
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
        let result = match p.signature(decl) {
            Some(sig) if sig.result != TypeId::VOID => Some(self.ty(sig.result)?),
            _ => None,
        };
        self.begin(item);
        let mut stmts = self.stmts(body)?;
        // A `return` without a value that ends a function says nothing.
        if result.is_none() && stmts.last() == Some(&Stmt::Return(None)) {
            stmts.pop();
        }
        Ok(Stmt::Function {
            export: self.exported(decl),
            name: self.name(decl),
            params: ps,
            result,
            body: stmts,
        })
    }

    /// Starts translating a function or the init block: plans which loops
    /// need labels, those that a jump from an inner loop leaves or
    /// continues. A label takes its source name, or for a compiler-made
    /// one, `outer`, numbered only if an enclosing label has it.
    fn begin(&mut self, scope: NodeId) {
        fn visit(p: &Program, id: NodeId, loops: &mut Vec<DeclId>, labeled: &mut Vec<DeclId>) {
            match p.nodes.get(id) {
                Node::While { label, .. } | Node::DoWhile { label, .. } => {
                    loops.push(label);
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, labeled);
                    }
                    loops.pop();
                }
                Node::Break { label } | Node::Continue { label } => {
                    if loops.last() != Some(&label) && !labeled.contains(&label) {
                        labeled.push(label);
                    }
                }
                _ => {
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, labeled);
                    }
                }
            }
        }
        let p = self.p;
        let mut labeled = Vec::new();
        visit(p, scope, &mut Vec::new(), &mut labeled);
        self.loops.clear();
        self.labels.clear();
        let mut taken: HashSet<String> = HashSet::new();
        for l in labeled {
            let d = p.decls.get(l);
            let mut base = if d.facts.contains(Facts::COMPILER_MADE) || p.name(l).is_empty() {
                "outer".to_string()
            } else {
                p.name(l).to_string()
            };
            if KEYWORDS.contains(&base.as_str()) || SPECIAL.contains(&base.as_str()) {
                base.push('_');
            }
            let name = (1..)
                .map(|n| match n {
                    1 => base.clone(),
                    n => format!("{base}{n}"),
                })
                .find(|n| !taken.contains(n))
                .unwrap_or_default();
            taken.insert(name.clone());
            self.labels.insert(l, name);
        }
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

    fn loop_body(&mut self, label: DeclId, body: NodeId) -> R<Vec<Stmt>> {
        self.loops.push(label);
        let body = self.stmts(body);
        self.loops.pop();
        body
    }

    fn stmt(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            // A block inside a block keeps its braces: its `let`s may
            // share names with another block's (names::RULES).
            Node::Block { .. } => out.push(Stmt::Block(self.stmts(id)?)),
            Node::Let { local, init } => {
                let name = self.name(local);
                out.push(match init {
                    Some(init) => Stmt::Decl {
                        export: false,
                        keyword: if self.assigned.contains(&local) {
                            Keyword::Let
                        } else {
                            Keyword::Const
                        },
                        name,
                        ty: None,
                        value: Some(self.expr(init)?),
                    },
                    None => Stmt::Decl {
                        export: false,
                        keyword: Keyword::Let,
                        name,
                        ty: Some(self.ty(p.decls.get(local).ty)?),
                        value: None,
                    },
                });
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
                out.push(Stmt::Assign {
                    target: self.name(target),
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
                let body = self.loop_body(label, body)?;
                out.push(Stmt::While {
                    label: self.labels.get(&label).cloned(),
                    cond,
                    body,
                });
            }
            Node::DoWhile { label, body, cond } => {
                let body = self.loop_body(label, body)?;
                let cond = self.expr(cond)?;
                out.push(Stmt::DoWhile {
                    label: self.labels.get(&label).cloned(),
                    body,
                    cond,
                });
            }
            Node::Break { label } | Node::Continue { label } => {
                let target = if self.loops.last() == Some(&label) {
                    None
                } else {
                    match self.labels.get(&label) {
                        Some(l) => Some(l.clone()),
                        None => return not_yet("a jump to a loop that doesn't enclose it"),
                    }
                };
                out.push(if matches!(p.nodes.get(id), Node::Break { .. }) {
                    Stmt::Break(target)
                } else {
                    Stmt::Continue(target)
                });
            }
            Node::Return { value } => {
                let value = match value {
                    Some(v) if !matches!(p.nodes.get(v), Node::VoidLit {}) => Some(self.expr(v)?),
                    _ => None,
                };
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
                Ok(Expr::name("Math").method(name, args))
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

    pub(crate) fn expr(&mut self, id: NodeId) -> R<Expr> {
        let p = self.p;
        Ok(match p.nodes.get(id) {
            Node::BoolLit { value } => Expr::Bool(value),
            Node::Int32Lit { value } => Expr::Int(value.into()),
            Node::Int64Lit { value } => Expr::BigInt(value),
            Node::Float64Lit { value } => Expr::Float(value),
            Node::StringLit { value } => Expr::Str(p.strings.get(value).to_string()),
            Node::VoidLit {} => Expr::Undefined,
            Node::Local { local: decl } => Expr::name(self.name(decl)),
            Node::GetModuleLet { target } => {
                if self.erased.contains(&target) {
                    return not_yet("a value whose type is erased");
                }
                Expr::name(self.name(target))
            }
            Node::CallFunction { callee, args, .. } => {
                let args = p.nodes.list(args);
                if is_external(p, callee) {
                    return self.connected(callee, args);
                }
                let args = self.args(args)?;
                Expr::name(self.name(callee)).call(args)
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
                let Some(spelling) = ops::spelling(op) else {
                    return not_yet("an operator without a spelling");
                };
                if spelling == Spelling::Cat {
                    return self.cat(id);
                }
                // A negated literal is a literal, wrapped as the type
                // wraps (`-(-2147483648)` is itself).
                if let [arg] = p.nodes.list(args) {
                    match (spelling, p.nodes.get(*arg)) {
                        (Spelling::Neg32, Node::Int32Lit { value }) => {
                            return Ok(Expr::Int(value.wrapping_neg().into()));
                        }
                        (Spelling::Neg64, Node::Int64Lit { value }) => {
                            return Ok(Expr::BigInt(value.wrapping_neg()));
                        }
                        _ => {}
                    }
                }
                let args: Vec<Expr> = p
                    .nodes
                    .list(args)
                    .iter()
                    .map(|&a| self.expr(a))
                    .collect::<R<_>>()?;
                if let Some(helper) = ops::helper(spelling) {
                    self.imports.insert(helper);
                }
                ops::apply(spelling, args)
            }
            Node::And { left, right } => Expr::bin(Bin::And, self.expr(left)?, self.expr(right)?),
            Node::Or { left, right } => Expr::bin(Bin::Or, self.expr(left)?, self.expr(right)?),
            Node::IfValue { cond, then, els } => Expr::Cond {
                cond: Box::new(self.expr(cond)?),
                then: Box::new(self.expr(then)?),
                els: Box::new(self.expr(els)?),
            },
            Node::Error { .. } => {
                self.imports.insert("bubble");
                Expr::name("bubble").call(Vec::new())
            }
            other => return not_yet(format!("a {} expression", other.kind().name())),
        })
    }

    /// A chain of `String.cat` as one template literal: its literal parts
    /// as they are, and an `Int32.toString()` or `Boolean.toString()`
    /// without a radix as the value itself, which a template writes the
    /// same way. All literals give a string; strings alone, `a + b`.
    fn cat(&mut self, id: NodeId) -> R<Expr> {
        let mut parts = Vec::new();
        let mut converted = false;
        self.cat_parts(id, &mut parts, &mut converted)?;
        let lits = parts.iter().filter(|p| matches!(p, TPart::Lit(_))).count();
        if lits == parts.len() {
            let text: String = parts
                .iter()
                .filter_map(|p| match p {
                    TPart::Lit(s) => Some(s.as_str()),
                    TPart::Expr(_) => None,
                })
                .collect();
            return Ok(Expr::Str(text));
        }
        if lits == 0 && !converted {
            let terms = parts.into_iter().map(|p| match p {
                TPart::Expr(e) => e,
                TPart::Lit(s) => Expr::Str(s),
            });
            return Ok(terms
                .reduce(|a, b| Expr::bin(Bin::Add, a, b))
                .unwrap_or(Expr::Str(String::new())));
        }
        Ok(Expr::Template(parts))
    }

    fn cat_parts(&mut self, id: NodeId, parts: &mut Vec<TPart>, converted: &mut bool) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Op { op, args, .. } if ops::spelling(op) == Some(Spelling::Cat) => {
                for &a in p.nodes.list(args) {
                    self.cat_parts(a, parts, converted)?;
                }
            }
            Node::StringLit { value } => {
                let text = p.strings.get(value);
                if let Some(TPart::Lit(last)) = parts.last_mut() {
                    last.push_str(text);
                } else {
                    parts.push(TPart::Lit(text.to_string()));
                }
            }
            Node::CallMethod {
                method,
                receiver,
                args,
                ..
            } if matches!(
                self.qname(method).as_str(),
                "core.type Int32.toString()" | "core.type Boolean.toString()"
            ) && p
                .nodes
                .list(args)
                .iter()
                .all(|&a| matches!(p.nodes.get(a), Node::Omitted {})) =>
            {
                let e = self.expr(receiver)?;
                parts.push(TPart::Expr(e));
                *converted = true;
            }
            _ => {
                let e = self.expr(id)?;
                parts.push(TPart::Expr(e));
            }
        }
        Ok(())
    }
}
