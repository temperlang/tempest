//! The steps after the grammar step that finish a module's tree, as the
//! rest of Kotlin Temper's `ParseStage`: the code's tree is wrapped in a
//! block if it isn't one, the metadata after `;;;` becomes a [`Json`] value,
//! and call joins are rewritten ([`build_module`]).

use crate::ast::{Ast, Level, Message, Value, build};
use crate::names::NameId;
use crate::parse::Parsed;
use crate::tree::{NodeId, NodeKind, Span};

/// What a module's code is for, which decides how call joins are rewritten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    /// Code translated into libraries.
    Library,
    /// Code that shows how to use a library, as in documentation.
    Documentation,
}

/// Module metadata, from the JSON after `;;;`.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Long(i64),
    Double(f64),
    String(String),
    Array(Vec<Json>),
    /// Properties in order, duplicates kept.
    Object(Vec<(String, Json)>),
}

/// Builds a module's tree: the grammar step ([`build`]), then the steps
/// that Kotlin's `ParseStage` adds.
pub fn build_module(parsed: &Parsed, genre: Genre) -> Ast {
    let mut ast = build(parsed);
    let root = ast.root;
    if ast.tree.kind(root) != NodeKind::Block {
        let span = ast.tree.span(root);
        ast.root = ast.add(NodeKind::Block, span, 0, &[root], ast.origin(root));
    }
    if let Some(metadata) = ast.metadata {
        ast.json = Some(to_json(&mut ast, metadata));
    }
    rewrite_call_joins(&mut ast, genre);
    ast
}

/// Converts the metadata's tree to JSON, as Kotlin's `toJsonValue`,
/// reporting `InvalidMetadata` for what isn't JSON and using null there.
fn to_json(ast: &mut Ast, id: NodeId) -> Json {
    let tree = &ast.tree;
    match tree.kind(id) {
        NodeKind::Value => match ast.value(id) {
            Some(Value::Bool(b)) => return Json::Bool(*b),
            Some(Value::Null) => return Json::Null,
            Some(Value::Int32(n)) => return Json::Long(*n as i64),
            Some(Value::Float64(f)) => return Json::Double(*f),
            Some(Value::String(s)) => return Json::String(s.clone()),
            _ => {}
        },
        NodeKind::Call => {
            let children = tree.children(id).to_vec();
            let callee = children.first().and_then(|&c| match tree.kind(c) {
                NodeKind::RightName | NodeKind::BuiltinName => ast.name(c).map(|n| ast.names.get(n)),
                _ => None,
            });
            match callee {
                Some("{}") if children.len() % 2 == 1 => {
                    let mut properties = Vec::new();
                    for pair in children[1..].chunks(2) {
                        let (key, value) = (pair[0], pair[1]);
                        match ast.value(key) {
                            Some(Value::String(key)) => {
                                let key = key.clone();
                                properties.push((key, to_json(ast, value)));
                            }
                            _ => {
                                let span = ast.tree.span(key).cover(ast.tree.span(value));
                                ast.report(Level::Error, Message::InvalidMetadata, span);
                            }
                        }
                    }
                    return Json::Object(properties);
                }
                Some("[]") => return Json::Array(children[1..].iter().map(|&c| to_json(ast, c)).collect()),
                Some("-") if children.len() == 2 => match ast.value(children[1]) {
                    Some(Value::Int32(n)) => return Json::Long(-(*n as i64)),
                    Some(Value::Float64(f)) => return Json::Double(-f),
                    _ => {}
                },
                _ => {}
            }
        }
        _ => {}
    }
    let span = ast.tree.span(id);
    ast.report(Level::Error, Message::InvalidMetadata, span);
    Json::Null
}

/// Rewrites call joins, as Kotlin's `rewriteCallJoins`. In a call like
/// `if (a) { b } else { c }`, the grammar step leaves `\callJoin:` before
/// the `\else` symbol that names what follows. For libraries, what follows
/// that symbol becomes a function that passes it all to its argument:
/// `if(a, fn { b }, \else, fn (f#0) { f#0(fn { c }) })`, so `if` can give
/// each continuation its own signature. For documentation, the
/// `\callJoin:` is just dropped.
fn rewrite_call_joins(ast: &mut Ast, genre: Genre) {
    let Some(call_join) = ast.names.find("callJoin:") else {
        return;
    };
    let mut calls = Vec::new();
    let mut stack = vec![ast.root];
    while let Some(id) = stack.pop() {
        if ast.tree.kind(id) == NodeKind::Call {
            calls.push(id);
        }
        stack.extend_from_slice(ast.tree.children(id));
    }
    // Nested calls first, as Kotlin's post-order visit, so temporaries
    // are made in its order.
    for &call in calls.iter().rev() {
        rewrite_call(ast, call, call_join, genre);
    }
}

fn rewrite_call(ast: &mut Ast, call: NodeId, call_join: NameId, genre: Genre) {
    let mut children = ast.tree.children(call).to_vec();
    let symbol = |ast: &Ast, id: Option<&NodeId>| match id.and_then(|&id| ast.value(id)) {
        Some(Value::Symbol(name)) => Some(*name),
        _ => None,
    };
    let mut starts = Vec::new();
    let mut i = 1;
    while i < children.len() {
        match symbol(ast, children.get(i)) {
            None => i += 1,
            Some(name) if name != call_join => i += 2,
            Some(_) if symbol(ast, children.get(i + 1)).is_none() => i += 2,
            Some(_) => {
                starts.push(i);
                i += 3;
            }
        }
    }
    if starts.is_empty() {
        return;
    }
    for &join in starts.iter().rev() {
        let join_node = children.remove(join);
        if genre == Genre::Documentation {
            continue;
        }
        let join_span = ast.tree.span(join_node);
        let origin = ast.origin(join_node);
        let rest = children.split_off(join + 1);
        let replaced = match (rest.first(), rest.last()) {
            (Some(&first), Some(&last)) => ast.tree.span(first).cover(ast.tree.span(last)),
            _ => {
                let at = ast.tree.span(children[join]).end;
                Span { start: at, end: at }
            }
        };
        let name = ast.names.temporary("f");
        let left = ast.add(NodeKind::LeftName, join_span, name.0, &[], origin);
        let decl = ast.add(NodeKind::Decl, join_span, 0, &[left], origin);
        let right = ast.add(NodeKind::RightName, replaced, name.0, &[], origin);
        let mut call_children = vec![right];
        call_children.extend(rest);
        let delegate = ast.add(NodeKind::Call, replaced, 0, &call_children, origin);
        let fun = ast.add(NodeKind::Fun, replaced, 0, &[decl, delegate], origin);
        children.push(fun);
    }
    ast.tree.set_children(call, &children);
}
