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

use crate::decls::DeclKind;
use crate::ids::{DeclId, NodeId};
use crate::qname::{QNames, is_external};
use crate::schema::Node;
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
