//! Named `@fun` interfaces as nominal types (notes/typing.md, "Fun
//! interfaces"): a function literal, or a reference to a declared
//! function or method, takes the interface its place wants, as Java types
//! a lambda or a method reference; a value of one `@fun` interface where
//! another is wanted is an error that suggests the function literal that
//! adapts it; a function literal that initializes a `let` without a type
//! is an error, as nothing gives it an interface; and a `@fun` interface's
//! declaration keeps to the rules Kotlin Temper documents for one (its
//! `FunDecorator.kt`), as is and as checks do.

use crate::core::{self, CLASS_FUN, LET_VAR, MEMBER_STATIC, ROLE_GETTER, ROLE_MASK, ROLE_METHOD};
use crate::core::{ROLE_CONSTRUCTOR, ROLE_SETTER};
use crate::library::{FunRule, Level, Message};
use crate::scope::SymbolKind;
use crate::settle::{Fact, Settle};
use crate::types::{ANY_VALUE, TypeData, TypeId};
use temper_syntax::tree::{NodeId, NodeKind, Span};

impl Settle {
    /// The type of `node`, of type `t`, where it's a function literal or a
    /// reference to a declared function or method of a function type:
    /// the `@fun` interface its place wants, if `t` fits it, so it's that
    /// interface's value with no adapter. Otherwise `t`.
    pub(super) fn target_fun(
        &mut self,
        node: NodeId,
        t: TypeId,
        forced: bool,
    ) -> Result<TypeId, Fact> {
        if !matches!(self.typing.types.get(t), TypeData::Fn { .. }) {
            return Ok(t);
        }
        let is_literal = self.lib.tree.kind(node) == NodeKind::Fn;
        if !is_literal && !self.is_fn_reference(node, forced)? {
            return Ok(t);
        }
        let Some(wanted) = self.wanted_type(node, forced)? else {
            return Ok(t);
        };
        let wanted = self.non_null(wanted);
        let types = &mut self.typing.types;
        let fits = types.is_fun_interface(wanted)
            && types.is_known(wanted)
            && types.is_subtype(t, wanted) != Some(false);
        Ok(if fits { wanted } else { t })
    }

    /// Whether `node` reads a declared function, as `isPos` for `let
    /// isPos(x: Int): Boolean { … }`, or a method, as `c.m` or a bare `m`
    /// in its class, but not a callee.
    fn is_fn_reference(&mut self, node: NodeId, forced: bool) -> Result<bool, Fact> {
        let parent = self.typing.parent(node);
        let tree = &self.lib.tree;
        if (parent.filter(|&p| tree.kind(p) == NodeKind::Call))
            .is_some_and(|p| tree.children(p).first() == Some(&node))
        {
            return Ok(false);
        }
        if tree.kind(node) == NodeKind::RightName {
            let Some(b) = self.scopes.resolved(node) else {
                return Ok(false);
            };
            let symbol = *self.scopes.symbol(b);
            let Some(decl) = symbol.decl else {
                return Ok(false);
            };
            return Ok(match symbol.kind {
                SymbolKind::TopLevel | SymbolKind::Local => {
                    tree.kind(decl) == NodeKind::Let
                        && tree.data(decl) & LET_VAR == 0
                        && (core::let_parts(tree, decl).2)
                            .is_some_and(|init| tree.kind(init) == NodeKind::Fn)
                }
                SymbolKind::Member => self.bare_member_decl(b).is_some_and(|m| self.is_method(m)),
                _ => false,
            });
        }
        Ok(match self.dot_member_decl(node, forced)? {
            Some(m) => self.is_method(m),
            None => false,
        })
    }

    /// Whether a member is a method, not a constructor.
    fn is_method(&self, m: NodeId) -> bool {
        self.lib.tree.data(m) & ROLE_MASK == ROLE_METHOD
    }

    /// Reports a value of `@fun` interface type `s` where another, `t`, is
    /// wanted, whose function type it would fit, at `span`, with the
    /// function literal that adapts it, as `fn (y) { p(y) }`; returns
    /// whether it did. Named function types are nominal, so a `Pred` isn't
    /// a `Check` even where their `apply`s' signatures match; Kotlin Temper
    /// accepts it.
    pub(super) fn check_fun_adapter(
        &mut self,
        s: TypeId,
        t: TypeId,
        value: NodeId,
        span: Span,
    ) -> bool {
        let nullable = |x: TypeId| matches!(self.typing.types.get(x), TypeData::Nullable(_));
        if nullable(s) && !nullable(t) {
            return false;
        }
        let (s, t) = (self.non_null(s), self.non_null(t));
        if !self.typing.types.fun_adaptable(s, t) {
            return false;
        }
        let params = self.apply_param_names(t).join(", ");
        let callee = match self.lib.tree.kind(value) {
            NodeKind::RightName => self.lib.source_text(self.lib.tree.span(value)).to_string(),
            _ => "f".to_string(),
        };
        let adapter = format!("fn ({params}) {{ {callee}({params}) }}");
        let message = Message::FunNotOther(self.show(s), self.show(t), adapter);
        self.lib.report(Level::Error, message, span);
        true
    }

    /// The names of the parameters of a `@fun` interface's `apply`, as
    /// declared, for type `t`.
    fn apply_param_names(&mut self, t: TypeId) -> Vec<String> {
        let base = self.typing.types.base(t);
        let TypeData::Class(symbol) = *self.typing.types.get(base) else {
            return Vec::new();
        };
        let Some(class) = self.type_decl(symbol) else {
            return Vec::new();
        };
        let tree = &self.lib.tree;
        let members = core::class_parts(tree, class).2;
        let apply = members.iter().copied().find(|&m| {
            tree.kind(m) == NodeKind::Let
                && tree.data(m) & ROLE_MASK == ROLE_METHOD
                && self.member_text(m) == "apply"
        });
        let Some(f) = apply.and_then(|m| core::let_parts(tree, m).2) else {
            return Vec::new();
        };
        (core::fn_parts(tree, f).0.iter())
            .map(|&p| match tree.kind(p) {
                NodeKind::Let => self.member_text(p).to_string(),
                _ => "x".to_string(),
            })
            .collect()
    }

    /// Reports a function literal, `fn (…) { … }` or `{ x => … }`, that
    /// initializes a `let` or `var` without a declared type, `decl`: no
    /// type is wanted there, so nothing gives it its interface.
    pub(super) fn check_untyped_fun_literal(&mut self, decl: NodeId, init: NodeId) {
        let tree = &self.lib.tree;
        if tree.kind(init) != NodeKind::Fn || !self.is_literal_syntax(init) {
            return;
        }
        let name = self.member_text(decl).to_string();
        let span = self.lib.tree.span(init);
        self.lib
            .report(Level::Error, Message::UntypedFunLiteral(name), span);
    }

    /// Whether a function is written as a literal, `fn (…) { … }` or a
    /// block lambda, rather than declared, as `let f(…) { … }`, which
    /// gives the same tree.
    fn is_literal_syntax(&self, f: NodeId) -> bool {
        let text = self.lib.source_text(self.lib.tree.span(f));
        text.starts_with('{')
            || (text.strip_prefix("fn"))
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    }

    /// Reports how a `@fun` interface's declaration breaks the rules
    /// Kotlin Temper documents for one, which it doesn't check: a
    /// supertype other than `AnyValue`, a member other than one method
    /// named `apply` (properties, getters, setters, and statics included),
    /// or no `apply`. A type parameter of `apply` is reported as for any
    /// interface's method, in Kotlin's words.
    pub(super) fn check_fun_interface(&mut self, class: NodeId) {
        let tree = &self.lib.tree;
        if tree.data(class) & CLASS_FUN == 0 {
            return;
        }
        let name = self.member_text(class).to_string();
        let (name_node, supers, members) = core::class_parts(tree, class);
        let (supers, members) = (supers.to_vec(), members.to_vec());
        for s in supers {
            if self.eval_type(s) != ANY_VALUE {
                let span = self.lib.tree.span(s);
                let written = self.lib.source_text(span).to_string();
                let message = Message::FunInterfaceRule(name.clone(), FunRule::Supertype(written));
                self.lib.report(Level::Error, message, span);
            }
        }
        let mut has_apply = false;
        for m in members {
            let tree = &self.lib.tree;
            if tree.kind(m) != NodeKind::Let {
                continue;
            }
            let data = tree.data(m);
            let member = self.member_text(m).to_string();
            let what = match data & ROLE_MASK {
                _ if data & MEMBER_STATIC != 0 => format!("static {member}"),
                ROLE_METHOD if member == "apply" && !has_apply => {
                    has_apply = true;
                    continue;
                }
                ROLE_METHOD => format!("method {member}"),
                ROLE_GETTER => format!("getter {member}"),
                ROLE_SETTER => format!("setter {member}"),
                ROLE_CONSTRUCTOR => "a constructor".to_string(),
                _ => format!("property {member}"),
            };
            let span = self.lib.tree.span(tree.children(m)[0]);
            let message = Message::FunInterfaceRule(name.clone(), FunRule::Member(what));
            self.lib.report(Level::Error, message, span);
        }
        if !has_apply {
            let span = self.lib.tree.span(name_node);
            let message = Message::FunInterfaceRule(name, FunRule::NoApply);
            self.lib.report(Level::Error, message, span);
        }
    }
}
