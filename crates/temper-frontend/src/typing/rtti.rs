//! Kotlin Temper's errors for run-time type checks, `x is T` and `x as T`
//! (its `TypeFuns.kt`, `problems`; notes/typing.md, "Run-time type
//! checks"): checks that can never succeed, between unrelated types, to a
//! type that may not be distinct on every backend, or to a type
//! parameter, and, as information, checks that are unnecessary.

use crate::builtins::{Builtin, Function};
use crate::core::{self, CLASS_INTERFACE, ROLE_MASK, ROLE_TYPE_PARAM};
use crate::library::{Level, Message, RttiProblem};
use crate::scope::SymbolKind;
use crate::settle::Settle;
use crate::types::{ANY_VALUE, NEVER, NULL, TypeData, TypeId, VOID};
use temper_syntax::tree::{NodeId, NodeKind};

impl Settle {
    /// Checks `subject is target` or `subject as target`, the call `call`,
    /// where both types are known as Kotlin knows them. As in Kotlin, the
    /// target's `?` is ignored: `s is String?` checks for a `String`.
    pub(super) fn check_rtti(&mut self, call: NodeId, f: Function, args: &[NodeId]) {
        let &[subject, target] = args else { return };
        let s = self.folded_type(subject, 0);
        // The subject is an `AnyValue?` argument.
        if s == VOID {
            let message = Message::ExpectedSubType("AnyValue?".into(), self.show(VOID));
            let span = self.lib.tree.span(subject);
            self.lib.report(Level::Error, message, span);
        }
        let s_inner = self.non_null(s);
        if !self.is_nominal(s_inner) {
            return;
        }
        let span = self.lib.tree.span(call);
        let target = self.strip_nullable(target);
        if let Some((name, bound)) = self.type_param_bound(target) {
            // `T` is a subtype of its bound, so checking for it is a
            // downcast only from a supertype of that.
            let problem = match self.rtti_subtype(bound, s_inner) {
                Some(true) => RttiProblem::Formals,
                Some(false) => RttiProblem::Unrelated,
                None => return,
            };
            let message = Message::IllegalRttiCheck(problem, name, self.show(s));
            self.lib.report(Level::Error, message, span);
            return;
        }
        let t = self.eval_type(target);
        if !self.is_nominal(t) {
            return;
        }
        if self.is_concrete(s_inner) && self.is_concrete(t) && s_inner != t {
            let message = Message::ImpossibleRttiCheck(self.show(s_inner), self.show(t));
            self.lib.report(Level::Error, message, span);
            return;
        }
        // Kotlin says nothing of `x is AnyValue` but for an `AnyValue` x.
        if t == ANY_VALUE && s_inner != ANY_VALUE {
            return;
        }
        let Some(is_up) = self.rtti_subtype(s_inner, t) else {
            return;
        };
        let is_down = !is_up && self.rtti_subtype(t, s_inner) == Some(true);
        let problem = if is_down {
            // The final builtin types are `@mayDowncastTo(false)` in core.
            match self.typing.types.is_final(t) && t != VOID {
                true => RttiProblem::MayDowncast,
                false => return,
            }
        } else if !is_up {
            RttiProblem::Unrelated
        } else {
            // Not said where flow facts narrowed the subject: they may
            // have where Kotlin Temper doesn't, as after an early return.
            let needs_null_check = s != s_inner;
            let narrowed = self.typing.non_null.contains(&subject);
            if !needs_null_check && !narrowed && (f == Function::Is || s == t) {
                let message = Message::UnnecessaryRttiCheck(self.show(t), self.show(s));
                self.lib.report(Level::Info, message, span);
            }
            return;
        };
        let message = Message::IllegalRttiCheck(problem, self.show(t), self.show(s));
        self.lib.report(Level::Error, message, span);
    }

    /// Whether `subject as target` may bubble: when it's allowed and
    /// checks something, a downcast or that the subject isn't null, and
    /// Kotlin Temper can't fold it. Not where either type isn't fully
    /// known, nor for a nullable target, as `x as T?`.
    pub(super) fn cast_bubbles(&mut self, args: &[NodeId]) -> bool {
        let &[subject, target] = args else {
            return false;
        };
        if self.may_fold(subject, 0) || self.strip_nullable(target) != target {
            return false;
        }
        let s = self.known_type(subject);
        let s_inner = self.non_null(s);
        let t = self.eval_type(target);
        if !self.is_nominal(s_inner) || !self.is_nominal(t) {
            return false;
        }
        if self.is_concrete(s_inner) && self.is_concrete(t) && s_inner != t {
            return false;
        }
        if self.rtti_subtype(s_inner, t) == Some(true) {
            return s != s_inner;
        }
        !self.typing.types.is_final(t) && self.rtti_subtype(t, s_inner) == Some(true)
    }

    /// An expression's type, or for a name Kotlin Temper folds to a
    /// literal, as `s` after `let s: AnyValue = "a"`, the literal's type,
    /// as Kotlin checks that. Not one it might fold to an object, as
    /// after `let s: I = new C()`, which it may not.
    fn folded_type(&mut self, node: NodeId, depth: u32) -> TypeId {
        let tree = &self.lib.tree;
        let decl = (self.scopes.resolved(node))
            .filter(|_| tree.kind(node) == NodeKind::RightName && depth < 8)
            .and_then(|b| self.scopes.symbol(b).decl)
            .filter(|&decl| tree.kind(decl) == NodeKind::Let);
        match decl.and_then(|decl| core::let_parts(tree, decl).2) {
            Some(init)
                if matches!(tree.kind(init), NodeKind::Value | NodeKind::RightName)
                    && self.may_fold(node, 0) =>
            {
                self.folded_type(init, depth + 1)
            }
            _ => self.known_type(node),
        }
    }

    fn non_null(&self, t: TypeId) -> TypeId {
        match self.typing.types.get(t) {
            TypeData::Nullable(inner) => *inner,
            _ => t,
        }
    }

    /// A type expression without its `?`, as `String` for `String?`.
    fn strip_nullable(&self, node: NodeId) -> NodeId {
        let tree = &self.lib.tree;
        match tree.children(node) {
            &[callee, inner]
                if tree.kind(node) == NodeKind::Call
                    && self.scopes.resolved_builtin(callee)
                        == Some(Builtin::Function(Function::Nullable)) =>
            {
                inner
            }
            _ => node,
        }
    }

    /// Whether these checks reason about a type: one fully known (a
    /// builtin type, a class or interface, or a function type), other
    /// than null's and `Never`.
    fn is_nominal(&self, t: TypeId) -> bool {
        let types = &self.typing.types;
        t != NEVER && t != NULL && types.is_known(t)
    }

    /// Whether no other type's values can be a `t`'s: a builtin type
    /// other than `AnyValue`, or a class, which Temper doesn't let other
    /// classes extend.
    fn is_concrete(&self, t: TypeId) -> bool {
        match *self.typing.types.get(t) {
            TypeData::Class(symbol) => self
                .type_decl(symbol)
                .is_some_and(|c| self.lib.tree.data(c) & CLASS_INTERFACE == 0),
            _ => self.typing.types.is_final(t),
        }
    }

    /// Whether `a` is a subtype of `b`, by declarations only, as Kotlin
    /// Temper's checks see it: `AnyValue` is no other type's, nor `Void`
    /// an `AnyValue`.
    fn rtti_subtype(&self, a: TypeId, b: TypeId) -> Option<bool> {
        match (a, b) {
            _ if a == b => Some(true),
            (ANY_VALUE, _) | (VOID, ANY_VALUE) => Some(false),
            _ => self.typing.types.is_subtype(a, b),
        }
    }

    /// The name and bound of the type parameter a type expression names,
    /// if it does: `AnyValue`, or the one type it `extends`, if typing
    /// knows it.
    fn type_param_bound(&mut self, node: NodeId) -> Option<(String, TypeId)> {
        if self.lib.tree.kind(node) != NodeKind::RightName {
            return None;
        }
        let symbol = *self.scopes.symbol(self.scopes.resolved(node)?);
        let decl = symbol.decl?;
        let tree = &self.lib.tree;
        if symbol.kind != SymbolKind::TypeParam || tree.data(decl) & ROLE_MASK != ROLE_TYPE_PARAM {
            return None;
        }
        let bound = match core::let_parts(tree, decl).1 {
            Some(bound) => self.eval_type(bound),
            None => ANY_VALUE,
        };
        let name = self.lib.names.get(symbol.name).to_string();
        self.typing.types.is_known(bound).then_some((name, bound))
    }
}
