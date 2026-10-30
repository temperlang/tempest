//! C#'s output tree and its printer.
//!
//! The printer lays code out in .NET's usual style, as Kotlin Temper's C#
//! output is laid out: four-space indents and braces on their own lines,
//! but with a file-scoped namespace (C# 10), and with blank lines between
//! a class's members. Parentheses go where precedence needs them, and
//! where they help a reader: around `&&` inside `||`, around a binary
//! operand of a bitwise operator or a shift, around `%` inside `+` or
//! `-`, and around a sum or difference inside a string concatenation
//! (`"n=" + (a + b)`). It doesn't break long lines.

use std::fmt::Write;

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bin {
    Or,
    And,
    BitOr,
    BitXor,
    BitAnd,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Bin {
    fn text(self) -> &'static str {
        match self {
            Bin::Or => "||",
            Bin::And => "&&",
            Bin::BitOr => "|",
            Bin::BitXor => "^",
            Bin::BitAnd => "&",
            Bin::Eq => "==",
            Bin::Ne => "!=",
            Bin::Lt => "<",
            Bin::Le => "<=",
            Bin::Gt => ">",
            Bin::Ge => ">=",
            Bin::Shl => "<<",
            Bin::Shr => ">>",
            Bin::Add => "+",
            Bin::Sub => "-",
            Bin::Mul => "*",
            Bin::Div => "/",
            Bin::Mod => "%",
        }
    }

    fn prec(self) -> u8 {
        match self {
            Bin::Or => OR,
            Bin::And => AND,
            Bin::BitOr => 5,
            Bin::BitXor => 6,
            Bin::BitAnd => 7,
            Bin::Eq | Bin::Ne => 8,
            Bin::Lt | Bin::Le | Bin::Gt | Bin::Ge => 9,
            Bin::Shl | Bin::Shr => 10,
            Bin::Add | Bin::Sub => ADD,
            Bin::Mul | Bin::Div | Bin::Mod => 12,
        }
    }

    fn bitwise(self) -> bool {
        matches!(
            self,
            Bin::BitOr | Bin::BitXor | Bin::BitAnd | Bin::Shl | Bin::Shr
        )
    }

    /// Whether it's arithmetic that can overflow, which C# checks when
    /// both operands are constants.
    pub fn arithmetic(self) -> bool {
        matches!(self, Bin::Add | Bin::Sub | Bin::Mul)
    }

    /// The comparison that's true exactly when this one is false, for
    /// operands that are never NaN.
    pub fn negated(self) -> Option<Bin> {
        Some(match self {
            Bin::Eq => Bin::Ne,
            Bin::Ne => Bin::Eq,
            Bin::Lt => Bin::Ge,
            Bin::Ge => Bin::Lt,
            Bin::Le => Bin::Gt,
            Bin::Gt => Bin::Le,
            _ => return None,
        })
    }
}

/// A prefix operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unary {
    Not,
    Neg,
    BitNot,
}

impl Unary {
    fn text(self) -> &'static str {
        match self {
            Unary::Not => "!",
            Unary::Neg => "-",
            Unary::BitNot => "~",
        }
    }
}

// Precedences, loosest first.
const TOP: u8 = 0;
const COND: u8 = 2;
const OR: u8 = 3;
const AND: u8 = 4;
const ADD: u8 = 11;
const UNARY: u8 = 14;
const CALL: u8 = 16;
const ATOM: u8 = 17;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Name(String),
    /// An `int` literal: `Int32`'s.
    Int(i64),
    /// A `long` literal: `Int64`'s, `5L`.
    Long(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Call(Box<Expr>, Vec<Expr>),
    Member(Box<Expr>, String),
    Bin(Bin, Box<Expr>, Box<Expr>),
    Unary(Unary, Box<Expr>),
    /// `cond ? then : els`.
    Cond {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
    /// String concatenation of strings, `a + b + c`.
    Concat(Vec<Expr>),
    /// An interpolated string, `$"a{b}c"`, its parts literal text or
    /// holes.
    Interpolated(Vec<Hole>),
    /// `unchecked(e)`: arithmetic on constants that may wrap, which C#
    /// otherwise rejects.
    Unchecked(Box<Expr>),
}

/// A part of an interpolated string.
#[derive(Clone, Debug, PartialEq)]
pub enum Hole {
    Text(String),
    Expr(Expr),
}

impl Expr {
    pub fn name(name: impl Into<String>) -> Expr {
        Expr::Name(name.into())
    }

    pub fn call(self, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(self), args)
    }

    pub fn member(self, name: impl Into<String>) -> Expr {
        Expr::Member(Box::new(self), name.into())
    }

    pub fn method(self, name: impl Into<String>, args: Vec<Expr>) -> Expr {
        self.member(name).call(args)
    }

    pub fn bin(op: Bin, left: Expr, right: Expr) -> Expr {
        Expr::Bin(op, Box::new(left), Box::new(right))
    }

    pub fn unary(op: Unary, operand: Expr) -> Expr {
        Expr::Unary(op, Box::new(operand))
    }

    fn prec(&self) -> u8 {
        match self {
            Expr::Int(n) | Expr::Long(n) if *n < 0 => UNARY,
            Expr::Float(f) if f.is_sign_negative() && !f.is_nan() => UNARY,
            Expr::Name(_)
            | Expr::Int(_)
            | Expr::Long(_)
            | Expr::Float(_)
            | Expr::Str(_)
            | Expr::Bool(_)
            | Expr::Interpolated(_)
            | Expr::Unchecked(_) => ATOM,
            Expr::Call(..) | Expr::Member(..) => CALL,
            Expr::Bin(op, ..) => op.prec(),
            Expr::Unary(..) => UNARY,
            Expr::Cond { .. } => COND,
            Expr::Concat(_) => ADD,
        }
    }

    /// Whether C# allows it as a statement: a call.
    pub fn is_statement(&self) -> bool {
        matches!(self, Expr::Call(..))
    }

    /// Whether it's a constant expression of numbers, which C# folds,
    /// checking for overflow.
    pub fn is_constant(&self) -> bool {
        match self {
            Expr::Int(_) | Expr::Long(_) => true,
            Expr::Unary(_, e) | Expr::Unchecked(e) => e.is_constant(),
            Expr::Bin(_, a, b) => a.is_constant() && b.is_constant(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    /// A call that always throws, `Core.Bubble();`, after which nothing
    /// runs.
    Bubble(Expr),
    /// A block of its own, which scopes its locals.
    Block(Vec<Stmt>),
    /// `ty name = value;`, or `ty name;`.
    Local {
        ty: String,
        name: String,
        value: Option<Expr>,
    },
    Assign {
        target: String,
        value: Expr,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        labels: Labels,
    },
    DoWhile {
        body: Vec<Stmt>,
        cond: Expr,
        labels: Labels,
    },
    Break,
    Continue,
    /// `goto label;`: a `break` or `continue` of a loop that isn't the
    /// innermost, which C# has no other way to write.
    Goto(String),
    Return(Option<Expr>),
    /// `lock (lock) { … }`.
    Lock {
        lock: Expr,
        body: Vec<Stmt>,
    },
    /// `try { … } finally { … }`.
    Try {
        body: Vec<Stmt>,
        finally: Vec<Stmt>,
    },
}

/// A loop's labels, for `goto`s from inner loops: one at the end of its
/// body, which continues it, and one after it, which breaks it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Labels {
    pub next: Option<String>,
    pub end: Option<String>,
}

/// A static field, declared without a value, as Kotlin's are: the static
/// constructor gives it one; or, with a value, the module's lock.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub access: Access,
    pub readonly: bool,
    pub volatile: bool,
    pub ty: String,
    pub name: String,
    pub value: Option<Expr>,
}

/// Who may use a member: anyone, the library (and its tests' project,
/// as Kotlin's), or only its class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Public,
    Internal,
    Private,
}

impl Access {
    fn text(self) -> &'static str {
        match self {
            Access::Public => "public ",
            Access::Internal => "internal ",
            Access::Private => "private ",
        }
    }
}

/// A static method.
#[derive(Clone, Debug, PartialEq)]
pub struct Method {
    pub public: bool,
    pub result: String,
    pub name: String,
    /// Each parameter's type and name.
    pub params: Vec<(String, String)>,
    pub body: Vec<Stmt>,
}

/// One `.cs` file: its `using`s, its namespace, and one static class.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Class {
    pub usings: Vec<String>,
    pub namespace: String,
    pub name: String,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    /// The static constructor's statements: none, no constructor.
    pub init: Vec<Stmt>,
}

/// Whether a character is escaped in a literal: controls, line
/// terminators (which C# rejects in a literal), and characters that
/// don't show, or show as something else: format characters, private
/// use, and noncharacters.
fn hidden(c: char) -> bool {
    let n = c as u32;
    n < 0x20
        || (0x7f..0xa0).contains(&n)
        || matches!(
            c,
            '\u{ad}'
                | '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
                | '\u{e000}'..='\u{f8ff}'
                | '\u{fdd0}'..='\u{fdef}'
                | '\u{f0000}'..
        )
        || (n & 0xfffe) == 0xfffe
}

fn escape_into(out: &mut String, text: &str, interpolated: bool) {
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '{' if interpolated => out.push_str("{{"),
            '}' if interpolated => out.push_str("}}"),
            _ if hidden(c) && (c as u32) < 0x10000 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            _ if hidden(c) => {
                let _ = write!(out, "\\U{:08X}", c as u32);
            }
            _ => out.push(c),
        }
    }
}

/// Prints a string literal.
pub fn string_literal(text: &str) -> String {
    let mut out = String::from("\"");
    escape_into(&mut out, text, false);
    out.push('"');
    out
}

fn float_literal(f: f64) -> String {
    if f.is_nan() {
        "double.NaN".into()
    } else if f.is_infinite() {
        if f > 0.0 {
            "double.PositiveInfinity".into()
        } else {
            "double.NegativeInfinity".into()
        }
    } else {
        // Rust's `{:?}` gives `1.0`, `0.1`, and `1e100`, all C#'s too.
        format!("{f:?}")
    }
}

/// Whether a binary operand needs parentheses beyond what precedence
/// says, for clarity.
fn clarify(parent: Bin, child: &Expr, right: bool) -> bool {
    let Expr::Bin(op, ..) = child else {
        return false;
    };
    let (pp, cp) = (parent.prec(), op.prec());
    if parent.bitwise() {
        return true;
    }
    if pp == cp {
        // Same level: left-associative chains only of operators that
        // flatten (`a + b - c`, `a * b / c`), never `%` mixed in.
        return right
            || matches!(parent, Bin::Eq | Bin::Ne)
            || (parent != *op && (parent == Bin::Mod || *op == Bin::Mod))
            || (parent == Bin::Mul && *op == Bin::Div)
            || (parent == Bin::Div && *op == Bin::Mul);
    }
    (parent == Bin::Or && *op == Bin::And)
        || (*op == Bin::Mod && matches!(parent, Bin::Add | Bin::Sub))
}

/// Prints an expression, parenthesized if it binds looser than `min`.
pub fn expr(e: &Expr, min: u8) -> String {
    let text = match e {
        Expr::Name(n) => n.clone(),
        Expr::Int(n) => n.to_string(),
        Expr::Long(n) => format!("{n}L"),
        Expr::Float(f) => float_literal(*f),
        Expr::Str(s) => string_literal(s),
        Expr::Bool(b) => b.to_string(),
        Expr::Call(f, args) => {
            let args: Vec<String> = args.iter().map(|a| expr(a, COND)).collect();
            format!("{}({})", expr(f, CALL), args.join(", "))
        }
        Expr::Member(obj, name) => format!("{}.{name}", expr(obj, CALL)),
        Expr::Bin(op, left, right) => {
            let p = op.prec();
            let side = |e: &Expr, min: u8, right: bool| {
                if clarify(*op, e, right) {
                    format!("({})", expr(e, TOP))
                } else {
                    expr(e, min)
                }
            };
            format!(
                "{} {} {}",
                side(left, p, false),
                op.text(),
                side(right, p + 1, true)
            )
        }
        Expr::Unary(op, operand) => {
            let inner = expr(operand, UNARY);
            // `- -a` and `--a` read badly or mean something else.
            if inner.starts_with(['-', '+']) && *op == Unary::Neg {
                format!("-({inner})")
            } else {
                format!("{}{inner}", op.text())
            }
        }
        Expr::Cond { cond, then, els } => format!(
            "{} ? {} : {}",
            expr(cond, COND + 1),
            expr(then, COND + 1),
            expr(els, COND)
        ),
        Expr::Concat(parts) => {
            let parts: Vec<String> = parts.iter().map(|p| expr(p, ADD + 1)).collect();
            parts.join(" + ")
        }
        Expr::Interpolated(holes) => {
            let mut out = String::from("$\"");
            for hole in holes {
                match hole {
                    Hole::Text(text) => escape_into(&mut out, text, true),
                    // A conditional's `:`, or `global::`'s, would start a
                    // format, so they take parentheses.
                    Hole::Expr(e) => {
                        let text = expr(e, COND + 1);
                        if text.contains("::") {
                            let _ = write!(out, "{{({text})}}");
                        } else {
                            let _ = write!(out, "{{{text}}}");
                        }
                    }
                }
            }
            out.push('"');
            out
        }
        Expr::Unchecked(e) => format!("unchecked({})", expr(e, TOP)),
    };
    if e.prec() < min {
        format!("({text})")
    } else {
        text
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("    ");
    }
}

/// A block, its braces on their own lines, starting where `out` is (at
/// the end of a line, or indented).
fn block(out: &mut String, body: &[Stmt], depth: usize) {
    indent(out, depth);
    out.push_str("{\n");
    for s in body {
        stmt(out, s, depth + 1);
    }
    indent(out, depth);
    out.push_str("}\n");
}

/// A label on an empty statement, where a `goto` lands.
fn label(out: &mut String, label: &Option<String>, depth: usize) {
    if let Some(l) = label {
        indent(out, depth);
        let _ = writeln!(out, "{l}: ;");
    }
}

/// A loop's body, with its label for `continue` at the end.
fn loop_body(out: &mut String, body: &[Stmt], labels: &Labels, depth: usize) {
    indent(out, depth);
    out.push_str("{\n");
    for s in body {
        stmt(out, s, depth + 1);
    }
    label(out, &labels.next, depth + 1);
    indent(out, depth);
    out.push_str("}\n");
}

fn stmt(out: &mut String, s: &Stmt, depth: usize) {
    if let Stmt::Block(body) = s {
        block(out, body, depth);
        return;
    }
    indent(out, depth);
    match s {
        Stmt::Expr(e) | Stmt::Bubble(e) => {
            let _ = writeln!(out, "{};", expr(e, TOP));
        }
        Stmt::Block(_) => {}
        Stmt::Local { ty, name, value } => {
            let _ = write!(out, "{ty} {name}");
            if let Some(v) = value {
                let _ = write!(out, " = {}", expr(v, COND));
            }
            out.push_str(";\n");
        }
        Stmt::Assign { target, value } => {
            let _ = writeln!(out, "{target} = {};", expr(value, COND));
        }
        Stmt::If { cond, then, els } => {
            let _ = writeln!(out, "if ({})", expr(cond, TOP));
            block(out, then, depth);
            let mut els = els;
            loop {
                match els.as_slice() {
                    [] => break,
                    [
                        Stmt::If {
                            cond,
                            then,
                            els: next,
                        },
                    ] => {
                        indent(out, depth);
                        let _ = writeln!(out, "else if ({})", expr(cond, TOP));
                        block(out, then, depth);
                        els = next;
                    }
                    _ => {
                        indent(out, depth);
                        out.push_str("else\n");
                        block(out, els, depth);
                        break;
                    }
                }
            }
        }
        Stmt::While { cond, body, labels } => {
            let _ = writeln!(out, "while ({})", expr(cond, TOP));
            loop_body(out, body, labels, depth);
            label(out, &labels.end, depth);
        }
        Stmt::DoWhile { body, cond, labels } => {
            out.push_str("do\n");
            loop_body(out, body, labels, depth);
            // `} while (…);`, as .NET's formatter has it.
            out.pop();
            let _ = writeln!(out, " while ({});", expr(cond, TOP));
            label(out, &labels.end, depth);
        }
        Stmt::Break => out.push_str("break;\n"),
        Stmt::Continue => out.push_str("continue;\n"),
        Stmt::Goto(l) => {
            let _ = writeln!(out, "goto {l};");
        }
        Stmt::Return(None) => out.push_str("return;\n"),
        Stmt::Return(Some(e)) => {
            let _ = writeln!(out, "return {};", expr(e, TOP));
        }
        Stmt::Lock { lock, body } => {
            let _ = writeln!(out, "lock ({})", expr(lock, TOP));
            block(out, body, depth);
        }
        Stmt::Try { body, finally } => {
            out.push_str("try\n");
            block(out, body, depth);
            indent(out, depth);
            out.push_str("finally\n");
            block(out, finally, depth);
        }
    }
}

/// A module's tests, as an MSTest class: each test a method.
pub struct TestClass {
    pub usings: Vec<String>,
    pub namespace: String,
    pub name: String,
    /// Each test's method name and body.
    pub methods: Vec<(String, Vec<Stmt>)>,
}

/// Prints a test class's file.
pub fn test_class(c: &TestClass) -> String {
    let mut out = String::new();
    for using in &c.usings {
        let _ = writeln!(out, "using {using};");
    }
    if !c.usings.is_empty() {
        out.push('\n');
    }
    let _ = writeln!(out, "namespace {};\n", c.namespace);
    let _ = writeln!(out, "[TestClass]\npublic class {}\n{{", c.name);
    for (i, (name, body)) in c.methods.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let _ = writeln!(out, "    [TestMethod]\n    public void {name}()");
        block(&mut out, body, 1);
    }
    out.push_str("}\n");
    out
}

/// Prints a class's file.
pub fn class(c: &Class) -> String {
    let mut out = String::new();
    for using in &c.usings {
        let _ = writeln!(out, "using {using};");
    }
    if !c.usings.is_empty() {
        out.push('\n');
    }
    let _ = writeln!(out, "namespace {};\n", c.namespace);
    let _ = writeln!(out, "public static class {}\n{{", c.name);
    let mut first = true;
    let mut gap = |out: &mut String| {
        if !first {
            out.push('\n');
        }
        first = false;
    };
    if !c.fields.is_empty() {
        gap(&mut out);
    }
    for f in &c.fields {
        out.push_str("    ");
        out.push_str(f.access.text());
        out.push_str("static ");
        if f.readonly {
            out.push_str("readonly ");
        }
        if f.volatile {
            out.push_str("volatile ");
        }
        let _ = write!(out, "{} {}", f.ty, f.name);
        if let Some(v) = &f.value {
            let _ = write!(out, " = {}", expr(v, COND));
        }
        out.push_str(";\n");
    }
    for m in &c.methods {
        gap(&mut out);
        out.push_str("    ");
        out.push_str(if m.public { "public " } else { "internal " });
        let params: Vec<String> = m.params.iter().map(|(t, n)| format!("{t} {n}")).collect();
        let _ = writeln!(out, "static {} {}({})", m.result, m.name, params.join(", "));
        block(&mut out, &m.body, 1);
    }
    if !c.init.is_empty() {
        gap(&mut out);
        let _ = writeln!(out, "    static {}()", c.name);
        block(&mut out, &c.init, 1);
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Expr {
        Expr::name(s)
    }

    #[test]
    fn precedence() {
        let sum = Expr::bin(Bin::Add, n("a"), n("b"));
        assert_eq!(
            expr(&Expr::bin(Bin::Shr, sum.clone(), Expr::Int(1)), 0),
            "(a + b) >> 1"
        );
        assert_eq!(
            expr(&Expr::bin(Bin::Sub, n("c"), sum.clone()), 0),
            "c - (a + b)"
        );
        let and = Expr::bin(Bin::And, n("a"), n("b"));
        assert_eq!(expr(&Expr::bin(Bin::Or, and, n("c")), 0), "(a && b) || c");
        assert_eq!(expr(&Expr::unary(Unary::Neg, Expr::Int(-2)), 0), "-(-2)");
        assert_eq!(
            expr(&Expr::Int(-3).method("CompareTo", vec![Expr::Int(5)]), 0),
            "(-3).CompareTo(5)"
        );
        let cond = Expr::Cond {
            cond: Box::new(n("c")),
            then: Box::new(Expr::Str("a".into())),
            els: Box::new(n("b")),
        };
        assert_eq!(
            expr(
                &Expr::Interpolated(vec![
                    Hole::Text("{x} ".into()),
                    Hole::Expr(cond),
                    Hole::Expr(sum)
                ]),
                0
            ),
            "$\"{{x}} {(c ? \"a\" : b)}{a + b}\""
        );
        let qualified = n("global::System.Math").method("Abs", vec![n("a")]);
        assert_eq!(
            expr(&Expr::Interpolated(vec![Hole::Expr(qualified)]), 0),
            "$\"{(global::System.Math.Abs(a))}\""
        );
        assert_eq!(expr(&Expr::Long(-5), 0), "-5L");
    }

    #[test]
    fn strings() {
        assert_eq!(string_literal("it's"), "\"it's\"");
        assert_eq!(string_literal("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(
            string_literal("a\n\\\u{2028}\u{1}\u{0}"),
            "\"a\\n\\\\\\u2028\\u0001\\0\""
        );
        assert_eq!(
            string_literal("\u{e000}\u{ffff}\u{10ffff}\u{1f600}"),
            "\"\\uE000\\uFFFF\\U0010FFFF\u{1f600}\""
        );
    }
}
