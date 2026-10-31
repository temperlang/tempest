//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry gives what Kotlin's C++ backend
//! gives for it (its `CppSupportNetwork.kt`), or a plainer equivalent:
//! `isEmpty` is `s.empty()` rather than `core::String::isEmpty(s)`, and
//! `Int32`'s `toString()`, `min`, and `max` are `std::to_string`,
//! `std::min`, and `std::max` rather than the runtime's wrappers of them.

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
    /// A function, given the arguments, a method's receiver first: of
    /// the runtime's, relative to `temper::core` (`Console::log`), or of
    /// the standard library's, qualified (`std::min`).
    Call(&'static str),
    /// A function template of the runtime's, given its receiver's type
    /// arguments, which C++ can't always deduce (a list of
    /// `std::string`s and a `"…"`), then the arguments, the receiver
    /// first: `core::List::getOr<std::string>(built, 5, "?")`.
    Templated(&'static str),
    /// A function template of the runtime's that makes the call's value,
    /// given that type's arguments: `core::Map::make<std::string,
    /// int32_t>(entries)`.
    Making(&'static str),
    /// Custom output.
    Translate(Hook),
}

/// Sorted by qname.
pub const TABLE: &[(&str, Connection)] = &[
    ("core.getConsole()", Connection::Translate(get_console)),
    (
        "core.type Boolean.toString()",
        Connection::Call("Boolean::toString"),
    ),
    ("core.type Console.log()", Connection::Call("Console::log")),
    (
        "core.type Float64.toInt32()",
        Connection::Call("Float64::toInt32"),
    ),
    (
        "core.type Float64.toInt32Unsafe()",
        Connection::Call("Float64::toInt32Unsafe"),
    ),
    (
        "core.type Float64.toInt64()",
        Connection::Call("Float64::toInt64"),
    ),
    (
        "core.type Float64.toInt64Unsafe()",
        Connection::Call("Float64::toInt64Unsafe"),
    ),
    (
        "core.type Float64.toString()",
        Connection::Call("Float64::toString"),
    ),
    ("core.type Int32.max()", Connection::Call("std::max")),
    ("core.type Int32.min()", Connection::Call("std::min")),
    (
        "core.type Int32.toFloat64()",
        Connection::Call("Int::toFloat64"),
    ),
    (
        "core.type Int32.toInt64()",
        Connection::Call("Int::toInt64"),
    ),
    (
        "core.type Int32.toString()",
        Connection::Translate(int_to_string),
    ),
    ("core.type Int64.max()", Connection::Call("std::max")),
    ("core.type Int64.min()", Connection::Call("std::min")),
    (
        "core.type Int64.toFloat64()",
        Connection::Call("Int64::toFloat64"),
    ),
    (
        "core.type Int64.toFloat64Unsafe()",
        Connection::Call("Int64::toFloat64Unsafe"),
    ),
    (
        "core.type Int64.toInt32()",
        Connection::Call("Int64::toInt32"),
    ),
    (
        "core.type Int64.toInt32Unsafe()",
        Connection::Call("Int64::toInt32Unsafe"),
    ),
    (
        "core.type Int64.toString()",
        Connection::Translate(int64_to_string),
    ),
    (
        "core.type List.get length()",
        Connection::Templated("List::length"),
    ),
    ("core.type List.get()", Connection::Templated("List::get")),
    ("core.type List.toList()", Connection::Translate(same)),
    (
        "core.type List.toListBuilder()",
        Connection::Templated("List::toListBuilder"),
    ),
    (
        "core.type ListBuilder.add()",
        Connection::Templated("ListBuilder::add"),
    ),
    (
        "core.type ListBuilder.addAll()",
        Connection::Templated("ListBuilder::addAll"),
    ),
    (
        "core.type ListBuilder.clear()",
        Connection::Templated("ListBuilder::clear"),
    ),
    (
        "core.type ListBuilder.constructor()",
        Connection::Making("ListBuilder::make"),
    ),
    (
        "core.type ListBuilder.get length()",
        Connection::Templated("List::length"),
    ),
    (
        "core.type ListBuilder.removeLast()",
        Connection::Templated("ListBuilder::removeLast"),
    ),
    (
        "core.type ListBuilder.reverse()",
        Connection::Templated("ListBuilder::reverse"),
    ),
    (
        "core.type ListBuilder.set()",
        Connection::Templated("ListBuilder::set"),
    ),
    (
        "core.type ListBuilder.toList()",
        Connection::Templated("List::toList"),
    ),
    (
        "core.type ListBuilder.toListBuilder()",
        Connection::Templated("List::toListBuilder"),
    ),
    (
        "core.type Listed.get isEmpty()",
        Connection::Templated("List::isEmpty"),
    ),
    (
        "core.type Listed.get length()",
        Connection::Templated("List::length"),
    ),
    ("core.type Listed.get()", Connection::Templated("List::get")),
    (
        "core.type Listed.getOr()",
        Connection::Templated("List::getOr"),
    ),
    (
        "core.type Listed.slice()",
        Connection::Templated("List::slice"),
    ),
    (
        "core.type Listed.toList()",
        Connection::Templated("List::toList"),
    ),
    (
        "core.type Listed.toListBuilder()",
        Connection::Templated("List::toListBuilder"),
    ),
    (
        "core.type Map.constructor()",
        Connection::Making("Map::make"),
    ),
    (
        "core.type MapBuilder.clear()",
        Connection::Templated("MapBuilder::clear"),
    ),
    (
        "core.type MapBuilder.constructor()",
        Connection::Making("Map::make"),
    ),
    (
        "core.type MapBuilder.remove()",
        Connection::Templated("MapBuilder::remove"),
    ),
    (
        "core.type MapBuilder.set()",
        Connection::Templated("MapBuilder::set"),
    ),
    (
        "core.type Mapped.get length()",
        Connection::Templated("Mapped::length"),
    ),
    (
        "core.type Mapped.get()",
        Connection::Templated("Mapped::get"),
    ),
    (
        "core.type Mapped.getOr()",
        Connection::Templated("Mapped::getOr"),
    ),
    (
        "core.type Mapped.has()",
        Connection::Templated("Mapped::has"),
    ),
    (
        "core.type Mapped.keys()",
        Connection::Templated("Mapped::keys"),
    ),
    (
        "core.type Mapped.toList()",
        Connection::Templated("Mapped::toList"),
    ),
    (
        "core.type Mapped.toMap()",
        Connection::Templated("Mapped::toMap"),
    ),
    (
        "core.type Mapped.toMapBuilder()",
        Connection::Templated("Mapped::toMapBuilder"),
    ),
    (
        "core.type Mapped.values()",
        Connection::Templated("Mapped::values"),
    ),
    (
        "core.type Pair.constructor()",
        Connection::Making("PairFactory::make"),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Translate(is_empty),
    ),
    (
        "core.type String.toFloat64()",
        Connection::Call("String::toFloat64"),
    ),
    (
        "core.type String.toInt32()",
        Connection::Call("String::toInt32"),
    ),
    (
        "core.type String.toInt64()",
        Connection::Call("String::toInt64"),
    ),
];

/// How a connected type is written, relative to `temper::core` where it
/// starts with `core::`, which the header spells out.
pub const TYPES: &[(&str, &str)] = &[("core.type Console", "std::shared_ptr<core::Console::Type>")];

/// The connection for a declaration's qname.
pub fn lookup(qname: &str) -> Option<Connection> {
    TABLE
        .binary_search_by_key(&qname, |(k, _)| k)
        .ok()
        .map(|i| TABLE[i].1)
}

/// The connection for a type's qname.
pub fn lookup_type(qname: &str) -> Option<&'static str> {
    TYPES
        .binary_search_by_key(&qname, |(k, _)| k)
        .ok()
        .map(|i| TYPES[i].1)
}

/// `core::Console::get_console()`, which logs to `std::cout`, as
/// Kotlin's; it takes no name.
fn get_console(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    match args {
        [] => {}
        [n] if matches!(t.node(*n), Node::Omitted {}) => {}
        _ => {
            return Err(NotYet {
                what: "`getConsole` with a name".into(),
            });
        }
    }
    Ok(t.core("Console::get_console").call(Vec::new()))
}

/// `s.empty()`, where Kotlin's is `core::String::isEmpty(s)`.
fn is_empty(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [s] = args else {
        return Err(NotYet {
            what: "`String.isEmpty` without its receiver".into(),
        });
    };
    let s = t.expr(*s)?;
    Ok(s.std_string().method("empty", Vec::new()))
}

/// `std::to_string(n)` without a radix, else the runtime's
/// `core::Int64::toString(n, radix)`.
fn int64_to_string(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    match args {
        [n] => Ok(Expr::name("std::to_string").call(vec![t.expr(*n)?])),
        [n, r] if matches!(t.node(*r), Node::Omitted {}) => {
            Ok(Expr::name("std::to_string").call(vec![t.expr(*n)?]))
        }
        [n, r] => {
            let args = t.operands(&[*n, *r])?;
            Ok(t.core("Int64::toString").call(args))
        }
        _ => Err(NotYet {
            what: "`Int64.toString` without its receiver".into(),
        }),
    }
}

/// `std::to_string(n)` without a radix, else the runtime's
/// `core::Int::toString(n, radix)`, which writes its own digits.
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
    match radix {
        None => {
            let n = t.expr(n)?;
            Ok(Expr::name("std::to_string").call(vec![n]))
        }
        Some(r) => {
            let args = t.operands(&[n, r])?;
            Ok(t.core("Int::toString").call(args))
        }
    }
}

/// The receiver itself: `List.toList()`, of a list that never changes.
fn same(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    match args {
        [x] => t.expr(*x),
        _ => Err(NotYet {
            what: "`toList` with arguments".into(),
        }),
    }
}
