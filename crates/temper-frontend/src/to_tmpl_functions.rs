//! Function values in [`to_tmpl`](super): function literals with their
//! captures, references to declared functions and static methods, calls
//! of function values, and core's `forEach`, which Kotlin Temper inlines
//! (its parameter is `@inlineUnrealizedGoal`), as `for … of` loops are
//! (notes/tmpl/nodes.md, "Function values").

use super::{Translator, Tx};
use crate::core::{self, LET_VAR};
use crate::typing::Resolved;
use temper_syntax::tree::{NodeId as FNode, NodeKind};
use temper_tmpl::{DeclId, DeclKind, Facts, NodeId, TypeId};

impl Translator<'_> {
    /// A function literal, `fn (x) { … }` or a trailing block, typed as
    /// typing gives it, a `@fun` interface or a function type, with the
    /// locals around it that it uses as its captures.
    pub(super) fn literal(&mut self, f: FNode) -> Tx<NodeId> {
        let ty = self.expr_ty(f)?;
        let Some(sig) = self.b.fun_signature(ty) else {
            return self.not_yet(f, "a function literal of another type");
        };
        let optional =
            (self.b.program().types.signature(sig)).is_some_and(|s| s.optional.contains(&true));
        if optional {
            return self.not_yet(f, "a parameter's default");
        }
        if !core::fn_supers(&self.s.library.tree, f).is_empty() {
            return self.not_yet(f, "a generator function");
        }
        let decl = self
            .b
            .declare_made(DeclKind::Lambda, &["fn"], self.owner, sig);
        let span = self.span(f);
        self.b.set_decl_spans(decl, span, span);
        let (params, body) = self.function_parts(f, decl)?;
        let made = self.b.function_literal(ty, decl, &params, body);
        let captures = temper_tmpl::analysis::captures(self.b.program(), made);
        self.b.set_captures(decl, &captures);
        Ok(self.own(made, f))
    }

    /// A declared function or static method as a value, typed as its place
    /// wants, as typing gives it.
    pub(super) fn function_ref(&mut self, decl: DeclId, node: FNode) -> Tx<NodeId> {
        if !self.b.program().decls.type_params(decl).is_empty() {
            return self.not_yet(node, "a generic function as a value");
        }
        let ty = self.expr_ty(node)?;
        let sig = self.b.program().decls.get(decl).ty;
        if self.b.fun_signature(ty) != Some(sig) {
            return self.not_yet(node, "a function as a value of another type");
        }
        let made = self.b.function_ref(ty, decl);
        Ok(self.own(made, node))
    }

    /// A call of a function value: a local, parameter, field, or call
    /// result whose type is a `@fun` interface or a function type.
    pub(super) fn call_value(&mut self, node: FNode, callee: FNode, args: &[FNode]) -> Tx<NodeId> {
        let args = self.plain_args(node, args)?;
        let f = self.expr(callee)?;
        self.call_value_of(node, f, &args)
    }

    /// A call of `f`, a function value already translated, with `args`.
    pub(super) fn call_value_of(&mut self, node: FNode, f: NodeId, args: &[FNode]) -> Tx<NodeId> {
        let fty = self.b.program().nodes.ty(f);
        let Some(sig) = self.b.fun_signature(fty) else {
            return self.not_yet(node, "a call of a value of another type");
        };
        let params =
            (self.b.program().types.signature(sig)).map_or_else(Vec::new, |s| s.params.to_vec());
        if args.len() != params.len() {
            return self.not_yet(node, "an argument left out");
        }
        let mut values = Vec::new();
        for (&arg, &ty) in args.iter().zip(&params) {
            let value = self.expr(arg)?;
            values.push(self.fit(value, ty));
        }
        let made = self.b.call_value(f, &values);
        Ok(self.own(made, node))
    }

    /// `xs.forEach(f)`'s list and function, if `call` is a call of core's
    /// `List.forEach`.
    pub(super) fn for_each_parts(&self, call: FNode) -> Option<(FNode, FNode)> {
        if self.kind(call) != NodeKind::Call {
            return None;
        }
        let Some(Resolved::Builtin(key)) = self.s.typing.resolved(call) else {
            return None;
        };
        if key != "List.forEach" {
            return None;
        }
        let children = self.children(call);
        let object = self.dot_object(children[0])?;
        match children[1..] {
            [f] if self.s.library.symbol(f).is_none() => Some((object, f)),
            _ => None,
        }
    }

    /// `xs.forEach(f)`, inlined as core's own body, as Kotlin Temper does:
    /// `let n = xs.length; var i = 0; while (i < n) { let x = xs[i]; i +=
    /// 1; … }`, where a literal's parameter is `x` and its body follows,
    /// in the function around it (so `return` returns from that, as
    /// Kotlin's does), and another value is called with `x`. `xs` is
    /// held in a local first unless it's a `let`'s read. A `for … of`
    /// loop (`each`) gives its label, which its `break` and `continue`
    /// name.
    pub(super) fn for_each(
        &mut self,
        call: FNode,
        object: FNode,
        f: FNode,
        each: Option<FNode>,
    ) -> Tx<NodeId> {
        let at = each.unwrap_or(call);
        let list = self.expr(object)?;
        let list_ty = self.b.program().nodes.ty(list);
        let (Some(length), Some(get)) = (self.core_decl("List.length"), self.core_decl("List.get"))
        else {
            return self.not_yet(call, "core's List.forEach");
        };
        let element = self.b.program().types.args(list_ty).first().copied();
        let Some(element) = element else {
            return self.not_yet(call, "core's List.forEach on another type");
        };
        let mut stmts = Vec::new();
        let plain_read = match self.b.program().nodes.get(list) {
            temper_tmpl::Node::Local { local } => {
                !self.b.program().decls.get(local).facts.contains(Facts::VAR)
            }
            temper_tmpl::Node::GetModuleLet { target } => !self
                .b
                .program()
                .decls
                .get(target)
                .facts
                .contains(Facts::VAR),
            _ => false,
        };
        let list_read = |t: &mut Self, held: Option<DeclId>| -> NodeId {
            match held {
                Some(d) => {
                    let read = t.b.local(d);
                    t.made(read, object)
                }
                None => t.expr(object).expect("translated once"),
            }
        };
        let held = if plain_read {
            None
        } else {
            let d = self
                .b
                .declare_made(DeclKind::Local, &["list"], self.owner, list_ty);
            let span = self.span(object);
            self.b.set_decl_spans(d, span, span);
            let init = self.b.let_stmt(d, Some(list));
            stmts.push(self.made(init, object));
            Some(d)
        };
        let n = self
            .b
            .declare_made(DeclKind::Local, &["length"], self.owner, TypeId::INT32);
        let i = self
            .b
            .declare_made(DeclKind::Local, &["i"], self.owner, TypeId::INT32);
        self.b.add_facts(i, Facts::VAR);
        let span = self.span(at);
        for d in [n, i] {
            self.b.set_decl_spans(d, span, span);
        }
        let receiver = if held.is_some() {
            list_read(self, held)
        } else {
            list
        };
        let len = self.b.call_method(length, receiver, &[]);
        let len = self.made(len, at);
        let init = self.b.let_stmt(n, Some(len));
        stmts.push(self.made(init, at));
        let zero = self.b.int32(0);
        let zero = self.made(zero, at);
        let init = self.b.let_stmt(i, Some(zero));
        stmts.push(self.made(init, at));
        let label = self
            .b
            .declare_made(DeclKind::Label, &["loop"], self.owner, TypeId::NONE);
        self.b.set_decl_spans(label, span, span);
        if let Some(each) = each {
            self.loops.insert(each, label);
        }
        let (ri, rn) = (self.b.local(i), self.b.local(n));
        let (ri, rn) = (self.made(ri, at), self.made(rn, at));
        let lt = temper_tmpl::ops::Op::named("Int32.lt").expect("an operator");
        let cond = self.b.op(lt, &[ri, rn]);
        let cond = self.made(cond, at);
        let mut body = Vec::new();
        // `let x = xs[i];`, `x` being the literal's parameter if it is one.
        let literal = (self.kind(f) == NodeKind::Fn).then_some(f);
        let params = literal.map(|f| core::fn_parts(&self.s.library.tree, f).0);
        let param = match params.as_deref() {
            Some(&[p]) if self.kind(p) == NodeKind::Let => Some(p),
            Some(&[]) | None => None,
            Some(_) => return self.not_yet(f, "a forEach block's parameters"),
        };
        let x = match param {
            Some(p) => {
                let name = self
                    .name_of(core::let_parts(&self.s.library.tree, p).0)
                    .to_string();
                let d = self.b.declare(DeclKind::Local, &name, self.owner, element);
                if self.data(p) & LET_VAR != 0 {
                    self.b.add_facts(d, Facts::VAR);
                }
                self.decl_spans(d, p);
                self.decls.insert(p, d);
                d
            }
            None => {
                let d = self
                    .b
                    .declare_made(DeclKind::Local, &["item"], self.owner, element);
                self.b.set_decl_spans(d, span, span);
                d
            }
        };
        let receiver = list_read(self, held);
        let ri = self.b.local(i);
        let ri = self.made(ri, at);
        let item = self.b.call_method(get, receiver, &[ri]);
        let item = self.made(item, at);
        let init = self.b.let_stmt(x, Some(item));
        body.push(self.made(init, at));
        let (ri, one) = (self.b.local(i), self.b.int32(1));
        let (ri, one) = (self.made(ri, at), self.made(one, at));
        let add = temper_tmpl::ops::Op::named("Int32.add").expect("an operator");
        let next = self.b.op(add, &[ri, one]);
        let next = self.made(next, at);
        let step = self.b.assign(i, next);
        body.push(self.made(step, at));
        match literal {
            Some(f) => {
                let block = core::fn_parts(&self.s.library.tree, f).2;
                let Some(block) = block else {
                    return self.not_yet(f, "a forEach block without a body");
                };
                self.stmts(block, &mut body)?;
            }
            None => {
                let value = self.expr(f)?;
                let read = self.b.local(x);
                let read = self.made(read, f);
                let fty = self.b.program().nodes.ty(value);
                let param_ty = self
                    .b
                    .fun_signature(fty)
                    .and_then(|s| {
                        self.b
                            .program()
                            .types
                            .signature(s)
                            .map(|s| s.params.to_vec())
                    })
                    .and_then(|p| p.first().copied())
                    .unwrap_or(element);
                let read = self.fit(read, param_ty);
                let called = self.b.call_value(value, &[read]);
                let called = self.made(called, f);
                let stmt = self.b.expr_statement(called);
                body.push(self.made(stmt, f));
            }
        }
        let body = self.b.block(&body);
        let body = self.made(body, at);
        let looped = self.b.while_loop(label, cond, body);
        stmts.push(self.own(looped, at));
        let block = self.b.block(&stmts);
        Ok(self.made(block, at))
    }
}
