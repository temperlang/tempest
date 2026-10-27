//! "Cannot bubble from a function without Bubble in its return type", as
//! Kotlin Temper's `BubblesAreNotFree` reports it: each call that may
//! bubble in the body of a function whose declared return type doesn't
//! say `throws Bubble`, other than in an `orelse`'s body. A `for … of`
//! body counts as part of the function around it; other functions are
//! checked on their own, and only if they declare a return type, so block
//! lambdas, which get theirs from where they're passed, aren't yet.
//!
//! What may bubble: `bubble()`; a call of a function whose type `throws
//! Bubble`; and `/` and `%`, unless the divisor is an integer constant that
//! can't make them fail (not zero, and for `%`, positive). Kotlin folds
//! constants first, so as for the other type errors, a call it might work
//! out while compiling isn't reported (`may_fold`), nor is a branch of an
//! `if` whose condition it might.

use crate::builtins::{Builtin, Constant, Function};
use crate::core;
use crate::library::{Level, Message};
use crate::settle::Settle;
use crate::types::{FLOAT64, TypeData};
use temper_syntax::ast::Value;
use temper_syntax::tree::{NodeId, NodeKind};

impl Settle {
    /// Reports what may bubble in a function that declares it doesn't,
    /// in its parameters' defaults and its body.
    pub(super) fn check_bubbles(&mut self, f: NodeId) {
        let (params, _, body) = core::fn_parts(&self.lib.tree, f);
        for param in params {
            if self.lib.tree.kind(param) == NodeKind::Let {
                if let Some(default) = core::let_parts(&self.lib.tree, param).2 {
                    self.find_bubbles(default);
                }
            }
        }
        self.find_bubbles(body);
    }

    /// Whether a function's declared return type rules out bubbling, so
    /// [`Settle::check_bubbles`] applies: not a generator's, nor a `for …
    /// of` body's, nor `throws Bubble`.
    pub(super) fn may_not_bubble(&self, f: NodeId) -> bool {
        let tree = &self.lib.tree;
        let Some(out_type) = core::fn_parts(tree, f).1 else {
            return false;
        };
        core::fn_supers(tree, f).is_empty()
            && tree.data(f) & core::FN_LOOP_BODY == 0
            && !self.is_throws(out_type)
    }

    /// Whether a type expression is `T throws Bubble`.
    pub(super) fn is_throws(&self, ty: NodeId) -> bool {
        self.lib.tree.kind(ty) == NodeKind::Call
            && self.lib.tree.children(ty).first().is_some_and(|&callee| {
                self.scopes.resolved_builtin(callee) == Some(Builtin::Function(Function::Throws))
            })
    }

    fn find_bubbles(&mut self, node: NodeId) {
        let tree = &self.lib.tree;
        let children = tree.children(node).to_vec();
        match (tree.kind(node), children.as_slice()) {
            (NodeKind::Fn, _) if tree.data(node) & core::FN_LOOP_BODY != 0 => {
                let body = core::fn_parts(tree, node).2;
                self.find_bubbles(body);
            }
            (NodeKind::Fn, _) => {}
            // What fails in the body goes to the fallback.
            (NodeKind::Orelse, [_, fallback]) => self.find_bubbles(*fallback),
            // Kotlin drops a branch that a constant condition rules out.
            (NodeKind::If, [condition, then, otherwise]) => {
                self.find_bubbles(*condition);
                match self.constant_bool(*condition) {
                    Some(true) => self.find_bubbles(*then),
                    Some(false) => self.find_bubbles(*otherwise),
                    None if self.may_fold(*condition, 0) => {}
                    None => {
                        self.find_bubbles(*then);
                        self.find_bubbles(*otherwise);
                    }
                }
            }
            (NodeKind::Fail, _) => self.report_bubble(node),
            (NodeKind::Call, _) if self.call_bubbles(node) => self.report_bubble(node),
            _ => {
                for child in children {
                    self.find_bubbles(child);
                }
            }
        }
    }

    fn report_bubble(&mut self, node: NodeId) {
        let span = self.lib.tree.span(node);
        self.lib
            .report(Level::Error, Message::ExpectedNoBubble, span);
    }

    /// Whether a call may bubble, other than through its arguments.
    fn call_bubbles(&mut self, call: NodeId) -> bool {
        let children = self.lib.tree.children(call).to_vec();
        let Some((&callee, args)) = children.split_first() else {
            return false;
        };
        match self.scopes.resolved_builtin(callee) {
            Some(Builtin::Function(f @ (Function::Div | Function::Mod))) => {
                self.division_bubbles(call, f, args)
            }
            Some(_) => false,
            None => {
                let t = self.known_callee_type(callee);
                matches!(self.typing.types.get(t), TypeData::Fn { throws: true, .. })
                    && !self.may_fold(call, 0)
            }
        }
    }

    /// Whether `a / b` or `a % b` may bubble. Kotlin Temper picks variants
    /// that can't for an integer divisor that's a constant, not zero, and
    /// for `%`, positive; dividing floats always may.
    fn division_bubbles(&mut self, call: NodeId, f: Function, args: &[NodeId]) -> bool {
        let &[a, b] = args else {
            return false;
        };
        if self.may_fold(call, 0) {
            return false;
        }
        if self.known_type(a) == FLOAT64 || self.known_type(b) == FLOAT64 {
            return true;
        }
        match self.constant_int(b) {
            Some(n) if f == Function::Div => n == 0,
            Some(n) => n <= 0,
            None => !self.may_fold(b, 0),
        }
    }

    /// An integer literal's value, maybe negated.
    fn constant_int(&self, node: NodeId) -> Option<i64> {
        match self.lib.tree.kind(node) {
            NodeKind::Value => match self.lib.value(node)? {
                Value::Int32(n) => Some(i64::from(*n)),
                Value::Int64(n) => Some(*n),
                _ => None,
            },
            NodeKind::Call => match self.lib.tree.children(node) {
                &[callee, operand]
                    if self.scopes.resolved_builtin(callee)
                        == Some(Builtin::Function(Function::Sub)) =>
                {
                    self.constant_int(operand)?.checked_neg()
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// `true` or `false`, written as such.
    fn constant_bool(&self, node: NodeId) -> Option<bool> {
        match self.lib.tree.kind(node) {
            NodeKind::Value => match self.lib.value(node)? {
                Value::Bool(b) => Some(*b),
                _ => None,
            },
            NodeKind::RightName => match self.scopes.resolved_builtin(node)? {
                Builtin::Value(Constant::True) => Some(true),
                Builtin::Value(Constant::False) => Some(false),
                _ => None,
            },
            _ => None,
        }
    }
}
