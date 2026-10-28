//! Overrides' signatures, as Kotlin Temper's `TypeDeclChecker` checks
//! them (notes/typing.md, "Classes' types"): a method, getter or setter,
//! or a field as a getter, must have the same signature as each one of
//! its name and kind in the class's supertypes.

use crate::classes::visibility_rank;
use crate::core::{
    self, MEMBER_PRIVATE, MEMBER_STATIC, ROLE_GETTER, ROLE_MASK, ROLE_METHOD, ROLE_PARAM,
    ROLE_SETTER, class_parts,
};
use crate::library::{Level, Message};
use crate::settle::Settle;
use crate::types::{TypeId, VOID};
use temper_syntax::names::NameId;
use temper_syntax::tree::{NodeId, NodeKind};

/// What an override must match: its parameters' types, with a default
/// making one nullable, as Kotlin Temper compares them; its result; and
/// whether it may bubble.
#[derive(PartialEq)]
struct OverrideSig {
    params: Vec<TypeId>,
    ret: TypeId,
    throws: bool,
}

impl Settle {
    /// Reports each of a class's or interface's own members whose
    /// signature differs from one it overrides, once, for the first such
    /// supertype in Kotlin Temper's order ([`Settle::lineage`]), as Kotlin
    /// reports one error per position. So a member that narrows the
    /// visibility of one it overrides, reported as that, isn't reported
    /// here. Only what typing fully knows is compared, so not generic
    /// classes or methods yet.
    pub(super) fn check_overrides(&mut self, class: NodeId) {
        if !core::type_params(&self.lib.tree, class).is_empty() {
            return;
        }
        let ancestors: Vec<NodeId> = (self.lineage(class).into_iter())
            .filter(|&a| core::type_params(&self.lib.tree, a).is_empty())
            .collect();
        let members = class_parts(&self.lib.tree, class).2.to_vec();
        for own in members {
            let Some(key) = self.overriding_key(own) else {
                continue;
            };
            let rank = visibility_rank(self.lib.tree.data(own));
            let overridden: Vec<(NodeId, NodeId)> = (ancestors.iter())
                .flat_map(|&a| {
                    let tree = &self.lib.tree;
                    (class_parts(tree, a).2.iter())
                        .filter(|&&m| tree.data(m) & MEMBER_PRIVATE == 0)
                        .map(move |&m| (a, m))
                })
                .filter(|&(_, m)| self.overriding_key(m) == Some(key))
                .collect();
            let tree = &self.lib.tree;
            if overridden
                .iter()
                .any(|&(_, m)| rank < visibility_rank(tree.data(m)))
            {
                continue;
            }
            let Some(mine) = overridden.first().and_then(|_| self.override_sig(own)) else {
                continue;
            };
            for (ancestor, other) in overridden {
                let Some(wanted) = self.override_sig(other) else {
                    continue;
                };
                if mine != wanted {
                    let message = Message::IncompatibleSignature(
                        self.member_text(class).to_string(),
                        self.override_description(own),
                        self.describe_sig(own, &mine),
                        self.describe_sig(other, &wanted),
                        self.member_text(ancestor).to_string(),
                    );
                    let span = self.lib.tree.span(own);
                    self.lib.report(Level::Error, message, span);
                    break;
                }
            }
        }
    }

    /// A class's supertypes, theirs and so on, depth-first in `extends`
    /// order, each once, as Kotlin Temper's `TypeDeclChecker` walks them.
    fn lineage(&self, class: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        let mut stack = vec![class];
        while let Some(c) = stack.pop() {
            if found.contains(&c) {
                continue;
            }
            found.push(c);
            stack.extend(self.super_decls(c).into_iter().rev());
        }
        found.remove(0);
        found
    }

    /// A member's name and kind, as overrides match them, a field being a
    /// getter; none for what doesn't override, as a static member.
    fn overriding_key(&self, member: NodeId) -> Option<(Option<NameId>, u32)> {
        let tree = &self.lib.tree;
        let data = tree.data(member);
        if data & MEMBER_STATIC != 0 {
            return None;
        }
        let role = match data & ROLE_MASK {
            0 | ROLE_PARAM => ROLE_GETTER,
            role @ (ROLE_METHOD | ROLE_GETTER | ROLE_SETTER) => role,
            _ => return None,
        };
        Some((self.lib.name(tree.children(member)[0]), role))
    }

    /// `f`, `get f` or `set f`, as Kotlin describes a method, a field
    /// being a getter.
    fn override_description(&self, member: NodeId) -> String {
        match self.lib.tree.data(member) & ROLE_MASK {
            0 | ROLE_PARAM => format!("get {}", self.member_text(member)),
            _ => self.method_description(member),
        }
    }

    /// A member's signature, if typing knows all of it.
    fn override_sig(&mut self, member: NodeId) -> Option<OverrideSig> {
        let tree = &self.lib.tree;
        let role = tree.data(member) & ROLE_MASK;
        let (_, ty, init) = core::let_parts(tree, member);
        let sig = match role {
            0 | ROLE_PARAM => OverrideSig {
                params: Vec::new(),
                ret: self.eval_type(ty?),
                throws: false,
            },
            _ => {
                let f = init?;
                if !core::type_params(tree, f).is_empty() {
                    return None;
                }
                let (params, out_type, _) = core::fn_parts(tree, f);
                if params.iter().any(|&p| tree.kind(p) != NodeKind::Let) {
                    return None;
                }
                let params = (params.into_iter())
                    .map(|p| self.member_param(p).ty)
                    .collect();
                let (ret, throws) = match out_type {
                    Some(t) => (self.eval_type(t), self.is_throws(t)),
                    // A setter returns `Void` without saying so.
                    None if role == ROLE_SETTER => (VOID, false),
                    None => return None,
                };
                OverrideSig {
                    params,
                    ret,
                    throws,
                }
            }
        };
        let types = &self.typing.types;
        let known = sig.params.iter().all(|&t| types.is_known(t)) && types.is_known(sig.ret);
        known.then_some(sig)
    }

    /// A signature as Kotlin Temper writes it in messages: `f(x: Int32):
    /// Void`, with a getter's or setter's bare name, and a result that may
    /// bubble as `Result<Int32, Bubble>`.
    fn describe_sig(&self, member: NodeId, sig: &OverrideSig) -> String {
        let tree = &self.lib.tree;
        let role = tree.data(member) & ROLE_MASK;
        let names = match core::let_parts(tree, member).2 {
            Some(f) if matches!(role, ROLE_METHOD | ROLE_GETTER | ROLE_SETTER) => {
                core::fn_parts(tree, f).0
            }
            _ => Vec::new(),
        };
        let params: Vec<String> = (sig.params.iter().enumerate())
            .map(|(i, &t)| {
                let name = (names.get(i))
                    .and_then(|&p| self.lib.name(tree.children(p)[0]))
                    .map_or("_", |n| self.lib.names.get(n));
                format!("{name}: {}", self.show(t))
            })
            .collect();
        let ret = self.show(sig.ret);
        let ret = match sig.throws {
            true => format!("Result<{ret}, Bubble>"),
            false => ret,
        };
        format!("{}({}): {ret}", self.member_text(member), params.join(", "))
    }
}
