//! C++'s output tree and its printer.
//!
//! The printer lays code out as Google's C++ style does, which Kotlin
//! Temper's C++ tours follow: two-space indents, braces on the line that
//! opens them (`} else {`), a namespace's members not indented, and blank
//! lines between functions. Parentheses go where precedence needs them,
//! and where they help a reader, as in the other backends: around `&&`
//! inside `||`, around a binary operand of a bitwise operator, and around
//! `%` inside `+` or `-`. It doesn't break long lines.

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

    /// Whether it compares.
    pub fn comparison(self) -> bool {
        matches!(
            self,
            Bin::Eq | Bin::Ne | Bin::Lt | Bin::Le | Bin::Gt | Bin::Ge
        )
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
    /// A name, maybe qualified (`core::Int::add`) or with template
    /// arguments.
    Name(String),
    /// An `int32_t` literal, `INT32_MIN` for the one C++ can't write as a
    /// literal.
    Int(i32),
    /// An `int64_t` literal: `int64_t{5}`, or `INT64_MIN`.
    Long(i64),
    Float(f64),
    /// A string literal: a `const char*` in C++, which converts to
    /// `std::string` where one is wanted.
    Str(String),
    Bool(bool),
    Call(Box<Expr>, Vec<Expr>),
    /// A member of an object: `s.empty`.
    Member(Box<Expr>, String),
    Bin(Bin, Box<Expr>, Box<Expr>),
    Unary(Unary, Box<Expr>),
    /// `cond ? then : els`.
    Cond {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
    /// String concatenation of `std::string`s and literals, `a + b + c`,
    /// never two literals in a row.
    Concat(Vec<Expr>),
}

impl Expr {
    pub fn name(name: impl Into<String>) -> Expr {
        Expr::Name(name.into())
    }

    pub fn call(self, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(self), args)
    }

    pub fn method(self, name: impl Into<String>, args: Vec<Expr>) -> Expr {
        Expr::Member(Box::new(self), name.into()).call(args)
    }

    pub fn bin(op: Bin, left: Expr, right: Expr) -> Expr {
        Expr::Bin(op, Box::new(left), Box::new(right))
    }

    pub fn unary(op: Unary, operand: Expr) -> Expr {
        Expr::Unary(op, Box::new(operand))
    }

    fn prec(&self) -> u8 {
        match self {
            Expr::Int(n) if *n < 0 && *n != i32::MIN => UNARY,
            Expr::Float(f) if f.is_sign_negative() && f.is_finite() => UNARY,
            Expr::Name(_)
            | Expr::Int(_)
            | Expr::Long(_)
            | Expr::Float(_)
            | Expr::Str(_)
            | Expr::Bool(_) => ATOM,
            Expr::Call(..) | Expr::Member(..) => CALL,
            Expr::Bin(op, ..) => op.prec(),
            Expr::Unary(..) => UNARY,
            Expr::Cond { .. } => COND,
            Expr::Concat(_) => ADD,
        }
    }

    /// Whether it's a `const char*` rather than a `std::string`: a string
    /// literal, or a choice of two.
    pub fn is_c_string(&self) -> bool {
        match self {
            // One with a NUL is printed as a `std::string` already.
            Expr::Str(s) => !s.contains('\0'),
            Expr::Cond { then, els, .. } => then.is_c_string() && els.is_c_string(),
            _ => false,
        }
    }

    /// A `std::string` for a string-typed expression: a literal as
    /// `std::string("…")`, where C++ would otherwise compare or add
    /// pointers, or deduce `const char*`.
    pub fn std_string(self) -> Expr {
        match self {
            Expr::Str(_) if self.is_c_string() => Expr::name("std::string").call(vec![self]),
            Expr::Cond { cond, then, els } if then.is_c_string() && els.is_c_string() => {
                Expr::Cond {
                    cond,
                    then: Box::new(then.std_string()),
                    els,
                }
            }
            e => e,
        }
    }

    /// Whether C++ allows it as a statement worth writing: a call.
    pub fn is_statement(&self) -> bool {
        matches!(self, Expr::Call(..))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    /// A call that always throws, `core::bubble();`, after which nothing
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
    /// innermost, which C++ has no other way to write.
    Goto(String),
    Return(Option<Expr>),
    /// `std::lock_guard<std::mutex> name(mutex);`: holds the module's lock
    /// to the end of its block.
    Guard {
        name: String,
        mutex: String,
    },
}

/// A loop's labels, for `goto`s from inner loops: one at the end of its
/// body, which continues it, and one after it, which breaks it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Labels {
    pub next: Option<String>,
    pub end: Option<String>,
}

/// A function: `static` unless exported, which the header declares.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub exported: bool,
    pub result: String,
    pub name: String,
    /// Each parameter's type and name.
    pub params: Vec<(String, String)>,
    pub body: Vec<Stmt>,
}

impl Function {
    fn signature(&self) -> String {
        let params: Vec<String> = (self.params.iter())
            .map(|(t, n)| format!("{t} {n}"))
            .collect();
        format!("{} {}({})", self.result, self.name, params.join(", "))
    }
}

/// A part of a module's `.cpp`, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A namespace-scope variable, declared without a value (the init
    /// function gives it one): `static` unless exported, which the header
    /// declares `extern`.
    Variable {
        exported: bool,
        ty: String,
        name: String,
    },
    /// Declares a function that's used before it's defined.
    Prototype(Function),
    Function(Function),
    /// Text as it is, such as a helper's definition.
    Raw(String),
}

/// A module's `.cpp` file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Source {
    /// What it includes, its header first: `<module-state/init.hpp>`.
    pub includes: Vec<String>,
    pub namespace: String,
    /// Namespace aliases, as `(alias, namespace)`.
    pub aliases: Vec<(String, String)>,
    pub items: Vec<Item>,
    /// The init function's name, and its body, which runs once.
    pub init: String,
    pub init_body: Vec<Stmt>,
}

/// Whether a character is escaped in a literal: controls, and characters
/// that don't show, or show as something else: format characters,
/// private use, and noncharacters.
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

/// Prints a string literal, in UTF-8, as C++ compilers read sources by
/// default and as `std::string` holds Temper's strings. A `?` after a `?`
/// is escaped, since C++14 still reads trigraphs (`??=`), and a string
/// with a NUL in it is a `std::string` of its length, since a literal
/// would end there.
pub fn string_literal(text: &str) -> String {
    let mut out = String::from("\"");
    let mut prev = '\0';
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Three digits, so a digit after it isn't read as octal.
            '\0' => out.push_str("\\000"),
            '?' if prev == '?' => out.push_str("\\?"),
            _ if hidden(c) && (c as u32) < 0x10000 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            _ if hidden(c) => {
                let _ = write!(out, "\\U{:08X}", c as u32);
            }
            _ => out.push(c),
        }
        prev = c;
    }
    out.push('"');
    if text.contains('\0') {
        return format!("std::string({out}, {})", text.len());
    }
    out
}

fn float_literal(f: f64) -> String {
    if f.is_nan() {
        "std::numeric_limits<double>::quiet_NaN()".into()
    } else if f.is_infinite() {
        if f > 0.0 {
            "std::numeric_limits<double>::infinity()".into()
        } else {
            "-std::numeric_limits<double>::infinity()".into()
        }
    } else {
        // Rust's `{:?}` gives `1.0`, `0.1`, and `1e100`, all C++'s too.
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
        Expr::Int(i32::MIN) => "INT32_MIN".into(),
        Expr::Int(n) => n.to_string(),
        Expr::Long(i64::MIN) => "INT64_MIN".into(),
        Expr::Long(n) => format!("int64_t{{{n}}}"),
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
    };
    if e.prec() < min {
        format!("({text})")
    } else {
        text
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// Statements inside braces, the `{` already written, and the closing
/// `}` without a newline.
fn body(out: &mut String, stmts: &[Stmt], depth: usize) {
    out.push_str("{\n");
    for s in stmts {
        stmt(out, s, depth + 1);
    }
    indent(out, depth);
    out.push('}');
}

/// A label on an empty statement, where a `goto` lands.
fn label(out: &mut String, label: &Option<String>, depth: usize) {
    if let Some(l) = label {
        indent(out, depth);
        let _ = writeln!(out, "{l}:;");
    }
}

/// A loop's body, with its label for `continue` at the end, after the
/// body's own block, so a `goto` there never skips a local's
/// initialization in the scope it lands in.
fn loop_body(out: &mut String, stmts: &[Stmt], labels: &Labels, depth: usize) {
    match &labels.next {
        None => body(out, stmts, depth),
        Some(_) => {
            out.push_str("{\n");
            indent(out, depth + 1);
            body(out, stmts, depth + 1);
            out.push('\n');
            label(out, &labels.next, depth + 1);
            indent(out, depth);
            out.push('}');
        }
    }
}

fn stmt(out: &mut String, s: &Stmt, depth: usize) {
    indent(out, depth);
    match s {
        Stmt::Expr(e) | Stmt::Bubble(e) => {
            let _ = writeln!(out, "{};", expr(e, TOP));
        }
        Stmt::Block(stmts) => {
            body(out, stmts, depth);
            out.push('\n');
        }
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
            let _ = write!(out, "if ({}) ", expr(cond, TOP));
            body(out, then, depth);
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
                        let _ = write!(out, " else if ({}) ", expr(cond, TOP));
                        body(out, then, depth);
                        els = next;
                    }
                    _ => {
                        out.push_str(" else ");
                        body(out, els, depth);
                        break;
                    }
                }
            }
            out.push('\n');
        }
        Stmt::While {
            cond,
            body: stmts,
            labels,
        } => {
            let _ = write!(out, "while ({}) ", expr(cond, TOP));
            loop_body(out, stmts, labels, depth);
            out.push('\n');
            label(out, &labels.end, depth);
        }
        Stmt::DoWhile {
            body: stmts,
            cond,
            labels,
        } => {
            out.push_str("do ");
            loop_body(out, stmts, labels, depth);
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
        Stmt::Guard { name, mutex } => {
            let _ = writeln!(out, "std::lock_guard<std::mutex> {name}({mutex});");
        }
    }
}

/// Prints a module's `.cpp`.
pub fn source(s: &Source) -> String {
    let mut out = String::new();
    let (header, rest) = s
        .includes
        .split_first()
        .map_or((None, &[][..]), |(h, r)| (Some(h), r));
    if let Some(h) = header {
        let _ = writeln!(out, "#include {h}\n");
    }
    for include in rest {
        let _ = writeln!(out, "#include {include}");
    }
    if !rest.is_empty() {
        out.push('\n');
    }
    let _ = writeln!(out, "namespace {} {{\n", s.namespace);
    for (alias, namespace) in &s.aliases {
        let _ = writeln!(out, "namespace {alias} = {namespace};");
    }
    if !s.aliases.is_empty() {
        out.push('\n');
    }
    // Variables and prototypes in runs, blank lines around the rest.
    let mut prev: Option<&Item> = None;
    for item in &s.items {
        let run = matches!(
            (prev, item),
            (Some(Item::Variable { .. }), Item::Variable { .. })
                | (Some(Item::Prototype(_)), Item::Prototype(_))
        );
        if prev.is_some() && !run {
            out.push('\n');
        }
        match item {
            Item::Variable { exported, ty, name } => {
                let storage = if *exported { "" } else { "static " };
                let _ = writeln!(out, "{storage}{ty} {name};");
            }
            Item::Prototype(f) => {
                let _ = writeln!(out, "static {};", f.signature());
            }
            Item::Function(f) => {
                if !f.exported {
                    out.push_str("static ");
                }
                let _ = write!(out, "{} ", f.signature());
                body(&mut out, &f.body, 0);
                out.push('\n');
            }
            Item::Raw(text) => out.push_str(text),
        }
        prev = Some(item);
    }
    if prev.is_some() {
        out.push('\n');
    }
    // The init function: its body runs once, however many threads call
    // it at once; a call while it runs waits for it to finish.
    let _ = writeln!(out, "void {}() {{", s.init);
    out.push_str("  static std::once_flag once;\n");
    if s.init_body.is_empty() {
        out.push_str("  std::call_once(once, [] {});\n");
    } else {
        out.push_str("  std::call_once(once, [] ");
        body(&mut out, &s.init_body, 1);
        out.push_str(");\n");
    }
    out.push_str("}\n");
    let _ = writeln!(out, "\n}}  // namespace {}", s.namespace);
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
            expr(&Expr::bin(Bin::BitAnd, sum.clone(), Expr::Int(1)), 0),
            "(a + b) & 1"
        );
        assert_eq!(
            expr(&Expr::bin(Bin::Sub, n("c"), sum.clone()), 0),
            "c - (a + b)"
        );
        let and = Expr::bin(Bin::And, n("a"), n("b"));
        assert_eq!(expr(&Expr::bin(Bin::Or, and, n("c")), 0), "(a && b) || c");
        assert_eq!(expr(&Expr::unary(Unary::Neg, Expr::Int(-2)), 0), "-(-2)");
        assert_eq!(expr(&Expr::Int(i32::MIN), 0), "INT32_MIN");
        assert_eq!(expr(&Expr::Long(-5), 0), "int64_t{-5}");
        assert_eq!(expr(&Expr::Long(i64::MIN), 0), "INT64_MIN");
        assert_eq!(
            expr(&Expr::Float(f64::NAN), 0),
            "std::numeric_limits<double>::quiet_NaN()"
        );
        assert_eq!(
            expr(
                &Expr::Str("a".into()).std_string().method("empty", vec![]),
                0
            ),
            "std::string(\"a\").empty()"
        );
    }

    #[test]
    fn strings() {
        assert_eq!(string_literal("it's"), "\"it's\"");
        assert_eq!(string_literal("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(string_literal("??=?"), "\"?\\?=?\"");
        assert_eq!(
            string_literal("a\n\\\u{2028}\u{1}"),
            "\"a\\n\\\\\\u2028\\u0001\""
        );
        assert_eq!(
            string_literal("\u{e000}\u{ffff}\u{10ffff}\u{1f600}"),
            "\"\\uE000\\uFFFF\\U0010FFFF\u{1f600}\""
        );
        assert_eq!(string_literal("a\u{0}1"), "std::string(\"a\\0001\", 3)");
    }
}
