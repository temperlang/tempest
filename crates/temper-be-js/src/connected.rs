//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry matches what Kotlin's JS backend
//! gives for it (its `JsSupportNetwork.kt`), or a prettier equivalent:
//! `Math.min(a, b)` for a destructured `min__2(a, b)`, and the global
//! `console` for `@temperlang/core`'s `globalConsole`, which is that same
//! object.

use crate::NotYet;
use crate::translate::Translator;
use crate::tree::{Expr, Unary};
use temper_tmpl::{Node, NodeId};

/// Custom output: from the call's argument nodes, a method's receiver
/// first, and `Omitted` for an argument left out.
pub type Hook = fn(&mut Translator<'_>, &[NodeId]) -> Result<Expr, NotYet>;

/// How a connected declaration is written.
#[derive(Clone, Copy)]
pub enum Connection {
    /// A function of JS's `Math`, given the arguments, a method's
    /// receiver first: `Math.min(a, b)`.
    Math(&'static str),
    /// A function of `@temperlang/core`, imported, given the arguments, a
    /// method's receiver first, and those left out dropped:
    /// `float64ToInt32(x)`.
    Helper(&'static str),
    /// Custom output.
    Translate(Hook),
    /// A function whose result JS doesn't keep, because the type it gives
    /// is erased ([`TYPES`]): its calls go where they'd be stored.
    Erased,
}

/// Sorted by qname.
pub const TABLE: &[(&str, Connection)] = &[
    ("core.getConsole()", Connection::Erased),
    (
        "core.type Boolean.toString()",
        Connection::Translate(to_string),
    ),
    (
        "core.type Console.log()",
        Connection::Translate(console_log),
    ),
    ("core.type Float64.abs()", Connection::Math("abs")),
    ("core.type Float64.acos()", Connection::Math("acos")),
    ("core.type Float64.asin()", Connection::Math("asin")),
    ("core.type Float64.atan()", Connection::Math("atan")),
    ("core.type Float64.atan2()", Connection::Math("atan2")),
    ("core.type Float64.ceil()", Connection::Math("ceil")),
    ("core.type Float64.cos()", Connection::Math("cos")),
    ("core.type Float64.cosh()", Connection::Math("cosh")),
    ("core.type Float64.exp()", Connection::Math("exp")),
    ("core.type Float64.expm1()", Connection::Math("expm1")),
    ("core.type Float64.floor()", Connection::Math("floor")),
    ("core.type Float64.log()", Connection::Math("log")),
    ("core.type Float64.log10()", Connection::Math("log10")),
    ("core.type Float64.log1p()", Connection::Math("log1p")),
    ("core.type Float64.sign()", Connection::Math("sign")),
    ("core.type Float64.sin()", Connection::Math("sin")),
    ("core.type Float64.sinh()", Connection::Math("sinh")),
    ("core.type Float64.sqrt()", Connection::Math("sqrt")),
    ("core.type Float64.tan()", Connection::Math("tan")),
    ("core.type Float64.tanh()", Connection::Math("tanh")),
    (
        "core.type Float64.toInt32()",
        Connection::Helper("float64ToInt32"),
    ),
    (
        "core.type Float64.toInt32Unsafe()",
        Connection::Helper("float64ToInt32Unsafe"),
    ),
    (
        "core.type Float64.toInt64()",
        Connection::Helper("float64ToInt64"),
    ),
    (
        "core.type Float64.toInt64Unsafe()",
        Connection::Helper("float64ToInt64Unsafe"),
    ),
    (
        "core.type Float64.toString()",
        Connection::Helper("float64ToString"),
    ),
    ("core.type Int32.max()", Connection::Math("max")),
    ("core.type Int32.min()", Connection::Math("min")),
    ("core.type Int32.toFloat64()", Connection::Translate(same)),
    ("core.type Int32.toInt64()", Connection::Translate(big_int)),
    (
        "core.type Int32.toString()",
        Connection::Translate(to_string),
    ),
    ("core.type Int64.max()", Connection::Helper("int64Max")),
    ("core.type Int64.min()", Connection::Helper("int64Min")),
    (
        "core.type Int64.toFloat64()",
        Connection::Helper("int64ToFloat64"),
    ),
    (
        "core.type Int64.toFloat64Unsafe()",
        Connection::Helper("int64ToFloat64Unsafe"),
    ),
    (
        "core.type Int64.toInt32()",
        Connection::Helper("int64ToInt32"),
    ),
    (
        "core.type Int64.toInt32Unsafe()",
        Connection::Helper("int64ToInt32Unsafe"),
    ),
    (
        "core.type Int64.toString()",
        Connection::Translate(to_string),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Translate(is_empty),
    ),
    (
        "core.type String.toFloat64()",
        Connection::Helper("stringToFloat64"),
    ),
    (
        "core.type String.toInt32()",
        Connection::Helper("stringToInt32"),
    ),
    (
        "core.type String.toInt64()",
        Connection::Helper("stringToInt64"),
    ),
    ("core.type String.toString()", Connection::Translate(same)),
];

/// How a connected type is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeConnection {
    /// Not kept: a module `let` of this type isn't declared, and its
    /// methods ignore it as a receiver. `Console` is, since `Console.log`
    /// is the global `console.log`, which Kotlin's `globalConsole` is.
    Erased,
}

/// Sorted by qname.
pub const TYPES: &[(&str, TypeConnection)] = &[("core.type Console", TypeConnection::Erased)];

/// The connection for a declaration's qname.
pub fn lookup(qname: &str) -> Option<Connection> {
    TABLE
        .binary_search_by_key(&qname, |(k, _)| k)
        .ok()
        .map(|i| TABLE[i].1)
}

/// The connection for a type's qname.
pub fn lookup_type(qname: &str) -> Option<TypeConnection> {
    TYPES
        .binary_search_by_key(&qname, |(k, _)| k)
        .ok()
        .map(|i| TYPES[i].1)
}

/// `console.log(message)`.
fn console_log(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [console, message] = args else {
        return Err(NotYet {
            what: "`Console.log` with other than a message".into(),
        });
    };
    if !t.is_erased(*console) {
        return Err(NotYet {
            what: "a console other than the module's".into(),
        });
    }
    let message = t.expr(*message)?;
    Ok(Expr::name("console").method("log", vec![message]))
}

/// `Boolean.toString()` and `Int32.toString()`: `n.toString()`, with the
/// radix if given, as Kotlin's.
fn to_string(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (n, radix) = match args {
        [n] => (*n, None),
        [n, r] if matches!(t.node(*r), Node::Omitted {}) => (*n, None),
        [n, r] => (*n, Some(*r)),
        _ => {
            return Err(NotYet {
                what: "`toString` without its receiver".into(),
            });
        }
    };
    let n = t.expr(n)?;
    let radix = radix.map(|r| t.expr(r)).transpose()?;
    Ok(n.method("toString", radix.into_iter().collect()))
}

/// `!s`, as Kotlin's `stringIsEmptyIdiomExpander`.
fn is_empty(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [s] = args else {
        return Err(NotYet {
            what: "`isEmpty` without its receiver".into(),
        });
    };
    Ok(Expr::unary(Unary::Not, t.expr(*s)?))
}

/// The value itself: `Int32.toFloat64()`, both JS numbers, and
/// `String.toString()`.
fn same(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [x] = args else {
        return Err(NotYet {
            what: "a conversion without its receiver".into(),
        });
    };
    t.expr(*x)
}

/// `BigInt(n)`, as Kotlin's `Int32.toInt64()`.
fn big_int(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [n] = args else {
        return Err(NotYet {
            what: "`toInt64` without its receiver".into(),
        });
    };
    Ok(Expr::name("BigInt").call(vec![t.expr(*n)?]))
}

/// The `@temperlang/core` helpers [`TABLE`] names.
pub fn helpers() -> Vec<&'static str> {
    TABLE
        .iter()
        .filter_map(|(_, c)| match c {
            Connection::Helper(h) => Some(*h),
            _ => None,
        })
        .collect()
}
