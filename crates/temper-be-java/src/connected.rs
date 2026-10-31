//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry matches what Kotlin's Java
//! backend gives for it (its `JavaSupportNetwork.kt`).

use crate::NotYet;
use crate::translate::Translator;
use crate::tree::Expr;
use temper_tmpl::{Node, NodeId};

/// Custom output: from the call's argument nodes, a method's receiver
/// first, and `Omitted` for an argument left out.
pub type Hook = fn(&mut Translator<'_>, &[NodeId]) -> Result<Expr, NotYet>;

/// How a connected declaration is written.
#[derive(Clone, Copy)]
pub enum Connection {
    /// A static method of a JDK class, given the arguments, a method's
    /// receiver first: `Math.min(a, b)`.
    Static(&'static str, &'static str),
    /// A method of the receiver, given the rest: `s.isEmpty()`.
    Method(&'static str),
    /// A primitive conversion of the receiver: `(int) x`.
    Cast(&'static str),
    /// Custom output.
    Translate(Hook),
}

/// Sorted by qname.
pub const TABLE: &[(&str, Connection)] = &[
    ("core.getConsole()", Connection::Translate(get_console)),
    (
        "core.type Boolean.toString()",
        Connection::Static("Boolean", "toString"),
    ),
    ("core.type Console.log()", Connection::Method("log")),
    (
        "core.type Float64.toInt32()",
        Connection::Static("Core", "float64ToInt"),
    ),
    ("core.type Float64.toInt32Unsafe()", Connection::Cast("int")),
    (
        "core.type Float64.toInt64()",
        Connection::Static("Core", "float64ToInt64"),
    ),
    (
        "core.type Float64.toInt64Unsafe()",
        Connection::Cast("long"),
    ),
    (
        "core.type Float64.toString()",
        Connection::Static("Core", "float64ToString"),
    ),
    ("core.type Int32.max()", Connection::Static("Math", "max")),
    ("core.type Int32.min()", Connection::Static("Math", "min")),
    ("core.type Int32.toFloat64()", Connection::Cast("double")),
    ("core.type Int32.toInt64()", Connection::Cast("long")),
    (
        "core.type Int32.toString()",
        Connection::Static("Integer", "toString"),
    ),
    ("core.type Int64.max()", Connection::Static("Math", "max")),
    ("core.type Int64.min()", Connection::Static("Math", "min")),
    (
        "core.type Int64.toFloat64()",
        Connection::Static("Core", "int64ToFloat64"),
    ),
    (
        "core.type Int64.toFloat64Unsafe()",
        Connection::Cast("double"),
    ),
    (
        "core.type Int64.toInt32()",
        Connection::Static("Core", "int64ToInt"),
    ),
    ("core.type Int64.toInt32Unsafe()", Connection::Cast("int")),
    (
        "core.type Int64.toString()",
        Connection::Static("Long", "toString"),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Method("isEmpty"),
    ),
    (
        "core.type String.toFloat64()",
        Connection::Static("Core", "stringToFloat64"),
    ),
    (
        "core.type String.toInt32()",
        Connection::Static("Core", "stringToInt"),
    ),
    (
        "core.type String.toInt64()",
        Connection::Static("Core", "stringToInt64"),
    ),
];

/// How a connected type is written: its simple name, and what to import
/// for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeConnection {
    pub name: &'static str,
    pub import: &'static str,
}

/// Sorted by qname.
pub const TYPES: &[(&str, TypeConnection)] = &[(
    "core.type Console",
    TypeConnection {
        name: "Console",
        import: "temper.core.Core.Console",
    },
)];

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

/// `Core.getConsole(Logger.getLogger(name))`, the logger named for the
/// module's package unless a name is given, as Kotlin's.
fn get_console(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let name = match args {
        [] => None,
        [n] if matches!(t.node(*n), Node::Omitted {}) => None,
        [n] => Some(t.expr(*n)?),
        _ => {
            return Err(NotYet {
                what: "`getConsole` with more than a name".into(),
            });
        }
    };
    let name = name.unwrap_or_else(|| Expr::Str(t.package().to_string()));
    t.import("java.util.logging.Logger");
    t.import("temper.core.Core");
    let logger = Expr::name("Logger").method("getLogger", vec![name]);
    Ok(Expr::name("Core").method("getConsole", vec![logger]))
}
