//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry matches what Kotlin's Lua backend
//! gives for it (its `LuaSupportNetwork.kt`), or a prettier equivalent:
//! `print(message)` for `temper.log(message)`, which is `print` in the
//! runtime; `tostring(n)` for `temper.int32_tostring(n)` without a radix,
//! which formats an integer the same way; and `s == ""` for `#s == 0`.

use crate::NotYet;
use crate::translate::Translator;
use crate::tree::{Bin, Expr};
use temper_tmpl::{Node, NodeId};

/// Custom output: from the call's argument nodes, a method's receiver
/// first, and `Omitted` for an argument left out.
pub type Hook = fn(&mut Translator<'_>, &[NodeId]) -> Result<Expr, NotYet>;

/// How a connected declaration is written.
#[derive(Clone, Copy)]
pub enum Connection {
    /// A function of Lua's `math`, given the arguments, a method's
    /// receiver first: `math.min(a, b)`.
    Math(&'static str),
    /// Custom output.
    Translate(Hook),
    /// A function whose result Lua doesn't keep, because the type it gives
    /// is erased ([`TYPES`]): its calls go where they'd be stored.
    Erased,
}

/// Sorted by qname.
pub const TABLE: &[(&str, Connection)] = &[
    ("core.getConsole()", Connection::Erased),
    (
        "core.type Boolean.toString()",
        Connection::Translate(boolean_to_string),
    ),
    (
        "core.type Console.log()",
        Connection::Translate(console_log),
    ),
    ("core.type Int32.max()", Connection::Math("max")),
    ("core.type Int32.min()", Connection::Math("min")),
    (
        "core.type Int32.toString()",
        Connection::Translate(int_to_string),
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
    /// is `print`, as Kotlin's `temper.log` is (Kotlin keeps a `0.0` for
    /// the console).
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

/// `print(message)`.
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
    Ok(Expr::name("print").call(vec![message]))
}

/// The receiver and the radix, if given.
fn receiver_and_radix(
    t: &Translator<'_>,
    args: &[NodeId],
) -> Result<(NodeId, Option<NodeId>), NotYet> {
    match args {
        [n] => Ok((*n, None)),
        [n, r] if matches!(t.node(*r), Node::Omitted {}) => Ok((*n, None)),
        [n, r] => Ok((*n, Some(*r))),
        _ => Err(NotYet {
            what: "`toString` without its receiver".into(),
        }),
    }
}

/// `Int32.toString()`: `tostring(n)`, or with a radix, the runtime's
/// `temper.int32_tostring(n, radix)`, as Kotlin's.
fn int_to_string(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (n, radix) = receiver_and_radix(t, args)?;
    let n = t.expr(n)?;
    match radix {
        None => Ok(Expr::name("tostring").call(vec![n])),
        Some(r) => {
            let r = t.expr(r)?;
            Ok(crate::ops::temper().method("int32_tostring", vec![n, r]))
        }
    }
}

/// `Boolean.toString()`: `tostring(b)`, as Kotlin's.
fn boolean_to_string(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (b, _) = receiver_and_radix(t, args)?;
    Ok(Expr::name("tostring").call(vec![t.expr(b)?]))
}

/// `s == ""`, where Kotlin's is `#s == 0`.
fn is_empty(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [s] = args else {
        return Err(NotYet {
            what: "`isEmpty` without its receiver".into(),
        });
    };
    Ok(Expr::bin(Bin::Eq, t.expr(*s)?, Expr::Str(String::new())))
}
