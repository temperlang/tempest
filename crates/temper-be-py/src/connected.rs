//! The connection table: how core's connected declarations are written,
//! by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry matches what Kotlin's Python
//! backend gives for it (its `PySupportNetwork.kt`), or a prettier
//! equivalent: `str(n)` for `int_to_string(n)` with the radix left out.

use crate::tree::{Expr, Unary};

/// A custom translation's output: the expression, and the `temper_core`
/// names it needs imported.
pub type Translated = (Expr, &'static [&'static str]);

/// How a connected declaration is written. Arguments come translated, a
/// method's receiver first; `None` is an argument left out (`Omitted`).
#[derive(Clone, Copy)]
pub enum Connection {
    /// A call of a Python builtin, with the arguments given.
    Builtin(&'static str),
    /// A call of a method of the receiver.
    Method(&'static str),
    /// A call of a `temper_core` function, imported, with the arguments
    /// given.
    Runtime(&'static str),
    /// The receiver itself.
    Identity,
    /// Custom output: the expression, and the `temper_core` names it
    /// needs imported.
    Translate(fn(Vec<Option<Expr>>) -> Translated),
}

/// Sorted by qname.
pub const TABLE: &[(&str, Connection)] = &[
    ("core.getConsole()", Connection::Translate(get_console)),
    (
        "core.type Boolean.toString()",
        Connection::Translate(boolean_to_string),
    ),
    ("core.type Console.log()", Connection::Method("log")),
    (
        "core.type Float64.toInt32()",
        Connection::Runtime("float64_to_int"),
    ),
    (
        "core.type Float64.toInt32Unsafe()",
        Connection::Runtime("float64_to_int_unsafe"),
    ),
    (
        "core.type Float64.toInt64()",
        Connection::Runtime("float64_to_int64"),
    ),
    (
        "core.type Float64.toInt64Unsafe()",
        Connection::Runtime("float64_to_int64_unsafe"),
    ),
    (
        "core.type Float64.toString()",
        Connection::Runtime("float64_to_string"),
    ),
    ("core.type Int32.max()", Connection::Builtin("max")),
    ("core.type Int32.min()", Connection::Builtin("min")),
    ("core.type Int32.toFloat64()", Connection::Builtin("float")),
    ("core.type Int32.toInt64()", Connection::Identity),
    (
        "core.type Int32.toString()",
        Connection::Translate(int_to_string),
    ),
    ("core.type Int64.max()", Connection::Builtin("max")),
    ("core.type Int64.min()", Connection::Builtin("min")),
    (
        "core.type Int64.toFloat64()",
        Connection::Runtime("int64_to_float64"),
    ),
    (
        "core.type Int64.toFloat64Unsafe()",
        Connection::Builtin("float"),
    ),
    (
        "core.type Int64.toInt32()",
        Connection::Runtime("int64_to_int32"),
    ),
    (
        "core.type Int64.toInt32Unsafe()",
        Connection::Runtime("int64_to_int32_unsafe"),
    ),
    (
        "core.type Int64.toString()",
        Connection::Translate(int_to_string),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Translate(is_empty),
    ),
    (
        "core.type String.toFloat64()",
        Connection::Runtime("string_to_float64"),
    ),
    (
        "core.type String.toInt32()",
        Connection::Runtime("string_to_int32"),
    ),
    (
        "core.type String.toInt64()",
        Connection::Runtime("string_to_int64"),
    ),
];

/// The connection for a qname.
pub fn lookup(qname: &str) -> Option<Connection> {
    TABLE
        .binary_search_by_key(&qname, |(k, _)| k)
        .ok()
        .map(|i| TABLE[i].1)
}

fn arg(args: &mut [Option<Expr>], i: usize) -> Option<Expr> {
    args.get_mut(i).and_then(Option::take)
}

/// `LoggingConsole(__name__)`, a logger named for the module, as Kotlin's
/// `GetConsole`.
fn get_console(mut args: Vec<Option<Expr>>) -> Translated {
    let name = arg(&mut args, 0).unwrap_or(Expr::name("__name__"));
    (
        Expr::name("LoggingConsole").call(vec![name]),
        &["LoggingConsole"],
    )
}

/// `boolean_to_string(b)`, as Kotlin's: `true` or `false`, where `str`
/// gives `True` or `False`.
fn boolean_to_string(mut args: Vec<Option<Expr>>) -> Translated {
    let b = arg(&mut args, 0).unwrap_or(Expr::None);
    (
        Expr::name("boolean_to_string").call(vec![b]),
        &["boolean_to_string"],
    )
}

/// `str(n)` without a radix, else `int_to_string(n, radix)`.
fn int_to_string(mut args: Vec<Option<Expr>>) -> Translated {
    let n = arg(&mut args, 0).unwrap_or(Expr::None);
    match arg(&mut args, 1) {
        None => (Expr::name("str").call(vec![n]), &[]),
        Some(radix) => (
            Expr::name("int_to_string").call(vec![n, radix]),
            &["int_to_string"],
        ),
    }
}

/// `not s`, as Kotlin's `GenericIsEmpty`.
fn is_empty(mut args: Vec<Option<Expr>>) -> Translated {
    let s = arg(&mut args, 0).unwrap_or(Expr::None);
    (Expr::unary(Unary::Not, s), &[])
}
