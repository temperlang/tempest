//! The translation from tmpl to C#'s output tree, for subset A.
//!
//! A module is one static class, `NAMEGlobal`, as Kotlin's: its functions
//! are static methods, and its init block is the class's static
//! constructor, which .NET runs once, on one thread, before the class is
//! first used. Module `let`s and `var`s that functions use, or that are
//! exported, are static fields, `readonly` for a `let`, declared without
//! a value and assigned by the static constructor, as Kotlin's are; the
//! rest are locals of the static constructor. A `var` that a function
//! writes is `volatile` (where C# allows it: not a `long` or `double`),
//! so every thread reads its latest value, and a function's update of one
//! from itself alone, through operators (`ticks = ticks + 1`), takes the
//! module's lock, so updates from several threads aren't lost; Kotlin's
//! are plain statics.
//!
//! C# has no labeled loops: a `break` or `continue` of a loop that isn't
//! the innermost is a `goto` to a label after the loop or at the end of
//! its body. Arithmetic on constants alone is `unchecked`, since C#
//! rejects a constant that overflows. A self-assignment (`zero = zero`),
//! which C# warns of, is left out: it does nothing.

use crate::NotYet;
use crate::connected::{self, Connection};
use crate::ops::{self, Spelling};
use crate::tree::{self, Access, Bin, Expr, Hole, Labels, Stmt, Unary};
use std::collections::{BTreeSet, HashMap, HashSet};
use temper_tmpl::analysis::{module_state, module_writes, reads_only_itself};
use temper_tmpl::naming::NameMap;
use temper_tmpl::qname::is_external;
use temper_tmpl::{DeclId, Facts, Module, Node, NodeId, Program, QNames, Scalar, TypeData, TypeId};

type R<T> = Result<T, NotYet>;

fn not_yet<T>(what: impl Into<String>) -> R<T> {
    Err(NotYet { what: what.into() })
}

/// The classes and namespaces the translator names, by simple name, with
/// the namespace that holds each.
const NAMESPACES: &[(&str, &str)] = &[
    ("Core", "TemperLang.Core"),
    ("CultureInfo", "System.Globalization"),
    ("Float64", "TemperLang.Core"),
    ("ILoggingConsole", "TemperLang.Core"),
    ("Math", "System"),
    ("StringUtil", "TemperLang.Core"),
];

pub struct Translator<'a> {
    p: &'a Program,
    names: &'a NameMap,
    qnames: QNames,
    /// The module's namespace and class.
    namespace: String,
    class: String,
    usings: BTreeSet<String>,
    /// Every name the program's declarations take, and the root
    /// namespace, which would hide a class of the same name.
    taken: HashSet<String>,
    /// Module `let`s and `var`s kept as static fields.
    statics: HashSet<DeclId>,
    /// The module's lock, if a function takes it.
    lock: Option<String>,
    /// Whether translating a function, not the static constructor, and its
    /// result type, if not `void`.
    in_function: bool,
    result: Option<String>,
    /// The enclosing loops, innermost last.
    loops: Vec<DeclId>,
    /// Labels printed, by loop: for a `continue` and a `break` from an
    /// inner loop, only where one needs them.
    labels: HashMap<DeclId, Labels>,
}

/// A part of a string concatenation, before it's written.
enum Part {
    Lit(String),
    /// A string-typed expression.
    Str(Expr),
    /// An `Int32` or `Int64` that `toString()` converts, by the invariant
    /// culture.
    Number(Expr),
    /// A `Boolean` that `toString()` converts.
    Boolean(Expr),
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap, namespace: &str, class: &str) -> Translator<'a> {
        // The root namespace hides a class of its name too (a library
        // `math` is namespace `Math`).
        let mut taken: HashSet<String> = (p.decls.ids())
            .filter(|&d| p.decls.contains(d))
            .map(|d| names.get(d).trim_start_matches('@').to_string())
            .filter(|n| !n.is_empty())
            .collect();
        taken.insert(namespace.split('.').next().unwrap_or_default().to_string());
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            namespace: namespace.to_string(),
            class: class.to_string(),
            usings: BTreeSet::new(),
            taken,
            statics: HashSet::new(),
            lock: None,
            in_function: false,
            result: None,
            loops: Vec::new(),
            labels: HashMap::new(),
        }
    }

    pub(crate) fn node(&self, id: NodeId) -> Node {
        self.p.nodes.get(id)
    }

    /// The module's namespace, which names its console.
    pub(crate) fn namespace(&self) -> &str {
        &self.namespace
    }

    /// A class or namespace the translator names, by its simple name, with
    /// a `using` for its namespace, or qualified from `global::` where a
    /// name of the program's would hide it.
    pub(crate) fn class(&mut self, name: &str) -> Expr {
        let namespace = NAMESPACES
            .iter()
            .find(|(n, _)| *n == name)
            .map_or("System", |(_, ns)| ns);
        if self.taken.contains(name) {
            Expr::name(format!("global::{namespace}.{name}"))
        } else {
            self.usings.insert(namespace.to_string());
            Expr::name(name)
        }
    }

    /// A class of the library's `Support` namespace, from its
    /// `Logging.cs`: `Support.Logging`, relative to the root namespace,
    /// since a `using` of it would make its `ILoggingConsole` and
    /// `TemperLang.Core`'s ambiguous.
    pub(crate) fn support(&mut self, name: &str) -> Expr {
        let root = self.namespace.split('.').next().unwrap_or_default();
        if self.taken.contains("Support") {
            Expr::name(format!("global::{root}.Support.{name}"))
        } else {
            Expr::name("Support").member(name)
        }
    }

    /// A number's text in the invariant culture.
    pub(crate) fn invariant(&mut self, n: Expr) -> Expr {
        let culture = self.class("CultureInfo").member("InvariantCulture");
        n.method("ToString", vec![culture])
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

    /// A type as C# writes it, its namespace imported if it needs to be.
    fn ty(&mut self, ty: TypeId) -> R<String> {
        Ok(match self.p.types.get(ty) {
            TypeData::Scalar(s) => match s {
                Scalar::Void => "void",
                Scalar::Boolean => "bool",
                Scalar::Int32 => "int",
                Scalar::Int64 => "long",
                Scalar::Float64 => "double",
                Scalar::String => "string",
                Scalar::AnyValue => "object",
            }
            .to_string(),
            TypeData::Nominal { decl } => {
                let qname = self.qname(decl);
                match connected::lookup_type(&qname) {
                    Some(c) => match self.class(c.name) {
                        Expr::Name(n) => n,
                        _ => c.name.to_string(),
                    },
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
                    let ty = self.ty(d.ty)?;
                    // C# has no `volatile` `long` or `double`.
                    let volatile = var && written.contains(&decl) && ty != "long" && ty != "double";
                    fields.push(tree::Field {
                        access: if self.exported(decl) {
                            Access::Public
                        } else {
                            Access::Internal
                        },
                        readonly: !var && state.initialized_at_top.contains(&decl),
                        volatile,
                        ty,
                        name: self.name(decl),
                        value: None,
                    });
                } else if !state.initialized_at_top.contains(&decl) {
                    // Assigned only inside the constructor's blocks, so
                    // declared first.
                    init_locals.push(Stmt::Local {
                        ty: self.ty(d.ty)?,
                        name: self.name(decl),
                        value: None,
                    });
                }
            }
        }
        if fields.iter().any(|f| f.name == self.class) {
            return not_yet(format!("a member named for its class, {}", self.class));
        }
        let mut methods = Vec::new();
        for &item in &module.items {
            match p.nodes.get(item) {
                Node::Function { decl, .. } => {
                    if self.name(decl) == self.class {
                        return not_yet(format!("a member named for its class, {}", self.class));
                    }
                    methods.push(self.function(item)?);
                }
                Node::BodilessFunction { decl, .. } => {
                    return not_yet(format!("a connected function, {}", p.name(decl)));
                }
                _ => {}
            }
        }
        self.in_function = false;
        self.result = None;
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
        if let Some(lock) = &self.lock {
            fields.push(tree::Field {
                access: Access::Private,
                readonly: true,
                volatile: false,
                ty: "object".into(),
                name: lock.clone(),
                value: Some(Expr::name("new").call(Vec::new())),
            });
        }
        Ok(tree::Class {
            usings: self.usings.iter().cloned().collect(),
            namespace: self.namespace.clone(),
            name: self.class.clone(),
            fields,
            methods,
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
        self.result = (result != TypeId::VOID).then(|| result_ty.clone());
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
            body: stmts,
        })
    }

    /// The module's lock, named so no name of the program's hides it.
    fn lock(&mut self) -> String {
        if let Some(lock) = &self.lock {
            return lock.clone();
        }
        let lock = (1..)
            .map(|n| match n {
                1 => "moduleLock".to_string(),
                n => format!("moduleLock{n}"),
            })
            .find(|n| !self.taken.contains(n))
            .unwrap_or_default();
        self.lock = Some(lock.clone());
        lock
    }

    /// Starts translating a function or the init block: plans which loops
    /// need labels, those that a jump from an inner loop leaves or
    /// continues. A label takes its loop's source name, or for a
    /// compiler-made one, `outer`, then `End` for a `break` and `Next` for
    /// a `continue`, numbered only if another label has it.
    fn begin(&mut self, scope: NodeId) {
        fn visit(
            p: &Program,
            id: NodeId,
            loops: &mut Vec<DeclId>,
            jumps: &mut Vec<(DeclId, bool)>,
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
                    if loops.last() != Some(&label) && !jumps.contains(&(label, brk)) {
                        jumps.push((label, brk));
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
        for (l, brk) in jumps {
            let d = p.decls.get(l);
            let base = if d.facts.contains(Facts::COMPILER_MADE) || p.name(l).is_empty() {
                "outer".to_string()
            } else {
                p.name(l).to_string()
            };
            let base = format!("{base}{}", if brk { "End" } else { "Next" });
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
                let lock = Expr::name(self.lock());
                let assign = Stmt::Assign {
                    target: self.name(target),
                    value,
                };
                out.push(Stmt::Lock {
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
                let target = self.name(target);
                // `x = x` does nothing, and C# warns of it.
                if value != Expr::Name(target.clone()) {
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
                out.push(Stmt::While {
                    cond,
                    body,
                    labels: self.labels.get(&label).cloned().unwrap_or_default(),
                });
            }
            Node::DoWhile { label, body, cond } => {
                let body = self.loop_body(label, body)?;
                let cond = self.expr(cond)?;
                out.push(Stmt::DoWhile {
                    body,
                    cond,
                    labels: self.labels.get(&label).cloned().unwrap_or_default(),
                });
            }
            Node::Break { label } | Node::Continue { label } => {
                let brk = matches!(p.nodes.get(id), Node::Break { .. });
                if self.loops.last() == Some(&label) {
                    out.push(if brk { Stmt::Break } else { Stmt::Continue });
                } else {
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
            }
            Node::Return { value } => match value.map(|v| (v, p.nodes.get(v))) {
                None | Some((_, Node::VoidLit {})) => out.push(Stmt::Return(None)),
                Some((_, Node::Error { .. })) => out.push(self.bubble()),
                Some((v, _)) => {
                    let e = self.expr(v)?;
                    out.push(Stmt::Return(Some(e)));
                }
            },
            Node::ExprStatement { expr } => {
                if let Node::Error { .. } = p.nodes.get(expr) {
                    out.push(self.bubble());
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

    /// A bubble as a statement: `Core.Bubble();`, or in a function with a
    /// result, `return Core.Bubble<T>();`, so C# sees the function end.
    fn bubble(&mut self) -> Stmt {
        let core = self.class("Core");
        match self.result.clone() {
            Some(ty) => Stmt::Return(Some(core.method(format!("Bubble<{ty}>"), Vec::new()))),
            None => Stmt::Bubble(core.method("Bubble", Vec::new())),
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
            Connection::Static(class, method) => {
                let args = self.args(args)?;
                Ok(self.class(class).method(method, args))
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

    /// Arguments, those left out at the end dropped, as C#'s optional
    /// parameters and overloads leave them out.
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
                let args: Vec<Expr> = p
                    .nodes
                    .list(args)
                    .iter()
                    .map(|&a| self.expr(a))
                    .collect::<R<_>>()?;
                if let Some(class) = ops::class(spelling) {
                    // Imported, or qualified, once for the call.
                    let written = self.class(class);
                    let e = ops::apply(spelling, args, |_| written.clone());
                    return Ok(e);
                }
                let e = ops::apply(spelling, args, |c| Expr::name(c));
                match &e {
                    // C# checks constant arithmetic for overflow, where
                    // Temper's wraps.
                    Expr::Bin(op, ..) if op.arithmetic() && e.is_constant() => {
                        Expr::Unchecked(Box::new(e))
                    }
                    Expr::Unary(Unary::Neg, _) if e.is_constant() => Expr::Unchecked(Box::new(e)),
                    _ => e,
                }
            }
            Node::And { left, right } => Expr::bin(Bin::And, self.expr(left)?, self.expr(right)?),
            Node::Or { left, right } => Expr::bin(Bin::Or, self.expr(left)?, self.expr(right)?),
            Node::IfValue { cond, then, els } => Expr::Cond {
                cond: Box::new(self.expr(cond)?),
                then: Box::new(self.expr(then)?),
                els: Box::new(self.expr(els)?),
            },
            Node::Error { .. } => {
                let ty = self.ty(p.nodes.ty(id))?;
                self.class("Core")
                    .method(format!("Bubble<{ty}>"), Vec::new())
            }
            other => return not_yet(format!("a {} expression", other.kind().name())),
        })
    }

    /// A chain of `String.cat` as one string: literals merged; an
    /// `Int32.toString()` or `Boolean.toString()` without a radix
    /// converted in place, a number by the invariant culture, since C#'s
    /// `+` and `$"…"` would convert it by the current one. With two or
    /// more numbers, an interpolated string in the invariant culture,
    /// `string.Create(CultureInfo.InvariantCulture, $"…")`, reads better;
    /// otherwise `+`.
    fn cat(&mut self, id: NodeId) -> R<Expr> {
        let mut parts = Vec::new();
        self.cat_parts(id, &mut parts)?;
        parts.retain(|p| !matches!(p, Part::Lit(s) if s.is_empty()));
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
        let numbers = parts
            .iter()
            .filter(|p| matches!(p, Part::Number(_)))
            .count();
        let boolean = |e: Expr| Expr::Cond {
            cond: Box::new(e),
            then: Box::new(Expr::Str("true".into())),
            els: Box::new(Expr::Str("false".into())),
        };
        if numbers >= 2 {
            let holes = parts
                .into_iter()
                .map(|part| match part {
                    Part::Lit(s) => Hole::Text(s),
                    Part::Str(e) | Part::Number(e) => Hole::Expr(e),
                    Part::Boolean(e) => Hole::Expr(boolean(e)),
                })
                .collect();
            let culture = self.class("CultureInfo").member("InvariantCulture");
            let text = Expr::Interpolated(holes);
            return Ok(Expr::name("string").method("Create", vec![culture, text]));
        }
        let mut out = Vec::new();
        for part in parts {
            out.push(match part {
                Part::Lit(s) => Expr::Str(s),
                Part::Str(e) => e,
                Part::Number(e) => self.invariant(e),
                Part::Boolean(e) => boolean(e),
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
                let qname = self.qname(method);
                let number = match qname.as_str() {
                    "core.type Int32.toString()" | "core.type Int64.toString()" => true,
                    "core.type Boolean.toString()" => false,
                    _ => {
                        let e = self.expr(id)?;
                        parts.push(Part::Str(e));
                        return Ok(());
                    }
                };
                let e = self.expr(receiver)?;
                parts.push(if number {
                    Part::Number(e)
                } else {
                    Part::Boolean(e)
                });
            }
            _ => {
                let e = self.expr(id)?;
                parts.push(Part::Str(e));
            }
        }
        Ok(())
    }
}

/// Statements up to the first that can't complete normally, by C#'s rules
/// (ECMA-334 13.2), since C# warns of a statement after one, and the
/// statements after a bubble, which never return.
fn reachable(mut stmts: Vec<Stmt>) -> Vec<Stmt> {
    if let Some(i) = stmts.iter().position(|s| !completes(s)) {
        stmts.truncate(i + 1);
    }
    stmts
}

/// Whether a statement can complete normally, by C#'s rules.
fn completes(s: &Stmt) -> bool {
    match s {
        Stmt::Break | Stmt::Continue | Stmt::Goto(_) | Stmt::Return(_) | Stmt::Bubble(_) => false,
        Stmt::Block(body) | Stmt::Lock { body, .. } => body.iter().all(completes),
        Stmt::If { then, els, .. } => {
            els.is_empty() || then.iter().all(completes) || els.iter().all(completes)
        }
        // A label after a loop is reached by a `goto` from inside it.
        Stmt::While { cond, body, labels } => {
            *cond != Expr::Bool(true) || breaks(body) || labels.end.is_some()
        }
        Stmt::DoWhile { body, cond, labels } => {
            breaks(body)
                || labels.end.is_some()
                || (*cond != Expr::Bool(true)
                    && (body.iter().all(completes) || continues(body) || labels.next.is_some()))
        }
        Stmt::Expr(_) | Stmt::Local { .. } | Stmt::Assign { .. } => true,
    }
}

/// Whether statements hold a `break` of the innermost loop.
fn breaks(body: &[Stmt]) -> bool {
    jumps(body, true)
}

fn continues(body: &[Stmt]) -> bool {
    jumps(body, false)
}

fn jumps(body: &[Stmt], brk: bool) -> bool {
    body.iter().any(|s| match s {
        Stmt::Break => brk,
        Stmt::Continue => !brk,
        Stmt::Block(b) | Stmt::Lock { body: b, .. } => jumps(b, brk),
        Stmt::If { then, els, .. } => jumps(then, brk) || jumps(els, brk),
        _ => false,
    })
}
