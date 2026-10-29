//! `class` and `interface`: the macro that lowers a declaration into a
//! `Class` node ([`crate::core`]), and the settle step that gives it a scope.
//!
//! A class's body is read by its shape, not expanded as code: `x: T = v`
//! and `@var` declare fields, `name(params): T { body }` a method, `get` and
//! `set` a property's accessors and `constructor(...)` a constructor, each
//! maybe decorated with a visibility or `static`. Header parameters, as in
//! `class C(public x: String)`, are fields set by the implied constructor.
//!
//! The class's scope holds its members, after a synthetic `this`, then
//! the members it inherits; it's complete once made, or with supertypes,
//! once theirs are ([`Settle::inherit`]), so members may use each other in
//! any order. A bare member name inside the class means that member of
//! `this`. Checks against the supertypes, and of members' visibility,
//! follow Kotlin Temper's; private members are usable in their class, on
//! any instance, which the interpreter checks.
//!
//! An object literal, `{ a: 1, b }`, is a constructor call with named
//! arguments, `new void(\a 1, \b b)`, whose class is found by
//! [`Settle::object_literal`].

use crate::builtins::{Builtin, Function, Macro};
use crate::core::{
    CLASS_AUTO_EQ, CLASS_FUN, CLASS_INTERFACE, LET_INIT, LET_VAR, MEMBER_CONNECTED,
    MEMBER_OPERATOR_CMP, MEMBER_OPERATOR_EQ, MEMBER_PRIVATE, MEMBER_PROTECTED, MEMBER_PUBLIC,
    MEMBER_STATIC, ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK, ROLE_METHOD, ROLE_PARAM, ROLE_SETTER,
    TYPE_PARAMS_SHIFT, class_parts, fn_parts, let_parts, member_fn, type_params,
};
use crate::library::{Level, Message};
use crate::scope::{ScopeId, ScopeKind, SymbolId, SymbolKind};
use crate::settle::{Fact, Settle};
use temper_syntax::ast::{Builtin as SyntaxBuiltin, Value};
use temper_syntax::names::NameId;
use temper_syntax::tree::{NodeId, NodeKind, Span};

impl Settle {
    /// `class C(params) extends S { members }` is `class \word ^C params...
    /// \super S fn { members }`; `interface` is the same without params.
    pub(crate) fn expand_class(&mut self, call: NodeId, args: &[NodeId], m: Macro) -> bool {
        let [word, name, rest @ ..] = args else {
            return false;
        };
        if self.symbol_text(*word) != Some("word")
            || self.lib.tree.kind(*name) != NodeKind::LeftName
        {
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
                    let Some(param) = self.lower_type_param(header[i + 1]) else {
                        return false;
                    };
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
                    let span = self.lib.tree.span(params[0]);
                    self.lib.report(
                        Level::Error,
                        Message::ConstructorArgumentInInterfaceType,
                        span,
                    );
                    params.clear();
                }
                let first_member = children.len();
                for param in params {
                    children.extend(self.lower_member(param, 0, true));
                }
                let statements = self.lib.tree.children(block).to_vec();
                for statement in statements {
                    children.extend(self.lower_member(statement, 0, false));
                }
                self.check_members(*name, &children[first_member..], interface);
            }
            None => {
                let Some(out_type) = out_type else {
                    return false;
                };
                let span = self.lib.tree.span(call);
                let apply = self.lib.names.intern("apply");
                let apply = self.lib.add_name(NodeKind::LeftName, apply, span, call);
                let sig = crate::macros::Signature {
                    type_params: Vec::new(),
                    params,
                    out_type: Some(out_type),
                    supers: Vec::new(),
                    body: None,
                };
                let Some(f) = self.make_bodiless_fn(&sig, span, call) else {
                    self.error(call, Message::MalformedFunction);
                    return true;
                };
                let (parts, data) = Self::let_layout(apply, None, Some(f), ROLE_METHOD);
                children.push(self.lib.add(NodeKind::Let, span, data, &parts, call));
            }
        }
        let data = supers.len() as u32
            | ((type_params.len() as u32) << TYPE_PARAMS_SHIFT)
            | if interface { CLASS_INTERFACE } else { 0 }
            | if self.auto_eq.remove(&call) {
                CLASS_AUTO_EQ
            } else {
                0
            }
            | if self.fun_interfaces.remove(&call) && interface {
                CLASS_FUN
            } else {
                0
            };
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
                            let is_assign = self
                                .lib
                                .name(op)
                                .is_some_and(|n| self.lib.names.get(n) == "=");
                            if let Some((name, ty)) = self.typed_name(target).filter(|_| is_assign)
                            {
                                let left = self.lib.add_name(
                                    NodeKind::LeftName,
                                    name,
                                    self.lib.tree.span(target),
                                    target,
                                );
                                let span = self.lib.tree.span(node);
                                let (children, data) =
                                    Self::let_layout(left, ty, Some(value), flags);
                                return Some(self.lib.add(
                                    NodeKind::Let,
                                    span,
                                    data,
                                    &children,
                                    node,
                                ));
                            }
                        }
                    }
                    _ => {}
                }
                let callee_text = match self.lib.tree.kind(callee) {
                    NodeKind::RightName => self
                        .lib
                        .name(callee)
                        .map(|n| self.lib.names.get(n).to_string()),
                    _ => None,
                };
                match (callee_text.as_deref(), args) {
                    (Some("@"), [decorator, target]) => {
                        // `@connected` and `@connected("key")`.
                        let name = match self.lib.tree.kind(*decorator) {
                            NodeKind::Call => self.lib.tree.children(*decorator).first().copied(),
                            _ => Some(*decorator),
                        };
                        let text = name
                            .and_then(|n| self.lib.name(n))
                            .map(|n| self.lib.names.get(n).to_string());
                        let more = match text.as_deref() {
                            Some("public") => MEMBER_PUBLIC,
                            Some("protected") => MEMBER_PROTECTED,
                            Some("private") => MEMBER_PRIVATE,
                            // A constructor input that isn't a property; as
                            // a private one here, readable only in the class.
                            Some("noProperty") if header => MEMBER_PRIVATE,
                            Some("var") => LET_VAR,
                            Some("static") => MEMBER_STATIC,
                            Some("connected") => MEMBER_CONNECTED,
                            Some("operator") => self.operator_flag(*decorator),
                            _ => 0,
                        };
                        return self.lower_member(*target, flags | more, header);
                    }
                    (Some(accessor @ ("get" | "set")), [word, name, rest @ ..])
                        if !header && self.symbol_text(*word) == Some("word") =>
                    {
                        let role = if accessor == "get" {
                            ROLE_GETTER
                        } else {
                            ROLE_SETTER
                        };
                        return self.lower_method(node, *name, rest, flags | role);
                    }
                    // `y: Int` without a value.
                    (Some(":"), [name, ty]) if self.lib.tree.kind(*name) == NodeKind::RightName => {
                        let text = self.lib.name(*name).unwrap();
                        let left = self.lib.add_name(
                            NodeKind::LeftName,
                            text,
                            self.lib.tree.span(*name),
                            *name,
                        );
                        let span = self.lib.tree.span(node);
                        let role = if header { ROLE_PARAM } else { 0 };
                        let (children, data) =
                            Self::let_layout(left, Some(*ty), None, flags | role);
                        return Some(self.lib.add(NodeKind::Let, span, data, &children, node));
                    }
                    (Some("let"), [word, name, rest @ ..])
                        if !header && self.symbol_text(*word) == Some("word") =>
                    {
                        return self.lower_method(node, *name, rest, flags | ROLE_METHOD);
                    }
                    (Some(text), _) if !header => {
                        let role = if text == "constructor" {
                            ROLE_CONSTRUCTOR
                        } else {
                            ROLE_METHOD
                        };
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
            self.error(
                node,
                Message::Unsupported("this kind of class member".into()),
            );
        }
        None
    }

    /// `x` or `x: T`, as the target of a field's initializer.
    fn typed_name(&self, target: NodeId) -> Option<(temper_syntax::names::NameId, Option<NodeId>)> {
        let tree = &self.lib.tree;
        match (tree.kind(target), tree.children(target)) {
            (NodeKind::RightName, _) => Some((self.lib.name(target)?, None)),
            (NodeKind::Call, &[colon, name, ty])
                if self
                    .lib
                    .name(colon)
                    .is_some_and(|n| self.lib.names.get(n) == ":")
                    && tree.kind(name) == NodeKind::RightName =>
            {
                Some((self.lib.name(name)?, Some(ty)))
            }
            _ => None,
        }
    }

    /// A method, accessor or constructor: a `Let` of its `Fn`, which has
    /// no body if it's abstract ([`FN_NO_BODY`]).
    fn lower_method(
        &mut self,
        node: NodeId,
        name: NodeId,
        parts: &[NodeId],
        flags: u32,
    ) -> Option<NodeId> {
        if self.lib.tree.kind(name) != NodeKind::LeftName {
            self.error(node, Message::MalformedFunction);
            return None;
        }
        let sig = self.signature(node, parts)?;
        let span = self.lib.tree.span(node);
        let f = match sig.body {
            Some(body) => {
                let Some(block) = self.block_of(body) else {
                    self.error(node, Message::MalformedFunction);
                    return None;
                };
                self.make_fn(&sig, block, span, node)
            }
            None => self.make_bodiless_fn(&sig, span, node),
        };
        let Some(f) = f else {
            self.error(node, Message::MalformedFunction);
            return None;
        };
        let (children, data) = Self::let_layout(name, None, Some(f), flags);
        Some(self.lib.add(NodeKind::Let, span, data, &children, node))
    }

    /// `extends A & B` lists `A` and `B`.
    fn flatten_supers(&mut self, node: NodeId, supers: &mut Vec<NodeId>) {
        let children = self.lib.tree.children(node).to_vec();
        if let [callee, op, a, b] = *children {
            let is_desugar =
                self.lib.value(callee) == Some(&Value::Builtin(SyntaxBuiltin::DesugarOperation));
            if is_desugar
                && self
                    .lib
                    .name(op)
                    .is_some_and(|n| self.lib.names.get(n) == "&")
            {
                self.flatten_supers(a, supers);
                self.flatten_supers(b, supers);
                return;
            }
        }
        supers.push(node);
    }

    /// What `@operator("==")` or `@operator("<=>")` marks a method as;
    /// other operators aren't defined in Temper yet.
    fn operator_flag(&self, decorator: NodeId) -> u32 {
        let tree = &self.lib.tree;
        let operator = match (tree.kind(decorator), tree.children(decorator)) {
            (NodeKind::Call, &[_, arg]) => self.lib.value(arg),
            _ => None,
        };
        match operator {
            Some(Value::String(s)) if &**s == "==" => MEMBER_OPERATOR_EQ,
            Some(Value::String(s)) if &**s == "<=>" => MEMBER_OPERATOR_CMP,
            _ => 0,
        }
    }

    /// Reports each type parameter of an interface's own methods but its
    /// static ones, at an empty position at its start, as Kotlin Temper's
    /// `checkAgainstVirtualGenericMethod` does: such a method may be
    /// overridden, and overrides can't be generic. Kotlin allows them in
    /// core, which tempest reads only for signatures.
    fn check_virtual_generics(&mut self, members: &[NodeId]) {
        let tree = &self.lib.tree;
        let params: Vec<NodeId> = (members.iter())
            .filter(|&&m| tree.data(m) & (MEMBER_STATIC | ROLE_MASK) == ROLE_METHOD)
            .filter_map(|&m| let_parts(tree, m).2)
            .flat_map(|f| type_params(tree, f).iter().copied())
            .collect();
        for param in params {
            let text = self.member_text(param).to_string();
            let start = self.lib.tree.span(let_parts(&self.lib.tree, param).0).start;
            let message = Message::TypeParameterInInterfaceMethod(text);
            (self.lib).report(Level::Error, message, Span { start, end: start });
        }
    }

    fn add_flags(&mut self, let_node: NodeId, flags: u32) -> NodeId {
        if flags != 0 {
            let data = self.lib.tree.data(let_node) | flags;
            let parts = self.lib.tree.children(let_node).to_vec();
            self.rewrite(let_node, NodeKind::Let, data, &parts);
        }
        let_node
    }

    /// Reports what's wrong with a class's or interface's own members:
    /// an interface's backed properties and generic methods, and a
    /// class's members without a visibility, listed as Kotlin's
    /// `checkForMissingVisibility` does, fields first, then methods, then
    /// static members.
    fn check_members(&mut self, name: NodeId, members: &[NodeId], interface: bool) {
        let tree = &self.lib.tree;
        let role = |m: NodeId| tree.data(m) & ROLE_MASK;
        let is_static = |m: NodeId| tree.data(m) & MEMBER_STATIC != 0;
        let is_field = |m: NodeId| matches!(role(m), 0 | ROLE_PARAM);
        if interface {
            let backed: Vec<NodeId> = members
                .iter()
                .copied()
                .filter(|&m| is_field(m) && !is_static(m) && let_parts(tree, m).2.is_some())
                .collect();
            for m in backed {
                let text = self.member_text(m).to_string();
                let span = self.lib.tree.span(m);
                self.lib
                    .report(Level::Error, Message::InterfaceBackedProperty(text), span);
            }
            self.check_virtual_generics(members);
            return;
        }
        let visibility = MEMBER_PUBLIC | MEMBER_PROTECTED | MEMBER_PRIVATE;
        let missing =
            |m: &&NodeId| tree.data(**m) & visibility == 0 && role(**m) != ROLE_CONSTRUCTOR;
        let missing: Vec<NodeId> = members.iter().filter(missing).copied().collect();
        let fields = missing.iter().filter(|&&m| is_field(m) && !is_static(m));
        let methods = missing.iter().filter(|&&m| !is_field(m) && !is_static(m));
        let statics = missing.iter().filter(|&&m| is_static(m));
        let ordered: Vec<NodeId> = fields.chain(methods).chain(statics).copied().collect();
        let Some(&first) = ordered.first() else {
            return;
        };
        let listed: Vec<String> = ordered
            .iter()
            .map(|&m| {
                let kind = match (is_static(m), role(m)) {
                    (true, _) => "static",
                    (_, ROLE_GETTER) => "get",
                    (_, ROLE_SETTER) => "set",
                    _ => "",
                };
                let args = if is_field(m) { "" } else { "(...)" };
                format!("{kind}.{}{args}", self.member_text(m))
            })
            .collect();
        let class = self.lib.names.get(self.lib.name(name).unwrap()).to_string();
        let span = self.lib.tree.span(first);
        let message = Message::MissingMemberVisibility(class, format!("[{}]", listed.join(", ")));
        self.lib.report(Level::Error, message, span);
    }

    /// A member's or class's name.
    pub(crate) fn member_text(&self, decl: NodeId) -> &str {
        let name = self.lib.tree.children(decl)[0];
        self.lib.names.get(self.lib.name(name).unwrap())
    }

    /// Makes a class's scope, with `this` and its members, and queues their
    /// parts. Supertypes resolve outside it, but inside its type
    /// parameters'. With supertypes, the scope is complete once it has
    /// what they give it ([`Settle::inherit`]).
    pub(crate) fn enter_class(&mut self, node: NodeId, scope: ScopeId) {
        self.declare(node, scope);
        let scope = self.enter_type_params(node, scope);
        let (_, supers, members) = class_parts(&self.lib.tree, node);
        let (supers, members) = (supers.to_vec(), members.to_vec());
        let inherits = !supers.is_empty();
        for s in supers {
            self.visit(s, scope);
        }
        let class_scope = self.scopes.add(scope, ScopeKind::Class, node, !inherits);
        let this = self.lib.names.temporary("this");
        self.scopes
            .declare(class_scope, this, SymbolKind::Synthetic, None, false);
        for member in members {
            self.declare(member, class_scope);
            let parts = self.lib.tree.children(member)[1..].to_vec();
            for part in parts {
                self.visit(part, class_scope);
            }
        }
        if inherits {
            self.queue_inherit(node, class_scope);
        } else {
            self.publish(Fact::ClassScope(node));
        }
    }

    /// Completes a class's scope with the members it inherits, once its
    /// supertypes' scopes are complete: as Kotlin Temper allows, a bare
    /// name in a class may mean an inherited member of `this`. Its own
    /// members come first, then its supertypes' breadth-first, as the
    /// interpreter finds them; private and static members and
    /// constructors aren't inherited. Then checks the class against its
    /// supertypes.
    pub(crate) fn inherit(
        &mut self,
        class: NodeId,
        scope: ScopeId,
        forced: bool,
    ) -> Result<(), Fact> {
        let outer = self.scopes.get(scope).parent.unwrap();
        let supers = class_parts(&self.lib.tree, class).1.to_vec();
        for sup in supers {
            let Some(name) = self.super_name(sup) else {
                continue;
            };
            self.resolve_name(name, outer, forced)?;
            let decl = self.scopes.resolved(name).and_then(|b| self.type_decl(b));
            if let Some(decl) = decl.filter(|_| !forced) {
                if !self.is_published(Fact::ClassScope(decl)) {
                    return Err(Fact::ClassScope(decl));
                }
            }
        }
        let ancestors = self.ancestors(class);
        // Each name and the class giving it, own members first.
        let mut given: Vec<(NameId, NodeId)> = (self.scopes.get(scope).names.iter())
            .map(|&(n, _)| (n, class))
            .collect();
        for &ancestor in &ancestors {
            self.read(ancestor);
            let members = class_parts(&self.lib.tree, ancestor).2.to_vec();
            for m in members {
                let data = self.lib.tree.data(m);
                if data & (MEMBER_PRIVATE | MEMBER_STATIC) != 0
                    || data & ROLE_MASK == ROLE_CONSTRUCTOR
                {
                    continue;
                }
                let name = self.lib.name(self.lib.tree.children(m)[0]).unwrap();
                match given.iter().find(|&&(n, _)| n == name) {
                    Some(&(_, from)) if from != ancestor => continue,
                    Some(_) => {}
                    None => given.push((name, ancestor)),
                }
                let mutable = data & LET_VAR != 0;
                self.scopes
                    .declare(scope, name, SymbolKind::Member, Some(m), mutable);
            }
        }
        self.scopes.complete(scope);
        self.publish(Fact::ScopeComplete(scope));
        self.publish(Fact::ClassScope(class));
        self.check_supertypes(class, &ancestors);
        Ok(())
    }

    /// The name a supertype's expression names: `I` in `I` or `I<T>`.
    fn super_name(&self, sup: NodeId) -> Option<NodeId> {
        let tree = &self.lib.tree;
        let name = match tree.kind(sup) {
            NodeKind::Call => {
                let &[callee, generic, ..] = tree.children(sup) else {
                    return None;
                };
                // Before or after its visit expands it.
                let is_angle = self.lib.value(callee)
                    == Some(&Value::Builtin(SyntaxBuiltin::Angle))
                    || self.scopes.resolved_builtin(callee)
                        == Some(Builtin::Function(Function::Angle));
                is_angle.then_some(generic)?
            }
            _ => sup,
        };
        (tree.kind(name) == NodeKind::RightName).then_some(name)
    }

    /// A class's supertypes that are classes or interfaces in this
    /// library, as resolved so far.
    pub(crate) fn super_decls(&self, class: NodeId) -> Vec<NodeId> {
        class_parts(&self.lib.tree, class)
            .1
            .iter()
            .filter_map(|&sup| self.super_name(sup))
            .filter_map(|name| self.scopes.resolved(name))
            .filter_map(|b| self.type_decl(b))
            .collect()
    }

    /// A class's supertypes, theirs and so on, breadth-first, each once,
    /// and never the class itself.
    pub(crate) fn ancestors(&self, class: NodeId) -> Vec<NodeId> {
        let mut found = vec![class];
        let mut next = 0;
        while next < found.len() {
            for sup in self.super_decls(found[next]) {
                if !found.contains(&sup) {
                    found.push(sup);
                }
            }
            next += 1;
        }
        found.remove(0);
        found
    }

    /// Checks a class against its supertypes, as Kotlin's
    /// `TypeDeclChecker` and `TypeDefinitionMacro` do: it extends only
    /// interfaces, implements what they leave abstract unless it's an
    /// interface itself, and doesn't narrow what it overrides.
    fn check_supertypes(&mut self, class: NodeId, ancestors: &[NodeId]) {
        let mut problems = Vec::new();
        let tree = &self.lib.tree;
        let is_interface = |c: NodeId| tree.data(c) & CLASS_INTERFACE != 0;
        let (name, supers, members) = class_parts(tree, class);
        for &sup in supers {
            let decl = self.super_name(sup).and_then(|n| self.scopes.resolved(n));
            if let Some(decl) = decl.and_then(|b| self.type_decl(b)) {
                if !is_interface(decl) {
                    let text = self.member_text(decl).to_string();
                    problems.push((Message::CannotExtendConcrete(text), tree.span(sup)));
                }
            }
        }
        // Kotlin reports a class at its name through its supertypes.
        let end = supers.last().map_or(tree.span(name), |&s| tree.span(s)).end;
        let header = Span {
            start: tree.span(name).start,
            end,
        };
        let class_text = self.member_text(class);
        // Methods, getters and setters, by name and role.
        let role = |m: NodeId| tree.data(m) & ROLE_MASK;
        let is_method = |m: NodeId| {
            matches!(role(m), ROLE_METHOD | ROLE_GETTER | ROLE_SETTER)
                && tree.data(m) & MEMBER_STATIC == 0
        };
        let key = |m: NodeId| (self.lib.name(tree.children(m)[0]), role(m));
        let is_abstract =
            |m: NodeId| member_fn(tree, m).is_none() && tree.data(m) & MEMBER_CONNECTED == 0;
        let own: Vec<NodeId> = members.iter().copied().filter(|&m| is_method(m)).collect();
        // As in Kotlin, an abstract method is missing if nothing in the
        // hierarchy implements it.
        let inherited = ancestors
            .iter()
            .flat_map(|&c| class_parts(tree, c).2.iter().copied())
            .filter(|&m| tree.data(m) & MEMBER_PRIVATE == 0);
        let mut seen = Vec::new();
        for m in members.iter().copied().chain(inherited) {
            let data = tree.data(m);
            if is_method(m) && !is_abstract(m) {
                seen.push(key(m));
            } else if matches!(role(m), 0 | ROLE_PARAM) && data & MEMBER_STATIC == 0 {
                // A field is a getter, and if it's a `var`, a setter.
                let name = self.lib.name(tree.children(m)[0]);
                seen.push((name, ROLE_GETTER));
                if data & LET_VAR != 0 {
                    seen.push((name, ROLE_SETTER));
                }
            }
        }
        for &ancestor in ancestors {
            let from = self.member_text(ancestor);
            for &m in class_parts(tree, ancestor).2 {
                // A supertype's private members aren't inherited.
                if !is_method(m) || tree.data(m) & MEMBER_PRIVATE != 0 {
                    continue;
                }
                for &o in own.iter().filter(|&&o| key(o) == key(m)) {
                    if visibility_rank(tree.data(o)) < visibility_rank(tree.data(m)) {
                        let visibility = match visibility_rank(tree.data(o)) {
                            0 => "private",
                            _ => "protected",
                        };
                        let message = Message::IncompatibleVisibility(
                            class_text.to_string(),
                            self.method_description(o),
                            visibility.into(),
                            from.to_string(),
                        );
                        problems.push((message, tree.span(o)));
                    }
                }
                if !is_abstract(m) || is_interface(class) || seen.contains(&key(m)) {
                    continue;
                }
                seen.push(key(m));
                let message = Message::MissingMethodDefinition(
                    class_text.to_string(),
                    self.method_description(m),
                    from.to_string(),
                    format!("public {}", self.declaration_text(m)),
                );
                problems.push((message, header));
            }
        }
        for (message, span) in problems {
            self.lib.report(Level::Error, message, span);
        }
    }

    /// `f`, `get f` or `set f`, as Kotlin describes a method.
    pub(crate) fn method_description(&self, m: NodeId) -> String {
        let name = self.member_text(m);
        match self.lib.tree.data(m) & ROLE_MASK {
            ROLE_GETTER => format!("get {name}"),
            ROLE_SETTER => format!("set {name}"),
            _ => name.to_string(),
        }
    }

    /// An abstract method's declaration as written, from its name on,
    /// with `get` or `set` before it, for a class to copy. Kotlin writes it
    /// from its signature, with types as it resolved them (`Int32` for
    /// `Int`).
    fn declaration_text(&self, m: NodeId) -> String {
        let tree = &self.lib.tree;
        let start = tree.span(tree.children(m)[0]).start;
        let out_type = let_parts(tree, m).2.and_then(|f| fn_parts(tree, f).1);
        let end = out_type.map_or(tree.span(m).end, |t| tree.span(t).end);
        let text = self.lib.source_text(Span { start, end });
        let text = text.trim_end().trim_end_matches(';').trim_end();
        match tree.data(m) & ROLE_MASK {
            ROLE_GETTER => format!("get {text}"),
            ROLE_SETTER => format!("set {text}"),
            _ => text.to_string(),
        }
    }

    /// Whether a bare member name may be assigned from `scope`: a `var`
    /// field, a property with a setter, or a field without an initializer
    /// in its class's constructor.
    pub(crate) fn member_assignable(&self, member: SymbolId, scope: ScopeId) -> bool {
        let symbol = *self.scopes.symbol(member);
        let tree = &self.lib.tree;
        let same_name = self
            .scopes
            .get(symbol.owner)
            .names
            .iter()
            .filter(|(n, _)| *n == symbol.name);
        let decls: Vec<NodeId> = same_name
            .filter_map(|&(_, b)| self.scopes.symbol(b).decl)
            .collect();
        if decls
            .iter()
            .any(|&d| tree.data(d) & ROLE_MASK == ROLE_SETTER)
        {
            return true;
        }
        let Some(decl) = symbol.decl else {
            return false;
        };
        let data = tree.data(decl);
        if data & LET_VAR != 0 {
            return true;
        }
        // An inherited field is abstract.
        let own = self.scopes.resolved(tree.children(decl)[0]) == Some(member);
        if data & ROLE_MASK != 0 || data & LET_INIT != 0 || !own {
            return false;
        }
        // The nearest function must be the constructor's, directly in the class.
        let Some(&(_, f)) = self.enclosing(scope).last() else {
            return false;
        };
        let in_class = self
            .scopes
            .of_node(f)
            .and_then(|s| self.scopes.get(s).parent)
            == Some(symbol.owner);
        let class = self.scopes.get(symbol.owner).node;
        in_class
            && crate::core::class_parts(tree, class).2.iter().any(|&m| {
                tree.data(m) & ROLE_MASK == ROLE_CONSTRUCTOR
                    && crate::core::let_parts(tree, m).2 == Some(f)
            })
    }

    /// The nearest class's scope around `scope`.
    fn class_scope(&self, scope: ScopeId) -> Option<ScopeId> {
        let mut at = Some(scope);
        while let Some(id) = at {
            let s = self.scopes.get(id);
            if s.kind == ScopeKind::Class {
                return Some(id);
            }
            at = s.parent;
        }
        None
    }

    /// `this` is the enclosing class's synthetic `this`.
    pub(crate) fn expand_this(&mut self, call: NodeId, scope: ScopeId) {
        match self.class_scope(scope) {
            Some(class_scope) => {
                let symbol = self.scopes.get(class_scope).names[0].1;
                let span = self.lib.tree.span(call);
                let name = self.builtin_name(symbol, span, call);
                self.become_node(call, name);
            }
            None => self.error(call, Message::ThisOutsideClassBody),
        }
    }

    /// The member `this.name` means, for `this.name = value`: the
    /// enclosing class's own or inherited one, if it has one.
    pub(crate) fn this_member(
        &self,
        target: NodeId,
        scope: ScopeId,
        forced: bool,
    ) -> Result<Option<SymbolId>, Fact> {
        let tree = &self.lib.tree;
        let (&[dot, object, key], Some(class_scope)) =
            (tree.children(target), self.class_scope(scope))
        else {
            return Ok(None);
        };
        let is_dot = self
            .lib
            .name(dot)
            .is_some_and(|n| self.lib.names.get(n) == ".");
        let this = self.scopes.get(class_scope).names[0].1;
        let is_this = match tree.kind(object) {
            NodeKind::Call => tree
                .children(object)
                .first()
                .is_some_and(|&c| self.lib.value(c) == Some(&Value::Builtin(SyntaxBuiltin::This))),
            _ => self.scopes.resolved(object) == Some(this),
        };
        let Some(name) = self.lib.symbol(key).filter(|_| is_dot && is_this) else {
            return Ok(None);
        };
        let class = self.scopes.get(class_scope);
        if !class.complete && !forced {
            return Err(Fact::ScopeComplete(class_scope));
        }
        Ok(class
            .names
            .iter()
            .rev()
            .find(|&&(n, _)| n == name)
            .map(|&(_, b)| b))
    }

    /// Reports assigning a member that isn't assignable there, as Kotlin's
    /// typer does.
    pub(crate) fn report_member_assignment(&mut self, member: SymbolId, target: NodeId) {
        let symbol = *self.scopes.symbol(member);
        // The class declaring it, maybe a supertype.
        let declared = symbol
            .decl
            .and_then(|d| self.scopes.resolved(self.lib.tree.children(d)[0]))
            .map_or(symbol.owner, |b| self.scopes.symbol(b).owner);
        let class = self.member_text(self.scopes.get(declared).node).to_string();
        let name = self.lib.names.get(symbol.name).to_string();
        let span = self.lib.tree.span(target);
        self.lib
            .report(Level::Error, Message::IncompatibleUsage(name, class), span);
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
    pub(crate) fn object_literal(
        &mut self,
        call: NodeId,
        scope: ScopeId,
        forced: bool,
    ) -> Result<bool, Fact> {
        let children = self.lib.tree.children(call).to_vec();
        let Some(&target) = children.get(1) else {
            return Ok(false);
        };
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
            let value = if is_pun {
                self.lib
                    .add_name(NodeKind::RightName, name, self.lib.tree.span(value), key)
            } else {
                value
            };
            props.push((name, value));
        }
        let (class_name, class) = if is_void {
            let names: Vec<NameId> = props.iter().map(|&(n, _)| n).collect();
            match self.matching_class(&names, scope, forced)? {
                Ok((symbol, class)) => (
                    self.builtin_name(symbol, self.lib.tree.span(target), target),
                    class,
                ),
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
            let at = params
                .iter()
                .position(|&p| self.lib.name(self.lib.tree.children(p)[0]) == Some(name));
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
            if in_order
                || matches!(
                    self.lib.tree.kind(value),
                    NodeKind::Value | NodeKind::Fn | NodeKind::Fun
                )
            {
                values.push(value);
            } else {
                let t = self.lib.names.temporary("arg");
                let left =
                    self.lib
                        .add_name(NodeKind::LeftName, t, self.lib.tree.span(value), value);
                statements.push(self.make_let(
                    left,
                    None,
                    Some(value),
                    0,
                    self.lib.tree.span(value),
                    value,
                ));
                values.push(self.lib.add_name(
                    NodeKind::RightName,
                    t,
                    self.lib.tree.span(value),
                    value,
                ));
            }
        }
        let last = placed
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |i| i + 1);
        let mut args = vec![children[0], class_name];
        for (slot, &param) in placed[..last].iter().zip(&params) {
            args.push(match *slot {
                Some(i) => values[i],
                None if crate::core::let_parts(&self.lib.tree, param).2.is_some() => {
                    self.lib.add_value(Value::Null, span, call)
                }
                None => {
                    let text = self
                        .lib
                        .names
                        .get(self.lib.name(self.lib.tree.children(param)[0]).unwrap())
                        .to_string();
                    self.error(call, Message::NoValuePassed(text));
                    return Ok(true);
                }
            });
        }
        if let Some(missing) = params[last..]
            .iter()
            .find(|&&p| crate::core::let_parts(&self.lib.tree, p).2.is_none())
        {
            let text = self
                .lib
                .names
                .get(self.lib.name(self.lib.tree.children(*missing)[0]).unwrap())
                .to_string();
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

    /// `new I()` can't make an instance of an interface: its name becomes
    /// an error.
    pub(crate) fn check_new(
        &mut self,
        call: NodeId,
        scope: ScopeId,
        forced: bool,
    ) -> Result<(), Fact> {
        let Some(&target) = self.lib.tree.children(call).get(1) else {
            return Ok(());
        };
        self.resolve_name(target, scope, forced)?;
        let decl = self.scopes.resolved(target).and_then(|b| self.type_decl(b));
        if let Some(decl) = decl.filter(|&d| self.lib.tree.data(d) & CLASS_INTERFACE != 0) {
            let name = self.member_text(decl).to_string();
            self.error(target, Message::CannotInstantiateAbstractType(name));
        }
        Ok(())
    }

    /// The one class visible from `scope` whose constructor takes every
    /// name, with the nearest symbol for it, or the error to report.
    fn matching_class(
        &mut self,
        names: &[NameId],
        scope: ScopeId,
        forced: bool,
    ) -> Result<Result<(SymbolId, NodeId), Message>, Fact> {
        let mut found: Vec<(SymbolId, NodeId)> = Vec::new();
        let mut at = Some(scope);
        while let Some(id) = at.filter(|&id| id != crate::scope::BUILTINS) {
            let s = self.scopes.get(id);
            if !s.complete && !forced {
                return Err(Fact::ScopeComplete(id));
            }
            for &(_, symbol) in &s.names {
                let Some(class) = self.class_of(symbol) else {
                    continue;
                };
                let params = crate::core::constructor_params(&self.lib.tree, class);
                let takes = |n: &NameId| {
                    params
                        .iter()
                        .any(|&p| self.lib.name(self.lib.tree.children(p)[0]) == Some(*n))
                };
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
                let mut texts: Vec<&str> = found
                    .iter()
                    .map(|&(b, _)| self.lib.names.get(self.scopes.symbol(b).name))
                    .collect();
                texts.sort();
                Err(Message::MultipleConstructorsMatch(texts.join(", ")))
            }
        })
    }

    /// The concrete class a symbol names.
    fn class_of(&self, symbol: SymbolId) -> Option<NodeId> {
        self.type_decl(symbol)
            .filter(|&c| self.lib.tree.data(c) & CLASS_INTERFACE == 0)
    }

    /// The class or interface a symbol names: one declared there, or one
    /// an import names, through its `let C = C'`.
    pub(crate) fn type_decl(&self, symbol: SymbolId) -> Option<NodeId> {
        let tree = &self.lib.tree;
        let mut symbol = symbol;
        for _ in 0..8 {
            let decl = self.scopes.symbol(symbol).decl?;
            match tree.kind(decl) {
                NodeKind::Class => return Some(decl),
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

/// How widely a member is visible: private, protected, then public, the
/// default.
pub(crate) fn visibility_rank(data: u32) -> u8 {
    if data & MEMBER_PRIVATE != 0 {
        0
    } else if data & MEMBER_PROTECTED != 0 {
        1
    } else {
        2
    }
}
