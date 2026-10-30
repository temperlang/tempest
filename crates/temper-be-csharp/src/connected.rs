//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry gives what Kotlin's C# backend
//! gives for it (its `CSharpSupportNetwork.kt`), or a prettier
//! equivalent, but for culture: C#'s string and number operations follow
//! the current culture unless told otherwise, so every one here is
//! ordinal or invariant. Kotlin's `Int32.toString()` is the runtime's
//! `Core.ConvertToString(n)`, which is `n.ToString()`, whose minus sign is
//! U+2212 in some cultures (`sv-SE`); here it's
//! `n.ToString(CultureInfo.InvariantCulture)`.

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
    /// A static method of a class, given the arguments, a method's
    /// receiver first: `Math.Min(a, b)`.
    Static(&'static str, &'static str),
    /// A method of the receiver, given the rest: `console.Log(s)`.
    Method(&'static str),
    /// Custom output.
    Translate(Hook),
}

/// Sorted by qname.
pub const TABLE: &[(&str, Connection)] = &[
    ("core.getConsole()", Connection::Translate(get_console)),
    (
        "core.type Boolean.toString()",
        Connection::Translate(boolean_to_string),
    ),
    ("core.type Console.log()", Connection::Method("Log")),
    ("core.type Int32.max()", Connection::Static("Math", "Max")),
    ("core.type Int32.min()", Connection::Static("Math", "Min")),
    (
        "core.type Int32.toString()",
        Connection::Translate(int_to_string),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Translate(is_empty),
    ),
];

/// How a connected type is written: its simple name, and the class or
/// namespace that holds it (so the translator can import or qualify it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeConnection {
    pub name: &'static str,
    pub namespace: &'static str,
}

/// Sorted by qname.
pub const TYPES: &[(&str, TypeConnection)] = &[(
    "core.type Console",
    TypeConnection {
        name: "ILoggingConsole",
        namespace: "TemperLang.Core",
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

/// `Support.Logging.LoggingConsoleFactory.CreateConsole(name)`, the
/// library's console factory from its `Logging.cs`, named for the
/// module's namespace unless a name is given, as Kotlin's.
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
    let name = name.unwrap_or_else(|| Expr::Str(t.namespace().to_string()));
    let factory = t.support("Logging").member("LoggingConsoleFactory");
    Ok(factory.method("CreateConsole", vec![name]))
}

/// `b ? "true" : "false"`, where Kotlin's `b.ToString().ToLower()` takes
/// `True` or `False` and lowercases it by the current culture.
fn boolean_to_string(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [b] = args else {
        return Err(NotYet {
            what: "`Boolean.toString` without its receiver".into(),
        });
    };
    Ok(Expr::Cond {
        cond: Box::new(t.expr(*b)?),
        then: Box::new(Expr::Str("true".into())),
        els: Box::new(Expr::Str("false".into())),
    })
}

/// `n.ToString(CultureInfo.InvariantCulture)` without a radix, else the
/// runtime's `Core.ConvertToString(n, radix)`, which writes its own
/// digits and `-`.
fn int_to_string(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (n, radix) = match args {
        [n] => (*n, None),
        [n, r] if matches!(t.node(*r), Node::Omitted {}) => (*n, None),
        [n, r] => (*n, Some(*r)),
        _ => {
            return Err(NotYet {
                what: "`Int32.toString` without its receiver".into(),
            });
        }
    };
    let n = t.expr(n)?;
    match radix {
        None => Ok(t.invariant(n)),
        Some(r) => {
            let r = t.expr(r)?;
            Ok(t.class("Core").method("ConvertToString", vec![n, r]))
        }
    }
}

/// `s.Length == 0`; Kotlin's `string.IsNullOrEmpty(s)` also checks for a
/// `null` that a Temper `String` never is.
fn is_empty(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [s] = args else {
        return Err(NotYet {
            what: "`String.isEmpty` without its receiver".into(),
        });
    };
    let s = t.expr(*s)?;
    Ok(Expr::bin(Bin::Eq, s.member("Length"), Expr::Int(0)))
}
