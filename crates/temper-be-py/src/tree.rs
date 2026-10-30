//! Python's output tree and its printer.
//!
//! No printing crate is shared among backends yet, so the tree and
//! printer live here; they'd move into one once a second backend wants
//! the same layout machinery (notes/tmpl/overview.md, "Boundaries").
//! The printer adds parentheses only where Python's precedence needs
//! them, and lays out code as PEP 8 does: four-space indents, and two
//! blank lines around top-level functions.

use std::fmt::Write;

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bin {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    BitOr,
    BitXor,
    BitAnd,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

impl Bin {
    fn text(self) -> &'static str {
        match self {
            Bin::Or => "or",
            Bin::And => "and",
            Bin::Eq => "==",
            Bin::Ne => "!=",
            Bin::Lt => "<",
            Bin::Le => "<=",
            Bin::Gt => ">",
            Bin::Ge => ">=",
            Bin::BitOr => "|",
            Bin::BitXor => "^",
            Bin::BitAnd => "&",
            Bin::Shl => "<<",
            Bin::Shr => ">>",
            Bin::Add => "+",
            Bin::Sub => "-",
            Bin::Mul => "*",
            Bin::Div => "/",
            Bin::Mod => "%",
            Bin::Pow => "**",
        }
    }

    fn prec(self) -> u8 {
        match self {
            Bin::Or => OR,
            Bin::And => AND,
            Bin::Eq | Bin::Ne | Bin::Lt | Bin::Le | Bin::Gt | Bin::Ge => COMPARE,
            Bin::BitOr => 6,
            Bin::BitXor => 7,
            Bin::BitAnd => 8,
            Bin::Shl | Bin::Shr => 9,
            Bin::Add | Bin::Sub => 10,
            Bin::Mul | Bin::Div | Bin::Mod => 11,
            Bin::Pow => POW,
        }
    }
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unary {
    Not,
    Neg,
    Invert,
}

// Precedences, loosest first, as Python's grammar has them.
const TOP: u8 = 0;
const IF_ELSE: u8 = 1;
const OR: u8 = 2;
const AND: u8 = 3;
const NOT: u8 = 4;
const COMPARE: u8 = 5;
const UNARY: u8 = 12;
const POW: u8 = 13;
const PRIMARY: u8 = 15;
const ATOM: u8 = 16;

/// A part of an f-string.
#[derive(Clone, Debug, PartialEq)]
pub enum FPart {
    Lit(String),
    /// An expression; `str` says it needs `str(…)` if the f-string falls
    /// back to `+`, since its value isn't a string (as `{n}` for an int).
    Expr {
        expr: Expr,
        str: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Name(String),
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    Call(Box<Expr>, Vec<Expr>),
    Attr(Box<Expr>, String),
    Bin(Bin, Box<Expr>, Box<Expr>),
    Unary(Unary, Box<Expr>),
    /// `then if cond else els`, written in that order.
    IfElse {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
    /// An f-string, or where its parts can't go in one in Python 3.11
    /// (quotes or backslashes inside the braces), a `+` chain.
    FString(Vec<FPart>),
}

impl Expr {
    pub fn name(name: impl Into<String>) -> Expr {
        Expr::Name(name.into())
    }

    pub fn call(self, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(self), args)
    }

    pub fn attr(self, name: impl Into<String>) -> Expr {
        Expr::Attr(Box::new(self), name.into())
    }

    pub fn bin(op: Bin, left: Expr, right: Expr) -> Expr {
        Expr::Bin(op, Box::new(left), Box::new(right))
    }

    pub fn unary(op: Unary, operand: Expr) -> Expr {
        Expr::Unary(op, Box::new(operand))
    }

    fn prec(&self) -> u8 {
        match self {
            Expr::Int(n) if *n < 0 => UNARY,
            Expr::Float(f) if f.is_sign_negative() && f.is_finite() => UNARY,
            Expr::Float(f) if !f.is_finite() => PRIMARY,
            Expr::Name(_)
            | Expr::Int(_)
            | Expr::Float(_)
            | Expr::Str(_)
            | Expr::Bool(_)
            | Expr::None
            | Expr::FString(_) => ATOM,
            Expr::Call(..) | Expr::Attr(..) => PRIMARY,
            Expr::Bin(op, ..) => op.prec(),
            Expr::Unary(Unary::Not, _) => NOT,
            Expr::Unary(..) => UNARY,
            Expr::IfElse { .. } => IF_ELSE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    /// `target: ann = value`, or without the annotation.
    Assign {
        target: String,
        ann: Option<String>,
        value: Expr,
    },
    /// `target: ann`, declared without a value.
    Declare {
        target: String,
        ann: String,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Return(Option<Expr>),
    Global(Vec<String>),
    /// A function; its parameters are positional-only, as Kotlin's are.
    Def {
        name: String,
        params: Vec<(String, String)>,
        result: String,
        body: Vec<Stmt>,
    },
}

/// One `.py` file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Module {
    /// `from MODULE import NAME, …`, in order.
    pub imports: Vec<(String, Vec<String>)>,
    pub body: Vec<Stmt>,
}

/// Prints a string literal, preferring single quotes, as Python's `repr`.
pub fn string_literal(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    escape_into(&mut out, text, quote, false);
    out.push(quote);
    out
}

fn escape_into(out: &mut String, text: &str, quote: char, braces: bool) {
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '{' if braces => out.push_str("{{"),
            '}' if braces => out.push_str("}}"),
            _ if c == quote => {
                out.push('\\');
                out.push(c);
            }
            _ if (c as u32) < 0x20 || (0x7f..0xa0).contains(&(c as u32)) => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            '\u{ad}'
            | '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            _ => out.push(c),
        }
    }
}

fn float_literal(f: f64) -> String {
    if f.is_nan() {
        "float('nan')".into()
    } else if f.is_infinite() {
        if f > 0.0 {
            "float('inf')".into()
        } else {
            "float('-inf')".into()
        }
    } else {
        format!("{f:?}")
    }
}

/// Prints an expression, parenthesized if it binds looser than `min`.
pub fn expr(e: &Expr, min: u8) -> String {
    let text = match e {
        Expr::Name(n) => n.clone(),
        Expr::Int(n) => n.to_string(),
        Expr::Float(f) => float_literal(*f),
        Expr::Str(s) => string_literal(s),
        Expr::Bool(true) => "True".into(),
        Expr::Bool(false) => "False".into(),
        Expr::None => "None".into(),
        Expr::Call(f, args) => {
            let args: Vec<String> = args.iter().map(|a| expr(a, IF_ELSE)).collect();
            format!("{}({})", expr(f, PRIMARY), args.join(", "))
        }
        Expr::Attr(obj, name) => format!("{}.{name}", expr(obj, PRIMARY)),
        Expr::Bin(op, left, right) => {
            let p = op.prec();
            let (lmin, rmin) = match op {
                Bin::Pow => (p + 1, UNARY),
                _ if p == COMPARE => (p + 1, p + 1),
                _ => (p, p + 1),
            };
            format!("{} {} {}", expr(left, lmin), op.text(), expr(right, rmin))
        }
        Expr::Unary(Unary::Not, operand) => format!("not {}", expr(operand, NOT)),
        Expr::Unary(Unary::Neg, operand) => format!("-{}", expr(operand, UNARY)),
        Expr::Unary(Unary::Invert, operand) => format!("~{}", expr(operand, UNARY)),
        Expr::IfElse { cond, then, els } => format!(
            "{} if {} else {}",
            expr(then, IF_ELSE + 1),
            expr(cond, IF_ELSE + 1),
            expr(els, IF_ELSE)
        ),
        Expr::FString(parts) => fstring(parts, min),
    };
    if e.prec() < min {
        format!("({text})")
    } else {
        text
    }
}

/// An f-string, or a `+` chain where Python 3.11 can't take the parts in
/// braces.
fn fstring(parts: &[FPart], min: u8) -> String {
    let exprs: Vec<String> = parts
        .iter()
        .map(|p| match p {
            FPart::Lit(_) => String::new(),
            FPart::Expr { expr: e, .. } => expr(e, IF_ELSE),
        })
        .collect();
    let lits: String = parts
        .iter()
        .filter_map(|p| match p {
            FPart::Lit(s) => Some(s.as_str()),
            FPart::Expr { .. } => None,
        })
        .collect();
    let inner = |q: char| {
        exprs
            .iter()
            .all(|e| !e.contains([q, '\\', '{', '}', '\n']) && !e.starts_with('='))
    };
    let preferred = if lits.contains('\'') && !lits.contains('"') {
        '"'
    } else {
        '\''
    };
    let other = if preferred == '\'' { '"' } else { '\'' };
    let quote = [preferred, other].into_iter().find(|&q| inner(q));
    if let Some(q) = quote {
        let mut out = format!("f{q}");
        for (part, e) in parts.iter().zip(&exprs) {
            match part {
                FPart::Lit(s) => escape_into(&mut out, s, q, true),
                FPart::Expr { .. } => {
                    out.push('{');
                    out.push_str(e);
                    out.push('}');
                }
            }
        }
        out.push(q);
        return out;
    }
    // Fall back to `+`.
    let terms: Vec<Expr> = parts
        .iter()
        .map(|p| match p {
            FPart::Lit(s) => Expr::Str(s.clone()),
            FPart::Expr { expr: e, str: true } => Expr::name("str").call(vec![e.clone()]),
            FPart::Expr {
                expr: e,
                str: false,
            } => e.clone(),
        })
        .collect();
    let chain = terms
        .into_iter()
        .reduce(|a, b| Expr::bin(Bin::Add, a, b))
        .unwrap_or(Expr::Str(String::new()));
    expr(&chain, min)
}

/// Prints a module.
pub fn module(m: &Module) -> String {
    let mut out = String::new();
    for (from, names) in &m.imports {
        let line = format!("from {from} import {}", names.join(", "));
        if line.len() <= 79 {
            let _ = writeln!(out, "{line}");
        } else {
            let _ = writeln!(out, "from {from} import (");
            for name in names {
                let _ = writeln!(out, "    {name},");
            }
            let _ = writeln!(out, ")");
        }
    }
    let mut previous_def = None;
    for (i, s) in m.body.iter().enumerate() {
        let is_def = matches!(s, Stmt::Def { .. });
        if i == 0 {
            if !m.imports.is_empty() {
                out.push_str(if is_def { "\n\n" } else { "\n" });
            }
        } else if is_def || previous_def == Some(true) {
            out.push_str("\n\n");
        }
        stmt(&mut out, s, 0);
        previous_def = Some(is_def);
    }
    out
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("    ");
    }
}

fn block(out: &mut String, body: &[Stmt], depth: usize) {
    if body.is_empty() {
        indent(out, depth);
        out.push_str("pass\n");
    }
    for s in body {
        stmt(out, s, depth);
    }
}

fn stmt(out: &mut String, s: &Stmt, depth: usize) {
    indent(out, depth);
    match s {
        Stmt::Expr(e) => {
            let _ = writeln!(out, "{}", expr(e, TOP));
        }
        Stmt::Assign { target, ann, value } => {
            let ann = ann.as_ref().map(|a| format!(": {a}")).unwrap_or_default();
            let _ = writeln!(out, "{target}{ann} = {}", expr(value, TOP));
        }
        Stmt::Declare { target, ann } => {
            let _ = writeln!(out, "{target}: {ann}");
        }
        Stmt::If { cond, then, els } => {
            let _ = writeln!(out, "if {}:", expr(cond, TOP));
            block(out, then, depth + 1);
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
                        let _ = writeln!(out, "elif {}:", expr(cond, TOP));
                        block(out, then, depth + 1);
                        els = next;
                    }
                    _ => {
                        indent(out, depth);
                        out.push_str("else:\n");
                        block(out, els, depth + 1);
                        break;
                    }
                }
            }
        }
        Stmt::While { cond, body } => {
            let _ = writeln!(out, "while {}:", expr(cond, TOP));
            block(out, body, depth + 1);
        }
        Stmt::Break => out.push_str("break\n"),
        Stmt::Continue => out.push_str("continue\n"),
        Stmt::Return(None) => out.push_str("return\n"),
        Stmt::Return(Some(e)) => {
            let _ = writeln!(out, "return {}", expr(e, TOP));
        }
        Stmt::Global(names) => {
            let _ = writeln!(out, "global {}", names.join(", "));
        }
        Stmt::Def {
            name,
            params,
            result,
            body,
        } => {
            let mut list: Vec<String> = params.iter().map(|(n, t)| format!("{n}: {t}")).collect();
            if !list.is_empty() {
                list.push("/".into());
            }
            let _ = writeln!(out, "def {name}({}) -> {result}:", list.join(", "));
            block(out, body, depth + 1);
        }
    }
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
            expr(&Expr::bin(Bin::Mul, sum.clone(), n("c")), 0),
            "(a + b) * c"
        );
        assert_eq!(
            expr(&Expr::bin(Bin::Sub, n("c"), sum.clone()), 0),
            "c - (a + b)"
        );
        assert_eq!(
            expr(&Expr::bin(Bin::Sub, sum.clone(), n("c")), 0),
            "a + b - c"
        );
        let lt = Expr::bin(Bin::Lt, n("a"), n("b"));
        assert_eq!(
            expr(&Expr::bin(Bin::Eq, lt.clone(), n("c")), 0),
            "(a < b) == c"
        );
        assert_eq!(expr(&Expr::unary(Unary::Not, lt), 0), "not a < b");
        assert_eq!(
            expr(&Expr::bin(Bin::Pow, Expr::Int(-2), n("x")), 0),
            "(-2) ** x"
        );
        assert_eq!(
            expr(&Expr::bin(Bin::Pow, n("x"), Expr::Int(-2)), 0),
            "x ** -2"
        );
        let cond = Expr::IfElse {
            cond: Box::new(n("c")),
            then: Box::new(n("a")),
            els: Box::new(n("b")),
        };
        assert_eq!(
            expr(&n("f").call(vec![cond.clone()]), 0),
            "f(a if c else b)"
        );
        assert_eq!(
            expr(&Expr::bin(Bin::Add, cond, n("d")), 0),
            "(a if c else b) + d"
        );
    }

    #[test]
    fn strings() {
        assert_eq!(string_literal("it's"), "\"it's\"");
        assert_eq!(string_literal("a\n'\"\\"), "'a\\n\\'\"\\\\'");
        let parts = vec![
            FPart::Lit("x={".into()),
            FPart::Expr {
                expr: n("x"),
                str: true,
            },
        ];
        assert_eq!(expr(&Expr::FString(parts), 0), "f'x={{{x}'");
        let quoted = vec![
            FPart::Lit("a".into()),
            FPart::Expr {
                expr: n("f").call(vec![Expr::Str("'\"".into())]),
                str: true,
            },
        ];
        assert_eq!(expr(&Expr::FString(quoted), 0), "'a' + str(f('\\'\"'))");
    }
}
