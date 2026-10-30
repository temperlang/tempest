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
    ("core.type Int32.max()", Connection::Math("max")),
    ("core.type Int32.min()", Connection::Math("min")),
    (
        "core.type Int32.toString()",
        Connection::Translate(to_string),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Translate(is_empty),
    ),
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
