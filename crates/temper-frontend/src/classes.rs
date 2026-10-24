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
//!
//! An object literal, `{ a: 1, b }`, is a constructor call with named
//! arguments, `new void(\a 1, \b b)`, whose class is found by
//! [`Settle::object_literal`].

use crate::builtins::Macro;
use crate::core::{
    CLASS_AUTO_EQ, CLASS_INTERFACE, LET_INIT, LET_VAR, MEMBER_CONNECTED, MEMBER_PRIVATE, MEMBER_STATIC, ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK,
    ROLE_METHOD, ROLE_PARAM, ROLE_SETTER, TYPE_PARAMS_SHIFT,
};
use crate::library::Message;
use crate::scope::{ScopeId, ScopeKind, SymbolId, SymbolKind};
use crate::settle::{Fact, Settle};
use temper_syntax::ast::{Builtin as SyntaxBuiltin, Value};
use temper_syntax::names::NameId;
use temper_syntax::tree::{NodeId, NodeKind};

impl Settle {
    /// `class C(params) extends S { members }` is `class \word ^C params...
    /// \super S fn { members }`; `interface` is the same without params.
    pub(crate) fn expand_class(&mut self, call: NodeId, args: &[NodeId], m: Macro) -> bool {
        let [word, name, rest @ ..] = args else { return false };
        if self.symbol_text(*word) != Some("word") || self.lib.tree.kind(*name) != NodeKind::LeftName {
            return false;
        }
        let interface = m == Macro::Interface;
        // `interface F(params): R;` has no body; it means `interface F {
        // apply(params): R }`, which `@fun` makes a function type.
        let (block, header) = match rest.split_last() {
            Some((&body, header)) if self.block_of(body).is_some() => (self.block_of(body), header),
            _ if interface => (None, rest),
            _ => return false,
        };
        let (mut type_params, mut supers, mut params) = (Vec::new(), Vec::new(), Vec::new());
        let mut out_type = None;
        let mut i = 0;
        while i < header.len() {
            match self.symbol_text(header[i]) {
                Some("outType") if block.is_none() && i + 1 < header.len() => {
                    out_type = Some(header[i + 1]);
                    i += 2;
                }
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
        let mut children = vec![*name];
        children.extend(&type_params);
        children.extend(&supers);
        match block {
            Some(block) => {
                if interface && !params.is_empty() {
                    return false;
                }
                for param in params {
                    children.extend(self.lower_member(param, 0, true));
                }
                let statements = self.lib.tree.children(block).to_vec();
                for statement in statements {
                    children.extend(self.lower_member(statement, 0, false));
                }
            }
            None => {
                let Some(out_type) = out_type else { return false };
                let span = self.lib.tree.span(call);
                let apply = self.lib.names.intern("apply");
                let apply = self.lib.add_name(NodeKind::LeftName, apply, span, call);
                let (parts, data) = Self::let_layout(apply, Some(out_type), None, ROLE_METHOD);
                children.push(self.lib.add(NodeKind::Let, span, data, &parts, call));
            }
        }
        let data = supers.len() as u32
            | ((type_params.len() as u32) << TYPE_PARAMS_SHIFT)
            | if interface { CLASS_INTERFACE } else { 0 }
            | if self.auto_eq.remove(&call) { CLASS_AUTO_EQ } else { 0 };
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
                    // `static x = 1` and `private x: T = v` arrive as assignments.
                    Some(Value::Builtin(SyntaxBuiltin::DesugarOperation)) if !header => {
                        if let [op, target, value] = *args {
                            let is_assign = self.lib.name(op).is_some_and(|n| self.lib.names.get(n) == "=");
                            if let Some((name, ty)) = self.typed_name(target).filter(|_| is_assign) {
                                let left = self.lib.add_name(NodeKind::LeftName, name, self.lib.tree.span(target), target);
                                let span = self.lib.tree.span(node);
                                let (children, data) = Self::let_layout(left, ty, Some(value), flags);
                                return Some(self.lib.add(NodeKind::Let, span, data, &children, node));
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
                        // `@connected` and `@connected("key")`.
                        let name = match self.lib.tree.kind(*decorator) {
                            NodeKind::Call => self.lib.tree.children(*decorator).first().copied(),
                            _ => Some(*decorator),
                        };
                        let text = name.and_then(|n| self.lib.name(n)).map(|n| self.lib.names.get(n).to_string());
                        let more = match text.as_deref() {
                            Some("private") => MEMBER_PRIVATE,
                            Some("var") => LET_VAR,
                            Some("static") => MEMBER_STATIC,
                            Some("connected") => MEMBER_CONNECTED,
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

    /// `x` or `x: T`, as the target of a field's initializer.
    fn typed_name(&self, target: NodeId) -> Option<(temper_syntax::names::NameId, Option<NodeId>)> {
        let tree = &self.lib.tree;
        match (tree.kind(target), tree.children(target)) {
            (NodeKind::RightName, _) => Some((self.lib.name(target)?, None)),
            (NodeKind::Call, &[colon, name, ty])
                if self.lib.name(colon).is_some_and(|n| self.lib.names.get(n) == ":")
                    && tree.kind(name) == NodeKind::RightName =>
            {
                Some((self.lib.name(name)?, Some(ty)))
            }
            _ => None,
        }
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

    /// `{ a: 1, b }` arrives as `new void(\a 1, \b desugarPun())`, and `{
    /// class: C, a: 1 }` as `new C(\a 1)`; this makes it a constructor
    /// call with its arguments placed, so no `\a` markers are left (see
    /// design.md, "Object literals, not named arguments"). A pun `b` is the
    /// name `b`. `void` becomes the class whose constructor takes every
    /// given name, among the classes visible here, imported ones included,
    /// as Kotlin's `ConvertObjectLiteralNew` finds it; it waits for
    /// incomplete scopes on the way out, so that none is missed. Skipped
    /// parameters get null, so their defaults. Values run in the order
    /// written: if that's not the parameters' order, through temporaries,
    /// `do { let t1 = b; let t2 = a; new C(t2, t1) }`. Returns whether the
    /// call became something else, a block or an error.
    pub(crate) fn object_literal(&mut self, call: NodeId, scope: ScopeId, forced: bool) -> Result<bool, Fact> {
        let children = self.lib.tree.children(call).to_vec();
        let Some(&target) = children.get(1) else { return Ok(false) };
        let is_void = self.lib.value(target) == Some(&Value::Void);
        if !is_void && !children[2..].iter().any(|&c| self.lib.symbol(c).is_some()) {
            return Ok(false);
        }
        let mut props = Vec::new();
        for pair in children[2..].chunks(2) {
            let (&[key, value], Some(name)) = (pair, self.lib.symbol(pair[0])) else {
                self.error(call, Message::MalformedSpecial("new".into()));
                return Ok(true);
            };
            let is_pun = self.lib.tree.children(value).first().is_some_and(|&c| {
                self.lib.value(c) == Some(&Value::Builtin(SyntaxBuiltin::DesugarPun))
            });
            let value = if is_pun { self.lib.add_name(NodeKind::RightName, name, self.lib.tree.span(value), key) } else { value };
            props.push((name, value));
        }
        let (class_name, class) = if is_void {
            let names: Vec<NameId> = props.iter().map(|&(n, _)| n).collect();
            match self.matching_class(&names, scope, forced)? {
                Ok((symbol, class)) => (self.builtin_name(symbol, self.lib.tree.span(target), target), class),
                Err(message) => {
                    self.error(call, message);
                    return Ok(true);
                }
            }
        } else {
            if self.lib.tree.kind(target) == NodeKind::RightName {
                self.resolve_name(target, scope, forced)?;
            }
            match self.scopes.resolved(target).and_then(|b| self.class_of(b)) {
                Some(class) => (target, class),
                None => {
                    self.error(call, Message::NoSignatureMatches);
                    return Ok(true);
                }
            }
        };
        self.read(class);
        // Each parameter's value, by its place in `props`.
        let params = crate::core::constructor_params(&self.lib.tree, class);
        let mut placed: Vec<Option<usize>> = vec![None; params.len()];
        for (i, &(name, _)) in props.iter().enumerate() {
            let at = params.iter().position(|&p| self.lib.name(self.lib.tree.children(p)[0]) == Some(name));
            match at {
                Some(at) if placed[at].is_none() => placed[at] = Some(i),
                _ => {
                    self.error(call, Message::NoSignatureMatches);
                    return Ok(true);
                }
            }
        }
        let in_order = placed.iter().flatten().is_sorted();
        let span = self.lib.tree.span(call);
        let mut statements = Vec::new();
        let mut values = Vec::with_capacity(props.len());
        for &(_, value) in &props {
            // Literals can't be affected by what runs between.
            if in_order || matches!(self.lib.tree.kind(value), NodeKind::Value | NodeKind::Fn | NodeKind::Fun) {
                values.push(value);
            } else {
                let t = self.lib.names.temporary("arg");
                let left = self.lib.add_name(NodeKind::LeftName, t, self.lib.tree.span(value), value);
                statements.push(self.make_let(left, None, Some(value), 0, self.lib.tree.span(value), value));
                values.push(self.lib.add_name(NodeKind::RightName, t, self.lib.tree.span(value), value));
            }
        }
        let last = placed.iter().rposition(Option::is_some).map_or(0, |i| i + 1);
        let mut args = vec![children[0], class_name];
        for (slot, &param) in placed[..last].iter().zip(&params) {
            args.push(match *slot {
                Some(i) => values[i],
                None if crate::core::let_parts(&self.lib.tree, param).2.is_some() => self.lib.add_value(Value::Null, span, call),
                None => {
                    let text = self.lib.names.get(self.lib.name(self.lib.tree.children(param)[0]).unwrap()).to_string();
                    self.error(call, Message::NoValuePassed(text));
                    return Ok(true);
                }
            });
        }
        if let Some(missing) = params[last..].iter().find(|&&p| crate::core::let_parts(&self.lib.tree, p).2.is_none()) {
            let text = self.lib.names.get(self.lib.name(self.lib.tree.children(*missing)[0]).unwrap()).to_string();
            self.error(call, Message::NoValuePassed(text));
            return Ok(true);
        }
        if statements.is_empty() {
            self.set_children(call, &args);
            return Ok(false);
        }
        statements.push(self.lib.add(NodeKind::Call, span, 0, &args, call));
        self.rewrite(call, NodeKind::Block, 0, &statements);
        Ok(true)
    }

    /// The one class visible from `scope` whose constructor takes every
    /// name, with the nearest symbol for it, or the error to report.
    fn matching_class(&mut self, names: &[NameId], scope: ScopeId, forced: bool) -> Result<Result<(SymbolId, NodeId), Message>, Fact> {
        let mut found: Vec<(SymbolId, NodeId)> = Vec::new();
        let mut at = Some(scope);
        while let Some(id) = at.filter(|&id| id != crate::scope::BUILTINS) {
            let s = self.scopes.get(id);
            if !s.complete && !forced {
                return Err(Fact::ScopeComplete(id));
            }
            for &(_, symbol) in &s.names {
                let Some(class) = self.class_of(symbol) else { continue };
                let params = crate::core::constructor_params(&self.lib.tree, class);
                let takes = |n: &NameId| params.iter().any(|&p| self.lib.name(self.lib.tree.children(p)[0]) == Some(*n));
                if names.iter().all(takes) && !found.iter().any(|&(_, c)| c == class) {
                    found.push((symbol, class));
                }
            }
            at = s.parent;
        }
        Ok(match *found {
            [one] => Ok(one),
            [] => Err(Message::NoSignatureMatches),
            _ => {
                let mut texts: Vec<&str> = found.iter().map(|&(b, _)| self.lib.names.get(self.scopes.symbol(b).name)).collect();
                texts.sort();
                Err(Message::MultipleConstructorsMatch(texts.join(", ")))
            }
        })
    }

    /// The concrete class a symbol names: one declared there, or one an
    /// import names, through its `let C = C'`.
    fn class_of(&self, symbol: SymbolId) -> Option<NodeId> {
        let tree = &self.lib.tree;
        let mut symbol = symbol;
        for _ in 0..8 {
            let decl = self.scopes.symbol(symbol).decl?;
            match tree.kind(decl) {
                NodeKind::Class if tree.data(decl) & CLASS_INTERFACE == 0 => return Some(decl),
                NodeKind::Let if tree.data(decl) & ROLE_MASK == 0 => {
                    let init = crate::core::let_parts(tree, decl).2?;
                    if tree.kind(init) != NodeKind::RightName {
                        return None;
                    }
                    symbol = self.scopes.resolved(init)?;
                }
                _ => return None,
            }
        }
        None
    }
}
