//! The translation from tmpl to Rust's output tree, for subset A.
//!
//! Module state, as Kotlin's Rust backend keeps it, and safe across
//! threads: a module `let` or `var` that a function reads or writes, or
//! that's exported, is a `static` with a getter function of its name, as
//! Kotlin's "toppers" are: a `let` a [`OnceLock`](std::sync::OnceLock)
//! set once by the module's init, a `var` an
//! [`RwLock`](std::sync::RwLock) of an `Option`, `None` until init sets
//! it. Every other module `let` and `var` is a local of the init, which
//! runs once (`INIT_ONCE`). One improvement on Kotlin: a `var` assigned
//! from itself (`count = count + 1`) is read and written under one write
//! lock, so concurrent updates aren't lost, where Kotlin reads and then
//! writes under separate locks. That holds only while the value reads no
//! other module state and calls nothing, which could take another lock
//! or this one again; other assignments evaluate their value first, then
//! take the lock just to store it, as Kotlin's do.

use crate::NotYet;
use crate::connected::{self, Connection, TypeConnection};
use crate::names::{RAW_KEYWORDS, static_name};
use crate::ops::{self, Spelling};
use crate::tree::{Bin, Block, Expr, Item, Stmt, Unary};
use std::collections::{BTreeSet, HashMap, HashSet};
use temper_tmpl::analysis::{module_state, reads_only_itself, walk};
use temper_tmpl::naming::{NameMap, snake_case};
use temper_tmpl::qname::is_external;
use temper_tmpl::{DeclId, Facts, Module, Node, NodeId, Program, QNames, Scalar, TypeData, TypeId};

type R<T> = Result<T, NotYet>;

fn not_yet<T>(what: impl Into<String>) -> R<T> {
    Err(NotYet { what: what.into() })
}

/// What a place wants of the value written there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    /// A value of its own: a local that isn't `Copy` is cloned.
    Owned,
    /// A borrow will do, as for a method's receiver or `==`'s operands.
    Borrowed,
    /// Anything that displays as the value: a format string's argument,
    /// where a string literal may stay `&str`.
    Display,
    /// An argument for an `impl ToArcString` parameter, which takes
    /// `&str` as well as `Arc<String>`.
    IntoArc,
}

/// An enclosing loop.
struct Frame {
    label: DeclId,
    /// A `do … while` with a `continue`, whose body is a labeled block
    /// that a `continue` leaves.
    labeled_body: bool,
}

pub struct Translator<'a> {
    p: &'a Program,
    names: &'a NameMap,
    qnames: QNames,
    /// Module `let`s kept in statics.
    statics: HashSet<DeclId>,
    /// Module `let`s of an erased type, which aren't declared.
    erased: HashSet<DeclId>,
    /// `use`d names, as paths.
    uses: BTreeSet<&'static str>,
    /// Whether the code being translated returns a `Result`.
    bubbles: bool,
    /// The enclosing loops, innermost last.
    loops: Vec<Frame>,
    /// Labels printed, by loop: each loop's, and a labeled body's.
    labels: HashMap<DeclId, String>,
    body_labels: HashMap<DeclId, String>,
    /// The `var` read and written under one lock, and its guard's name.
    guard: Option<(DeclId, String)>,
    /// The function or init block being translated, which says what's
    /// assigned where.
    scope: NodeId,
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap) -> Translator<'a> {
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            statics: HashSet::new(),
            erased: HashSet::new(),
            uses: BTreeSet::new(),
            bubbles: false,
            loops: Vec::new(),
            labels: HashMap::new(),
            body_labels: HashMap::new(),
            guard: None,
            scope: NodeId(0),
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

    fn is_copy(&self, ty: TypeId) -> bool {
        matches!(
            self.p.types.get(ty),
            TypeData::Scalar(
                Scalar::Void | Scalar::Boolean | Scalar::Int32 | Scalar::Int64 | Scalar::Float64
            )
        )
    }

    /// A type as Rust writes it.
    fn ty(&mut self, ty: TypeId) -> R<String> {
        Ok(match self.p.types.get(ty) {
            TypeData::Scalar(s) => match s {
                Scalar::Void => "()",
                Scalar::Boolean => "bool",
                Scalar::Int32 => "i32",
                Scalar::Int64 => "i64",
                Scalar::Float64 => "f64",
                Scalar::String => {
                    self.uses.insert("std::sync::Arc");
                    "Arc<String>"
                }
                Scalar::AnyValue => return not_yet("`AnyValue` as a type"),
            }
            .to_string(),
            TypeData::Nominal { decl } => {
                return not_yet(format!("a type for {}", self.qname(decl)));
            }
            TypeData::Function { .. } => return not_yet("a function type"),
        })
    }

    /// `Arc::new(e)`.
    pub(crate) fn arc(&mut self, e: Expr) -> Expr {
        self.uses.insert("std::sync::Arc");
        Expr::path("Arc::new").call(vec![e])
    }

    /// The module's items, and its `use`s first.
    pub fn module(&mut self, module: &Module) -> R<Vec<Item>> {
        let p = self.p;
        // Which module `let`s are erased, and which are statics.
        let state = module_state(p, module);
        for &item in &module.items {
            match p.nodes.get(item) {
                Node::ModuleLet { decl } => {
                    if self.type_connection(p.decls.get(decl).ty) == Some(TypeConnection::Erased) {
                        self.erased.insert(decl);
                    } else if state.used_by_functions.contains(&decl)
                        || p.decls.get(decl).facts.contains(Facts::EXPORTED)
                        || !state.initialized_at_top.contains(&decl)
                    {
                        self.statics.insert(decl);
                    }
                }
                Node::BodilessFunction { decl, .. } => {
                    return not_yet(format!("a connected function, {}", p.name(decl)));
                }
                _ => {}
            }
        }

        let init = self.init(module.init)?;
        let mut items = vec![init];
        for &item in &module.items {
            match p.nodes.get(item) {
                Node::Function { .. } => items.push(self.function(item)?),
                Node::ModuleLet { decl } if self.statics.contains(&decl) => {
                    items.extend(self.static_items(decl)?);
                }
                _ => {}
            }
        }
        let mut uses = Vec::new();
        let std_sync: Vec<&str> = self
            .uses
            .iter()
            .filter_map(|u| u.strip_prefix("std::sync::"))
            .collect();
        match std_sync[..] {
            [] => {}
            [one] => uses.push(Item::Use(format!("std::sync::{one}"))),
            _ => uses.push(Item::Use(format!("std::sync::{{{}}}", std_sync.join(", ")))),
        }
        for u in &self.uses {
            if !u.starts_with("std::sync::") {
                uses.push(Item::Use(u.to_string()));
            }
        }
        uses.extend(items);
        Ok(uses)
    }

    fn vis(&self, decl: DeclId) -> &'static str {
        if self.p.decls.get(decl).facts.contains(Facts::EXPORTED) {
            "pub"
        } else {
            "pub(crate)"
        }
    }

    /// A module `let`'s static and its getter.
    fn static_items(&mut self, decl: DeclId) -> R<Vec<Item>> {
        let d = self.p.decls.get(decl);
        let ty = self.ty(d.ty)?;
        let name = self.name(decl);
        let stat = static_name(&name);
        let copy = self.is_copy(d.ty);
        let var = d.facts.contains(Facts::VAR);
        let (static_ty, value, read) = if var {
            self.uses.insert("std::sync::RwLock");
            let read = Expr::path(&stat)
                .method("read", Vec::new())
                .method("unwrap", Vec::new());
            let read = if copy {
                read
            } else {
                read.method("clone", Vec::new())
            };
            (
                format!("RwLock<Option<{ty}>>"),
                Expr::path("RwLock::new").call(vec![Expr::path("None")]),
                read.method("unwrap", Vec::new()),
            )
        } else {
            self.uses.insert("std::sync::OnceLock");
            let get = Expr::path(&stat)
                .method("get", Vec::new())
                .method("unwrap", Vec::new());
            let read = if copy {
                Expr::unary(Unary::Deref, get)
            } else {
                get.method("clone", Vec::new())
            };
            (
                format!("OnceLock<{ty}>"),
                Expr::path("OnceLock::new").call(Vec::new()),
                read,
            )
        };
        Ok(vec![
            Item::Static {
                vis: "",
                name: stat,
                ty: static_ty,
                value,
            },
            Item::Fn {
                vis: self.vis(decl),
                name,
                params: Vec::new(),
                result: Some(ty),
                body: Block::of(read),
            },
        ])
    }

    /// `init`, which runs the init block once, as Kotlin's.
    fn init(&mut self, block: NodeId) -> R<Item> {
        self.uses.insert("std::sync::OnceLock");
        self.begin(block, true);
        let mut stmts = Vec::new();
        self.block_into(block, &mut stmts)?;
        let result = "temper_core::Result<()>".to_string();
        let run = Expr::Closure(Vec::new(), Block::new(stmts, Some(Expr::path("()").ok())));
        let call = Expr::path("INIT_ONCE")
            .method("get_or_init", vec![run])
            .method("clone", Vec::new());
        Ok(Item::Fn {
            vis: "pub(crate)",
            name: "init".into(),
            params: Vec::new(),
            result: Some(result.clone()),
            body: Block::new(
                vec![Stmt::Static {
                    name: "INIT_ONCE".into(),
                    ty: format!("OnceLock<{result}>"),
                    value: Expr::path("OnceLock::new").call(Vec::new()),
                }],
                Some(call),
            ),
        })
    }

    /// Starts translating a function or the init block.
    fn begin(&mut self, scope: NodeId, bubbles: bool) {
        self.scope = scope;
        self.bubbles = bubbles;
        self.loops.clear();
        self.labels.clear();
        self.body_labels.clear();
        self.plan_labels(scope);
    }

    fn function(&mut self, item: NodeId) -> R<Item> {
        let p = self.p;
        let Node::Function { decl, params, body } = p.nodes.get(item) else {
            return not_yet("a function that isn't one");
        };
        let Some(sig) = p.signature(decl) else {
            return not_yet("a function without a signature");
        };
        self.begin(item, sig.bubbles);
        let mut ps = Vec::new();
        let mut stmts = Vec::new();
        for &param in p.nodes.list(params) {
            let Node::Param { decl: d } = p.nodes.get(param) else {
                continue;
            };
            let name = self.name(d);
            let ty = p.decls.get(d).ty;
            let mutable = self.assigned(item, d) >= 1;
            if ty == TypeId::STRING {
                // As Kotlin's: callers may pass `&str` or `Arc<String>`.
                self.uses.insert("temper_core::ToArcString");
                ps.push(format!("{name}: impl ToArcString"));
                stmts.push(Stmt::Let {
                    name: name.clone(),
                    mutable,
                    ty: None,
                    value: Some(Expr::path(&name).method("to_arc_string", Vec::new())),
                });
            } else {
                let m = if mutable { "mut " } else { "" };
                ps.push(format!("{m}{name}: {}", self.ty(ty)?));
            }
        }
        self.block_into(body, &mut stmts)?;
        let void = sig.result == TypeId::VOID;
        let mut result = if void {
            None
        } else {
            Some(self.ty(sig.result)?)
        };
        let mut tail = tail_of(&mut stmts);
        if let Some(Stmt::Return(None)) = stmts.last() {
            stmts.pop();
        }
        // A block that ends the function needn't end before it does.
        if tail.is_none() && !self.bubbles {
            if let Some(Stmt::Block(b)) = stmts.last() {
                if b.tail.is_none() {
                    let Some(Stmt::Block(b)) = stmts.pop() else {
                        unreachable!()
                    };
                    stmts.extend(b.stmts);
                }
            }
        }
        if self.bubbles {
            result = Some(format!(
                "temper_core::Result<{}>",
                result.unwrap_or_else(|| "()".into())
            ));
            if void && tail.is_none() {
                tail = Some(Expr::path("()").ok());
            }
        }
        Ok(Item::Fn {
            vis: self.vis(decl),
            name: self.name(decl),
            params: ps,
            result,
            body: Block::new(stmts, tail),
        })
    }

    /// How many times `decl` may be assigned on one path through `id`: 0,
    /// 1, or 2 for more, or for any assignment in a loop.
    fn assigned(&self, id: NodeId, decl: DeclId) -> u32 {
        let p = self.p;
        let sum = |kids: &[NodeId]| {
            kids.iter()
                .map(|&k| self.assigned(k, decl))
                .sum::<u32>()
                .min(2)
        };
        match p.nodes.get(id) {
            Node::Assign { local: d, value } | Node::SetModuleLet { target: d, value } => {
                (u32::from(d == decl) + self.assigned(value, decl)).min(2)
            }
            Node::If { cond, then, els } => {
                let branches = self
                    .assigned(then, decl)
                    .max(els.map_or(0, |e| self.assigned(e, decl)));
                (self.assigned(cond, decl) + branches).min(2)
            }
            Node::IfValue { cond, then, els } => {
                let branches = self.assigned(then, decl).max(self.assigned(els, decl));
                (self.assigned(cond, decl) + branches).min(2)
            }
            Node::While { .. } | Node::DoWhile { .. } => {
                if sum(p.nodes.kids(id)) > 0 {
                    2
                } else {
                    0
                }
            }
            _ => sum(p.nodes.kids(id)),
        }
    }

    /// Whether a local needs `mut`: assigned at all after its
    /// initializer, or, without one, more than once on some path.
    fn is_mut(&self, decl: DeclId, initialized: bool) -> bool {
        let n = self.assigned(self.scope, decl);
        if initialized { n >= 1 } else { n >= 2 }
    }

    /// Whether a jump to `label` from here needs it: a loop's, unless
    /// it's the innermost loop's and that loop's body isn't a labeled
    /// block, inside which Rust allows only labeled jumps.
    fn jump_needs_label(loops: &[Frame], label: DeclId) -> bool {
        match loops.last() {
            Some(f) => f.label != label || f.labeled_body,
            None => true,
        }
    }

    /// Finds which loops need labels, and the labels of `do … while`
    /// bodies that a `continue` leaves.
    fn plan_labels(&mut self, scope: NodeId) {
        let p = self.p;
        let mut loops: Vec<Frame> = Vec::new();
        let mut labeled: Vec<DeclId> = Vec::new();
        let mut bodies: Vec<DeclId> = Vec::new();
        fn visit(
            p: &Program,
            id: NodeId,
            loops: &mut Vec<Frame>,
            labeled: &mut Vec<DeclId>,
            bodies: &mut Vec<DeclId>,
        ) {
            match p.nodes.get(id) {
                Node::While { label, .. } | Node::DoWhile { label, .. } => {
                    let dw = matches!(p.nodes.get(id), Node::DoWhile { .. });
                    let labeled_body = dw && continues(p, id, label);
                    if labeled_body {
                        bodies.push(label);
                    }
                    loops.push(Frame {
                        label,
                        labeled_body,
                    });
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, labeled, bodies);
                    }
                    loops.pop();
                }
                Node::Break { label } | Node::Continue { label } => {
                    let to_body = matches!(p.nodes.get(id), Node::Continue { .. })
                        && loops.iter().any(|f| f.label == label && f.labeled_body);
                    if !to_body
                        && Translator::jump_needs_label(loops, label)
                        && !labeled.contains(&label)
                    {
                        labeled.push(label);
                    }
                }
                _ => {
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, labeled, bodies);
                    }
                }
            }
        }
        visit(p, scope, &mut loops, &mut labeled, &mut bodies);
        // A label's source name, else `'outer`, or for a body `'body`;
        // numbered only if taken.
        let mut taken: HashSet<String> = HashSet::new();
        let mut name = |base: String| {
            let base = if RAW_KEYWORDS.contains(&base.as_str()) || base == "static" {
                format!("{base}_")
            } else {
                base
            };
            let name = (1..)
                .map(|n| match n {
                    1 => format!("'{base}"),
                    n => format!("'{base}{n}"),
                })
                .find(|n| !taken.contains(n))
                .unwrap_or_default();
            taken.insert(name.clone());
            name
        };
        for l in labeled {
            let d = p.decls.get(l);
            let base = if d.facts.contains(Facts::COMPILER_MADE) || p.name(l).is_empty() {
                "outer".to_string()
            } else {
                snake_case(p.name(l))
            };
            self.labels.insert(l, name(base));
        }
        for l in bodies {
            self.body_labels.insert(l, name("body".into()));
        }
    }

    fn block_into(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
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

    fn block(&mut self, id: NodeId) -> R<Block> {
        let mut stmts = Vec::new();
        self.block_into(id, &mut stmts)?;
        Ok(Block::new(stmts, None))
    }

    /// A loop's body, with its frame pushed.
    fn loop_body(&mut self, label: DeclId, labeled_body: bool, body: NodeId) -> R<Block> {
        self.loops.push(Frame {
            label,
            labeled_body,
        });
        let b = self.block(body);
        self.loops.pop();
        b
    }

    /// A `return` with its value, `Ok` if the code returns a `Result`.
    fn ret(&mut self, value: Option<Expr>) -> Stmt {
        if self.bubbles {
            Stmt::Return(Some(match value {
                // `Ok(e?)` is `e`.
                Some(Expr::Try(e)) => *e,
                Some(e) => e.ok(),
                None => Expr::path("()").ok(),
            }))
        } else {
            Stmt::Return(value)
        }
    }

    fn stmt(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Block { .. } => {
                let b = self.block(id)?;
                out.push(Stmt::Block(b));
            }
            Node::Let { local, init } => {
                let value = init.map(|v| self.expr(v, Want::Owned)).transpose()?;
                let annotate = value.as_ref().is_none_or(numeric_literal);
                let ty = p.decls.get(local).ty;
                let ty = if annotate { Some(self.ty(ty)?) } else { None };
                out.push(Stmt::Let {
                    name: self.name(local),
                    mutable: self.is_mut(local, init.is_some()),
                    ty,
                    value,
                });
            }
            Node::Assign { local, value } => {
                let value = self.expr(value, Want::Owned)?;
                out.push(Stmt::Assign(Expr::path(self.name(local)), value));
            }
            Node::SetModuleLet { target, value } => {
                if self.statics.contains(&target) {
                    out.push(self.write_var(target, value)?);
                } else {
                    let value = self.expr(value, Want::Owned)?;
                    out.push(Stmt::Assign(Expr::path(self.name(target)), value));
                }
            }
            Node::InitLet { target, value } => {
                if self.erased.contains(&target) {
                    if !self.is_erased(value) {
                        return not_yet("a console from elsewhere than `getConsole()`");
                    }
                } else if !self.statics.contains(&target) {
                    let v = self.expr(value, Want::Owned)?;
                    let ty = if numeric_literal(&v) {
                        Some(self.ty(p.decls.get(target).ty)?)
                    } else {
                        None
                    };
                    out.push(Stmt::Let {
                        name: self.name(target),
                        mutable: self.is_mut(target, true),
                        ty,
                        value: Some(v),
                    });
                } else if p.decls.get(target).facts.contains(Facts::VAR) {
                    out.push(self.write_var(target, value)?);
                } else {
                    // Set once: init runs once, unless an earlier run
                    // panicked, as in Kotlin.
                    let v = self.expr(value, Want::Owned)?;
                    let fail = Expr::Closure(
                        vec!["_".into()],
                        Block::of(Expr::Macro("panic".into(), Vec::new())),
                    );
                    let set = Expr::path(static_name(&self.name(target)))
                        .method("set", vec![v])
                        .method("unwrap_or_else", vec![fail]);
                    out.push(Stmt::Expr(set));
                }
            }
            Node::If { cond, then, els } => {
                let cond = self.expr(cond, Want::Owned)?;
                let then = self.block(then)?;
                let els = els.map(|e| self.block(e)).transpose()?;
                out.push(Stmt::If { cond, then, els });
            }
            Node::While { label, cond, body } => {
                let forever = matches!(p.nodes.get(cond), Node::BoolLit { value: true });
                let cond = self.expr(cond, Want::Owned)?;
                let body = self.loop_body(label, false, body)?;
                let label = self.labels.get(&label).cloned();
                out.push(if forever {
                    Stmt::Loop { label, body }
                } else {
                    Stmt::While { label, cond, body }
                });
            }
            Node::DoWhile { label, body, cond } => {
                let body_label = self.body_labels.get(&label).cloned();
                let body = self.loop_body(label, body_label.is_some(), body)?;
                let mut stmts = match body_label {
                    Some(l) => vec![Stmt::Labeled { label: l, body }],
                    None => body.stmts,
                };
                let cond = self.expr(cond, Want::Owned)?;
                stmts.push(Stmt::If {
                    cond: ops::apply(Spelling::Unary(Unary::Not), vec![cond])
                        .unwrap_or_else(|| Expr::path("false")),
                    then: Block::new(vec![Stmt::Break(None)], None),
                    els: None,
                });
                out.push(Stmt::Loop {
                    label: self.labels.get(&label).cloned(),
                    body: Block::new(stmts, None),
                });
            }
            Node::Break { label } => {
                let l = Translator::jump_needs_label(&self.loops, label)
                    .then(|| self.labels.get(&label).cloned())
                    .flatten();
                out.push(Stmt::Break(l));
            }
            Node::Continue { label } => {
                if let Some(body) = self.body_labels.get(&label) {
                    out.push(Stmt::Break(Some(body.clone())));
                } else {
                    let l = Translator::jump_needs_label(&self.loops, label)
                        .then(|| self.labels.get(&label).cloned())
                        .flatten();
                    out.push(Stmt::Continue(l));
                }
            }
            Node::Return { value } => {
                let value = value.map(|v| self.expr(v, Want::Owned)).transpose()?;
                let r = self.ret(value);
                out.push(r);
            }
            Node::ExprStatement { expr } => {
                let e = self.expr(expr, Want::Owned)?;
                out.push(Stmt::Expr(e));
            }
            other => return not_yet(format!("a {} statement", other.kind().name())),
        }
        Ok(())
    }

    /// Assigns a static `var`: under one write lock if the value reads
    /// only it, locals, and literals, through operators; else the value
    /// first, then the lock to store it.
    fn write_var(&mut self, target: DeclId, value: NodeId) -> R<Stmt> {
        let stat = Expr::path(static_name(&self.name(target)));
        let write = stat
            .method("write", Vec::new())
            .method("unwrap", Vec::new());
        if reads_only_itself(self.p, target, value) {
            let guard = self.name(target);
            self.guard = Some((target, guard.clone()));
            let v = self.expr(value, Want::Owned);
            self.guard = None;
            let stored = Expr::path("Some").call(vec![v?]);
            return Ok(Stmt::Block(Block::new(
                vec![
                    Stmt::Let {
                        name: guard.clone(),
                        mutable: true,
                        ty: None,
                        value: Some(write),
                    },
                    Stmt::Assign(Expr::unary(Unary::Deref, Expr::path(guard)), stored),
                ],
                None,
            )));
        }
        let v = self.expr(value, Want::Owned)?;
        Ok(Stmt::Assign(
            Expr::unary(Unary::Deref, write),
            Expr::path("Some").call(vec![v]),
        ))
    }

    /// Arguments of a call of the program's own function.
    fn args(&mut self, callee: DeclId, args: &[NodeId]) -> R<Vec<Expr>> {
        let params: Vec<TypeId> = self
            .p
            .signature(callee)
            .map(|s| s.params.to_vec())
            .unwrap_or_default();
        let mut out = Vec::new();
        for (i, &a) in args.iter().enumerate() {
            if matches!(self.p.nodes.get(a), Node::Omitted {}) {
                return not_yet("a parameter's default");
            }
            let want = if params.get(i) == Some(&TypeId::STRING) {
                Want::IntoArc
            } else {
                Want::Owned
            };
            out.push(self.expr(a, want)?);
        }
        Ok(out)
    }

    /// A method's receiver: borrowed, and for an inherent method (not
    /// `Ord`'s `min`, `max`, or `cmp`), a number literal with its type's
    /// suffix, since Rust can't call an inherent method of `{integer}`.
    pub(crate) fn receiver(&mut self, id: NodeId, inherent: bool) -> R<Expr> {
        let e = self.expr(id, Want::Borrowed)?;
        Ok(if inherent {
            self.typed(e, self.p.nodes.ty(id))
        } else {
            e
        })
    }

    /// A number literal with its type's suffix (`7i32`).
    fn typed(&self, e: Expr, ty: TypeId) -> Expr {
        let suffix = match ty {
            TypeId::INT32 => "i32",
            TypeId::INT64 => "i64",
            TypeId::FLOAT64 => "f64",
            _ => return e,
        };
        match e {
            Expr::Lit(text)
                if numeric_literal(&Expr::Lit(text.clone())) && !text.contains(['i', 'f']) =>
            {
                Expr::Lit(format!("{text}{suffix}"))
            }
            e => e,
        }
    }

    /// A call of a connected declaration, by the connection table.
    fn connected(&mut self, decl: DeclId, args: &[NodeId], want: Want) -> R<Expr> {
        let qname = self.qname(decl);
        let Some(connection) = connected::lookup(&qname) else {
            return not_yet(format!("a connection for {qname}"));
        };
        match connection {
            Connection::Method(name) => {
                let Some((&receiver, rest)) = args.split_first() else {
                    return not_yet(format!("a receiver for {qname}"));
                };
                let inherent = !matches!(name, "min" | "max");
                let receiver = self.receiver(receiver, inherent)?;
                let mut margs = Vec::new();
                for &a in rest {
                    if matches!(self.p.nodes.get(a), Node::Omitted {}) {
                        return not_yet(format!("an argument left out of {qname}"));
                    }
                    margs.push(self.expr(a, Want::Owned)?);
                }
                Ok(receiver.method(name, margs))
            }
            Connection::Translate(hook) => hook(self, args, want),
            Connection::Erased => not_yet(format!("{qname}'s result as a value")),
        }
    }

    /// A string literal as its place wants it.
    fn string(&mut self, text: &str, want: Want) -> Expr {
        match want {
            Want::Owned if text.is_empty() => self.arc(Expr::path("String::new").call(Vec::new())),
            Want::Owned => self.arc(Expr::Str(text.into()).method("to_string", Vec::new())),
            Want::Borrowed | Want::Display | Want::IntoArc => Expr::Str(text.into()),
        }
    }

    pub(crate) fn expr(&mut self, id: NodeId, want: Want) -> R<Expr> {
        let p = self.p;
        let ty = p.nodes.ty(id);
        let e = match p.nodes.get(id) {
            Node::BoolLit { value } => Expr::Lit(value.to_string()),
            Node::Int32Lit { value } => Expr::Lit(value.to_string()),
            Node::Int64Lit { value } => Expr::Lit(value.to_string()),
            Node::Float64Lit { value } => float(value),
            Node::StringLit { value } => {
                let text = p.strings.get(value).to_string();
                self.string(&text, want)
            }
            Node::VoidLit {} => Expr::path("()"),
            Node::Local { local } => self.read(local, ty, want),
            Node::GetModuleLet { target } => {
                if self.erased.contains(&target) {
                    return not_yet("a console as a value");
                }
                match &self.guard {
                    Some((g, name)) if *g == target => {
                        let read = Expr::path(name.clone());
                        let read = if self.is_copy(ty) {
                            read
                        } else {
                            read.method("clone", Vec::new())
                        };
                        read.method("unwrap", Vec::new())
                    }
                    _ if self.statics.contains(&target) => {
                        Expr::path(self.name(target)).call(Vec::new())
                    }
                    _ => self.read(target, ty, want),
                }
            }
            Node::CallFunction {
                callee,
                may_bubble,
                args,
            } => {
                let args = p.nodes.list(args);
                let call = if is_external(p, callee) {
                    self.connected(callee, args, want)?
                } else {
                    let args = self.args(callee, args)?;
                    Expr::path(self.name(callee)).call(args)
                };
                if may_bubble { call.try_() } else { call }
            }
            Node::CallMethod {
                method,
                may_bubble,
                receiver,
                args,
            } => {
                if !is_external(p, method) {
                    return not_yet("a method of the program's own");
                }
                let mut all = vec![receiver];
                all.extend_from_slice(p.nodes.list(args));
                let call = self.connected(method, &all, want)?;
                if may_bubble { call.try_() } else { call }
            }
            Node::Op {
                op,
                may_bubble,
                args,
            } => {
                let e = self.op(id, op, p.nodes.list(args), want)?;
                if may_bubble { e.try_() } else { e }
            }
            Node::And { left, right } => Expr::bin(
                Bin::And,
                self.expr(left, Want::Owned)?,
                self.expr(right, Want::Owned)?,
            ),
            Node::Or { left, right } => Expr::bin(
                Bin::Or,
                self.expr(left, Want::Owned)?,
                self.expr(right, Want::Owned)?,
            ),
            Node::IfValue { cond, then, els } => {
                // Literals may stay `&str` only if both branches are.
                let lits = matches!(p.nodes.get(then), Node::StringLit { .. })
                    && matches!(p.nodes.get(els), Node::StringLit { .. });
                let branch = match want {
                    Want::Display | Want::Borrowed if lits => want,
                    Want::IntoArc if lits => want,
                    _ => Want::Owned,
                };
                Expr::If(
                    Box::new(self.expr(cond, Want::Owned)?),
                    Block::of(self.expr(then, branch)?),
                    Block::of(self.expr(els, branch)?),
                )
            }
            Node::BlockValue { stmts, value } => {
                let mut out = Vec::new();
                for &s in p.nodes.list(stmts) {
                    self.stmt(s, &mut out)?;
                }
                Expr::Block(Block::new(out, Some(self.expr(value, want)?)))
            }
            Node::Error { .. } => {
                if self.bubbles {
                    let err = Expr::path("Err")
                        .call(vec![Expr::path("temper_core::Error::new").call(Vec::new())]);
                    Expr::Return(Some(Box::new(err)))
                } else {
                    Expr::Macro("panic".into(), Vec::new())
                }
            }
            other => return not_yet(format!("a {} expression", other.kind().name())),
        };
        Ok(e)
    }

    /// A local's value, cloned if its place wants one of its own and it
    /// isn't `Copy`.
    fn read(&self, decl: DeclId, ty: TypeId, want: Want) -> Expr {
        let name = Expr::path(self.name(decl));
        if matches!(want, Want::Owned | Want::IntoArc) && !self.is_copy(ty) {
            name.method("clone", Vec::new())
        } else {
            name
        }
    }

    fn op(&mut self, id: NodeId, op: temper_tmpl::ops::Op, args: &[NodeId], want: Want) -> R<Expr> {
        let p = self.p;
        let Some(spelling) = ops::spelling(op) else {
            return not_yet("an operator without a spelling");
        };
        let lit = |a: NodeId| matches!(p.nodes.get(a), Node::StringLit { .. });
        match (spelling, args) {
            (Spelling::Cat, _) => {
                let (format, rest) = self.format(id)?;
                let mut margs = vec![Expr::Str(format)];
                margs.extend(rest);
                let f = Expr::Macro("format".into(), margs);
                Ok(match want {
                    Want::Display | Want::Borrowed => f,
                    Want::Owned | Want::IntoArc => self.arc(f),
                })
            }
            (Spelling::StrBin(op), &[a, b]) => {
                // `==` and the rest compare `Arc<String>`s as they are;
                // with a literal, `as_str()` and `&str`.
                let either = lit(a) || lit(b);
                let side = |t: &mut Translator<'a>, n: NodeId| -> R<Expr> {
                    let e = t.expr(n, Want::Borrowed)?;
                    Ok(if either && !lit(n) {
                        e.method("as_str", Vec::new())
                    } else {
                        e
                    })
                };
                let a = side(self, a)?;
                let b = side(self, b)?;
                Ok(Expr::bin(op, a, b))
            }
            (Spelling::StrCmp, &[a, b]) => {
                let a = self.expr(a, Want::Borrowed)?;
                let b = self.expr(b, Want::Borrowed)?;
                let str = |e: Expr| match e {
                    Expr::Str(_) => e,
                    e => e.method("as_str", Vec::new()),
                };
                Ok(str(a).method("cmp", vec![str(b)]).cast("i32"))
            }
            (Spelling::Method("wrapping_neg"), &[a]) => {
                // A literal negated is a negative literal, but for the
                // least value, which has none.
                Ok(match self.expr(a, Want::Owned)? {
                    Expr::Lit(text) if text.starts_with(|c: char| c.is_ascii_digit()) => {
                        Expr::Lit(format!("-{text}"))
                    }
                    e => self
                        .typed(e, p.nodes.ty(a))
                        .method("wrapping_neg", Vec::new()),
                })
            }
            _ => {
                let mut xs = Vec::new();
                for (i, &a) in args.iter().enumerate() {
                    // The first operand of an operator spelled as a method
                    // is its receiver.
                    xs.push(match spelling {
                        Spelling::Method(_) | Spelling::Shift(_) if i == 0 => {
                            self.receiver(a, true)?
                        }
                        Spelling::Cmp if i == 0 => self.receiver(a, false)?,
                        _ => self.expr(a, Want::Owned)?,
                    });
                }
                match ops::apply(spelling, xs) {
                    Some(e) => Ok(e),
                    None => not_yet("an operator with the wrong operands"),
                }
            }
        }
    }

    /// A string as a format string and its arguments: a chain of
    /// `String.cat`'s literal parts in the format string, an
    /// `Int32.toString()` without a radix as the int itself, and a local
    /// named in the format string (`{name}`).
    pub(crate) fn format(&mut self, id: NodeId) -> R<(String, Vec<Expr>)> {
        let mut format = String::new();
        let mut args = Vec::new();
        self.format_parts(id, &mut format, &mut args)?;
        Ok((format, args))
    }

    fn format_parts(&mut self, id: NodeId, format: &mut String, args: &mut Vec<Expr>) -> R<()> {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Op { op, args: kids, .. } if ops::spelling(op) == Some(Spelling::Cat) => {
                for &a in p.nodes.list(kids) {
                    self.format_parts(a, format, args)?;
                }
            }
            Node::StringLit { value } => {
                format.push_str(&p.strings.get(value).replace('{', "{{").replace('}', "}}"));
            }
            _ => {
                let e = self.expr(id, Want::Display)?;
                match e {
                    Expr::Path(name) if is_plain_name(&name) => {
                        format.push_str(&format!("{{{name}}}"));
                    }
                    e => {
                        format.push_str("{}");
                        args.push(e);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Whether a name may go in a format string as `{name}`: a plain
/// identifier, not a raw one or a path.
fn is_plain_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric())
        && name != "_"
}

/// Whether an initializer is a number literal, which leaves a local's
/// type open in Rust (`{integer}`), so its methods can't be called; such
/// a local is annotated.
fn numeric_literal(e: &Expr) -> bool {
    matches!(e, Expr::Lit(text) if text.trim_start_matches('-').starts_with(|c: char| c.is_ascii_digit()))
}

/// A `Float64` literal.
fn float(value: f64) -> Expr {
    if value.is_nan() {
        Expr::path("f64::NAN")
    } else if value.is_infinite() {
        Expr::path(if value > 0.0 {
            "f64::INFINITY"
        } else {
            "f64::NEG_INFINITY"
        })
    } else {
        Expr::Lit(format!("{value:?}"))
    }
}

/// Whether a `return` continues in a function's last statement, so it
/// may end the function as its value instead: a `return` with a value,
/// or an `if` … `else` whose branches each end so.
fn tail_of(stmts: &mut Vec<Stmt>) -> Option<Expr> {
    match stmts.last() {
        Some(Stmt::Return(Some(_))) => match stmts.pop() {
            Some(Stmt::Return(value)) => value,
            _ => None,
        },
        Some(Stmt::If {
            els: Some(els),
            then,
            ..
        }) => {
            let (mut then, mut els) = (then.clone(), els.clone());
            let then_tail = tail_of(&mut then.stmts)?;
            let els_tail = tail_of(&mut els.stmts)?;
            let Some(Stmt::If { cond, .. }) = stmts.pop() else {
                return None;
            };
            then.tail = Some(Box::new(then_tail));
            els.tail = Some(Box::new(els_tail));
            Some(Expr::If(Box::new(cond), then, els))
        }
        _ => None,
    }
}

/// Whether a loop has a `continue` of its own.
fn continues(p: &Program, id: NodeId, label: DeclId) -> bool {
    let mut found = false;
    walk(p, id, &mut |n| {
        found |= matches!(p.nodes.get(n), Node::Continue { label: l } if l == label);
    });
    found
}
