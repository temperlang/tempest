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
    /// A function of the runtime's, given the arguments, a method's
    /// receiver first, and leaving out trailing ones left out:
    /// `temper.string_toint32(s)`.
    Helper(&'static str),
    /// The receiver itself, as `Int32.toInt64()`, both Lua integers.
    Same,
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
    (
        "core.type Float64.sign()",
        Connection::Helper("float64_sign"),
    ),
    ("core.type Float64.sqrt()", Connection::Math("sqrt")),
    (
        "core.type Float64.toInt32()",
        Connection::Helper("float64_toint32"),
    ),
    (
        "core.type Float64.toInt32Unsafe()",
        Connection::Helper("float64_toint32unsafe"),
    ),
    (
        "core.type Float64.toInt64()",
        Connection::Helper("float64_toint64"),
    ),
    (
        "core.type Float64.toInt64Unsafe()",
        Connection::Helper("float64_toint64unsafe"),
    ),
    (
        "core.type Float64.toString()",
        Connection::Helper("float64_tostring"),
    ),
    ("core.type Int32.max()", Connection::Math("max")),
    ("core.type Int32.min()", Connection::Math("min")),
    (
        "core.type Int32.toFloat64()",
        Connection::Helper("int32_tofloat64"),
    ),
    ("core.type Int32.toInt64()", Connection::Same),
    (
        "core.type Int32.toString()",
        Connection::Translate(int_to_string),
    ),
    ("core.type Int64.max()", Connection::Math("max")),
    ("core.type Int64.min()", Connection::Math("min")),
    (
        "core.type Int64.toFloat64()",
        Connection::Helper("int64_tofloat64"),
    ),
    (
        "core.type Int64.toFloat64Unsafe()",
        Connection::Helper("int64_tofloat64unsafe"),
    ),
    (
        "core.type Int64.toInt32()",
        Connection::Helper("float64_toint32"),
    ),
    (
        "core.type Int64.toInt32Unsafe()",
        Connection::Helper("int64_toint32unsafe"),
    ),
    (
        "core.type Int64.toString()",
        Connection::Helper("int64_tostring"),
    ),
    (
        "core.type List.get length()",
        Connection::Helper("list_length"),
    ),
    ("core.type List.get()", Connection::Helper("list_get")),
    ("core.type List.toList()", Connection::Helper("list_tolist")),
    (
        "core.type List.toListBuilder()",
        Connection::Helper("list_tolistbuilder"),
    ),
    (
        "core.type ListBuilder.add()",
        Connection::Helper("listbuilder_add"),
    ),
    (
        "core.type ListBuilder.addAll()",
        Connection::Helper("listbuilder_addall"),
    ),
    (
        "core.type ListBuilder.clear()",
        Connection::Helper("listbuilder_clear"),
    ),
    (
        "core.type ListBuilder.constructor()",
        Connection::Helper("listbuilder_constructor"),
    ),
    (
        "core.type ListBuilder.get length()",
        Connection::Helper("listbuilder_length"),
    ),
    (
        "core.type ListBuilder.removeLast()",
        Connection::Helper("listbuilder_removelast"),
    ),
    (
        "core.type ListBuilder.reverse()",
        Connection::Helper("listbuilder_reverse"),
    ),
    (
        "core.type ListBuilder.set()",
        Connection::Helper("listbuilder_set"),
    ),
    (
        "core.type ListBuilder.sort()",
        Connection::Helper("listbuilder_sort"),
    ),
    (
        "core.type ListBuilder.toList()",
        Connection::Helper("listbuilder_tolist"),
    ),
    (
        "core.type ListBuilder.toListBuilder()",
        Connection::Helper("listbuilder_tolistbuilder"),
    ),
    (
        "core.type Listed.filter()",
        Connection::Helper("listed_filter"),
    ),
    (
        "core.type Listed.get isEmpty()",
        Connection::Helper("listed_isempty"),
    ),
    (
        "core.type Listed.get length()",
        Connection::Helper("listed_length"),
    ),
    ("core.type Listed.get()", Connection::Helper("listed_get")),
    (
        "core.type Listed.getOr()",
        Connection::Helper("listed_getor"),
    ),
    ("core.type Listed.join()", Connection::Helper("listed_join")),
    ("core.type Listed.map()", Connection::Helper("listed_map")),
    (
        "core.type Listed.reduce()",
        Connection::Helper("listed_reduce"),
    ),
    (
        "core.type Listed.reduceFrom()",
        Connection::Helper("listed_reducefrom"),
    ),
    (
        "core.type Listed.slice()",
        Connection::Helper("listed_slice"),
    ),
    (
        "core.type Listed.sorted()",
        Connection::Helper("listed_sorted"),
    ),
    (
        "core.type Listed.toList()",
        Connection::Helper("listed_tolist"),
    ),
    (
        "core.type Listed.toListBuilder()",
        Connection::Helper("listed_tolistbuilder"),
    ),
    (
        "core.type Map.constructor()",
        Connection::Helper("map_constructor"),
    ),
    (
        "core.type MapBuilder.clear()",
        Connection::Helper("mapbuilder_clear"),
    ),
    (
        "core.type MapBuilder.constructor()",
        Connection::Helper("mapbuilder_constructor"),
    ),
    (
        "core.type MapBuilder.remove()",
        Connection::Helper("mapbuilder_remove"),
    ),
    (
        "core.type MapBuilder.set()",
        Connection::Helper("mapbuilder_set"),
    ),
    (
        "core.type Mapped.forEach()",
        Connection::Helper("mapped_foreach"),
    ),
    (
        "core.type Mapped.get length()",
        Connection::Helper("mapped_length"),
    ),
    ("core.type Mapped.get()", Connection::Helper("mapped_get")),
    (
        "core.type Mapped.getOr()",
        Connection::Helper("mapped_getor"),
    ),
    ("core.type Mapped.has()", Connection::Helper("mapped_has")),
    ("core.type Mapped.keys()", Connection::Helper("mapped_keys")),
    (
        "core.type Mapped.toList()",
        Connection::Helper("mapped_tolist"),
    ),
    (
        "core.type Mapped.toMap()",
        Connection::Helper("mapped_tomap"),
    ),
    (
        "core.type Mapped.toMapBuilder()",
        Connection::Helper("mapped_tomapbuilder"),
    ),
    (
        "core.type Mapped.values()",
        Connection::Helper("mapped_values"),
    ),
    (
        "core.type Pair.constructor()",
        Connection::Helper("pair_constructor"),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Translate(is_empty),
    ),
    (
        "core.type String.toFloat64()",
        Connection::Helper("string_tofloat64"),
    ),
    (
        "core.type String.toInt32()",
        Connection::Helper("string_toint32"),
    ),
    (
        "core.type String.toInt64()",
        Connection::Helper("string_toint64"),
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
