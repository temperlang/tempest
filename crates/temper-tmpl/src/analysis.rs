//! Analyses that passes and backends share (notes/tmpl/analyses.md).
//!
//! So far: a generic walk, and module state: what each function writes
//! of its module's `var`s (Python's `global`), what it refers to, and per
//! module, which `let`s and `var`s functions use and which the init block
//! initializes at its top level ([`module_state`]).

use crate::decls::DeclKind;
use crate::ids::{DeclId, NodeId};
use crate::schema::Node;
use crate::{Module, Program};
use std::collections::HashSet;

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
