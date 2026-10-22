//! `class` and `interface`: the macro that lowers a declaration into a
//! `Class` node ([`crate::core`]), and the settle step that gives it a scope.
//!
//! A class's body is read by its shape, not expanded as code: `x: T = v`
//! and `@var` declare fields, `name(params): T { body }` a method, `get` and
//! `set` a property's accessors and `constructor(...)` a constructor, each
//! maybe decorated with a visibility or `static`. Header parameters, as in
//! `class C(public x: String)`, are fields set by the implied constructor.
//!
//! The class's scope holds its members, after a synthetic `this`; it's
//! complete once made, so members may use each other in any order. A bare
//! member name inside the class means that member of `this`.

use crate::builtins::Macro;
use crate::core::{
    CLASS_INTERFACE, LET_INIT, LET_VAR, MEMBER_PRIVATE, MEMBER_STATIC, ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK,
    ROLE_METHOD, ROLE_PARAM, ROLE_SETTER, TYPE_PARAMS_SHIFT,
};
use crate::library::Message;
use crate::scope::{ScopeId, ScopeKind, SymbolId, SymbolKind};
use crate::settle::Settle;
use temper_syntax::ast::{Builtin as SyntaxBuiltin, Value};
use temper_syntax::tree::{NodeId, NodeKind};

impl Settle {
    /// `class C(params) extends S { members }` is `class \word ^C params...
    /// \super S fn { members }`; `interface` is the same without params.
    pub(crate) fn expand_class(&mut self, call: NodeId, args: &[NodeId], m: Macro) -> bool {
        let [word, name, rest @ ..] = args else { return false };
        if self.symbol_text(*word) != Some("word") || self.lib.tree.kind(*name) != NodeKind::LeftName {
            return false;
        }
        let Some((&body, header)) = rest.split_last() else { return false };
        let Some(block) = self.block_of(body) else { return false };
        let (mut type_params, mut supers, mut params) = (Vec::new(), Vec::new(), Vec::new());
        let mut i = 0;
        while i < header.len() {
            match self.symbol_text(header[i]) {
                Some("super") if i + 1 < header.len() => {
                    self.flatten_supers(header[i + 1], &mut supers);
                    i += 2;
                }
                Some("typeArg") if i + 1 < header.len() => {
                    let Some(param) = self.lower_type_param(header[i + 1]) else { return false };
                    type_params.push(param);
                    i += 2;
                }
                Some(_) => return false,
                None => {
                    params.push(header[i]);
                    i += 1;
                }
            }
        }
        let interface = m == Macro::Interface;
        if interface && !params.is_empty() {
            return false;
        }
        let mut children = vec![*name];
        children.extend(&type_params);
        children.extend(&supers);
        for param in params {
            children.extend(self.lower_member(param, 0, true));
        }
        let statements = self.lib.tree.children(block).to_vec();
        for statement in statements {
            children.extend(self.lower_member(statement, 0, false));
        }
        let data = supers.len() as u32
            | ((type_params.len() as u32) << TYPE_PARAMS_SHIFT)
            | if interface { CLASS_INTERFACE } else { 0 };
        self.rewrite(call, NodeKind::Class, data, &children);
        true
    }

    /// A member as a `Let` with its role and `flags`, or none for what
    /// isn't one, like comments, or what's malformed, which becomes an
    /// error node.
    fn lower_member(&mut self, node: NodeId, flags: u32, header: bool) -> Option<NodeId> {
        let children = self.lib.tree.children(node).to_vec();
        match self.lib.tree.kind(node) {
            NodeKind::Value if self.lib.value(node) == Some(&Value::Void) => return None,
            NodeKind::Decl | NodeKind::Block => {
                let lowered = if header { self.lower_param(node) } else { None };
                let lowered = match lowered {
                    Some(lowered) => lowered,
                    None if self.lib.tree.kind(node) == NodeKind::Decl => {
                        self.lower_decl(node);
                        node
                    }
                    None => node,
                };
                if self.lib.tree.kind(lowered) == NodeKind::Let {
                    let role = if header { ROLE_PARAM } else { 0 };
                    return Some(self.add_flags(lowered, flags | role));
                }
            }
            NodeKind::Call => {
                let (&callee, args) = children.split_first()?;
                match self.lib.value(callee) {
                    Some(Value::Builtin(SyntaxBuiltin::EmbeddedComment)) => return None,
                    Some(Value::Builtin(SyntaxBuiltin::DocComment)) => {
                        return self.lower_member(*args.first()?, flags, header);
                    }
                    // `static x = 1` arrives as an assignment.
                    Some(Value::Builtin(SyntaxBuiltin::DesugarOperation)) if !header => {
                        if let [op, target, value] = *args {
                            let is_assign = self.lib.name(op).is_some_and(|n| self.lib.names.get(n) == "=");
                            if is_assign && self.lib.tree.kind(target) == NodeKind::RightName {
                                let name = self.lib.name(target).unwrap();
                                let left = self.lib.add_name(NodeKind::LeftName, name, self.lib.tree.span(target), target);
                                let span = self.lib.tree.span(node);
                                let children = [left, value];
                                return Some(self.lib.add(NodeKind::Let, span, LET_INIT | flags, &children, node));
                            }
                        }
                    }
                    _ => {}
                }
                let callee_text = match self.lib.tree.kind(callee) {
                    NodeKind::RightName => self.lib.name(callee).map(|n| self.lib.names.get(n).to_string()),
                    _ => None,
                };
                match (callee_text.as_deref(), args) {
                    (Some("@"), [decorator, target]) => {
                        let text = self.lib.name(*decorator).map(|n| self.lib.names.get(n).to_string());
                        let more = match text.as_deref() {
                            Some("private") => MEMBER_PRIVATE,
                            Some("var") => LET_VAR,
                            Some("static") => MEMBER_STATIC,
                            // Public and protected are the default here.
                            _ => 0,
                        };
                        return self.lower_member(*target, flags | more, header);
                    }
                    (Some(accessor @ ("get" | "set")), [word, name, rest @ ..])
                        if !header && self.symbol_text(*word) == Some("word") =>
                    {
                        let role = if accessor == "get" { ROLE_GETTER } else { ROLE_SETTER };
                        return self.lower_method(node, *name, rest, flags | role);
                    }
                    // `y: Int` without a value.
                    (Some(":"), [name, ty]) if self.lib.tree.kind(*name) == NodeKind::RightName => {
                        let text = self.lib.name(*name).unwrap();
                        let left = self.lib.add_name(NodeKind::LeftName, text, self.lib.tree.span(*name), *name);
                        let span = self.lib.tree.span(node);
                        let role = if header { ROLE_PARAM } else { 0 };
                        let (children, data) = Self::let_layout(left, Some(*ty), None, flags | role);
                        return Some(self.lib.add(NodeKind::Let, span, data, &children, node));
                    }
                    (Some("let"), [word, name, rest @ ..]) if !header && self.symbol_text(*word) == Some("word") => {
                        return self.lower_method(node, *name, rest, flags | ROLE_METHOD);
                    }
                    (Some(text), _) if !header => {
                        let role = if text == "constructor" { ROLE_CONSTRUCTOR } else { ROLE_METHOD };
                        let name = self.lib.name(callee).unwrap();
                        let span = self.lib.tree.span(callee);
                        let left = self.lib.add_name(NodeKind::LeftName, name, span, callee);
                        return self.lower_method(node, left, args, flags | role);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if self.lib.tree.kind(node) != NodeKind::Error {
            self.error(node, Message::Unsupported("this kind of class member".into()));
        }
        None
    }

    /// A method, accessor or constructor: a `Let` of its `Fn`, or with no
    /// body, abstract, typed by its return type. Abstract members' parameters
    /// aren't kept yet.
    fn lower_method(&mut self, node: NodeId, name: NodeId, parts: &[NodeId], flags: u32) -> Option<NodeId> {
        if self.lib.tree.kind(name) != NodeKind::LeftName {
            self.error(node, Message::MalformedFunction);
            return None;
        }
        let sig = self.signature(node, parts)?;
        let span = self.lib.tree.span(node);
        let (ty, init) = match sig.body {
            Some(body) => {
                let Some(block) = self.block_of(body) else {
                    self.error(node, Message::MalformedFunction);
                    return None;
                };
                let Some(f) = self.make_fn(&sig, block, span, node) else {
                    self.error(node, Message::MalformedFunction);
                    return None;
                };
                (None, Some(f))
            }
            None => (sig.out_type, None),
        };
        let (children, data) = Self::let_layout(name, ty, init, flags);
        Some(self.lib.add(NodeKind::Let, span, data, &children, node))
    }

    /// `extends A & B` lists `A` and `B`.
    fn flatten_supers(&mut self, node: NodeId, supers: &mut Vec<NodeId>) {
        let children = self.lib.tree.children(node).to_vec();
        if let [callee, op, a, b] = *children {
            let is_desugar = self.lib.value(callee) == Some(&Value::Builtin(SyntaxBuiltin::DesugarOperation));
            if is_desugar && self.lib.name(op).is_some_and(|n| self.lib.names.get(n) == "&") {
                self.flatten_supers(a, supers);
                self.flatten_supers(b, supers);
                return;
            }
        }
        supers.push(node);
    }

    fn add_flags(&mut self, let_node: NodeId, flags: u32) -> NodeId {
        if flags != 0 {
            let data = self.lib.tree.data(let_node) | flags;
            let parts = self.lib.tree.children(let_node).to_vec();
            self.rewrite(let_node, NodeKind::Let, data, &parts);
        }
        let_node
    }

    /// Makes a class's scope, with `this` and its members, and queues their
    /// parts. Supertypes resolve outside it, but inside its type parameters'.
    pub(crate) fn enter_class(&mut self, node: NodeId, scope: ScopeId) {
        self.declare(node, scope);
        let scope = self.enter_type_params(node, scope);
        let (_, supers, members) = crate::core::class_parts(&self.lib.tree, node);
        let (supers, members) = (supers.to_vec(), members.to_vec());
        for s in supers {
            self.visit(s, scope);
        }
        let class_scope = self.scopes.add(scope, ScopeKind::Class, node, true);
        let this = self.lib.names.temporary("this");
        self.scopes.declare(class_scope, this, SymbolKind::Synthetic, None, false);
        for member in members {
            self.declare(member, class_scope);
            let parts = self.lib.tree.children(member)[1..].to_vec();
            for part in parts {
                self.visit(part, class_scope);
            }
        }
    }

    /// Whether a bare member name may be assigned from `scope`: a `var`
    /// field, a property with a setter, or a field without an initializer
    /// in its class's constructor.
    pub(crate) fn member_assignable(&self, member: SymbolId, scope: ScopeId) -> bool {
        let symbol = *self.scopes.symbol(member);
        let tree = &self.lib.tree;
        let same_name = self.scopes.get(symbol.owner).names.iter().filter(|(n, _)| *n == symbol.name);
        let decls: Vec<NodeId> = same_name.filter_map(|&(_, b)| self.scopes.symbol(b).decl).collect();
        if decls.iter().any(|&d| tree.data(d) & ROLE_MASK == ROLE_SETTER) {
            return true;
        }
        let Some(decl) = symbol.decl else { return false };
        let data = tree.data(decl);
        if data & LET_VAR != 0 {
            return true;
        }
        if data & ROLE_MASK != 0 || data & LET_INIT != 0 {
            return false;
        }
        // The nearest function must be the constructor's, directly in the class.
        let Some(&(_, f)) = self.enclosing(scope).last() else { return false };
        let in_class = self.scopes.of_node(f).and_then(|s| self.scopes.get(s).parent) == Some(symbol.owner);
        let class = self.scopes.get(symbol.owner).node;
        in_class
            && crate::core::class_parts(tree, class).2.iter().any(|&m| {
                tree.data(m) & ROLE_MASK == ROLE_CONSTRUCTOR && crate::core::let_parts(tree, m).2 == Some(f)
            })
    }

    /// `this` is the enclosing class's synthetic `this`.
    pub(crate) fn expand_this(&mut self, call: NodeId, scope: ScopeId) {
        let mut at = Some(scope);
        while let Some(id) = at {
            let s = self.scopes.get(id);
            if s.kind == ScopeKind::Class {
                let symbol = s.names[0].1;
                let span = self.lib.tree.span(call);
                let name = self.builtin_name(symbol, span, call);
                return self.become_node(call, name);
            }
            at = s.parent;
        }
        self.error(call, Message::Unsupported("this outside a class".into()));
    }
}
