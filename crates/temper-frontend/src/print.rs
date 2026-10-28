//! Settled trees as text, for tests: nested lists as in
//! `temper_syntax::print::write_ast`, with core constructs, and names
//! with what they resolved to. A local's symbol is its number after the
//! builtins' symbols, as `x:3`, declared `^x:3`; builtins are bare; an
//! unresolved name ends in `?`; temporaries are `t#3`. Jumps show their
//! target's position, `(Break @12)`. Then diagnostics, one per line.

use crate::core::{self, LET_VAR};
use crate::scope::SymbolKind;
use crate::settle::Settled;
use std::fmt::Write;
use temper_syntax::ast::Value;
use temper_syntax::lex::escape_into;
use temper_syntax::tree::{NodeId, NodeKind};

/// Appends each file's settled tree, then all diagnostics.
pub fn write_settled(s: &Settled, width: usize, out: &mut String) {
    let first_local = s
        .scopes
        .symbols
        .iter()
        .take_while(|b| matches!(b.kind, SymbolKind::Builtin(_)))
        .count();
    let printer = Printer {
        s,
        first_local: first_local as u32,
    };
    for file in &s.library.files {
        printer.node(file.root, 0, width, out);
        out.push('\n');
    }
    write_diagnostics(s, out);
}

/// Appends diagnostics as `level start..end: message`, with positions in
/// their files.
pub fn write_diagnostics(s: &Settled, out: &mut String) {
    for d in &s.library.diagnostics {
        let level = format!("{:?}", d.level).to_lowercase();
        let (_, start) = s.library.locate(d.span.start);
        let end = start + (d.span.end - d.span.start);
        let _ = writeln!(out, "{level} {start}..{end}: {}", d.message.text());
    }
}

struct Printer<'a> {
    s: &'a Settled,
    first_local: u32,
}

impl Printer<'_> {
    fn node(&self, id: NodeId, indent: usize, width: usize, out: &mut String) {
        let mut flat = String::new();
        self.flat(id, &mut flat);
        let children = self.s.library.tree.children(id);
        if indent + flat.len() <= width || children.is_empty() || self.is_leafy(id) {
            out.push_str(&flat);
            return;
        }
        out.push('(');
        out.push_str(&self.head(id));
        for &child in children {
            out.push('\n');
            out.push_str(&" ".repeat(indent + 2));
            self.node(child, indent + 2, width, out);
        }
        out.push(')');
    }

    fn is_leafy(&self, id: NodeId) -> bool {
        matches!(
            self.s.library.tree.kind(id),
            NodeKind::Value
                | NodeKind::RightName
                | NodeKind::LeftName
                | NodeKind::BuiltinName
                | NodeKind::Error
        )
    }

    /// A node's label inside its parentheses.
    fn head(&self, id: NodeId) -> String {
        let tree = &self.s.library.tree;
        match tree.kind(id) {
            NodeKind::Let => {
                let data = tree.data(id);
                let mut head = match data & core::ROLE_MASK {
                    core::ROLE_PARAM => "Param",
                    core::ROLE_METHOD => "Method",
                    core::ROLE_GETTER => "Get",
                    core::ROLE_SETTER => "Set",
                    core::ROLE_CONSTRUCTOR => "Constructor",
                    core::ROLE_TYPE_PARAM => "TypeParam",
                    _ => "Let",
                }
                .to_string();
                if data & LET_VAR != 0 {
                    head = if head == "Let" {
                        "Var".into()
                    } else {
                        format!("{head} var")
                    };
                }
                if data & core::MEMBER_PRIVATE != 0 {
                    head.push_str(" private");
                }
                if data & core::MEMBER_STATIC != 0 {
                    head.push_str(" static");
                }
                if data & core::MEMBER_CONNECTED != 0 {
                    head.push_str(" connected");
                }
                head
            }
            NodeKind::Class if tree.data(id) & core::CLASS_INTERFACE != 0 => "Interface".into(),
            // A bodiless member's signature.
            NodeKind::Fn if tree.data(id) & core::FN_NO_BODY != 0 => "Signature".into(),
            NodeKind::Break
            | NodeKind::Continue
            | NodeKind::Return
            | NodeKind::Yield
            | NodeKind::Await => {
                let target = NodeId(tree.data(id));
                let (_, at) = self.s.library.locate(tree.span(target).start);
                format!("{:?} @{at}", tree.kind(id))
            }
            kind => format!("{kind:?}"),
        }
    }

    fn flat(&self, id: NodeId, out: &mut String) {
        let lib = &self.s.library;
        let tree = &lib.tree;
        match tree.kind(id) {
            NodeKind::RightName | NodeKind::LeftName | NodeKind::BuiltinName => {
                if tree.kind(id) == NodeKind::LeftName {
                    out.push('^');
                }
                let name = lib.name(id).unwrap();
                let temporary = lib.names.is_temporary(name);
                out.push_str(lib.names.get(name));
                match self.s.scopes.resolved(id) {
                    Some(b) if matches!(self.s.scopes.symbol(b).kind, SymbolKind::Builtin(_)) => {}
                    Some(b) => {
                        let _ = write!(
                            out,
                            "{}{}",
                            if temporary { '#' } else { ':' },
                            b.0 - self.first_local
                        );
                    }
                    None => out.push_str(if temporary { "#?" } else { "?" }),
                }
            }
            NodeKind::Value => write_value(lib.value(id).unwrap(), lib, out),
            NodeKind::Error => out.push_str("(Error)"),
            _ => {
                out.push('(');
                out.push_str(&self.head(id));
                for &child in tree.children(id) {
                    out.push(' ');
                    self.flat(child, out);
                }
                out.push(')');
            }
        }
    }
}

fn write_value(value: &Value, lib: &crate::library::Library, out: &mut String) {
    let _ = match value {
        Value::Int32(n) => write!(out, "{n}"),
        Value::Int64(n) => write!(out, "{n}i64"),
        Value::Float64(f) => write!(out, "{f:?}"),
        Value::String(text) => {
            let mut escaped = String::new();
            escape_into(text, &mut escaped);
            write!(out, "\"{}\"", escaped.replace('"', "\\\""))
        }
        Value::Symbol(name) => write!(out, "\\{}", lib.names.get(*name)),
        Value::Bool(b) => write!(out, "#{b}"),
        Value::Null => write!(out, "#null"),
        Value::Void => write!(out, "#void"),
        Value::Builtin(b) => write!(out, "#{}", b.name()),
        Value::Strings(_) => write!(out, "#strings"),
    };
}
