//! The builtin macros: each rewrites its own call node into core constructs
//! ([`crate::core`]), needing only syntax, or into an error node. What they
//! produce is visited afterwards, so nothing under a macro call is resolved
//! before it expands.
//!
//! Call joins arrive rewritten for a library (`temper_syntax::module`):
//! `if (a) { b } else { c }` is `if(a, fn { b }, \else, fn (f#0) { f#0(fn
//! { c }) })`. Rather than calling the delegate, as Kotlin does, `if` reads
//! the arguments it passes on.

use crate::builtins::{Builtin, Function, Macro};
use crate::core::{
    FN_LOOP_BODY, FN_OUT_TYPE, LET_INIT, LET_TYPE, LET_VAR, LOOP_EACH, LOOP_NEXT, ROLE_TYPE_PARAM, TYPE_PARAMS_SHIFT,
};
use crate::library::Message;
use crate::scope::{ScopeId, ScopeKind};
use crate::settle::Settle;
use temper_syntax::ast::{Builtin as SyntaxBuiltin, Value};
use temper_syntax::tree::{NodeId, NodeKind, Span};
use temper_syntax::names::NameId;

/// A function's parts after its name, from [`Settle::signature`].
pub(crate) struct Signature {
    /// Each a `Let`.
    pub type_params: Vec<NodeId>,
    pub params: Vec<NodeId>,
    pub out_type: Option<NodeId>,
    pub body: Option<NodeId>,
}

impl Settle {
    /// Expands a call to a builtin macro.
    pub(crate) fn expand(&mut self, call: NodeId, scope: ScopeId, m: Macro) {
        let args = self.lib.tree.children(call)[1..].to_vec();
        let ok = match m {
            Macro::Let => self.expand_let(call, &args),
            Macro::Decorate => self.expand_decorate(call, &args),
            Macro::Fn => self.expand_fn(call, &args),
            Macro::If => match self.if_parts(&args) {
                Some(parts) => {
                    self.rewrite(call, NodeKind::If, 0, &parts);
                    true
                }
                None => false,
            },
            Macro::While => self.expand_while(call, &args),
            Macro::For => self.expand_for(call, &args),
            Macro::Do => match *args {
                [body] => self.block_of(body).map(|b| self.become_node(call, b)).is_some(),
                _ => false,
            },
            Macro::Return => self.expand_return(call, scope, &args),
            Macro::Break => self.expand_jump(call, scope, &args, true),
            Macro::Continue => self.expand_jump(call, scope, &args, false),
            Macro::Bubble => {
                self.rewrite(call, NodeKind::Fail, 0, &[]);
                true
            }
            Macro::Assign => match *args {
                [target, value] if self.lib.tree.kind(target) == NodeKind::RightName => {
                    self.rewrite(call, NodeKind::Assign, 0, &[target, value]);
                    true
                }
                [target, value] if self.is_dot_call(target) => {
                    self.rewrite(call, NodeKind::Assign, 0, &[target, value]);
                    true
                }
                [target, value] if self.is_index_call(target) => {
                    // `ls[i] = v` is `ls.set(i, v)`.
                    let [index, ls, i] = *self.lib.tree.children(target) else { unreachable!() };
                    let span = self.lib.tree.span(call);
                    let dot = self.lib.names.intern(".");
                    let dot = self.lib.add_name(NodeKind::BuiltinName, dot, span, index);
                    let key = Value::Symbol(self.lib.names.intern("set"));
                    let key = self.lib.add_value(key, span, index);
                    let member = self.lib.add(NodeKind::Call, span, 0, &[dot, ls, key], target);
                    self.rewrite(call, NodeKind::Call, 0, &[member, i, value]);
                    true
                }
                [_, _] => {
                    self.error(call, Message::Unsupported("assigning to anything but a name, member or element".into()));
                    true
                }
                _ => false,
            },
            Macro::And | Macro::Or => match *args {
                [a, b] => {
                    let span = self.lib.tree.span(call);
                    let constant = self.lib.add_value(Value::Bool(m == Macro::Or), span, call);
                    let parts = if m == Macro::And { [a, b, constant] } else { [a, constant, b] };
                    self.rewrite(call, NodeKind::If, 0, &parts);
                    true
                }
                _ => false,
            },
            Macro::Class | Macro::Interface => self.expand_class(call, &args, m),
            Macro::When => self.expand_when(call, &args),
            Macro::Coalesce => match *args {
                [a, b] => {
                    let span = self.lib.tree.span(call);
                    let (mut statements, name) = self.subject(a, "subject", call);
                    let null_check = self.null_check(name, span, call);
                    let reread = self.lib.add_name(NodeKind::RightName, name, span, a);
                    statements.push(self.lib.add(NodeKind::If, span, 0, &[null_check, b, reread], call));
                    self.rewrite(call, NodeKind::Block, 0, &statements);
                    true
                }
                _ => false,
            },
            // Expanded by `expand_chain_null`, as its callee isn't a name.
            Macro::ChainNull => false,
            Macro::Orelse => match *args {
                [a, b] => {
                    self.rewrite(call, NodeKind::Orelse, 0, &[a, b]);
                    true
                }
                _ => false,
            },
        };
        if !ok && self.lib.tree.kind(call) == NodeKind::Call {
            self.error(call, Message::MalformedSpecial(Builtin::Macro(m).name().to_string()));
        }
    }

    /// Expands a call to a builtin the grammar step refers to directly.
    pub(crate) fn expand_syntax(&mut self, call: NodeId, builtin: SyntaxBuiltin) {
        let children = self.lib.tree.children(call).to_vec();
        let span = self.lib.tree.span(call);
        match (builtin, &children[1..]) {
            (SyntaxBuiltin::DesugarOperation, [op, operands @ ..]) => self.desugar_operation(call, *op, operands),
            (SyntaxBuiltin::StringExpr, [tag, tagged, parts @ ..]) => {
                let plain = self.lib.value(*tag) == Some(&Value::Null)
                    && self.lib.value(*tagged) == Some(&Value::Bool(false))
                    && parts.iter().all(|&p| self.lib.symbol(p).is_none());
                if plain {
                    let cat = self.scopes.hidden(Builtin::Function(Function::Cat));
                    let callee = self.builtin_name(cat, span, call);
                    let mut new = vec![callee];
                    new.extend_from_slice(parts);
                    self.rewrite(call, NodeKind::Call, 0, &new);
                } else if let Some(new) = self.tagged_string(call, *tag, *tagged, parts) {
                    self.rewrite(call, NodeKind::Call, 0, &new);
                } else {
                    self.error(call, Message::Unsupported("statements in strings".into()));
                }
            }
            // Type arguments, `T<A>`, and list literals, `[a, b]`.
            (SyntaxBuiltin::Angle, [_, ..]) | (SyntaxBuiltin::List, _) => {
                let function = if builtin == SyntaxBuiltin::Angle { Function::Angle } else { Function::List };
                let hidden = self.scopes.hidden(Builtin::Function(function));
                let callee = self.builtin_name(hidden, span, call);
                let mut new = vec![callee];
                new.extend_from_slice(&children[1..]);
                self.rewrite(call, NodeKind::Call, 0, &new);
            }
            (SyntaxBuiltin::EmbeddedComment, _) => {
                let void = self.lib.add_value(Value::Void, span, call);
                self.become_node(call, void);
            }
            (SyntaxBuiltin::DocComment, [target, ..]) => self.become_node(call, *target),
            _ => self.error(call, Message::Unsupported(format!("builtin {}", builtin.name()))),
        }
    }

    /// A tagged string, `tag"a${b}c"`, is a call of its tag with a list of
    /// its literal parts, escapes not decoded, and a list of what's
    /// interpolated: `tag(["a", "c"], [b])`, as Kotlin Temper's
    /// `StringExprMacro` does for a tag that's a function. There's one more
    /// literal part than interpolations, empty where two are adjacent.
    /// `None` for anything else, as statements in strings.
    fn tagged_string(&mut self, call: NodeId, tag: NodeId, tagged: NodeId, parts: &[NodeId]) -> Option<Vec<NodeId>> {
        if self.lib.value(tagged) != Some(&Value::Bool(true)) || self.lib.tree.kind(tag) != NodeKind::RightName {
            return None;
        }
        let span = self.lib.tree.span(call);
        let (mut literals, mut values) = (Vec::<String>::new(), Vec::new());
        let mut rest = parts;
        while let Some((&part, more)) = rest.split_first() {
            rest = more;
            match (self.symbol_text(part), self.lib.value(part)) {
                (Some("interpolate"), _) => {
                    let (&value, more) = rest.split_first()?;
                    rest = more;
                    if literals.len() <= values.len() {
                        literals.push(String::new());
                    }
                    values.push(value);
                }
                // Adjacent literal parts, as escapes broken out, are joined.
                (None, Some(Value::String(text))) if literals.len() > values.len() => {
                    literals.last_mut()?.push_str(text);
                }
                (None, Some(Value::String(text))) => literals.push(text.to_string()),
                _ => return None,
            }
        }
        if literals.len() <= values.len() {
            literals.push(String::new());
        }
        let list = self.scopes.hidden(Builtin::Function(Function::List));
        let mut literal_list = vec![self.builtin_name(list, span, call)];
        for text in literals {
            literal_list.push(self.lib.add_value(Value::String(text), span, call));
        }
        let mut value_list = vec![self.builtin_name(list, span, call)];
        value_list.extend(values);
        let literal_list = self.lib.add(NodeKind::Call, span, 0, &literal_list, call);
        let value_list = self.lib.add(NodeKind::Call, span, 0, &value_list, call);
        Some(vec![tag, literal_list, value_list])
    }

    /// `a + b` is `desugarOperation(+, a, b)`: a call of the operator's
    /// name, or for `+=`, `++` and the like, an assignment.
    fn desugar_operation(&mut self, call: NodeId, op: NodeId, operands: &[NodeId]) {
        let Some(name) = self.lib.name(op) else {
            return self.error(call, Message::MalformedSpecial("desugarOperation".into()));
        };
        let text = self.lib.names.get(name).to_string();
        let span = self.lib.tree.span(call);
        let simple = match (text.as_str(), operands) {
            ("++" | "_++", [_]) => Some("+"),
            ("--" | "_--", [_]) => Some("-"),
            (t, [_, _]) if t.len() >= 2 && t.ends_with('=') && !matches!(t, "==" | "!=" | "<=" | ">=") => {
                Some(&t[..t.len() - 1])
            }
            _ => None,
        };
        let Some(simple) = simple else {
            let mut children = vec![op];
            children.extend_from_slice(operands);
            return self.rewrite(call, NodeKind::Call, 0, &children);
        };
        let target = operands[0];
        if self.is_dot_call(target) {
            return self.desugar_member_operation(call, &text, simple, target, operands.get(1).copied());
        }
        let Some(target_name) = self.lib.name(target).filter(|_| self.lib.tree.kind(target) == NodeKind::RightName)
        else {
            return self.error(call, Message::Unsupported(format!("{text} on anything but a name")));
        };
        let Some(function) = self.lib.names.find(simple).and_then(|n| self.scopes.builtin(n)) else {
            return self.error(call, Message::Unsupported(format!("operator {text}")));
        };
        let operator = self.builtin_name(function, span, op);
        let reread = self.lib.add_name(NodeKind::RightName, target_name, self.lib.tree.span(target), target);
        let amount = match operands {
            [_, value] => *value,
            _ => self.lib.add_value(Value::Int32(1), span, call),
        };
        if text.starts_with('_') {
            // `x++`: `do { let t = x; x = t + 1; t }`.
            let t = self.lib.names.temporary("t");
            let t_left = self.lib.add_name(NodeKind::LeftName, t, span, call);
            let t_let = self.lib.add(NodeKind::Let, span, LET_INIT, &[t_left, reread], call);
            let t_use = self.lib.add_name(NodeKind::RightName, t, span, call);
            let sum = self.lib.add(NodeKind::Call, span, 0, &[operator, t_use, amount], call);
            let assign = self.lib.add(NodeKind::Assign, span, 0, &[target, sum], call);
            let t_result = self.lib.add_name(NodeKind::RightName, t, span, call);
            self.rewrite(call, NodeKind::Block, 0, &[t_let, assign, t_result]);
        } else {
            let sum = self.lib.add(NodeKind::Call, span, 0, &[operator, reread, amount], call);
            self.rewrite(call, NodeKind::Assign, 0, &[target, sum]);
        }
    }

    /// `obj.name += v`, `obj.name++` and the like, reading `obj` once:
    /// `do { let o = obj; o.name = o.name + v }`, or for `obj.name++`,
    /// `do { let o = obj; let t = o.name; o.name = t + 1; t }`.
    fn desugar_member_operation(&mut self, call: NodeId, text: &str, simple: &str, target: NodeId, value: Option<NodeId>) {
        let span = self.lib.tree.span(call);
        let Some(function) = self.lib.names.find(simple).and_then(|n| self.scopes.builtin(n)) else {
            return self.error(call, Message::Unsupported(format!("operator {text}")));
        };
        let [dot, object, key] = *self.lib.tree.children(target) else { unreachable!() };
        let (dot_name, key_value) = (self.lib.name(dot).unwrap(), self.lib.value(key).cloned());
        let Some(key_value) = key_value else {
            return self.error(call, Message::MalformedSpecial(".".into()));
        };
        let o = self.lib.names.temporary("o");
        let o_left = self.lib.add_name(NodeKind::LeftName, o, span, call);
        let mut statements = vec![self.lib.add(NodeKind::Let, span, LET_INIT, &[o_left, object], call)];
        let member = |s: &mut Self| {
            let callee = s.lib.add_name(NodeKind::BuiltinName, dot_name, span, dot);
            let o_use = s.lib.add_name(NodeKind::RightName, o, span, call);
            let key = s.lib.add_value(key_value.clone(), span, key);
            s.lib.add(NodeKind::Call, span, 0, &[callee, o_use, key], target)
        };
        let amount = match value {
            Some(v) => v,
            None => self.lib.add_value(Value::Int32(1), span, call),
        };
        let operator = self.builtin_name(function, span, call);
        let mut old = member(self);
        let postfix = text.starts_with('_');
        if postfix {
            let t = self.lib.names.temporary("t");
            let t_left = self.lib.add_name(NodeKind::LeftName, t, span, call);
            statements.push(self.lib.add(NodeKind::Let, span, LET_INIT, &[t_left, old], call));
            old = self.lib.add_name(NodeKind::RightName, t, span, call);
        }
        let sum = self.lib.add(NodeKind::Call, span, 0, &[operator, old, amount], call);
        let assigned = member(self);
        statements.push(self.lib.add(NodeKind::Assign, span, 0, &[assigned, sum], call));
        if postfix {
            let t = self.lib.name(old).unwrap();
            statements.push(self.lib.add_name(NodeKind::RightName, t, span, call));
        }
        self.rewrite(call, NodeKind::Block, 0, &statements);
    }

    /// A temporary holding `value`, unless it's a name already: the
    /// statements declaring it, if any, and the name to use.
    fn subject(&mut self, value: NodeId, hint: &str, like: NodeId) -> (Vec<NodeId>, NameId) {
        if self.lib.tree.kind(value) == NodeKind::RightName {
            return (Vec::new(), self.lib.name(value).unwrap());
        }
        let span = self.lib.tree.span(value);
        let t = self.lib.names.temporary(hint);
        let left = self.lib.add_name(NodeKind::LeftName, t, span, like);
        (vec![self.lib.add(NodeKind::Let, span, LET_INIT, &[left, value], like)], t)
    }

    /// `name == null`.
    fn null_check(&mut self, name: NameId, span: Span, like: NodeId) -> NodeId {
        let eq = self.scopes.hidden(Builtin::Function(Function::Eq));
        let eq = self.builtin_name(eq, span, like);
        let used = self.lib.add_name(NodeKind::RightName, name, span, like);
        let null = self.lib.add_value(Value::Null, span, like);
        self.lib.add(NodeKind::Call, span, 0, &[eq, used, null], like)
    }

    /// `a?.name`, or as `dot` in a call, `a?.name(args)`: `do { let t = a;
    /// if (t == null) { null } else { t.name(args) } }`, as Kotlin Temper
    /// lowers it. A later `.` isn't skipped: `a?.b.c` fails if `a` is null.
    pub(crate) fn expand_chain_null(&mut self, call: NodeId, dot: NodeId) {
        let span = self.lib.tree.span(call);
        let [op, object, key] = *self.lib.tree.children(dot) else { unreachable!() };
        let (mut statements, name) = self.subject(object, "subject", call);
        let null_check = self.null_check(name, span, call);
        let null = self.lib.add_value(Value::Null, span, call);
        let dot_name = self.lib.names.intern(".");
        let dot_name = self.lib.add_name(NodeKind::BuiltinName, dot_name, self.lib.tree.span(op), op);
        let used = self.lib.add_name(NodeKind::RightName, name, self.lib.tree.span(object), object);
        let mut member = self.lib.add(NodeKind::Call, self.lib.tree.span(dot), 0, &[dot_name, used, key], dot);
        if dot != call {
            let mut parts = self.lib.tree.children(call).to_vec();
            parts[0] = member;
            member = self.lib.add(NodeKind::Call, span, 0, &parts, call);
        }
        statements.push(self.lib.add(NodeKind::If, span, 0, &[null_check, null, member], call));
        self.rewrite(call, NodeKind::Block, 0, &statements);
    }

    /// `when (x) { 0, 1 -> a; is T -> b; else -> c }` is, as in Kotlin
    /// Temper, `do { let t = x; if (t == 0 || t == 1) { a } else if (t is
    /// T) { b } else { c } }`, with no temporary if `x` is a name. Without
    /// `else`, it's void when nothing matches.
    fn expand_when(&mut self, call: NodeId, args: &[NodeId]) -> bool {
        let [subject, body] = *args else { return false };
        let Some(block) = self.block_of(body) else { return false };
        let span = self.lib.tree.span(call);
        let (mut statements, name) = self.subject(subject, "subject", call);
        let kids = self.lib.tree.children(block).to_vec();
        // Each branch's tests (none for `else`) and value.
        let mut branches: Vec<(Vec<NodeId>, NodeId)> = Vec::new();
        let mut has_else = false;
        let mut i = 0;
        while i < kids.len() {
            let start = i;
            let mut tests = Vec::new();
            let mut is_else = false;
            while let Some(kind) = kids.get(i).and_then(|&k| self.symbol_text(k)) {
                let kind = match kind {
                    "default" => None,
                    "case" => Some(Function::Eq),
                    "case_is" => Some(Function::Is),
                    "case_case" => {
                        self.error(call, Message::Unsupported("case patterns in when".into()));
                        return true;
                    }
                    _ => break,
                };
                i += 1;
                let Some(kind) = kind else {
                    is_else = true;
                    continue;
                };
                let Some(&test) = kids.get(i) else {
                    self.error(call, Message::MissingCaseValue);
                    return true;
                };
                i += 1;
                let function = self.scopes.hidden(Builtin::Function(kind));
                let test_span = self.lib.tree.span(test);
                let callee = self.builtin_name(function, test_span, test);
                let used = self.lib.add_name(NodeKind::RightName, name, test_span, subject);
                tests.push(self.lib.add(NodeKind::Call, test_span, 0, &[callee, used, test], test));
            }
            if i == start {
                // Not a case: a trailing `;`'s void and comments are fine.
                let kid = kids[i];
                let is_comment = self.lib.tree.kind(kid) == NodeKind::Call
                    && self.lib.tree.children(kid).first().and_then(|&c| self.lib.value(c))
                        == Some(&Value::Builtin(SyntaxBuiltin::EmbeddedComment));
                if self.lib.value(kid) != Some(&Value::Void) && !is_comment {
                    self.error(kid, Message::InvalidBlockContent);
                }
                i += 1;
                continue;
            }
            let Some(&value) = kids.get(i) else {
                self.error(call, Message::MissingCaseValue);
                return true;
            };
            i += 1;
            if has_else {
                self.error(kids[start], Message::ElseMustBeLast);
                continue;
            }
            has_else = is_else;
            branches.push((if is_else { Vec::new() } else { tests }, value));
        }
        // Later branches nest in earlier ones' `else`, so fold from the right.
        let mut chain = None;
        for (tests, value) in branches.into_iter().rev() {
            let Some(test) = tests.into_iter().rev().reduce(|later, test| {
                let yes = self.lib.add_value(Value::Bool(true), span, call);
                self.lib.add(NodeKind::If, self.lib.tree.span(test), 0, &[test, yes, later], call)
            }) else {
                chain = Some(value);
                continue;
            };
            let otherwise = match chain {
                Some(c) => c,
                None => self.lib.add(NodeKind::Block, Span { start: span.end, end: span.end }, 0, &[], call),
            };
            chain = Some(self.lib.add(NodeKind::If, self.lib.tree.span(value), 0, &[test, value, otherwise], call));
        }
        if let Some(chain) = chain {
            statements.push(chain);
        }
        self.rewrite(call, NodeKind::Block, 0, &statements);
        true
    }

    /// `let f(x: Int): Int { body }`: `let \word ^f params... \outType Int
    /// fn { body }`, a constant function.
    fn expand_let(&mut self, call: NodeId, args: &[NodeId]) -> bool {
        let [word, name, rest @ ..] = args else {
            return false;
        };
        if self.symbol_text(*word) != Some("word") || self.lib.tree.kind(*name) != NodeKind::LeftName {
            return false;
        }
        let Some(sig) = self.signature(call, rest) else {
            return self.lib.tree.kind(call) == NodeKind::Error;
        };
        let Some(body) = sig.body.and_then(|b| self.block_of(b)) else {
            self.error(call, Message::Unsupported("a function without a body".into()));
            return true;
        };
        let span = self.lib.tree.span(call);
        let Some(f) = self.make_fn(&sig, body, span, call) else {
            self.error(call, Message::MalformedFunction);
            return true;
        };
        self.rewrite(call, NodeKind::Let, LET_INIT, &[*name, f]);
        true
    }

    /// `fn (x: Int): Int { body }` is a function, and without a body, a
    /// function type.
    fn expand_fn(&mut self, call: NodeId, args: &[NodeId]) -> bool {
        let Some(sig) = self.signature(call, args) else {
            return self.lib.tree.kind(call) == NodeKind::Error;
        };
        let span = self.lib.tree.span(call);
        match sig.body {
            Some(body) => {
                let Some(block) = self.block_of(body) else {
                    return false;
                };
                let Some((children, data)) = self.fn_layout(&sig, block) else {
                    self.error(call, Message::MalformedFunction);
                    return true;
                };
                self.rewrite(call, NodeKind::Fn, data, &children);
            }
            None => {
                // Type parameters on function types are dropped for now.
                let fn_type = self.scopes.hidden(Builtin::Function(Function::FnType));
                let callee = self.builtin_name(fn_type, span, call);
                let mut children = vec![callee];
                children.extend(sig.params);
                children.extend(sig.out_type);
                self.rewrite(call, NodeKind::Call, 0, &children);
            }
        }
        true
    }

    /// Splits a function's parts after its name: type parameters (each
    /// `\typeArg T`, lowered to a `Let`), parameters, `\outType T`, and a
    /// final `Fun` for the body. Malformed parts make `node` an error.
    pub(crate) fn signature(&mut self, node: NodeId, parts: &[NodeId]) -> Option<Signature> {
        let mut sig = Signature { type_params: Vec::new(), params: Vec::new(), out_type: None, body: None };
        let mut i = 0;
        while i < parts.len() {
            let part = parts[i];
            match self.symbol_text(part) {
                Some("outType") if i + 1 < parts.len() => {
                    sig.out_type = Some(parts[i + 1]);
                    i += 2;
                    continue;
                }
                Some("typeArg") if i + 1 < parts.len() => {
                    let Some(param) = self.lower_type_param(parts[i + 1]) else {
                        self.error(node, Message::MalformedFunction);
                        return None;
                    };
                    sig.type_params.push(param);
                    i += 2;
                    continue;
                }
                Some(_) => return None,
                None => {}
            }
            if i == parts.len() - 1 && self.lib.tree.kind(part) == NodeKind::Fun {
                sig.body = Some(part);
            } else {
                sig.params.push(part);
            }
            i += 1;
        }
        Some(sig)
    }

    /// A type parameter, as `T`, `T extends Bound` or with decorators, which
    /// are ignored, becomes a `Let` whose type is its bound. Variance isn't
    /// in the grammar: type parameters are invariant.
    pub(crate) fn lower_type_param(&mut self, node: NodeId) -> Option<NodeId> {
        let children = self.lib.tree.children(node).to_vec();
        let span = self.lib.tree.span(node);
        let (name, bound) = match (self.lib.tree.kind(node), &children[..]) {
            (NodeKind::RightName, _) => (node, None),
            (NodeKind::Block, [key, name, rest @ ..]) if self.symbol_text(*key) == Some("typeArg") => match rest {
                [] => (*name, None),
                [sup, bound] if self.symbol_text(*sup) == Some("super") => (*name, Some(*bound)),
                _ => return None,
            },
            (NodeKind::Call, [callee, _, target]) if self.lib.name(*callee).is_some_and(|n| self.lib.names.get(n) == "@") => {
                return self.lower_type_param(*target);
            }
            _ => return None,
        };
        if self.lib.tree.kind(name) != NodeKind::RightName {
            return None;
        }
        let text = self.lib.name(name).unwrap();
        let left = self.lib.add_name(NodeKind::LeftName, text, self.lib.tree.span(name), name);
        Some(self.make_let(left, bound, None, ROLE_TYPE_PARAM, span, node))
    }

    /// A grammar `Fun`, as a block argument `{ (x: Int): Int => body }`,
    /// becomes a `Fn`.
    pub(crate) fn lower_fun(&mut self, fun: NodeId) {
        let children = self.lib.tree.children(fun).to_vec();
        let Some((body, rest)) = children.split_last() else {
            return self.error(fun, Message::MalformedFunction);
        };
        let lowered = self.signature(fun, rest).filter(|sig| sig.body.is_none());
        let Some(sig) = lowered else {
            if self.lib.tree.kind(fun) == NodeKind::Fun {
                self.error(fun, Message::MalformedFunction);
            }
            return;
        };
        let Some((new, data)) = self.fn_layout(&sig, *body) else {
            return self.error(fun, Message::MalformedFunction);
        };
        self.rewrite(fun, NodeKind::Fn, data, &new);
    }

    pub(crate) fn make_fn(&mut self, sig: &Signature, body: NodeId, span: Span, like: NodeId) -> Option<NodeId> {
        let (children, data) = self.fn_layout(sig, body)?;
        Some(self.lib.add(NodeKind::Fn, span, data, &children, like))
    }

    /// A `Fn`'s children and data, lowering its parameters.
    fn fn_layout(&mut self, sig: &Signature, body: NodeId) -> Option<(Vec<NodeId>, u32)> {
        let mut children = sig.type_params.clone();
        children.extend(self.lower_params(&sig.params)?);
        children.extend(sig.out_type);
        children.push(body);
        let data = sig.params.len() as u32
            | ((sig.type_params.len() as u32) << TYPE_PARAMS_SHIFT)
            | if sig.out_type.is_some() { FN_OUT_TYPE } else { 0 };
        Some((children, data))
    }

    fn lower_params(&mut self, params: &[NodeId]) -> Option<Vec<NodeId>> {
        params.iter().map(|&p| self.lower_param(p)).collect()
    }

    /// A parameter, as `(Block \_complexArg_ x \type Int \default 1)`,
    /// a declaration, or either with decorators or doc comments, becomes a
    /// `Let`. Decorators here are read by their text: only `var` matters.
    pub(crate) fn lower_param(&mut self, param: NodeId) -> Option<NodeId> {
        let tree = &self.lib.tree;
        let children = tree.children(param).to_vec();
        match tree.kind(param) {
            NodeKind::Block if children.first().is_some_and(|&c| self.symbol_text(c) == Some("_complexArg_")) => {
                let first = *children.get(1)?;
                match self.lib.tree.kind(first) {
                    NodeKind::RightName => {
                        let name = self.lib.name(first).unwrap();
                        let left = self.lib.add_name(NodeKind::LeftName, name, self.lib.tree.span(first), first);
                        let (ty, init) = self.pairs(&children[2..], "default")?;
                        let span = self.lib.tree.span(param);
                        Some(self.make_let(left, ty, init, 0, span, param))
                    }
                    NodeKind::Decl if children.len() == 2 => self.lower_param(first),
                    _ => None,
                }
            }
            NodeKind::Decl => {
                self.lower_decl(param);
                (self.lib.tree.kind(param) == NodeKind::Let).then_some(param)
            }
            NodeKind::RightName => {
                let name = self.lib.name(param).unwrap();
                let left = self.lib.add_name(NodeKind::LeftName, name, self.lib.tree.span(param), param);
                Some(self.make_let(left, None, None, 0, self.lib.tree.span(param), param))
            }
            NodeKind::Call => {
                let callee = *children.first()?;
                if let Some(Value::Builtin(SyntaxBuiltin::DocComment)) = self.lib.value(callee) {
                    return self.lower_param(*children.get(1)?);
                }
                let decorator = self.lib.name(callee).map(|n| self.lib.names.get(n));
                match (decorator, &children[1..]) {
                    (Some("@"), [what, target]) => {
                        let var = self.lib.name(*what).is_some_and(|n| self.lib.names.get(n) == "var");
                        let lowered = self.lower_param(*target)?;
                        if var {
                            let data = self.lib.tree.data(lowered) | LET_VAR;
                            let parts = self.lib.tree.children(lowered).to_vec();
                            self.rewrite(lowered, NodeKind::Let, data, &parts);
                        }
                        Some(lowered)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Reads `\type T` and `\init V` (or `default_key`) pairs.
    fn pairs(&self, pairs: &[NodeId], init_key: &str) -> Option<(Option<NodeId>, Option<NodeId>)> {
        let (mut ty, mut init) = (None, None);
        for pair in pairs.chunks(2) {
            let [key, value] = *pair else { return None };
            match self.symbol_text(key)? {
                "type" => ty = Some(value),
                // A parameter declared as `var x = 1` has a `\default`.
                k if k == init_key || k == "default" => init = Some(value),
                _ => return None,
            }
        }
        Some((ty, init))
    }

    fn make_let(&mut self, name: NodeId, ty: Option<NodeId>, init: Option<NodeId>, flags: u32, span: Span, like: NodeId) -> NodeId {
        let (children, data) = Self::let_layout(name, ty, init, flags);
        self.lib.add(NodeKind::Let, span, data, &children, like)
    }

    pub(crate) fn let_layout(name: NodeId, ty: Option<NodeId>, init: Option<NodeId>, flags: u32) -> (Vec<NodeId>, u32) {
        let mut children = vec![name];
        let mut data = flags;
        if let Some(t) = ty {
            children.push(t);
            data |= LET_TYPE;
        }
        if let Some(i) = init {
            children.push(i);
            data |= LET_INIT;
        }
        (children, data)
    }

    /// A grammar declaration, `^x \type T \init v`, becomes a `Let`, or an
    /// error.
    pub(crate) fn lower_decl(&mut self, decl: NodeId) {
        let children = self.lib.tree.children(decl).to_vec();
        let lowered = match children.split_first() {
            Some((&name, pairs)) if self.lib.tree.kind(name) == NodeKind::LeftName => {
                self.pairs(pairs, "init").map(|(ty, init)| Self::let_layout(name, ty, init, 0))
            }
            _ => None,
        };
        match lowered {
            Some((children, data)) => self.rewrite(decl, NodeKind::Let, data, &children),
            None => self.error(decl, Message::MalformedDeclaration),
        }
    }

    /// A destructuring declaration, `let { a, b as c, d is T } = x`, as the
    /// statements it stands for, as Kotlin's `flattenMultiInit` does: `let
    /// t = x; let a = t.a; let c = t.b; let d: T = t.d`, with no temporary
    /// when `x` is a name. `None` for any other declaration; a malformed one
    /// becomes an error.
    pub(crate) fn destructure(&mut self, decl: NodeId) -> Option<Vec<NodeId>> {
        let tree = &self.lib.tree;
        let (&targets, pairs) = tree.children(decl).split_first()?;
        let is_curlies = tree.kind(targets) == NodeKind::Call
            && tree.children(targets).first().is_some_and(|&c| {
                tree.kind(c) == NodeKind::BuiltinName && self.lib.name(c).is_some_and(|n| self.lib.names.get(n) == "{}")
            });
        if !is_curlies {
            return None;
        }
        let parts = tree.children(targets)[1..].to_vec();
        let init = match *pairs {
            [key, value] if self.symbol_text(key) == Some("init") => Some(value),
            _ => None,
        };
        // Each target: its source name, the name it's declared as, and its type.
        let mut each: Vec<(NameId, NodeId, Option<NodeId>)> = Vec::new();
        let mut rest = &parts[..];
        let mut ok = init.is_some();
        while let (true, Some((&source, more))) = (ok, rest.split_first()) {
            rest = more;
            let Some(name) = self.lib.name(source).filter(|_| self.lib.tree.kind(source) == NodeKind::LeftName) else {
                ok = false;
                break;
            };
            let (mut target, mut ty) = (source, None);
            while let [key, value, more @ ..] = rest {
                match self.symbol_text(*key) {
                    Some("as") if target == source && self.lib.tree.kind(*value) == NodeKind::LeftName => target = *value,
                    Some("type") if ty.is_none() => ty = Some(*value),
                    // Other metadata, a second rename, nested patterns,
                    // and `...`, which is only for imports.
                    Some(_) => ok = false,
                    None => break,
                }
                rest = more;
            }
            each.push((name, target, ty));
        }
        let (true, Some(init)) = (ok, init) else {
            self.error(decl, Message::MalformedDeclaration);
            return None;
        };
        let span = self.lib.tree.span(decl);
        let (mut statements, t) = self.subject(init, "destructured", decl);
        for (name, target, ty) in each {
            let at = self.lib.tree.span(target);
            let dot = self.lib.names.intern(".");
            let dot = self.lib.add_name(NodeKind::BuiltinName, dot, at, decl);
            let object = self.lib.add_name(NodeKind::RightName, t, at, decl);
            let key = self.lib.add_value(Value::Symbol(name), at, decl);
            let value = self.lib.add(NodeKind::Call, at, 0, &[dot, object, key], decl);
            statements.push(self.make_let(target, ty, Some(value), 0, span, decl));
        }
        Some(statements)
    }

    /// `@var decl` makes a `var`; other decorators are ignored for now.
    fn expand_decorate(&mut self, call: NodeId, args: &[NodeId]) -> bool {
        let [decorator, target] = *args else {
            return false;
        };
        let var = self.lib.name(decorator).is_some_and(|n| self.lib.names.get(n) == "var");
        if var {
            if self.lib.tree.kind(target) == NodeKind::Decl {
                self.lower_decl(target);
            }
            if self.lib.tree.kind(target) != NodeKind::Let {
                return false;
            }
            let data = self.lib.tree.data(target) | LET_VAR;
            let parts = self.lib.tree.children(target).to_vec();
            self.rewrite(call, NodeKind::Let, data, &parts);
        } else {
            self.become_node(call, target);
        }
        true
    }

    /// The parts of `if`: condition, then and else, with `else if` as a
    /// nested `If`.
    fn if_parts(&mut self, args: &[NodeId]) -> Option<[NodeId; 3]> {
        let [condition, then, rest @ ..] = args else {
            return None;
        };
        let then = self.block_of(*then)?;
        let otherwise = match rest {
            [] => {
                let at = self.lib.tree.span(then).end;
                self.lib.add(NodeKind::Block, Span { start: at, end: at }, 0, &[], then)
            }
            [key, delegate] => {
                let passed = self.delegated(*delegate)?;
                match self.symbol_text(*key)? {
                    "else" => match *passed {
                        [body] => self.block_of(body)?,
                        _ => return None,
                    },
                    "else_if" => {
                        let parts = self.if_parts(&passed)?;
                        let span = self.lib.tree.span(*delegate);
                        self.lib.add(NodeKind::If, span, 0, &parts, *delegate)
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        Some([*condition, then, otherwise])
    }

    /// What a call join's delegate, `fn (f#0) { f#0(args...) }`, passes on.
    fn delegated(&self, fun: NodeId) -> Option<Vec<NodeId>> {
        let tree = &self.lib.tree;
        let [decl, call] = *tree.children(fun) else { return None };
        if tree.kind(fun) != NodeKind::Fun || tree.kind(decl) != NodeKind::Decl || tree.kind(call) != NodeKind::Call {
            return None;
        }
        let param = self.lib.name(*tree.children(decl).first()?)?;
        let (&callee, args) = tree.children(call).split_first()?;
        (self.lib.name(callee) == Some(param) && self.lib.names.is_temporary(param)).then(|| args.to_vec())
    }

    /// The block of a block argument without parameters, `{ ... }`.
    pub(crate) fn block_of(&self, fun: NodeId) -> Option<NodeId> {
        let tree = &self.lib.tree;
        match *tree.children(fun) {
            [block] if tree.kind(fun) == NodeKind::Fun && tree.kind(block) == NodeKind::Block => Some(block),
            _ => None,
        }
    }

    /// `while (c) { body }` is `Loop(If(c, body, Break))`.
    fn expand_while(&mut self, call: NodeId, args: &[NodeId]) -> bool {
        let [condition, body] = *args else { return false };
        let Some(body) = self.block_of(body) else { return false };
        let span = self.lib.tree.span(call);
        let exit = self.lib.add(NodeKind::Break, span, call.0, &[], call);
        let check = self.lib.add(NodeKind::If, span, 0, &[condition, body, exit], call);
        self.rewrite(call, NodeKind::Loop, 0, &[check]);
        true
    }

    /// `for (init; c; next) { body }` is `{ init; Loop(If(c, body, Break),
    /// next) }`.
    fn expand_for(&mut self, call: NodeId, args: &[NodeId]) -> bool {
        let Some((&body, pairs)) = args.split_last() else { return false };
        let Some(body) = self.block_of(body) else { return false };
        if let [of] = *pairs {
            if let Some([decl, group]) = self.of_parts(of) {
                return self.expand_for_of(call, decl, group, body);
            }
        }
        let (mut init, mut condition, mut next) = (None, None, None);
        for pair in pairs.chunks(2) {
            let [key, value] = *pair else { return false };
            match self.symbol_text(key) {
                Some("__flowInit") => init = Some(value),
                Some("cond") => condition = Some(value),
                Some("incr") => next = Some(value),
                _ => {
                    self.error(call, Message::Unsupported("this kind of for loop".into()));
                    return true;
                }
            }
        }
        let span = self.lib.tree.span(call);
        let looping = self.lib.add(NodeKind::Loop, span, 0, &[], call);
        let pass = match condition {
            Some(c) => {
                let exit = self.lib.add(NodeKind::Break, span, looping.0, &[], call);
                self.lib.add(NodeKind::If, span, 0, &[c, body, exit], call)
            }
            None => body,
        };
        let mut parts = vec![pass];
        parts.extend(next);
        self.rewrite(looping, NodeKind::Loop, if next.is_some() { LOOP_NEXT } else { 0 }, &parts);
        let mut statements: Vec<NodeId> = init.into_iter().collect();
        statements.push(looping);
        self.rewrite(call, NodeKind::Block, 0, &statements);
        true
    }

    /// `x of xs` as its two sides, before or after `desugarOperation`.
    fn of_parts(&self, node: NodeId) -> Option<[NodeId; 2]> {
        let tree = &self.lib.tree;
        if tree.kind(node) != NodeKind::Call {
            return None;
        }
        let children = tree.children(node);
        let parts = match *children {
            [op, a, b] if self.lib.name(op).is_some() => (op, a, b),
            [f, op, a, b] if self.lib.value(f) == Some(&Value::Builtin(SyntaxBuiltin::DesugarOperation)) => (op, a, b),
            _ => return None,
        };
        let is_of = self.lib.name(parts.0).is_some_and(|n| self.lib.names.get(n) == "of");
        is_of.then_some([parts.1, parts.2])
    }

    /// `for (let x of xs) { body }` is `Loop({ xs.forEach(fn (x) { body });
    /// Break })`, as Kotlin Temper lowers it to `xs.forEach`; see
    /// [`LOOP_EACH`] and [`FN_LOOP_BODY`].
    fn expand_for_of(&mut self, call: NodeId, decl: NodeId, group: NodeId, body: NodeId) -> bool {
        let span = self.lib.tree.span(call);
        if self.lib.tree.kind(decl) != NodeKind::Decl {
            self.error(call, Message::ExpectedDeclarationForOf);
            return true;
        }
        let Some(param) = self.lower_param(decl) else { return false };
        if self.lib.tree.data(param) & LET_INIT != 0 {
            self.error(call, Message::OfDeclarationInitializer);
            return true;
        }
        let each = self.lib.add(NodeKind::Fn, span, 1 | FN_LOOP_BODY, &[param, body], call);
        let dot = self.lib.names.intern(".");
        let dot = self.lib.add_name(NodeKind::BuiltinName, dot, span, call);
        let key = Value::Symbol(self.lib.names.intern("forEach"));
        let key = self.lib.add_value(key, span, call);
        let member = self.lib.add(NodeKind::Call, span, 0, &[dot, group, key], call);
        let looped = self.lib.add(NodeKind::Call, span, 0, &[member, each], call);
        let exit = self.lib.add(NodeKind::Break, span, call.0, &[], call);
        let block = self.lib.add(NodeKind::Block, span, 0, &[looped, exit], call);
        self.rewrite(call, NodeKind::Loop, LOOP_EACH, &[block]);
        true
    }

    fn expand_return(&mut self, call: NodeId, scope: ScopeId, args: &[NodeId]) -> bool {
        let value = match *args {
            [] => self.lib.add_value(Value::Void, self.lib.tree.span(call), call),
            [value] => value,
            _ => return false,
        };
        match self.enclosing(scope).last() {
            Some(&(ScopeKind::Fn, f)) => self.rewrite(call, NodeKind::Return, f.0, &[value]),
            _ => self.error(call, Message::ReturnOutsideFunction),
        }
        true
    }

    /// `break` and `continue`, with a label or to the nearest loop.
    fn expand_jump(&mut self, call: NodeId, scope: ScopeId, args: &[NodeId], is_break: bool) -> bool {
        let label = match *args {
            [] => None,
            [key, name] if self.symbol_text(key) == Some("label") => Some(self.lib.name(name)),
            _ => return false,
        };
        let target = self.enclosing(scope).into_iter().find_map(|(kind, node)| match (kind, label) {
            (ScopeKind::Loop, None) => Some(node),
            (ScopeKind::LoopBody, None) if !is_break => Some(node),
            (ScopeKind::Label(l), Some(Some(name))) if l == name => {
                if is_break { Some(node) } else { self.labeled_loop(node) }
            }
            _ => None,
        });
        let (kind, what) = if is_break { (NodeKind::Break, "break") } else { (NodeKind::Continue, "continue") };
        match target {
            Some(target) => self.rewrite(call, kind, target.0, &[]),
            None => self.error(call, Message::NoJumpTarget(what)),
        }
        true
    }

    /// The loop a label names, for `continue`: its statement, or what that
    /// expanded to, as a `for`'s block ending in its loop, or for `for …
    /// of`, its body `Fn`.
    fn labeled_loop(&self, block: NodeId) -> Option<NodeId> {
        let tree = &self.lib.tree;
        let mut at = *tree.children(block).first()?;
        loop {
            match tree.kind(at) {
                NodeKind::Loop if tree.data(at) & LOOP_EACH != 0 => {
                    let looped = *tree.children(*tree.children(at).first()?).first()?;
                    return tree.children(looped).last().copied();
                }
                NodeKind::Loop => return Some(at),
                NodeKind::Block => at = *tree.children(at).last()?,
                _ => return None,
            }
        }
    }

    /// Whether a node is `obj.name`, before or after resolution.
    fn is_dot_call(&self, node: NodeId) -> bool {
        self.is_builtin_call(node, ".")
    }

    /// Whether a node is `ls[i]`.
    fn is_index_call(&self, node: NodeId) -> bool {
        self.is_builtin_call(node, "[]")
    }

    /// Whether a node is `a?.name`.
    pub(crate) fn is_chain_null(&self, node: NodeId) -> bool {
        self.is_builtin_call(node, "?.")
    }

    /// Whether a node is a call of a builtin name, with two arguments.
    fn is_builtin_call(&self, node: NodeId, text: &str) -> bool {
        let tree = &self.lib.tree;
        tree.kind(node) == NodeKind::Call
            && tree.children(node).len() == 3
            && tree.children(node).first().is_some_and(|&c| {
                tree.kind(c) == NodeKind::BuiltinName && self.lib.name(c).is_some_and(|n| self.lib.names.get(n) == text)
            })
    }

    pub(crate) fn symbol_text(&self, node: NodeId) -> Option<&str> {
        self.lib.symbol(node).map(|s| self.lib.names.get(s))
    }
}
