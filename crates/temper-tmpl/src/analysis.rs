//! Analyses that passes and backends share (notes/tmpl/analyses.md).
//!
//! So far: a generic walk, and module state, which says what each
//! function writes of its module's `var`s (Python's `global`).

use crate::Program;
use crate::ids::{DeclId, NodeId};
use crate::schema::Node;

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
