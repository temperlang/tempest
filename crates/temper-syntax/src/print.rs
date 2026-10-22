//! Text forms of parse trees and the grammar step's trees: a readable one
//! for tests, and a line-based one for comparing with Kotlin Temper
//! (`compat/oracle/ParseTree.java` and `AstTree.java`).

use crate::ast::{Ast, Level, Value};
use crate::lex::escape_into;
use crate::parse::Parsed;
use crate::tree::{NodeId, NodeKind};
use std::fmt::Write;

/// The node to print for a tree: the root's only child if it's an operator,
/// as Kotlin Temper's parser returns, else the root.
fn top(parsed: &Parsed) -> NodeId {
    match parsed.tree().children(parsed.root()) {
        [only] if parsed.tree().op(*only).is_some() => *only,
        _ => parsed.root(),
    }
}

/// Appends the tree as lines, one per node in pre-order, then errors and
/// comments, as described in `compat/oracle/ParseTree.java`.
pub fn write_tree_lines(parsed: &Parsed, out: &mut String) {
    let mut stack = vec![(top(parsed), 0)];
    while let Some((id, depth)) = stack.pop() {
        match parsed.tree().kind(id) {
            NodeKind::Op(op) => {
                let _ = writeln!(out, "N\t{depth}\t{op:?}");
                for &child in parsed.tree().children(id).iter().rev() {
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
            _ => {}
        }
    }
    for e in parsed.errors() {
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
    for e in parsed.errors() {
        let _ = writeln!(out, "error {}..{}: {}", e.span.start, e.span.end, e.message());
    }
    for c in parsed.comments() {
        let _ = writeln!(out, "comment {:?} {}..{}", c.kind, c.start, c.end);
    }
}

fn write_node(parsed: &Parsed, id: NodeId, indent: usize, width: usize, out: &mut String) {
    let flat = flat(parsed, id);
    let children = parsed.tree().children(id);
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
    match parsed.tree().op(id) {
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
    let children = parsed.tree().children(id);
    match parsed.tree().op(id) {
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
        write_quoted(text, out);
    }
}

/// Appends the grammar step's tree as lines, one per node in pre-order,
/// then its diagnostics, as described in `compat/oracle/AstTree.java`.
pub fn write_ast_lines(ast: &Ast, out: &mut String) {
    write_ast_node_lines(ast, ast.root(), out);
    if let Some(metadata) = ast.metadata() {
        out.push_str("M\n");
        write_ast_node_lines(ast, metadata, out);
    }
    for d in ast.diagnostics() {
        let level = match d.level {
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        };
        let _ = write!(out, "E\t{level}\t{}", d.message.template());
        if let Some(argument) = d.message.argument() {
            out.push('\t');
            escape_into(argument, out);
        }
        let _ = writeln!(out, "\t{}\t{}", d.span.start, d.span.end);
    }
}

fn write_ast_node_lines(ast: &Ast, root: NodeId, out: &mut String) {
    let tree = ast.tree();
    let mut stack = vec![(root, 0)];
    while let Some((id, depth)) = stack.pop() {
        let span = tree.span(id);
        let (start, end) = (span.start, span.end);
        let kind = tree.kind(id);
        match kind {
            NodeKind::LeftName | NodeKind::RightName | NodeKind::BuiltinName => {
                let _ = write!(out, "L\t{depth}\t{kind:?}\t{start}\t{end}\t");
                escape_into(ast.names().get(ast.name(id).unwrap()), out);
                out.push('\n');
            }
            NodeKind::Value => {
                let _ = write!(out, "V\t{depth}\t{start}\t{end}\t");
                write_value_line(ast, ast.value(id).unwrap(), out);
                out.push('\n');
            }
            NodeKind::Error => {
                let _ = writeln!(out, "X\t{depth}\t{start}\t{end}");
            }
            _ => {
                let _ = writeln!(out, "N\t{depth}\t{kind:?}\t{start}\t{end}");
                for &child in tree.children(id).iter().rev() {
                    stack.push((child, depth + 1));
                }
            }
        }
    }
}

fn write_value_line(ast: &Ast, value: &Value, out: &mut String) {
    let _ = match value {
        Value::Int32(n) => write!(out, "Int32\t{n}"),
        Value::Int64(n) => write!(out, "Int64\t{n}"),
        Value::Float64(f) => write!(out, "Float64\t{:x}", f.to_bits()),
        Value::String(s) => {
            out.push_str("String\t");
            escape_into(s, out);
            Ok(())
        }
        Value::Symbol(name) => {
            out.push_str("Symbol\t");
            escape_into(ast.names().get(*name), out);
            Ok(())
        }
        Value::Bool(b) => write!(out, "Boolean\t{b}"),
        Value::Null => write!(out, "Null"),
        Value::Void => write!(out, "Void"),
        Value::Builtin(b) => write!(out, "Function\t{}", kotlin_name(b.name())),
        Value::Strings(texts) => {
            let _ = write!(out, "List\t{}", texts.len());
            for text in texts {
                out.push('\t');
                escape_into(text, out);
            }
            Ok(())
        }
    };
}

/// A name as Kotlin shows it, quoted as ``nym`...` `` unless it's a word.
fn kotlin_name(name: &str) -> String {
    let word = name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_alphanumeric() || c == '_');
    if word { name.to_string() } else { format!("nym`{name}`") }
}

/// Appends the grammar step's tree as nested lists, broken over lines
/// where they don't fit in `width` columns, then its diagnostics. Calls,
/// blocks and the like are `(Call ...)`; a used name is bare, a declared
/// one `^x` and a builtin one `%x`; values are numbers (`1i64` for Int64,
/// floats with a `.` or exponent), quoted strings, symbols `\x`, `#true`,
/// `#null`, `#void`, builtins like `#list`, and doc comment lists `["a"]`;
/// errors are `(Error)`.
pub fn write_ast(ast: &Ast, width: usize, out: &mut String) {
    write_ast_node(ast, ast.root(), 0, width, out);
    out.push('\n');
    if let Some(metadata) = ast.metadata() {
        out.push_str("metadata ");
        write_ast_node(ast, metadata, 9, width, out);
        out.push('\n');
    }
    for d in ast.diagnostics() {
        let level = format!("{:?}", d.level).to_lowercase();
        let _ = writeln!(out, "{level} {}..{}: {}", d.span.start, d.span.end, d.message.text());
    }
}

fn write_ast_node(ast: &Ast, id: NodeId, indent: usize, width: usize, out: &mut String) {
    let mut flat = String::new();
    write_ast_flat(ast, id, &mut flat);
    let children = ast.tree().children(id);
    if indent + flat.len() <= width || children.is_empty() {
        out.push_str(&flat);
        return;
    }
    let _ = write!(out, "({:?}", ast.tree().kind(id));
    for &child in children {
        out.push('\n');
        out.push_str(&" ".repeat(indent + 2));
        write_ast_node(ast, child, indent + 2, width, out);
    }
    out.push(')');
}

fn write_ast_flat(ast: &Ast, id: NodeId, out: &mut String) {
    let tree = ast.tree();
    match tree.kind(id) {
        NodeKind::RightName => write_token(ast.names().get(ast.name(id).unwrap()), out),
        NodeKind::LeftName => {
            out.push('^');
            write_token(ast.names().get(ast.name(id).unwrap()), out);
        }
        NodeKind::BuiltinName => {
            out.push('%');
            write_token(ast.names().get(ast.name(id).unwrap()), out);
        }
        NodeKind::Value => write_value(ast, ast.value(id).unwrap(), out),
        NodeKind::Error => out.push_str("(Error)"),
        kind => {
            let _ = write!(out, "({kind:?}");
            for &child in tree.children(id) {
                out.push(' ');
                write_ast_flat(ast, child, out);
            }
            out.push(')');
        }
    }
}

fn write_value(ast: &Ast, value: &Value, out: &mut String) {
    let _ = match value {
        Value::Int32(n) => write!(out, "{n}"),
        Value::Int64(n) => write!(out, "{n}i64"),
        Value::Float64(f) => write!(out, "{f:?}"),
        Value::String(s) => {
            write_quoted(s, out);
            Ok(())
        }
        Value::Symbol(name) => {
            out.push('\\');
            write_token(ast.names().get(*name), out);
            Ok(())
        }
        Value::Bool(b) => write!(out, "#{b}"),
        Value::Null => write!(out, "#null"),
        Value::Void => write!(out, "#void"),
        Value::Builtin(b) => write!(out, "#{}", b.name()),
        Value::Strings(texts) => {
            out.push('[');
            for (i, text) in texts.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_quoted(text, out);
            }
            out.push(']');
            Ok(())
        }
    };
}

fn write_quoted(text: &str, out: &mut String) {
    out.push('"');
    let mut escaped = String::new();
    escape_into(text, &mut escaped);
    out.push_str(&escaped.replace('"', "\\\""));
    out.push('"');
}
