//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry matches what Kotlin's Rust
//! backend gives for it (its `RustSupportNetwork.kt`), or a prettier
//! equivalent: `println!("fib(1)={}", …)` with literals in the format
//! string, never `println!("{}", "…")`, and `n.to_string()` for
//! `int_to_string(n, None)`.

use crate::NotYet;
use crate::translate::{Translator, Want};
use crate::tree::Expr;
use temper_tmpl::{Node, NodeId};

/// Custom output: from the call's argument nodes (a method's receiver
/// first, and `Omitted` for an argument left out) and what its place
/// wants.
pub type Hook = fn(&mut Translator<'_>, &[NodeId], Want) -> Result<Expr, NotYet>;

/// How a connected declaration is written.
#[derive(Clone, Copy)]
pub enum Connection {
    /// A method of the receiver, borrowed, with the other arguments
    /// given: `a.min(b)`, `s.is_empty()`.
    Method(&'static str),
    /// Custom output.
    Translate(Hook),
    /// A function whose result Rust doesn't keep, because the type it
    /// gives is erased ([`TYPES`]): its calls go where they'd be stored.
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
    ("core.type Int32.max()", Connection::Method("max")),
    ("core.type Int32.min()", Connection::Method("min")),
    (
        "core.type Int32.toString()",
        Connection::Translate(to_string),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Method("is_empty"),
    ),
];

/// How a connected type is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeConnection {
    /// Not kept: a module `let` of this type isn't declared, and its
    /// methods ignore it as a receiver. `Console` is, as Kotlin's, since
    /// `Console.log` prints with `println!`.
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

/// `println!`, with the message's literal parts in its format string, as
/// Kotlin's `ConsoleLog`.
fn console_log(t: &mut Translator<'_>, args: &[NodeId], _: Want) -> Result<Expr, NotYet> {
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
    let (format, rest) = t.format(*message)?;
    let mut margs = vec![Expr::Str(format)];
    margs.extend(rest);
    Ok(Expr::Macro("println".into(), margs))
}

/// `Boolean.toString()` and `Int32.toString()`: the value itself in a
/// format string, whose `{}` writes it as Temper does, else
/// `Arc::new(n.to_string())` without a radix, or
/// `temper_core::int_to_string(n, Some(radix))`.
fn to_string(t: &mut Translator<'_>, args: &[NodeId], want: Want) -> Result<Expr, NotYet> {
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
    let n = t.receiver(n, true)?;
    Ok(match radix {
        None if want == Want::Display => n,
        None => t.arc(n.method("to_string", Vec::new())),
        Some(radix) => {
            let radix = t.expr(radix, Want::Owned)?;
            Expr::path("temper_core::int_to_string")
                .call(vec![n, Expr::path("Some").call(vec![radix])])
        }
    })
}
