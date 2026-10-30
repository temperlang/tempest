//! Java's spelling of the 65 operators on builtin types, indexed by
//! [`Op`] (notes/tmpl/nodes.md, "Ops, and core as declarations").
//!
//! As Kotlin's Java backend spells them (its `JavaSupportNetwork.kt`),
//! where that's right: `Int32` and `Int64` are `int` and `long`, whose
//! arithmetic wraps and whose shifts mask their count, as Temper's do, so
//! they're Java's own operators, but for division and remainder, which
//! call `Core.divIntInt` and `Core.modIntInt` to bubble on a zero divisor;
//! three-way comparisons are `Integer.compare` and its kin, and a
//! string's `compareTo`; string equality is `equals`; `Float64.pow` is
//! `Math.pow`; and `Float64` comparisons go through `Double.compare`,
//! which orders NaN and signed zeros as Temper does. Two differences from
//! Kotlin's: `Float64.eq` is `Double.compare(a, b) == 0`, where Kotlin's
//! compares `Double.doubleToLongBits`, which is the same; and `Boolean`
//! comparisons go through `Boolean.compare`, since Java has no `<` on
//! booleans. As Kotlin's, string comparisons are `compareTo`, which
//! orders UTF-16 code units, not code points as Temper does, so a
//! character above U+FFFF sorts before one from U+E000 to U+FFFF; the
//! runtime has no helper that compares code points yet.

use crate::tree::{Bin, Expr, Unary};
use temper_tmpl::ops::{OPS, Op};

/// How an operator is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    /// Java's own binary operator.
    Bin(Bin),
    /// Java's own prefix operator.
    Unary(Unary),
    /// A static method of a class, given the operands:
    /// `Integer.compare(a, b)`.
    Static(&'static str, &'static str),
    /// A static method of a class, compared with 0 by the operator given:
    /// `Double.compare(a, b) < 0`.
    CmpWith(&'static str, Bin),
    /// `a.compareTo(b)`.
    CompareTo,
    /// `a.compareTo(b)`, compared with 0 by the operator given.
    CompareToWith(Bin),
    /// `a.equals(b)`.
    Equals,
    /// String concatenation, flattened by the translator.
    Cat,
}

fn compare(op: &str) -> Option<Bin> {
    Some(match op {
        "eq" => Bin::Eq,
        "lt" => Bin::Lt,
        "le" => Bin::Le,
        "gt" => Bin::Gt,
        "ge" => Bin::Ge,
        _ => return None,
    })
}

/// The class whose `compare` orders a type's values.
fn boxed(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "Boolean" => "Boolean",
        "Int32" => "Integer",
        "Int64" => "Long",
        "Float64" => "Double",
        _ => return None,
    })
}

/// The spelling of each operator, by its name in [`OPS`]; every one has
/// one (a test checks).
pub fn spelling(op: Op) -> Option<Spelling> {
    use Spelling::*;
    let name = op.info().map_or("", |i| i.name);
    let (ty, op) = name.split_once('.').unwrap_or(("", ""));
    Some(match (ty, op) {
        ("Boolean", "not") => Unary(crate::tree::Unary::Not),
        ("Boolean", "eq") => Bin(crate::tree::Bin::Eq),
        ("String", "cat") => Cat,
        ("String", "cmp") => CompareTo,
        ("String", "eq") => Equals,
        ("String", _) => CompareToWith(compare(op)?),
        (_, "cmp") => Static(boxed(ty)?, "compare"),
        ("Boolean" | "Float64", _) if compare(op).is_some() => CmpWith(boxed(ty)?, compare(op)?),
        (_, _) if compare(op).is_some() => Bin(compare(op)?),
        ("Float64", "pow") => Static("Math", "pow"),
        ("Int32" | "Int64", "div") => Static("Core", "divIntInt"),
        ("Int32" | "Int64", "mod") => Static("Core", "modIntInt"),
        (_, "add") => Bin(crate::tree::Bin::Add),
        (_, "sub") => Bin(crate::tree::Bin::Sub),
        (_, "mul") => Bin(crate::tree::Bin::Mul),
        (_, "div") => Bin(crate::tree::Bin::Div),
        (_, "mod") => Bin(crate::tree::Bin::Mod),
        (_, "neg") => Unary(crate::tree::Unary::Neg),
        (_, "bitAnd") => Bin(crate::tree::Bin::BitAnd),
        (_, "bitOr") => Bin(crate::tree::Bin::BitOr),
        (_, "bitXor") => Bin(crate::tree::Bin::BitXor),
        (_, "bitNot") => Unary(crate::tree::Unary::BitNot),
        (_, "shl") => Bin(crate::tree::Bin::Shl),
        (_, "shr") => Bin(crate::tree::Bin::Shr),
        (_, "ushr") => Bin(crate::tree::Bin::Ushr),
        _ => return None,
    })
}

/// Writes an operator applied to its translated operands.
pub fn apply(spelling: Spelling, args: Vec<Expr>) -> Expr {
    let mut args = args.into_iter();
    let mut next = || args.next().unwrap_or(Expr::Name("null".into()));
    match spelling {
        Spelling::Bin(op) => {
            let a = next();
            Expr::bin(op, a, next())
        }
        Spelling::Unary(op) => match (op, next()) {
            // `!(a < b)` reads better as `a >= b`; no comparison here has
            // a NaN operand (floats' compare `Double.compare`'s result).
            (Unary::Not, Expr::Bin(cmp, a, b)) if cmp.negated().is_some() => {
                Expr::Bin(cmp.negated().unwrap_or(cmp), a, b)
            }
            (op, operand) => Expr::unary(op, operand),
        },
        Spelling::Static(class, method) => {
            let a = next();
            Expr::name(class).method(method, vec![a, next()])
        }
        Spelling::CmpWith(class, op) => {
            let a = next();
            let call = Expr::name(class).method("compare", vec![a, next()]);
            Expr::bin(op, call, Expr::Int(0))
        }
        Spelling::CompareTo => {
            let a = next();
            a.method("compareTo", vec![next()])
        }
        Spelling::CompareToWith(op) => {
            let a = next();
            Expr::bin(op, a.method("compareTo", vec![next()]), Expr::Int(0))
        }
        Spelling::Equals => {
            let a = next();
            a.method("equals", vec![next()])
        }
        Spelling::Cat => {
            let a = next();
            Expr::Concat(vec![a, next()])
        }
    }
}

/// Every operator's name, if it has no spelling.
pub fn unspelled() -> Vec<&'static str> {
    (0..OPS.len() as u32)
        .filter(|&i| spelling(Op(i)).is_none())
        .map(|i| OPS[i as usize].name)
        .collect()
}
