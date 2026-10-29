//! Writing canonical text.

use super::{FORMAT_VERSION, MODEL_VERSION, writing_order};
use crate::decls::{DeclKind, Facts};
use crate::ids::{DeclId, NodeId, Span, TypeId};
use crate::nodes::{Flags, Kids};
use crate::schema::{Kind, Node};
use crate::{Module, Program};
use std::fmt::Write as _;

/// Writes a program as canonical text, with its spans section if it has
/// spans. Expects a structurally sound program (the validator's S rules).
pub fn write(p: &Program) -> String {
    let mut w = Writer {
        out: String::new(),
        indent: 0,
        flat: false,
    };
    let _ = write!(
        w.out,
        "tmpl {FORMAT_VERSION} model {MODEL_VERSION} library {} {}",
        name(&p.library.name),
        p.library.version
    );
    if let Some(profile) = &p.profile {
        let _ = write!(w.out, " profile {}", name(profile));
    }
    w.out.push('\n');
    for module in &p.modules {
        w.out.push('\n');
        w.module(p, module);
    }
    if p.has_spans {
        w.out.push('\n');
        write_spans(p, &mut w.out);
    }
    w.out
}

struct Writer {
    out: String,
    indent: usize,
    /// Writing on one line, inside a `BlockValue`.
    flat: bool,
}

impl Writer {
    /// Starts a new line, or in flat mode, separates with a space.
    fn line(&mut self) {
        if self.flat {
            self.out.push(' ');
        } else {
            self.out.push('\n');
            for _ in 0..self.indent {
                self.out.push_str("  ");
            }
        }
    }

    fn module(&mut self, p: &Program, module: &Module) {
        self.out.push_str("module ");
        self.decl(p, module.decl);
        self.out.push_str(" {");
        self.indent += 1;
        for &item in &module.items {
            self.item(p, item);
            self.out.push('\n');
        }
        self.line();
        self.out.push_str("init ");
        self.block(p, module.init);
        self.indent -= 1;
        self.out.push_str("\n}\n");
    }

    fn item(&mut self, p: &Program, item: NodeId) {
        let decl = match p.nodes.get(item) {
            Node::Function { decl, .. } | Node::ModuleLet { decl } => decl,
            _ => DeclId::NONE,
        };
        let d = p.decls.get(decl);
        if !d.doc.is_none() {
            for line in p.strings.get(d.doc).split('\n') {
                self.line();
                self.out.push_str("///");
                if !line.is_empty() {
                    self.out.push(' ');
                    self.out.push_str(line);
                }
            }
        }
        self.line();
        if d.facts.contains(Facts::EXPORTED) {
            self.out.push_str("export ");
        }
        match p.nodes.get(item) {
            Node::Function { params, body, .. } => {
                self.out.push_str("fn ");
                self.decl(p, decl);
                self.out.push('(');
                for (i, &param) in p.nodes.list(params).iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    if let Node::Param { decl } = p.nodes.get(param) {
                        self.typed_decl(p, decl);
                    }
                }
                let (result, bubbles) = match p.signature(decl) {
                    Some((_, result, bubbles)) => (result, bubbles),
                    None => (TypeId::NONE, false),
                };
                self.out.push_str("): ");
                self.ty(p, result);
                if bubbles {
                    self.out.push_str(" throws Bubble");
                }
                self.out.push(' ');
                self.block(p, body);
            }
            _ => {
                self.let_word(p, decl);
                self.typed_decl(p, decl);
                self.out.push(';');
            }
        }
    }

    /// `let` or `var`, with a space.
    fn let_word(&mut self, p: &Program, decl: DeclId) {
        let var = p.decls.get(decl).facts.contains(Facts::VAR);
        self.out.push_str(if var { "var " } else { "let " });
    }

    /// A declaration where it's defined: `name#id`, or `~hint|alt#id` if
    /// compiler-made.
    fn decl(&mut self, p: &Program, decl: DeclId) {
        let d = p.decls.get(decl);
        if d.facts.contains(Facts::COMPILER_MADE) {
            self.out.push('~');
            self.out.push_str(&name(p.name(decl)));
            for &alt in p.decls.alts(decl) {
                self.out.push('|');
                self.out.push_str(&name(p.strings.get(alt)));
            }
            let _ = write!(self.out, "#{}", decl.0);
        } else {
            self.reference(p, decl);
        }
    }

    fn typed_decl(&mut self, p: &Program, decl: DeclId) {
        self.decl(p, decl);
        self.out.push_str(": ");
        self.ty(p, p.decls.get(decl).ty);
    }

    /// A use of a declaration: `name#id`, or `~hint#id`.
    fn reference(&mut self, p: &Program, decl: DeclId) {
        if p.decls.get(decl).facts.contains(Facts::COMPILER_MADE) {
            self.out.push('~');
        }
        let _ = write!(self.out, "{}#{}", name(p.name(decl)), decl.0);
    }

    fn ty(&mut self, p: &Program, ty: TypeId) {
        self.out.push_str(&p.types.show(ty));
    }

    fn block(&mut self, p: &Program, block: NodeId) {
        let Node::Block { stmts } = p.nodes.get(block) else {
            return;
        };
        self.stmts_in_braces(p, stmts);
    }

    fn stmts_in_braces(&mut self, p: &Program, stmts: Kids) {
        if stmts.is_empty() {
            self.out.push_str("{}");
            return;
        }
        self.out.push('{');
        self.indent += 1;
        for &stmt in p.nodes.list(stmts) {
            self.line();
            self.stmt(p, stmt);
        }
        self.indent -= 1;
        self.line();
        self.out.push('}');
    }

    fn stmt(&mut self, p: &Program, id: NodeId) {
        match p.nodes.get(id) {
            Node::Block { stmts } => self.stmts_in_braces(p, stmts),
            Node::Let { local, init } => {
                self.let_word(p, local);
                self.typed_decl(p, local);
                if let Some(init) = init {
                    self.out.push_str(" = ");
                    self.expr(p, init);
                }
                self.out.push(';');
            }
            Node::Assign {
                local: target,
                value,
            }
            | Node::SetModuleLet { target, value } => {
                self.reference(p, target);
                self.out.push_str(" = ");
                self.expr(p, value);
                self.out.push(';');
            }
            Node::InitLet { target, value } => {
                self.out.push_str("init ");
                self.reference(p, target);
                self.out.push_str(" = ");
                self.expr(p, value);
                self.out.push(';');
            }
            Node::If { cond, then, els } => {
                self.out.push_str("if (");
                self.expr(p, cond);
                self.out.push_str(") ");
                self.block(p, then);
                if let Some(els) = els {
                    self.out.push_str(" else ");
                    match else_if(p, els) {
                        Some(inner) => self.stmt(p, inner),
                        None => self.block(p, els),
                    }
                }
            }
            Node::While { label, cond, body } => {
                self.decl(p, label);
                self.out.push_str(": while (");
                self.expr(p, cond);
                self.out.push_str(") ");
                self.block(p, body);
            }
            Node::DoWhile { label, body, cond } => {
                self.decl(p, label);
                self.out.push_str(": do ");
                self.block(p, body);
                self.out.push_str(" while (");
                self.expr(p, cond);
                self.out.push_str(");");
            }
            Node::Break { label } => {
                self.out.push_str("break ");
                self.reference(p, label);
                self.out.push(';');
            }
            Node::Continue { label } => {
                self.out.push_str("continue ");
                self.reference(p, label);
                self.out.push(';');
            }
            Node::Return { value } => {
                self.out.push_str("return");
                if let Some(value) = value {
                    self.out.push(' ');
                    self.expr(p, value);
                }
                self.out.push(';');
            }
            Node::ExprStatement { expr } => {
                self.expr(p, expr);
                self.out.push(';');
            }
            _ => {
                // An expression in a statement's place (S3): written so
                // the problem shows.
                self.expr(p, id);
                self.out.push(';');
            }
        }
    }

    fn expr(&mut self, p: &Program, id: NodeId) {
        match p.nodes.get(id) {
            Node::BoolLit { value } => self.out.push_str(if value { "true" } else { "false" }),
            Node::Int32Lit { value } => {
                let _ = write!(self.out, "{value}");
            }
            Node::Int64Lit { value } => {
                let _ = write!(self.out, "{value}i64");
            }
            Node::Float64Lit { value } => self.out.push_str(&float(value)),
            Node::StringLit { value } => quote(p.strings.get(value), &mut self.out),
            Node::VoidLit {} => self.out.push_str("void"),
            Node::Local { local: decl } | Node::GetModuleLet { target: decl } => {
                self.reference(p, decl)
            }
            Node::CallFunction { callee, args, .. } => {
                self.reference(p, callee);
                self.args(p, args, id);
            }
            Node::Builtin { key, args, .. } => {
                self.out.push('%');
                self.out.push_str(key.info().map_or("?", |info| info.name));
                self.args(p, args, id);
            }
            Node::And { left, right } => {
                self.operand(p, left, Kind::And, false);
                self.out.push_str(" && ");
                self.operand(p, right, Kind::And, true);
            }
            Node::Or { left, right } => {
                self.operand(p, left, Kind::Or, false);
                self.out.push_str(" || ");
                self.operand(p, right, Kind::Or, true);
            }
            Node::IfValue { cond, then, els } => {
                self.out.push_str("(if (");
                self.expr(p, cond);
                self.out.push_str(") ");
                self.expr(p, then);
                self.out.push_str(" else ");
                self.expr(p, els);
                self.written_type(p, id);
            }
            Node::BlockValue { stmts, value } => {
                self.out.push_str("(value {");
                let flat = std::mem::replace(&mut self.flat, true);
                for &stmt in p.nodes.list(stmts) {
                    self.line();
                    self.stmt(p, stmt);
                }
                self.line();
                self.expr(p, value);
                self.flat = flat;
                self.out.push_str(" }");
                self.written_type(p, id);
            }
            Node::Error { message } => {
                self.out.push_str("(error(");
                quote(p.strings.get(message), &mut self.out);
                self.out.push(')');
                self.written_type(p, id);
            }
            _ => {
                // A statement in an expression's place (S3).
                self.out.push_str("(value {");
                let flat = std::mem::replace(&mut self.flat, true);
                self.line();
                self.stmt(p, id);
                self.flat = flat;
                self.out.push_str(" })");
            }
        }
    }

    /// An operand of `&&` or `||`. `&&` binds tighter, and both group
    /// from the left, so an `||` inside `&&`, or a right operand of the
    /// same kind, needs parentheses.
    fn operand(&mut self, p: &Program, id: NodeId, parent: Kind, right: bool) {
        let needs = match p.nodes.kind(id) {
            Kind::Or => parent == Kind::And || right,
            Kind::And => parent == Kind::And && right,
            _ => false,
        };
        if needs {
            self.out.push('(');
            self.expr(p, id);
            self.out.push(')');
        } else {
            self.expr(p, id);
        }
    }

    fn args(&mut self, p: &Program, args: Kids, call: NodeId) {
        self.out.push('(');
        for (i, &arg) in p.nodes.list(args).iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.expr(p, arg);
        }
        self.out.push(')');
        if p.nodes.flags(call).contains(Flags::MAY_BUBBLE) {
            self.out.push('^');
        }
    }

    /// Closes a form whose type is written: `): T`.
    fn written_type(&mut self, p: &Program, id: NodeId) {
        self.out.push_str("): ");
        self.ty(p, p.nodes.ty(id));
    }
}

/// The `If` that an else block holds alone, written `else if`.
fn else_if(p: &Program, els: NodeId) -> Option<NodeId> {
    match p.nodes.kids(els) {
        &[only] if matches!(p.nodes.get(only), Node::If { .. }) => Some(only),
        _ => None,
    }
}

/// A name, bare if it can be, else in backquotes.
pub(super) fn name(name: &str) -> String {
    if super::is_plain_name(name) {
        return name.into();
    }
    let mut out = String::from("`");
    for c in name.chars() {
        if matches!(c, '`' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('`');
    out
}

/// A `Float64` in the shortest form that reads back to the same bits,
/// always with `.` or an exponent.
fn float(value: f64) -> String {
    if value.is_nan() {
        return "nan".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.into();
    }
    // Debug is the shortest form that reads back, as `1.5`, `1e16`, or
    // `1e-7`; the grammar wants a `.` before any exponent.
    let text = format!("{value:?}");
    match text.find('e') {
        Some(e) if !text[..e].contains('.') => format!("{}.0{}", &text[..e], &text[e..]),
        _ => text,
    }
}

/// A string literal, escaping `"`, `\`, and control characters only.
fn quote(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The trailing section: files, then declarations' spans by id, then
/// nodes' by writing order.
fn write_spans(p: &Program, out: &mut String) {
    out.push_str("spans {\n");
    for (i, file) in p.files.iter().enumerate() {
        let _ = write!(out, "  file {i} ");
        quote(&file.path, out);
        out.push(' ');
        for byte in file.hash {
            let _ = write!(out, "{byte:02x}");
        }
        out.push_str(";\n");
    }
    for decl in p.decls.ids() {
        let d = p.decls.get(decl);
        if d.kind == DeclKind::Removed || !(d.span.is_known() || d.name_span.is_known()) {
            continue;
        }
        let _ = write!(out, "  #{}", decl.0);
        span(d.span, out);
        if d.name_span.is_known() {
            out.push_str(" name");
            span(d.name_span, out);
        }
        out.push_str(";\n");
    }
    for (n, id) in writing_order(p).into_iter().enumerate() {
        let derived = p.nodes.flags(id).contains(Flags::DERIVED);
        if !p.nodes.span(id).is_known() && !derived {
            continue;
        }
        let _ = write!(out, "  @{n}");
        span(p.nodes.span(id), out);
        if derived {
            out.push_str(" ~");
        }
        out.push_str(";\n");
    }
    out.push_str("}\n");
}

fn span(span: Span, out: &mut String) {
    if span.is_known() {
        let _ = write!(out, " {}:{}-{}", span.file.0, span.start, span.end);
    }
}
