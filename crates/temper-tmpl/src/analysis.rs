//! Analyses that passes and backends share (notes/tmpl/analyses.md).
//!
//! So far: a generic walk, and module state: what each function writes
//! of its module's `var`s (Python's `global`), what it refers to, and per
//! module, which `let`s and `var`s functions use and which the init block
//! initializes at its top level ([`module_state`]); and whether a `var`'s
//! new value reads only itself, so the update may run under one lock
//! ([`reads_only_itself`]); whether an expression calls nothing, so it may
//! run under a lock ([`calls_nothing`]); and what evaluating an
//! expression may do that the order of evaluation would show, for
//! targets that don't evaluate operands left to right ([`Effects`]).
//! And what a module imports: the declarations of other modules, the
//! program's own or other libraries', that its nodes and declared types
//! name ([`imports`]).

use crate::decls::{DeclKind, Facts};
use crate::ids::TypeId;
use crate::ids::{DeclId, NodeId};
use crate::qname::{QNames, is_external};
use crate::schema::Node;
use crate::types::TypeData;
use crate::{Module, Program};
use std::collections::{HashMap, HashSet};

/// Calls `visit` on `id` and everything under it, parents before
/// children, children in evaluation order.
pub fn walk(p: &Program, id: NodeId, visit: &mut impl FnMut(NodeId)) {
    visit(id);
    for &kid in p.nodes.kids(id) {
        walk(p, kid, visit);
    }
}

/// The module `var`s a function (or any subtree) assigns, in the order
/// first assigned: what Python declares `global` in it.
pub fn module_writes(p: &Program, function: NodeId) -> Vec<DeclId> {
    let mut found = Vec::new();
    walk(p, function, &mut |id| {
        if let Node::SetModuleLet { target, .. } = p.nodes.get(id) {
            if !found.contains(&target) {
                found.push(target);
            }
        }
    });
    found
}

/// The module-level declarations a function (or any subtree) refers to:
/// functions it calls and module `let`s it reads or writes, in the order
/// first referred to. External declarations aren't included.
pub fn module_refs(p: &Program, function: NodeId) -> Vec<DeclId> {
    let mut found = Vec::new();
    walk(p, function, &mut |id| {
        let decl = match p.nodes.get(id) {
            Node::CallFunction { callee, .. } => callee,
            Node::GetModuleLet { target } | Node::SetModuleLet { target, .. } => target,
            _ => return,
        };
        if !crate::qname::is_external(p, decl) && !found.contains(&decl) {
            found.push(decl);
        }
    });
    found
}

/// The module a declaration belongs to: the program's own module, or
/// another library's module (a [`DeclKind::Module`] owned by a `uses`
/// library), or for a declaration owned directly by a `uses` library, as
/// core's are, that library. `None` for one that isn't owned by either.
pub fn home(p: &Program, decl: DeclId) -> Option<DeclId> {
    let mut d = decl;
    for _ in 0..p.decls.len() {
        if !p.decls.contains(d) {
            return None;
        }
        match p.decls.get(d).kind {
            DeclKind::Module | DeclKind::Library => return Some(d),
            _ => d = p.decls.get(d).owner,
        }
    }
    None
}

/// What one module imports from another, the program's own or another
/// library's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    /// The module: the program's own, another library's (owned by its
    /// `uses` library), or a `uses` library itself for what it owns
    /// directly.
    pub from: DeclId,
    /// The top-level declarations used from it (functions, module `let`s
    /// and `var`s, classes, and interfaces), in `DeclId` order.
    pub decls: Vec<DeclId>,
}

/// What `roots` (a module's items and init block, or its tests) import:
/// the top-level declarations of modules other than `module` that they
/// call, read, or name in a declared type (of a function, a parameter, a
/// local, or a module `let`), grouped by module, the program's own
/// modules first in their order, then other libraries' in `DeclId` order.
/// Core's declarations (`temper-core`'s), which every backend reaches
/// through its runtime, aren't included.
pub fn imports(p: &Program, module: DeclId, roots: &[NodeId]) -> Vec<Import> {
    let mut found: Vec<DeclId> = Vec::new();
    let add = |d: DeclId, found: &mut Vec<DeclId>| {
        let top = top_level(p, d);
        if let Some(top) = top {
            if !found.contains(&top) {
                found.push(top);
            }
        }
    };
    for &root in roots {
        walk(p, root, &mut |id| {
            let node = p.nodes.get(id);
            match node {
                Node::CallFunction { callee, .. } => add(callee, &mut found),
                Node::GetModuleLet { target } | Node::SetModuleLet { target, .. } => {
                    add(target, &mut found)
                }
                // Classes, through what's used of them.
                Node::New { ctor: d, .. }
                | Node::GetStatic { target: d }
                | Node::InitStatic { target: d, .. } => add(d, &mut found),
                Node::Is { ty, .. } => {
                    let mut types = Vec::new();
                    nominals(p, ty, &mut types);
                    for t in types {
                        add(t, &mut found);
                    }
                }
                _ => {}
            }
            let ty = p.nodes.ty(id);
            let written = matches!(node, Node::Upcast { .. } | Node::Downcast { .. });
            if written && !ty.is_none() {
                let mut types = Vec::new();
                nominals(p, ty, &mut types);
                for t in types {
                    add(t, &mut found);
                }
            }
            let defined = match node {
                Node::Function { decl, .. }
                | Node::BodilessFunction { decl, .. }
                | Node::Test { decl, .. }
                | Node::Param { decl }
                | Node::ModuleLet { decl }
                | Node::Field { decl }
                | Node::StaticField { decl }
                | Node::Method { decl, .. }
                | Node::AbstractMethod { decl, .. } => decl,
                Node::Class { decl, .. } | Node::Interface { decl, .. } => {
                    for &s in p.decls.supers(decl) {
                        let mut types = Vec::new();
                        nominals(p, s, &mut types);
                        for t in types {
                            add(t, &mut found);
                        }
                    }
                    return;
                }
                Node::Let { local, .. } => local,
                _ => return,
            };
            let mut types = Vec::new();
            nominals(p, p.decls.get(defined).ty, &mut types);
            for t in types {
                add(t, &mut found);
            }
        });
    }
    let mut groups: Vec<Import> = Vec::new();
    for d in found {
        let Some(from) = home(p, d) else { continue };
        if from == module || crate::qname::library_of(p, d) == "core" {
            continue;
        }
        match groups.iter_mut().find(|g| g.from == from) {
            Some(g) => g.decls.push(d),
            None => groups.push(Import {
                from,
                decls: vec![d],
            }),
        }
    }
    let order = |from: DeclId| match p.modules.iter().position(|m| m.decl == from) {
        Some(i) => (0, i as u32),
        None => (1, from.0),
    };
    groups.sort_by_key(|g| order(g.from));
    for g in &mut groups {
        g.decls.sort();
    }
    groups
}

/// What a module's items and init block import ([`imports`]).
pub fn module_imports(p: &Program, module: &Module) -> Vec<Import> {
    let roots: Vec<NodeId> = (module.items.iter().copied())
        .chain([module.init])
        .collect();
    imports(p, module.decl, &roots)
}

/// What a module's tests import ([`imports`]), std's `Test` among them.
pub fn test_imports(p: &Program, module: &Module) -> Vec<Import> {
    imports(p, module.decl, &module.tests)
}

/// The top-level declaration a declaration is or is a member of: itself
/// for a function, module `let`, class, or interface owned by a module or
/// a `uses` library, else its owner's.
fn top_level(p: &Program, decl: DeclId) -> Option<DeclId> {
    let mut d = decl;
    for _ in 0..p.decls.len() {
        if !p.decls.contains(d) {
            return None;
        }
        let owner = p.decls.get(d).owner;
        if !p.decls.contains(owner) {
            return None;
        }
        if matches!(
            p.decls.get(owner).kind,
            DeclKind::Module | DeclKind::Library
        ) {
            return Some(d);
        }
        d = owner;
    }
    None
}

/// The classes and interfaces a type names, through function types.
fn nominals(p: &Program, ty: crate::ids::TypeId, out: &mut Vec<DeclId>) {
    if !p.types.contains(ty) {
        return;
    }
    match p.types.get(ty) {
        crate::types::TypeData::Nominal { decl } => out.push(decl),
        crate::types::TypeData::Function { .. } => {
            if let Some(sig) = p.types.signature(ty) {
                for &t in sig.params.iter().chain([&sig.result]) {
                    nominals(p, t, out);
                }
            }
        }
        crate::types::TypeData::Scalar(_) => {}
    }
}

/// What a module's functions and init block do with its module `let`s
/// and `var`s.
#[derive(Clone, Debug, Default)]
pub struct ModuleState {
    /// Those a function reads or writes, so they outlive the init block
    /// (Rust's statics).
    pub used_by_functions: HashSet<DeclId>,
    /// Those initialized by an `init` statement directly in the init
    /// block, not inside an `if` or a loop, so a declaration there can
    /// give the value (JS's `const x = …`).
    pub initialized_at_top: HashSet<DeclId>,
}

/// The module state of `module`.
pub fn module_state(p: &Program, module: &Module) -> ModuleState {
    let mut state = ModuleState::default();
    for &item in &module.items {
        if let Node::Function { .. } = p.nodes.get(item) {
            for d in module_refs(p, item) {
                if p.decls.get(d).kind == DeclKind::ModuleLet {
                    state.used_by_functions.insert(d);
                }
            }
        }
    }
    if let Node::Block { stmts } = p.nodes.get(module.init) {
        for &s in p.nodes.list(stmts) {
            if let Node::InitLet { target, .. } = p.nodes.get(s) {
                state.initialized_at_top.insert(target);
            }
        }
    }
    state
}

/// Whether a module `var`'s new value reads it, and else only locals and
/// literals, through operators: an update such as `ticks = ticks + 1`,
/// which may be computed and stored under one lock, since it calls
/// nothing that could take another (Rust's and Java's synchronized
/// module `var`s).
pub fn reads_only_itself(p: &Program, target: DeclId, value: NodeId) -> bool {
    let mut reads = false;
    let mut other = false;
    walk(p, value, &mut |id| match p.nodes.get(id) {
        Node::GetModuleLet { target: t } if t == target => reads = true,
        Node::BoolLit { .. }
        | Node::Int32Lit { .. }
        | Node::Int64Lit { .. }
        | Node::Float64Lit { .. }
        | Node::StringLit { .. }
        | Node::VoidLit {}
        | Node::Local { .. }
        | Node::Op { .. }
        | Node::And { .. }
        | Node::Or { .. }
        | Node::IfValue { .. } => {}
        _ => other = true,
    });
    reads && !other
}

/// Whether an expression calls nothing: only literals, locals, module
/// `let`s, and operators, so it may run while a lock is held, as no call
/// in it could take the lock again (C++'s module `var`s).
pub fn calls_nothing(p: &Program, value: NodeId) -> bool {
    let mut other = false;
    walk(p, value, &mut |id| match p.nodes.get(id) {
        Node::BoolLit { .. }
        | Node::Int32Lit { .. }
        | Node::Int64Lit { .. }
        | Node::Float64Lit { .. }
        | Node::StringLit { .. }
        | Node::VoidLit {}
        | Node::Local { .. }
        | Node::GetModuleLet { .. }
        | Node::Op { .. }
        | Node::And { .. }
        | Node::Or { .. }
        | Node::IfValue { .. } => {}
        _ => other = true,
    });
    !other
}

/// Core's connected declarations known to have no side effects, by qname,
/// sorted: calls of them need no order kept among themselves. Any other
/// connected declaration is taken to have some (`console.log` does).
pub const PURE_CORE: &[&str] = &[
    "core.getConsole()",
    "core.type Boolean.toString()",
    "core.type Float64.toString()",
    "core.type Int32.max()",
    "core.type Int32.min()",
    "core.type Int32.toString()",
    "core.type Int64.toString()",
    "core.type String.get isEmpty()",
];

/// What evaluating an expression may do that the order of evaluation
/// would show (notes/tmpl/analyses.md, "Order-sensitive operands").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effect {
    /// Writes module state, or calls something that may: a function of the
    /// program that does, or a connected declaration not in
    /// [`PURE_CORE`].
    pub writes: bool,
    /// Reads a module `var` that a function writes, itself or through a
    /// call.
    pub reads: bool,
    /// May bubble.
    pub bubbles: bool,
}

impl Effect {
    /// Both effects.
    pub fn and(self, other: Effect) -> Effect {
        Effect {
            writes: self.writes || other.writes,
            reads: self.reads || other.reads,
            bubbles: self.bubbles || other.bubbles,
        }
    }

    /// Whether two operands' order shows: one writes, and the other writes
    /// too, reads what may be written, or bubbles, which would skip the
    /// write or not.
    pub fn conflicts(self, other: Effect) -> bool {
        (self.writes && (other.writes || other.reads || other.bubbles))
            || (other.writes && (self.reads || self.bubbles))
    }

    /// Whether it does anything at all that order could show.
    pub fn any(self) -> bool {
        self.writes || self.reads || self.bubbles
    }
}

/// The effects of a module's functions, for [`Effect`]s of expressions
/// in it: which module `var`s functions write (so reading them is
/// order-sensitive), and each function's own effect, through what it
/// calls. In subset A no call can assign a local, since there are no
/// captures, so reading a local is never order-sensitive.
#[derive(Clone, Debug, Default)]
pub struct Effects {
    /// Module `var`s that a function writes.
    pub mutable: HashSet<DeclId>,
    functions: HashMap<DeclId, Effect>,
    pure: HashSet<DeclId>,
}

impl Effects {
    /// The effects of `module`'s functions.
    pub fn of(p: &Program, module: &Module) -> Effects {
        let qnames = QNames::derive(p);
        let pure = p
            .decls
            .ids()
            .filter(|&d| p.decls.contains(d) && is_external(p, d))
            .filter(|&d| {
                qnames
                    .get(d)
                    .is_some_and(|q| PURE_CORE.binary_search(&q.to_string().as_str()).is_ok())
            })
            .collect();
        let mut effects = Effects {
            pure,
            ..Effects::default()
        };
        let mut functions = Vec::new();
        for &item in &module.items {
            if let Node::Function { decl, .. } = p.nodes.get(item) {
                effects.mutable.extend(module_writes(p, item));
                functions.push((decl, item));
            }
        }
        // Each function's own effect, then through calls, until nothing
        // changes.
        loop {
            let mut changed = false;
            for &(decl, item) in &functions {
                let effect = effects.node(p, item);
                if effects.functions.get(&decl) != Some(&effect) {
                    effects.functions.insert(decl, effect);
                    changed = true;
                }
            }
            if !changed {
                return effects;
            }
        }
    }

    /// What evaluating `id` may do.
    pub fn node(&self, p: &Program, id: NodeId) -> Effect {
        let mut effect = Effect::default();
        walk(p, id, &mut |n| {
            let here = match p.nodes.get(n) {
                Node::SetModuleLet { .. } => Effect {
                    writes: true,
                    ..Effect::default()
                },
                Node::GetModuleLet { target } => Effect {
                    reads: self.mutable.contains(&target),
                    ..Effect::default()
                },
                Node::CallFunction {
                    callee: d,
                    may_bubble,
                    ..
                }
                | Node::CallMethod {
                    method: d,
                    may_bubble,
                    ..
                } => {
                    let called = if is_external(p, d) {
                        Effect {
                            writes: !self.pure.contains(&d),
                            ..Effect::default()
                        }
                    } else {
                        self.functions.get(&d).copied().unwrap_or_default()
                    };
                    called.and(Effect {
                        bubbles: may_bubble,
                        ..Effect::default()
                    })
                }
                Node::Op { may_bubble, .. } => Effect {
                    bubbles: may_bubble,
                    ..Effect::default()
                },
                Node::Error { .. } => Effect {
                    bubbles: true,
                    ..Effect::default()
                },
                _ => Effect::default(),
            };
            effect = effect.and(here);
        });
        effect
    }
}

/// Whether a value of type `sub` is also of type `sup`: the same type, or
/// a class or interface whose declared supertypes, theirs, and so on,
/// include `sup`; and every class and interface is an `AnyValue`.
pub fn is_subtype(p: &Program, sub: TypeId, sup: TypeId) -> bool {
    if sub == sup {
        return true;
    }
    let TypeData::Nominal { decl } = (if p.types.contains(sub) {
        p.types.get(sub)
    } else {
        return false;
    }) else {
        return false;
    };
    if sup == TypeId::ANY_VALUE {
        return true;
    }
    let mut seen = vec![decl];
    let mut next = 0;
    while next < seen.len() {
        let d = seen[next];
        next += 1;
        if !p.decls.contains(d) {
            continue;
        }
        for &s in p.decls.supers(d) {
            if s == sup {
                return true;
            }
            if let TypeData::Nominal { decl: up } = p.types.get(s) {
                if !seen.contains(&up) {
                    seen.push(up);
                }
            }
        }
    }
    false
}

/// The class or interface a type names, if it's one.
pub fn nominal_decl(p: &Program, ty: TypeId) -> Option<DeclId> {
    match p.types.contains(ty).then(|| p.types.get(ty))? {
        TypeData::Nominal { decl } => Some(decl),
        _ => None,
    }
}

/// Whether a method is its class's plain `toString()`, which a backend
/// presents as its host's display string (C#'s `ToString` override,
/// Python's `__str__`, Rust's `Display`, Lua's `__tostring`): a public
/// instance method of a class (not an interface) named `toString`, taking
/// nothing and returning `String`, as Kotlin's `isPlainToString` decides
/// (notes/tmpl/surface.md, "Show").
pub fn is_plain_to_string(p: &Program, method: DeclId) -> bool {
    if !p.decls.contains(method) {
        return false;
    }
    let d = p.decls.get(method);
    let owner_is_class = p.decls.contains(d.owner) && p.decls.get(d.owner).kind == DeclKind::Class;
    d.kind == DeclKind::Method
        && owner_is_class
        && p.name(method) == "toString"
        && !d.facts.contains(Facts::PRIVATE)
        && !d.facts.contains(Facts::PROTECTED)
        && p.signature(method)
            .is_some_and(|s| s.params.is_empty() && s.result == TypeId::STRING)
}

/// A class's or interface's members' declarations, in order, from its
/// node.
pub fn members(p: &Program, class: NodeId) -> Vec<DeclId> {
    match p.nodes.get(class) {
        Node::Class { members, .. } | Node::Interface { members, .. } => p
            .nodes
            .list(members)
            .iter()
            .filter_map(|&m| match p.nodes.get(m) {
                Node::Field { decl }
                | Node::StaticField { decl }
                | Node::Method { decl, .. }
                | Node::AbstractMethod { decl, .. }
                | Node::Function { decl, .. } => Some(decl),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A class's or interface's supertypes, theirs, and so on, breadth-first,
/// each once, not itself.
pub fn ancestors(p: &Program, class: DeclId) -> Vec<DeclId> {
    let mut found = vec![class];
    let mut next = 0;
    while next < found.len() {
        let d = found[next];
        next += 1;
        if !p.decls.contains(d) {
            continue;
        }
        for &s in p.decls.supers(d) {
            if let Some(up) = nominal_decl(p, s) {
                if !found.contains(&up) {
                    found.push(up);
                }
            }
        }
    }
    found.remove(0);
    found
}
