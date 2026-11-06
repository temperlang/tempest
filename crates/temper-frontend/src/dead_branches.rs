//! Branches that never run: `if (DEBUG) { … }`, with `let DEBUG = false;`,
//! where the branch names something undeclared. Kotlin Temper folds such an
//! `if` away before it looks names up, so what's in the branch is never
//! reported, and the library builds. Tempest reports it anyway, as a
//! warning, and drops the branch, so the library builds here too
//! ([`Settle::drop_dead_branches`]; notes/kotlin-differences.md).

use crate::core::{LET_INIT, LET_VAR, let_parts};
use crate::library::Level;
use crate::scope::SymbolKind;
use crate::settle::Settle;
use temper_syntax::tree::{NodeId, NodeKind};

impl Settle {
    /// Once names are resolved, and before typing, makes each `if` whose
    /// condition is a constant, `true`, `false`, or a `let` initialized to
    /// one, its live branch, where its dead branch holds an error, and
    /// makes that branch's errors warnings. An `if` whose dead branch is
    /// fine stays as it is.
    pub(crate) fn drop_dead_branches(&mut self) {
        let roots: Vec<NodeId> = (self.lib.files.iter()).map(|f| f.root).collect();
        let mut stack = roots;
        while let Some(node) = stack.pop() {
            let tree = &self.lib.tree;
            let children = tree.children(node);
            if let (NodeKind::If, &[condition, then, otherwise]) = (tree.kind(node), children) {
                if let Some(value) = self.known_bool(condition) {
                    let (live, dead) = if value {
                        (then, otherwise)
                    } else {
                        (otherwise, then)
                    };
                    let errors = self.errors_in(dead);
                    if !errors.is_empty() {
                        for index in errors {
                            if let Some(d) = self.lib.diagnostics.get_mut(index as usize) {
                                d.level = Level::Warn;
                            }
                        }
                        self.become_node(node, live);
                        stack.push(node);
                        continue;
                    }
                }
            }
            stack.extend_from_slice(self.lib.tree.children(node));
        }
    }

    /// `true` or `false`, written so ([`Settle::constant_bool`]) or as
    /// the name of a `let`, not a `var`, initialized so.
    fn known_bool(&self, node: NodeId) -> Option<bool> {
        let tree = &self.lib.tree;
        let resolved = (tree.kind(node) == NodeKind::RightName)
            .then(|| self.scopes.resolved(node))
            .flatten();
        let Some(symbol) = resolved.map(|b| self.scopes.symbol(b)) else {
            return self.constant_bool(node);
        };
        let Some(decl) = symbol.decl else {
            return self.constant_bool(node);
        };
        // Not a parameter's default, nor a field's.
        let local = matches!(symbol.kind, SymbolKind::TopLevel | SymbolKind::Local);
        let mutable = symbol.mutable || tree.data(decl) & LET_VAR != 0;
        if !local || mutable || tree.kind(decl) != NodeKind::Let || tree.data(decl) & LET_INIT == 0
        {
            return None;
        }
        self.constant_bool(let_parts(tree, decl).2?)
    }

    /// The diagnostics of the error nodes under `node`.
    fn errors_in(&self, node: NodeId) -> Vec<u32> {
        let tree = &self.lib.tree;
        let (mut stack, mut found) = (vec![node], Vec::new());
        while let Some(n) = stack.pop() {
            if tree.kind(n) == NodeKind::Error {
                found.push(tree.data(n));
            }
            stack.extend_from_slice(tree.children(n));
        }
        found
    }
}
