//! The translation from tmpl to Java's output tree, for subset A.
//!
//! A module is one final class of statics, `NAMEGlobal`, as Kotlin's:
//! its functions are static methods, and its init block is the class's
//! static initializer, which the JVM runs once, on one thread, when the
//! class is first used. Module `let`s and `var`s that functions use, or
//! that are exported, are static fields, `final` for a `let`, declared
//! without a value and assigned by the static initializer, as Kotlin's
//! are; the rest are locals of the static initializer. A `var` that a
//! function writes is `volatile`, so every thread reads its latest
//! value, and a function's update of one from itself alone, through
//! operators (`ticks = ticks + 1`), takes the class's lock, so updates
//! from several threads aren't lost; Kotlin's are plain statics.

use crate::NotYet;
use crate::connected::{self, Connection};
use crate::names::KEYWORDS;
use crate::ops::{self, Spelling};
use crate::tree::{self, Bin, Expr, Stmt, Unary};
use std::collections::{BTreeSet, HashMap, HashSet};
use temper_tmpl::analysis::{module_state, module_writes, reads_only_itself};
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
    /// The module's package and class.
    package: String,
    class: String,
    imports: BTreeSet<String>,
    /// Module `let`s and `var`s kept as static fields.
    statics: HashSet<DeclId>,
    /// Whether translating a function, not the static initializer.
    in_function: bool,
    /// The enclosing loops, innermost last.
    loops: Vec<DeclId>,
    /// Labels printed, by loop: only those a jump from an inner loop
    /// needs.
    labels: HashMap<DeclId, String>,
    /// The name of the string comparison helper
    /// ([`ops::COMPARE_CODE_POINTS`]), and whether a call needs it.
    compare_helper: String,
    compares_strings: bool,
}

/// A part of a string concatenation, before it's written.
enum Part {
    Lit(String),
    /// A string-typed expression.
    Str(Expr),
    /// An `Int32` or `Boolean` that `Integer.toString` or
    /// `Boolean.toString` converts, by its class, which a concatenation
    /// converts the same way once it's concatenating.
    Converted(&'static str, Expr),
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap, package: &str, class: &str) -> Translator<'a> {
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            package: package.to_string(),
            class: class.to_string(),
            imports: BTreeSet::new(),
            statics: HashSet::new(),
            in_function: false,
            loops: Vec::new(),
            labels: HashMap::new(),
            compare_helper: ops::COMPARE_CODE_POINTS.to_string(),
            compares_strings: false,
        }
    }

    pub(crate) fn node(&self, id: NodeId) -> Node {
        self.p.nodes.get(id)
    }

    /// The module's package, which names its logger.
    pub(crate) fn package(&self) -> &str {
        &self.package
    }

    /// Imports a class, by its qualified name.
    pub(crate) fn import(&mut self, name: &str) {
        self.imports.insert(name.to_string());
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

    /// A type as Java writes it, imported if it needs to be.
    fn ty(&mut self, ty: TypeId) -> R<String> {
        Ok(match self.p.types.get(ty) {
            TypeData::Scalar(s) => match s {
                Scalar::Void => "void",
                Scalar::Boolean => "boolean",
                Scalar::Int32 => "int",
                Scalar::Int64 => "long",
                Scalar::Float64 => "double",
                Scalar::String => "String",
                Scalar::AnyValue => "Object",
            }
            .to_string(),
            TypeData::Nominal { decl } => {
                let qname = self.qname(decl);
                match connected::lookup_type(&qname) {
                    Some(c) => {
                        self.import(c.import);
                        c.name.to_string()
                    }
                    None => return not_yet(format!("a type for {qname}")),
                }
            }
            TypeData::Function { .. } => return not_yet("a function type"),
        })
    }

    fn exported(&self, decl: DeclId) -> bool {
        self.p.decls.get(decl).facts.contains(Facts::EXPORTED)
    }

    /// Translates a module into its class.
    pub fn module(&mut self, module: &Module) -> R<tree::Class> {
        let p = self.p;
        let state = module_state(p, module);
        let mut written = HashSet::new();
        for &item in &module.items {
            if let Node::Function { .. } = p.nodes.get(item) {
                written.extend(module_writes(p, item));
            }
        }
        let mut fields = Vec::new();
        let mut init_locals = Vec::new();
        for &item in &module.items {
            if let Node::ModuleLet { decl } = p.nodes.get(item) {
                let d = p.decls.get(decl);
                let var = d.facts.contains(Facts::VAR);
                if self.exported(decl) || state.used_by_functions.contains(&decl) {
                    self.statics.insert(decl);
                    fields.push(tree::Field {
                        public: self.exported(decl),
                        is_final: !var && state.initialized_at_top.contains(&decl),
                        volatile: var && written.contains(&decl),
                        ty: self.ty(d.ty)?,
                        name: self.name(decl),
                    });
                } else if !state.initialized_at_top.contains(&decl) {
                    // Assigned only inside the initializer's blocks, so
                    // declared first.
                    init_locals.push(Stmt::Local {
                        ty: self.ty(d.ty)?,
                        name: self.name(decl),
                        value: None,
                    });
                }
            }
        }
        // The helper takes a name none of the module's functions has.
        let functions: HashSet<String> = (module.items.iter())
            .filter_map(|&item| match p.nodes.get(item) {
                Node::Function { decl, .. } | Node::BodilessFunction { decl, .. } => {
                    Some(self.name(decl))
                }
                _ => None,
            })
            .collect();
        self.compare_helper = (1..)
            .map(|n| match n {
                1 => ops::COMPARE_CODE_POINTS.to_string(),
                n => format!("{}{n}", ops::COMPARE_CODE_POINTS),
            })
            .find(|n| !functions.contains(n))
            .unwrap_or_default();
        let mut methods = Vec::new();
        for &item in &module.items {
            match p.nodes.get(item) {
                Node::Function { .. } => methods.push(self.function(item)?),
                Node::BodilessFunction { decl, .. } => {
                    return not_yet(format!("a connected function, {}", p.name(decl)));
                }
                _ => {}
            }
        }
        self.in_function = false;
        self.begin(module.init);
        let mut init = init_locals;
        match p.nodes.get(module.init) {
            Node::Block { stmts } => {
                for &s in p.nodes.list(stmts) {
                    match p.nodes.get(s) {
                        Node::InitLet { target, value } if !self.statics.contains(&target) => {
                            let ty = self.ty(p.decls.get(target).ty)?;
                            init.push(Stmt::Local {
                                ty,
                                name: self.name(target),
                                value: Some(self.expr(value)?),
                            });
                        }
                        _ => self.stmt(s, &mut init)?,
                    }
                }
            }
            _ => self.stmt(module.init, &mut init)?,
        }
        Ok(tree::Class {
            package: self.package.clone(),
            imports: self.imports.iter().cloned().collect(),
            name: self.class.clone(),
            fields,
            methods,
            helpers: if self.compares_strings {
                vec![ops::compare_code_points(&self.compare_helper)]
            } else {
                Vec::new()
            },
            init: reachable(init),
        })
    }

    fn function(&mut self, item: NodeId) -> R<tree::Method> {
        let p = self.p;
        let Node::Function { decl, params, body } = p.nodes.get(item) else {
            return not_yet("a function that isn't one");
        };
        let mut ps = Vec::new();
        for &param in p.nodes.list(params) {
            if let Node::Param { decl } = p.nodes.get(param) {
                ps.push((self.ty(p.decls.get(decl).ty)?, self.name(decl)));
            }
        }
        let result = match p.signature(decl) {
            Some(sig) => sig.result,
            None => TypeId::VOID,
        };
        let result_ty = self.ty(result)?;
        self.in_function = true;
        self.begin(item);
        let mut stmts = reachable(self.stmts(body)?);
        // A `return` without a value that ends a function says nothing.
        if result == TypeId::VOID && stmts.last() == Some(&Stmt::Return(None)) {
            stmts.pop();
        }
        Ok(tree::Method {
            public: self.exported(decl),
            result: result_ty,
            name: self.name(decl),
            params: ps,
            throws: Vec::new(),
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
            if KEYWORDS.contains(&base.as_str()) {
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
            // A block inside a block keeps its braces: its locals may
            // share names with another block's (names::RULES).
            Node::Block { .. } => out.push(Stmt::Block(self.stmts(id)?)),
            Node::Let { local, init } => {
                let ty = self.ty(p.decls.get(local).ty)?;
                let value = init.map(|v| self.expr(v)).transpose()?;
                out.push(Stmt::Local {
                    ty,
                    name: self.name(local),
                    value,
                });
            }
            Node::SetModuleLet { target, value }
                if self.in_function && reads_only_itself(p, target, value) =>
            {
                let value = self.expr(value)?;
                let lock = Expr::name(self.class.clone()).member("class");
                let assign = Stmt::Assign {
                    target: self.name(target),
                    value,
                };
                out.push(Stmt::Synchronized {
                    lock,
                    body: vec![assign],
                });
            }
            Node::Assign {
                local: target,
                value,
            }
            | Node::SetModuleLet { target, value }
            | Node::InitLet { target, value } => {
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
            Node::Return { value } => match value.map(|v| (v, p.nodes.get(v))) {
                None | Some((_, Node::VoidLit {})) => out.push(Stmt::Return(None)),
                Some((_, Node::Error { .. })) => out.push(self.throw()),
                Some((v, _)) => {
                    let e = self.expr(v)?;
                    out.push(Stmt::Return(Some(e)));
                }
            },
            Node::ExprStatement { expr } => {
                if let Node::Error { .. } = p.nodes.get(expr) {
                    out.push(self.throw());
                    return Ok(());
                }
                let e = self.expr(expr)?;
                if !e.is_statement() {
                    return not_yet("an expression statement that isn't a call");
                }
                out.push(Stmt::Expr(e));
            }
            other => return not_yet(format!("a {} statement", other.kind().name())),
        }
        Ok(())
    }

    /// `throw Core.bubble();`.
    fn throw(&mut self) -> Stmt {
        self.import("temper.core.Core");
        Stmt::Throw(Expr::name("Core").method("bubble", Vec::new()))
    }

    /// A call of a connected declaration, by the connection table; `args`
    /// has a method's receiver first.
    fn connected(&mut self, decl: DeclId, args: &[NodeId]) -> R<Expr> {
        let qname = self.qname(decl);
        let Some(connection) = connected::lookup(&qname) else {
            return not_yet(format!("a connection for {qname}"));
        };
        match connection {
            Connection::Static(class, method) => {
                let args = self.args(args)?;
                Ok(Expr::name(class).method(method, args))
            }
            Connection::Method(method) => {
                let Some((&receiver, rest)) = args.split_first() else {
                    return not_yet(format!("{qname} without its receiver"));
                };
                let receiver = self.expr(receiver)?;
                let rest = self.args(rest)?;
                Ok(receiver.method(method, rest))
            }
            Connection::Translate(f) => f(self, args),
        }
    }

    /// Arguments, those left out at the end dropped, as Java's overloads
    /// leave them out.
    fn args(&mut self, args: &[NodeId]) -> R<Vec<Expr>> {
        let p = self.p;
        let given = args
            .iter()
            .rposition(|&a| !matches!(p.nodes.get(a), Node::Omitted {}))
            .map_or(0, |i| i + 1);
        args[..given]
            .iter()
            .map(|&a| match p.nodes.get(a) {
                Node::Omitted {} => not_yet("a default parameter before a given one"),
                _ => self.expr(a),
            })
            .collect()
    }

    pub(crate) fn expr(&mut self, id: NodeId) -> R<Expr> {
        let p = self.p;
        Ok(match p.nodes.get(id) {
            Node::BoolLit { value } => Expr::Bool(value),
            Node::Int32Lit { value } => Expr::Int(value.into()),
            Node::Int64Lit { value } => Expr::Long(value),
            Node::Float64Lit { value } => Expr::Float(value),
            Node::StringLit { value } => Expr::Str(p.strings.get(value).to_string()),
            Node::Local { local: decl } => Expr::name(self.name(decl)),
            Node::GetModuleLet { target } => Expr::name(self.name(target)),
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
                        (Spelling::Unary(Unary::Neg), Node::Int32Lit { value }) => {
                            return Ok(Expr::Int(value.wrapping_neg().into()));
                        }
                        (Spelling::Unary(Unary::Neg), Node::Int64Lit { value }) => {
                            return Ok(Expr::Long(value.wrapping_neg()));
                        }
                        (Spelling::Unary(Unary::Neg), Node::Float64Lit { value }) => {
                            return Ok(Expr::Float(-value));
                        }
                        _ => {}
                    }
                }
                if let Spelling::Static("Core", _) = spelling {
                    self.import("temper.core.Core");
                }
                let args: Vec<Expr> = p
                    .nodes
                    .list(args)
                    .iter()
                    .map(|&a| self.expr(a))
                    .collect::<R<_>>()?;
                let mut e = ops::apply(spelling, args);
                if let Spelling::CodePoints | Spelling::CodePointsWith(_) = spelling {
                    self.compares_strings = true;
                    rename_helper(&mut e, &self.compare_helper);
                }
                e
            }
            Node::And { left, right } => Expr::bin(Bin::And, self.expr(left)?, self.expr(right)?),
            Node::Or { left, right } => Expr::bin(Bin::Or, self.expr(left)?, self.expr(right)?),
            Node::IfValue { cond, then, els } => Expr::Cond {
                cond: Box::new(self.expr(cond)?),
                then: Box::new(self.expr(then)?),
                els: Box::new(self.expr(els)?),
            },
            Node::Error { .. } => {
                self.import("temper.core.Core");
                Expr::name("Core").method("throwBubble", Vec::new())
            }
            other => return not_yet(format!("a {} expression", other.kind().name())),
        })
    }

    /// A chain of `String.cat` as one concatenation: literals merged, and
    /// an `Int32.toString()` or `Boolean.toString()` without a radix as
    /// the value itself wherever a string comes before it, or right after
    /// it at the start, so that Java's `+` concatenates there and converts
    /// it the same way.
    fn cat(&mut self, id: NodeId) -> R<Expr> {
        let mut parts = Vec::new();
        self.cat_parts(id, &mut parts)?;
        parts.retain(|p| !matches!(p, Part::Lit(s) if s.is_empty()));
        let is_string = |p: &Part| matches!(p, Part::Lit(_) | Part::Str(_));
        if parts.iter().all(|p| matches!(p, Part::Lit(_))) {
            let text: String = parts
                .iter()
                .filter_map(|p| match p {
                    Part::Lit(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect();
            return Ok(Expr::Str(text));
        }
        // Java's `+` concatenates once either operand is a string: from
        // the second part on, since the first is a string, or converted,
        // or (as the value itself) followed by a string.
        let second_is_string = parts.get(1).is_some_and(is_string);
        let mut out = Vec::new();
        for (i, part) in parts.into_iter().enumerate() {
            out.push(match part {
                Part::Lit(s) => Expr::Str(s),
                Part::Str(e) => e,
                Part::Converted(_, e) if i > 0 || second_is_string => e,
                Part::Converted(class, e) => Expr::name(class).method("toString", vec![e]),
            });
        }
        Ok(match <[Expr; 1]>::try_from(out) {
            Ok([one]) => one,
            Err(out) => Expr::Concat(out),
        })
    }

    fn cat_parts(&mut self, id: NodeId, parts: &mut Vec<Part>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Op { op, args, .. } if ops::spelling(op) == Some(Spelling::Cat) => {
                for &a in p.nodes.list(args) {
                    self.cat_parts(a, parts)?;
                }
            }
            Node::StringLit { value } => {
                let text = p.strings.get(value);
                if let Some(Part::Lit(last)) = parts.last_mut() {
                    last.push_str(text);
                } else {
                    parts.push(Part::Lit(text.to_string()));
                }
            }
            Node::CallMethod {
                method,
                receiver,
                args,
                ..
            } if p
                .nodes
                .list(args)
                .iter()
                .all(|&a| matches!(p.nodes.get(a), Node::Omitted {})) =>
            {
                let class = match self.qname(method).as_str() {
                    "core.type Int32.toString()" => "Integer",
                    "core.type Boolean.toString()" => "Boolean",
                    _ => {
                        let e = self.expr(id)?;
                        parts.push(Part::Str(e));
                        return Ok(());
                    }
                };
                let e = self.expr(receiver)?;
                parts.push(Part::Converted(class, e));
            }
            _ => {
                let e = self.expr(id)?;
                parts.push(Part::Str(e));
            }
        }
        Ok(())
    }
}

/// Names the helper call in a comparison by the name the module gives it.
fn rename_helper(e: &mut Expr, name: &str) {
    match e {
        Expr::Call(f, _) => {
            if let Expr::Name(n) = f.as_mut() {
                if n == ops::COMPARE_CODE_POINTS {
                    *n = name.to_string();
                }
            }
        }
        Expr::Bin(_, a, _) => rename_helper(a, name),
        _ => {}
    }
}

/// Statements up to the first that can't complete normally, by Java's
/// rules (JLS 14.22), since Java rejects a statement after one.
fn reachable(mut stmts: Vec<Stmt>) -> Vec<Stmt> {
    if let Some(i) = stmts.iter().position(|s| !completes(s)) {
        stmts.truncate(i + 1);
    }
    stmts
}

/// Whether a statement can complete normally, by Java's rules.
fn completes(s: &Stmt) -> bool {
    match s {
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Return(_) | Stmt::Throw(_) => false,
        Stmt::Block(body) | Stmt::Synchronized { body, .. } => body.iter().all(completes),
        Stmt::If { then, els, .. } => {
            els.is_empty() || then.iter().all(completes) || els.iter().all(completes)
        }
        Stmt::While { label, cond, body } => {
            *cond != Expr::Bool(true) || breaks(body, label.as_deref(), true)
        }
        Stmt::DoWhile { label, body, cond } => {
            breaks(body, label.as_deref(), true)
                || (*cond != Expr::Bool(true)
                    && (body.iter().all(completes) || continues(body, label.as_deref(), true)))
        }
        Stmt::Expr(_) | Stmt::Local { .. } | Stmt::Assign { .. } => true,
    }
}

/// Whether statements hold a `break` of the loop labeled `label`, or of
/// the innermost loop if `innermost`.
fn breaks(body: &[Stmt], label: Option<&str>, innermost: bool) -> bool {
    jumps(body, label, innermost, true)
}

fn continues(body: &[Stmt], label: Option<&str>, innermost: bool) -> bool {
    jumps(body, label, innermost, false)
}

fn jumps(body: &[Stmt], label: Option<&str>, innermost: bool, brk: bool) -> bool {
    body.iter().any(|s| match s {
        Stmt::Break(l) if brk => l.as_deref().map_or(innermost, |l| Some(l) == label),
        Stmt::Continue(l) if !brk => l.as_deref().map_or(innermost, |l| Some(l) == label),
        Stmt::Block(b) | Stmt::Synchronized { body: b, .. } => jumps(b, label, innermost, brk),
        Stmt::If { then, els, .. } => {
            jumps(then, label, innermost, brk) || jumps(els, label, innermost, brk)
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => jumps(body, label, false, brk),
        _ => false,
    })
}
