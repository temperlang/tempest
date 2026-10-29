//! Target typing, milestone 4's sixth slice (notes/typing.md, "List
//! literals and target typing"): the type a value's place wants, from a
//! declared type, a function's result, a known callee's parameter, or a
//! list literal's element type, types a literal that has no type of its
//! own to give.
//!
//! So far, list literals: `[]` where a `List<Int32>` is wanted is one,
//! and `[d]` for a `d: Dog` where a `List<Animal>` is wanted is a
//! `List<Animal>`; elsewhere, a list literal is a list of its elements'
//! common type, as Kotlin Temper's solver finds it.

use crate::builtins::{Builtin, Function};
use crate::core::{self, ROLE_MASK, ROLE_TYPE_PARAM};
use crate::settle::{Fact, Settle};
use crate::types::{NEVER, NULL, TypeData, TypeId, UNKNOWN, VOID};
use temper_syntax::ast::Value;
use temper_syntax::tree::{NodeId, NodeKind};

impl Settle {
    /// Whether a node is a list literal, `[a, b]`.
    pub(super) fn is_list_literal(&self, node: NodeId) -> bool {
        let tree = &self.lib.tree;
        tree.kind(node) == NodeKind::Call
            && (tree.children(node).first()).is_some_and(|&callee| {
                self.scopes.resolved_builtin(callee) == Some(Builtin::Function(Function::List))
            })
    }

    /// A list literal's type: a `List` of the element type its place
    /// wants, if every element is one, else of its elements' common type
    /// ([`Settle::common_type`]), unknown if they have none.
    pub(super) fn list_type(&mut self, list: NodeId, forced: bool) -> Result<TypeId, Fact> {
        let elements = self.lib.tree.children(list)[1..].to_vec();
        let mut types = Vec::with_capacity(elements.len());
        for &element in &elements {
            types.push(self.expr_type(element, forced)?);
        }
        let wanted = self.wanted_type(list, forced)?;
        let element = match wanted.and_then(|t| self.wanted_element(t)) {
            Some(x)
                if (types.iter()).all(|&t| self.typing.types.is_subtype(t, x) == Some(true)) =>
            {
                x
            }
            _ => self.common_type(&types),
        };
        Ok(self.typing.types.list_of(element))
    }

    /// The element type of a list where a `wanted` is wanted, if it's a
    /// `List` or `Listed` (maybe nullable) whose element type is known.
    pub(super) fn wanted_element(&self, wanted: TypeId) -> Option<TypeId> {
        let wanted = self.non_null(wanted);
        let types = &self.typing.types;
        let base = types.builtin_name(types.base(wanted));
        match types.args(wanted) {
            &[x] if matches!(base, Some("List" | "Listed")) && types.is_known(x) => Some(x),
            _ => None,
        }
    }

    /// List elements' common type, as Kotlin Temper's solver finds one
    /// where tempest can be sure it's the same: their type if they're all
    /// one, made nullable for null or a nullable one, or the one the
    /// others are subtypes of, as `Animal` for a `Dog` and an `Animal`. A
    /// `Never`, as `panic()`, counts for nothing. Unknown where it's
    /// anything else, as for a `Dog` and a `Cat` (Kotlin: their common
    /// supertype), for elements that are only null or `Never`, or none
    /// (Kotlin: `AnyValue`), and for a `Void` element, an error itself.
    fn common_type(&mut self, elements: &[TypeId]) -> TypeId {
        let mut common = NEVER;
        for &t in elements {
            if t == VOID || !self.typing.types.is_known(t) {
                return UNKNOWN;
            }
            let joined = self.typing.types.join(common, t);
            common = match joined {
                UNKNOWN => {
                    let (a, b) = (self.non_null(common), self.non_null(t));
                    let types = &mut self.typing.types;
                    let wider = if types.is_subtype(a, b) == Some(true) {
                        b
                    } else if types.is_subtype(b, a) == Some(true) {
                        a
                    } else {
                        return UNKNOWN;
                    };
                    match a != common || b != t {
                        true => types.nullable(wider),
                        false => wider,
                    }
                }
                _ => joined,
            };
        }
        match common {
            NEVER | NULL => UNKNOWN,
            _ => common,
        }
    }

    /// The type a value's place wants, if it says: a declaration's,
    /// field's, or parameter's declared type, for its initializer or
    /// default; a local's, for a value assigned to it, or a member's
    /// setter's; a function's declared result, for what it returns or
    /// its body gives as its result, through blocks and the branches of
    /// `if` and `orelse`; a known callee's parameter, for an argument,
    /// but not a generic function's or class's, nor in a call with a
    /// function literal, which may be a trailing block placed elsewhere,
    /// or with named arguments; and the element
    /// type of a list wanted, for a list literal's element.
    fn wanted_type(&mut self, node: NodeId, forced: bool) -> Result<Option<TypeId>, Fact> {
        let Some(parent) = self.typing.parent(node) else {
            return Ok(None);
        };
        let tree = &self.lib.tree;
        let children = tree.children(parent);
        let (index, count) = (children.iter().position(|&c| c == node), children.len());
        let is_last = index == Some(count - 1);
        Ok(match tree.kind(parent) {
            NodeKind::Block if is_last => self.wanted_type(parent, forced)?,
            // An `if`'s branches, after its condition, and an `orelse`'s.
            NodeKind::If if matches!(index, Some(1 | 2)) => self.wanted_type(parent, forced)?,
            NodeKind::Orelse => self.wanted_type(parent, forced)?,
            NodeKind::Fn if core::fn_parts(tree, parent).2 == Some(node) => {
                self.declared_result(parent)
            }
            NodeKind::Return => self.declared_result(NodeId(tree.data(parent))),
            NodeKind::Let => {
                let (_, ty, init) = core::let_parts(tree, parent);
                let is_type_param = tree.data(parent) & ROLE_MASK == ROLE_TYPE_PARAM;
                (ty.filter(|_| init == Some(node) && !is_type_param)).map(|ty| self.eval_type(ty))
            }
            NodeKind::Assign if is_last && count == 2 => {
                let target = children[0];
                match self.local_target(target) {
                    Some(decl) => Some(self.decl_type(decl, forced)?),
                    None => Some(self.member_target_type(target)),
                }
            }
            NodeKind::Call => match index {
                Some(i) if i > 0 => self.wanted_arg(parent, i - 1, forced)?,
                _ => None,
            },
            _ => None,
        })
    }

    /// The type a call's `index`th argument's place wants, as
    /// [`Settle::wanted_type`] says.
    fn wanted_arg(
        &mut self,
        call: NodeId,
        index: usize,
        forced: bool,
    ) -> Result<Option<TypeId>, Fact> {
        let children = self.lib.tree.children(call).to_vec();
        let (callee, args) = (children[0], &children[1..]);
        let builtin = self.scopes.resolved_builtin(callee);
        if builtin == Some(Builtin::Function(Function::List)) {
            let wanted = self.wanted_type(call, forced)?;
            return Ok(wanted.and_then(|t| self.wanted_element(t)));
        }
        let tree = &self.lib.tree;
        let placed_later = args.iter().any(|&a| {
            tree.kind(a) == NodeKind::Fn || matches!(self.lib.value(a), Some(Value::Symbol(_)))
        });
        if placed_later {
            return Ok(None);
        }
        let params = match builtin {
            // Not a generic class's, solved per call.
            Some(Builtin::Function(Function::New)) => match self.new_class(args) {
                Some(class) if index > 0 => {
                    let params = self.constructor_type(class);
                    return Ok(params.get(index - 1).map(|p| p.ty));
                }
                _ => return Ok(None),
            },
            Some(_) => return Ok(None),
            // A generic function's own type parameters are unknown here,
            // as it's solved per call.
            None => self.callee_type(callee, forced)?,
        };
        let types = &self.typing.types;
        Ok(match types.get(params) {
            TypeData::Fn { params, .. } => {
                (params.get(index).map(|p| p.ty)).filter(|&t| types.is_known(t))
            }
            _ => None,
        })
    }
}
