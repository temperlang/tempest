//! Sealed types and `when` (notes/typing.md, "Sealed types and `when`"):
//! an `if` chain without a final `else`, as `when` makes, whose `is`
//! checks of one name cover every subtype of a sealed interface that the
//! name's type is, never reaches its missing `else`, so the chain's value
//! is the join of its branches', and the `else` panics, as Kotlin
//! Temper's `anyExhaustiveSealedType` and `replaceVoidishPanics` make it.

use super::Settle;
use crate::builtins::{Builtin, Function};
use crate::core::{self, CLASS_SEALED};
use crate::scope::SymbolId;
use crate::settle::Fact;
use crate::types::TypeData;
use std::collections::{HashMap, HashSet};
use temper_syntax::tree::{NodeId, NodeKind};

impl Settle {
    /// Whether `block`, the empty block standing for an `if`'s missing
    /// `else`, is never reached, noted in [`super::Typing::unreachable`].
    pub(super) fn unreachable_else(&mut self, block: NodeId, forced: bool) -> Result<bool, Fact> {
        if self.typing.unreachable.contains(&block) {
            return Ok(true);
        }
        let tree = &self.lib.tree;
        let else_of = |tree: &temper_syntax::tree::Tree, parent: NodeId, kid: NodeId| {
            tree.kind(parent) == NodeKind::If && tree.children(parent).get(2) == Some(&kid)
        };
        if tree.kind(block) != NodeKind::Block || !tree.children(block).is_empty() {
            return Ok(false);
        }
        let Some(parent) = self.typing.parent(block) else {
            return Ok(false);
        };
        if !else_of(tree, parent, block) {
            return Ok(false);
        }
        // The chain's first `if`, then each condition in order.
        let mut head = parent;
        while let Some(up) = self.typing.parent(head) {
            if !else_of(tree, up, head) {
                break;
            }
            head = up;
        }
        let mut conditions = Vec::new();
        let mut at = head;
        loop {
            let kids = tree.children(at);
            conditions.push(kids[0]);
            if kids[2] == block {
                break;
            }
            at = kids[2];
        }
        let mut subject = None;
        let mut found: HashMap<SymbolId, HashSet<SymbolId>> = HashMap::new();
        for cond in conditions {
            let tree = &self.lib.tree;
            let &[callee, value, ty] = tree.children(cond) else {
                continue;
            };
            let is = self.scopes.resolved_builtin(callee) == Some(Builtin::Function(Function::Is));
            if tree.kind(cond) != NodeKind::Call || !is {
                continue;
            }
            let Some(name) = self.lib.name(value) else {
                continue;
            };
            match subject {
                None => subject = Some((name, value)),
                // Checks of another name end it.
                Some((n, _)) if n != name => break,
                Some(_) => {}
            }
            let t = self.eval_type(ty);
            let base = self.typing.types.base(t);
            let TypeData::Class(sub) = *self.typing.types.get(base) else {
                continue;
            };
            for sealed in self.sealed_supers(sub) {
                let all = self.sealed_subtypes(sealed).len();
                let seen = found.entry(sealed).or_default();
                seen.insert(sub);
                if seen.len() < all {
                    continue;
                }
                // The name's own type must be the sealed type's, or below.
                let (_, first) = subject.unwrap_or((name, value));
                let subject_type = self.expr_type(first, forced)?;
                let sealed_type = self.typing.types.intern(TypeData::Class(sealed));
                let below = self.typing.types.supertype_as(subject_type, sealed_type);
                if below.is_some() {
                    self.typing.unreachable.insert(block);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// The class of a generic type given arguments, by its base.
    fn class_symbol_of(&self, t: crate::types::TypeId) -> Option<SymbolId> {
        match *self.typing.types.get(self.typing.types.base(t)) {
            TypeData::Class(s) => Some(s),
            _ => None,
        }
    }

    /// The sealed interfaces a class or interface extends directly.
    fn sealed_supers(&mut self, class: SymbolId) -> Vec<SymbolId> {
        let subtypes = self.all_sealed_subtypes();
        (subtypes.iter())
            .filter(|(_, subs)| subs.contains(&class))
            .map(|(&s, _)| s)
            .collect()
    }

    /// A sealed interface's subtypes.
    fn sealed_subtypes(&mut self, sealed: SymbolId) -> Vec<SymbolId> {
        self.all_sealed_subtypes()
            .get(&sealed)
            .cloned()
            .unwrap_or_default()
    }

    /// Each sealed interface of the library, by its symbol, with the
    /// classes and interfaces that extend it directly, worked out once.
    fn all_sealed_subtypes(&mut self) -> &HashMap<SymbolId, Vec<SymbolId>> {
        if self.typing.sealed_subtypes.is_none() {
            let mut classes = Vec::new();
            for i in 0..self.lib.tree.len() as u32 {
                let node = NodeId(i);
                if self.lib.tree.kind(node) == NodeKind::Class {
                    classes.push(node);
                }
            }
            let mut out: HashMap<SymbolId, Vec<SymbolId>> = HashMap::new();
            for &c in &classes {
                if self.lib.tree.data(c) & CLASS_SEALED != 0 {
                    if let Some(s) = self.scopes.resolved(core::class_parts(&self.lib.tree, c).0) {
                        out.entry(s).or_default();
                    }
                }
            }
            for &c in &classes {
                let Some(own) = self.scopes.resolved(core::class_parts(&self.lib.tree, c).0) else {
                    continue;
                };
                let supers = core::class_parts(&self.lib.tree, c).1.to_vec();
                for s in supers {
                    let t = self.eval_type(s);
                    if let Some(sup) = self.class_symbol_of(t) {
                        if let Some(subs) = out.get_mut(&sup) {
                            subs.push(own);
                        }
                    }
                }
            }
            self.typing.sealed_subtypes = Some(out);
        }
        self.typing.sealed_subtypes.as_ref().expect("just made")
    }
}
