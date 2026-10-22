//! Text forms of parse trees: a readable one for tests, and a line-based
//! one for comparing with Kotlin Temper (`compat/oracle/ParseTree.java`).

use crate::lex::escape_into;
use crate::parse::Parsed;
use crate::tree::{NodeId, NodeKind};
use std::fmt::Write;

/// The node to print for a tree: the root's only child if it's an operator,
/// as Kotlin Temper's parser returns, else the root.
fn top(parsed: &Parsed) -> NodeId {
    match parsed.tree.children(parsed.root) {
        [only] if parsed.tree.op(*only).is_some() => *only,
        _ => parsed.root,
    }
}

/// Appends the tree as lines, one per node in pre-order, then errors and
/// comments, as described in `compat/oracle/ParseTree.java`.
pub fn write_tree_lines(parsed: &Parsed, out: &mut String) {
    let mut stack = vec![(top(parsed), 0)];
    while let Some((id, depth)) = stack.pop() {
        match parsed.tree.kind(id) {
            NodeKind::Op(op) => {
                let _ = writeln!(out, "N\t{depth}\t{op:?}");
                for &child in parsed.tree.children(id).iter().rev() {
                    stack.push((child, depth + 1));
                }
            }
            NodeKind::Token => {
                let token = parsed.token(id).unwrap();
                let flags = if token.synthetic { "s" } else { "-" };
                let _ = write!(
                    out,
                    "T\t{depth}\t{:?}\t{}\t{}\t{flags}\t",
                    token.kind, token.start, token.end
                );
                escape_into(parsed.token_text(token), out);
                out.push('\n');
            }
        }
    }
    for e in &parsed.errors {
        let _ = writeln!(
            out,
            "E\t{:?}\t{:?}\t{}\t{}\t{}\t{}",
            e.kind, e.op, e.bound, e.operands, e.span.start, e.span.end
        );
    }
    for c in parsed.comments() {
        let _ = writeln!(out, "C\t{:?}\t{}\t{}", c.kind, c.start, c.end);
    }
}

/// Appends the tree as nested lists: `(Op child ...)` for operators,
/// `[a b]` for several value tokens, tokens bare or quoted if they contain
/// space, brackets or quotes. Lists that don't fit in `width` columns are
/// broken over lines. Then errors and comments, one per line.
pub fn write_tree(parsed: &Parsed, width: usize, out: &mut String) {
    write_node(parsed, top(parsed), 0, width, out);
    out.push('\n');
    for e in &parsed.errors {
        let _ = writeln!(out, "error {}..{}: {}", e.span.start, e.span.end, e.message());
    }
    for c in parsed.comments() {
        let _ = writeln!(out, "comment {:?} {}..{}", c.kind, c.start, c.end);
    }
}

fn write_node(parsed: &Parsed, id: NodeId, indent: usize, width: usize, out: &mut String) {
    let flat = flat(parsed, id);
    let children = parsed.tree.children(id);
    if indent + flat.len() <= width || children.is_empty() {
        out.push_str(&flat);
        return;
    }
    let (open, close) = brackets(parsed, id);
    out.push_str(&open);
    for &child in children {
        out.push('\n');
        out.push_str(&" ".repeat(indent + 2));
        write_node(parsed, child, indent + 2, width, out);
    }
    out.push_str(close);
}

fn brackets(parsed: &Parsed, id: NodeId) -> (String, &'static str) {
    match parsed.tree.op(id) {
        Some(crate::operator::Operator::Leaf) => ("[".to_string(), "]"),
        Some(op) => (format!("({op:?}"), ")"),
        None => (String::new(), ""),
    }
}

fn flat(parsed: &Parsed, id: NodeId) -> String {
    let mut out = String::new();
    write_flat(parsed, id, &mut out);
    out
}

fn write_flat(parsed: &Parsed, id: NodeId, out: &mut String) {
    let children = parsed.tree.children(id);
    match parsed.tree.op(id) {
        None => write_token(parsed.token_text(parsed.token(id).unwrap()), out),
        Some(crate::operator::Operator::Leaf) if children.len() == 1 => {
            write_flat(parsed, children[0], out)
        }
        Some(_) => {
            let (open, close) = brackets(parsed, id);
            out.push_str(&open);
            for (i, &child) in children.iter().enumerate() {
                if i > 0 || !open.ends_with('[') {
                    out.push(' ');
                }
                write_flat(parsed, child, out);
            }
            out.push_str(close);
        }
    }
}

fn write_token(text: &str, out: &mut String) {
    let bare = !text.is_empty()
        && !text.chars().any(|c| c.is_whitespace() || c.is_control() || "()[]\"\\".contains(c));
    if bare {
        out.push_str(text);
    } else {
        out.push('"');
        let mut escaped = String::new();
        escape_into(text, &mut escaped);
        out.push_str(&escaped.replace('"', "\\\""));
        out.push('"');
    }
}
