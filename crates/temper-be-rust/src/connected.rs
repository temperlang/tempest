//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry matches what Kotlin's Rust
//! backend gives for it (its `RustSupportNetwork.kt`), or a prettier
//! equivalent: `println!("fib(1)={}", …)` with literals in the format
//! string, never `println!("{}", "…")`, and `n.to_string()` for
//! `int_to_string(n, None)`.

use crate::NotYet;
use crate::translate::{Translator, Want};
use crate::tree::{Bin, Expr, Unary};
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
    /// A runtime function of the receiver, owned, or for a string
    /// borrowed, then the other arguments, an optional one as an
    /// `Option`: `temper_core::string::to_int(&s, None)`.
    Function(&'static str),
    /// `x as T`, a conversion that can't fail or that wraps, as Kotlin's.
    Cast(&'static str),
    /// A function whose result Rust doesn't keep, because the type it
    /// gives is erased ([`TYPES`]): its calls go where they'd be stored.
    Erased,
    /// A function by its path, called with a borrow of the receiver (of
    /// a `Listed` or `Mapped` handle, its trait object), then the other
    /// arguments, an optional one as an `Option`, as Kotlin's:
    /// `temper_core::ListedTrait::get(&ns, 0)`.
    Ufcs(&'static str),
}

/// Sorted by qname.
pub const TABLE: &[(&str, Connection)] = &[
    ("core.empty()", Connection::Translate(empty)),
    ("core.getConsole()", Connection::Erased),
    (
        "core.type Boolean.toString()",
        Connection::Translate(to_string),
    ),
    (
        "core.type Console.log()",
        Connection::Translate(console_log),
    ),
    (
        "core.type DenseBitVector.constructor()",
        Connection::Function("temper_core::DenseBitVector::with_capacity"),
    ),
    ("core.type DenseBitVector.get()", Connection::Method("get")),
    ("core.type DenseBitVector.set()", Connection::Method("set")),
    (
        "core.type Deque.add()",
        Connection::Ufcs("temper_core::deque::add"),
    ),
    (
        "core.type Deque.constructor()",
        Connection::Translate(new_deque),
    ),
    (
        "core.type Deque.get isEmpty()",
        Connection::Ufcs("temper_core::deque::is_empty"),
    ),
    (
        "core.type Deque.removeFirst()",
        Connection::Ufcs("temper_core::deque::remove_first"),
    ),
    ("core.type Float64.abs()", Connection::Method("abs")),
    ("core.type Float64.acos()", Connection::Method("acos")),
    ("core.type Float64.asin()", Connection::Method("asin")),
    ("core.type Float64.atan()", Connection::Method("atan")),
    ("core.type Float64.atan2()", Connection::Method("atan2")),
    ("core.type Float64.ceil()", Connection::Method("ceil")),
    ("core.type Float64.cos()", Connection::Method("cos")),
    ("core.type Float64.cosh()", Connection::Method("cosh")),
    ("core.type Float64.e", Connection::Translate(float64_e)),
    ("core.type Float64.exp()", Connection::Method("exp")),
    ("core.type Float64.expm1()", Connection::Method("exp_m1")),
    ("core.type Float64.floor()", Connection::Method("floor")),
    ("core.type Float64.log()", Connection::Method("ln")),
    ("core.type Float64.log10()", Connection::Method("log10")),
    ("core.type Float64.log1p()", Connection::Method("ln_1p")),
    (
        "core.type Float64.max()",
        Connection::Function("temper_core::float64::max"),
    ),
    (
        "core.type Float64.min()",
        Connection::Function("temper_core::float64::min"),
    ),
    (
        "core.type Float64.near()",
        Connection::Function("temper_core::float64::near"),
    ),
    ("core.type Float64.pi", Connection::Translate(float64_pi)),
    ("core.type Float64.round()", Connection::Method("round")),
    (
        "core.type Float64.sign()",
        Connection::Function("temper_core::float64::sign"),
    ),
    ("core.type Float64.sin()", Connection::Method("sin")),
    ("core.type Float64.sinh()", Connection::Method("sinh")),
    ("core.type Float64.sqrt()", Connection::Method("sqrt")),
    ("core.type Float64.tan()", Connection::Method("tan")),
    ("core.type Float64.tanh()", Connection::Method("tanh")),
    (
        "core.type Float64.toInt32()",
        Connection::Function("temper_core::float64::to_int"),
    ),
    ("core.type Float64.toInt32Unsafe()", Connection::Cast("i32")),
    (
        "core.type Float64.toInt64()",
        Connection::Function("temper_core::float64::to_int64"),
    ),
    ("core.type Float64.toInt64Unsafe()", Connection::Cast("i64")),
    (
        "core.type Float64.toString()",
        Connection::Function("temper_core::float64::to_string"),
    ),
    ("core.type Int32.max()", Connection::Method("max")),
    ("core.type Int32.min()", Connection::Method("min")),
    ("core.type Int32.toFloat64()", Connection::Cast("f64")),
    ("core.type Int32.toInt64()", Connection::Cast("i64")),
    (
        "core.type Int32.toString()",
        Connection::Translate(to_string),
    ),
    ("core.type Int64.max()", Connection::Method("max")),
    ("core.type Int64.min()", Connection::Method("min")),
    (
        "core.type Int64.toFloat64()",
        Connection::Function("temper_core::int64_to_float64"),
    ),
    ("core.type Int64.toFloat64Unsafe()", Connection::Cast("f64")),
    (
        "core.type Int64.toInt32()",
        Connection::Function("temper_core::int64_to_int32"),
    ),
    ("core.type Int64.toInt32Unsafe()", Connection::Cast("i32")),
    (
        "core.type Int64.toString()",
        Connection::Translate(to_string),
    ),
    (
        "core.type List.get length()",
        Connection::Ufcs("temper_core::ListedTrait::len"),
    ),
    (
        "core.type List.get()",
        Connection::Ufcs("temper_core::ListedTrait::get"),
    ),
    ("core.type List.toList()", Connection::Translate(itself)),
    (
        "core.type List.toListBuilder()",
        Connection::Ufcs("temper_core::ListedTrait::to_list_builder"),
    ),
    (
        "core.type ListBuilder.add()",
        Connection::Ufcs("temper_core::listed::add"),
    ),
    (
        "core.type ListBuilder.addAll()",
        Connection::Ufcs("temper_core::listed::add_all"),
    ),
    (
        "core.type ListBuilder.clear()",
        Connection::Ufcs("temper_core::listed::clear"),
    ),
    (
        "core.type ListBuilder.constructor()",
        Connection::Translate(new_list_builder),
    ),
    (
        "core.type ListBuilder.get length()",
        Connection::Ufcs("temper_core::ListedTrait::len"),
    ),
    (
        "core.type ListBuilder.removeLast()",
        Connection::Ufcs("temper_core::listed::remove_last"),
    ),
    (
        "core.type ListBuilder.reverse()",
        Connection::Ufcs("temper_core::listed::reverse"),
    ),
    (
        "core.type ListBuilder.set()",
        Connection::Ufcs("temper_core::listed::set"),
    ),
    (
        "core.type ListBuilder.sort()",
        Connection::Ufcs("temper_core::listed::sort"),
    ),
    (
        "core.type ListBuilder.toList()",
        Connection::Ufcs("temper_core::ListedTrait::to_list"),
    ),
    (
        "core.type ListBuilder.toListBuilder()",
        Connection::Ufcs("temper_core::ListedTrait::to_list_builder"),
    ),
    (
        "core.type Listed.filter()",
        Connection::Ufcs("temper_core::listed::filter"),
    ),
    (
        "core.type Listed.get isEmpty()",
        Connection::Ufcs("temper_core::ListedTrait::is_empty"),
    ),
    (
        "core.type Listed.get length()",
        Connection::Ufcs("temper_core::ListedTrait::len"),
    ),
    (
        "core.type Listed.get()",
        Connection::Ufcs("temper_core::ListedTrait::get"),
    ),
    (
        "core.type Listed.getOr()",
        Connection::Ufcs("temper_core::ListedTrait::get_or"),
    ),
    (
        "core.type Listed.join()",
        Connection::Ufcs("temper_core::listed::join"),
    ),
    (
        "core.type Listed.map()",
        Connection::Ufcs("temper_core::listed::map"),
    ),
    (
        "core.type Listed.reduce()",
        Connection::Ufcs("temper_core::listed::reduce"),
    ),
    (
        "core.type Listed.reduceFrom()",
        Connection::Ufcs("temper_core::listed::reduce_from"),
    ),
    (
        "core.type Listed.slice()",
        Connection::Ufcs("temper_core::listed::slice"),
    ),
    (
        "core.type Listed.sorted()",
        Connection::Ufcs("temper_core::listed::sorted"),
    ),
    (
        "core.type Listed.toList()",
        Connection::Ufcs("temper_core::ListedTrait::to_list"),
    ),
    (
        "core.type Listed.toListBuilder()",
        Connection::Ufcs("temper_core::ListedTrait::to_list_builder"),
    ),
    (
        "core.type Map.constructor()",
        Connection::Translate(new_map),
    ),
    (
        "core.type MapBuilder.clear()",
        Connection::Ufcs("temper_core::MapBuilder::clear"),
    ),
    (
        "core.type MapBuilder.constructor()",
        Connection::Translate(new_map_builder),
    ),
    (
        "core.type MapBuilder.remove()",
        Connection::Ufcs("temper_core::MapBuilder::remove"),
    ),
    (
        "core.type MapBuilder.set()",
        Connection::Ufcs("temper_core::MapBuilder::set"),
    ),
    (
        "core.type Mapped.forEach()",
        Connection::Ufcs("temper_core::MappedTrait::for_each"),
    ),
    (
        "core.type Mapped.get length()",
        Connection::Ufcs("temper_core::MappedTrait::len"),
    ),
    (
        "core.type Mapped.get()",
        Connection::Ufcs("temper_core::MappedTrait::get"),
    ),
    (
        "core.type Mapped.getOr()",
        Connection::Ufcs("temper_core::MappedTrait::get_or"),
    ),
    (
        "core.type Mapped.has()",
        Connection::Ufcs("temper_core::MappedTrait::has"),
    ),
    (
        "core.type Mapped.keys()",
        Connection::Ufcs("temper_core::MappedTrait::keys"),
    ),
    (
        "core.type Mapped.toList()",
        Connection::Ufcs("temper_core::MappedTrait::to_list"),
    ),
    (
        "core.type Mapped.toMap()",
        Connection::Ufcs("temper_core::MappedTrait::to_map"),
    ),
    (
        "core.type Mapped.toMapBuilder()",
        Connection::Ufcs("temper_core::MappedTrait::to_map_builder"),
    ),
    (
        "core.type Mapped.values()",
        Connection::Ufcs("temper_core::MappedTrait::values"),
    ),
    (
        "core.type Pair.constructor()",
        Connection::Translate(new_pair),
    ),
    (
        "core.type String.begin",
        Connection::Translate(string_begin),
    ),
    (
        "core.type String.countBetween()",
        Connection::Function("temper_core::string::count_between"),
    ),
    (
        "core.type String.fromCodePoint()",
        Connection::Function("temper_core::string::from_code_point"),
    ),
    (
        "core.type String.fromCodePoints()",
        Connection::Ufcs("temper_core::string::from_code_points"),
    ),
    ("core.type String.get end()", Connection::Method("len")),
    (
        "core.type String.get isEmpty()",
        Connection::Method("is_empty"),
    ),
    (
        "core.type String.get()",
        Connection::Function("temper_core::string::get"),
    ),
    (
        "core.type String.hasAtLeast()",
        Connection::Function("temper_core::string::has_at_least"),
    ),
    (
        "core.type String.hasIndex()",
        Connection::Function("temper_core::string::has_index"),
    ),
    (
        "core.type String.indexOf()",
        Connection::Function("temper_core::string::index_of"),
    ),
    (
        "core.type String.next()",
        Connection::Function("temper_core::string::next"),
    ),
    (
        "core.type String.prev()",
        Connection::Function("temper_core::string::prev"),
    ),
    (
        "core.type String.slice()",
        Connection::Function("temper_core::string::slice"),
    ),
    (
        "core.type String.split()",
        Connection::Function("temper_core::string::split"),
    ),
    (
        "core.type String.step()",
        Connection::Function("temper_core::string::step"),
    ),
    (
        "core.type String.toFloat64()",
        Connection::Function("temper_core::string::to_float64"),
    ),
    (
        "core.type String.toInt32()",
        Connection::Function("temper_core::string::to_int"),
    ),
    (
        "core.type String.toInt64()",
        Connection::Function("temper_core::string::to_int64"),
    ),
    ("core.type String.toString()", Connection::Translate(itself)),
    (
        "core.type StringBuilder.append()",
        Connection::Ufcs("temper_core::string::builder::append"),
    ),
    (
        "core.type StringBuilder.appendBetween()",
        Connection::Ufcs("temper_core::string::builder::append_between"),
    ),
    (
        "core.type StringBuilder.appendCodePoint()",
        Connection::Ufcs("temper_core::string::builder::append_code_point"),
    ),
    (
        "core.type StringBuilder.clear()",
        Connection::Ufcs("temper_core::string::builder::clear"),
    ),
    (
        "core.type StringBuilder.constructor()",
        Connection::Translate(new_string_builder),
    ),
    (
        "core.type StringBuilder.get end()",
        Connection::Ufcs("temper_core::string::builder::end"),
    ),
    (
        "core.type StringBuilder.toString()",
        Connection::Ufcs("temper_core::string::builder::to_string"),
    ),
    (
        "core.type StringIndex.none",
        Connection::Translate(string_index_none),
    ),
    (
        "core.type StringIndexOption.compareTo()",
        Connection::Translate(string_index_compare_to),
    ),
    (
        "core.type StringIndexOption.eq()",
        Connection::Translate(string_index_eq),
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
    let helper = if t.p_ty(n) == temper_tmpl::TypeId::INT64 {
        "temper_core::int64_to_string"
    } else {
        "temper_core::int_to_string"
    };
    let n = t.receiver(n, true)?;
    Ok(match radix {
        None if want == Want::Display => n,
        None => t.arc(n.method("to_string", Vec::new())),
        Some(radix) => {
            let radix = t.expr(radix, Want::Owned)?;
            Expr::path(helper).call(vec![n, Expr::path("Some").call(vec![radix])])
        }
    })
}

/// The receiver itself, owned: `List.toList()`.
fn itself(t: &mut Translator<'_>, args: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    match args {
        [x] => t.expr(*x, Want::Owned),
        _ => Err(NotYet {
            what: "`toList` with arguments".into(),
        }),
    }
}

/// `temper_core::listed::new_builder()`, as Kotlin's.
fn new_list_builder(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::path("temper_core::listed::new_builder").call(Vec::new()))
}

/// `temper_core::MapBuilder::new()`, as Kotlin's.
fn new_map_builder(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::path("temper_core::MapBuilder::new").call(Vec::new()))
}

/// `temper_core::Map::new(entries)`, from a list of pairs.
fn new_map(t: &mut Translator<'_>, args: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    match args {
        [entries] => {
            let e = t.expr(*entries, Want::Owned)?;
            Ok(Expr::path("temper_core::Map::new").call(vec![e]))
        }
        _ => Err(NotYet {
            what: "a `Map` from other than its entries".into(),
        }),
    }
}

/// A pair, `(key, value)`, a tuple as Kotlin's.
fn new_pair(t: &mut Translator<'_>, args: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    match args {
        [k, v] => {
            let k = t.expr(*k, Want::Owned)?;
            let v = t.expr(*v, Want::Owned)?;
            Ok(Expr::tuple(vec![k, v]))
        }
        _ => Err(NotYet {
            what: "a `Pair` of other than a key and a value".into(),
        }),
    }
}

/// `()`, core's `Empty`, as Kotlin's.
fn empty(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::tuple(Vec::new()))
}

/// `std::f64::consts::E`, as Kotlin's.
fn float64_e(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::path("std::f64::consts::E"))
}

/// `std::f64::consts::PI`, as Kotlin's.
fn float64_pi(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::path("std::f64::consts::PI"))
}

/// `0usize`: string indices are byte offsets, as Kotlin's.
fn string_begin(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::Lit("0usize".into()))
}

/// `()`, `NoStringIndex`'s one value, as Kotlin's.
fn string_index_none(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::tuple(Vec::new()))
}

/// `temper_core::deque::new()`, as Kotlin's.
fn new_deque(_: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    Ok(Expr::path("temper_core::deque::new").call(Vec::new()))
}

/// `Arc::new(RwLock::new(String::new()))`, as Kotlin's.
fn new_string_builder(t: &mut Translator<'_>, _: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    let lock =
        Expr::path("std::sync::RwLock::new").call(vec![Expr::path("String::new").call(Vec::new())]);
    Ok(t.arc(lock))
}

/// `a.cmp(&b) as i32`: string index options are `Option<usize>`, where
/// `None` sorts first, as core says, and as Kotlin's.
fn string_index_compare_to(
    t: &mut Translator<'_>,
    args: &[NodeId],
    _: Want,
) -> Result<Expr, NotYet> {
    let [a, b] = args else {
        return Err(NotYet {
            what: "`compareTo` of other than two".into(),
        });
    };
    let a = t.expr(*a, Want::Owned)?;
    let b = t.expr(*b, Want::Owned)?;
    Ok(a.method("cmp", vec![Expr::unary(Unary::Ref, b)])
        .cast("i32"))
}

/// `a == b`.
fn string_index_eq(t: &mut Translator<'_>, args: &[NodeId], _: Want) -> Result<Expr, NotYet> {
    let [a, b] = args else {
        return Err(NotYet {
            what: "`eq` of other than two".into(),
        });
    };
    let a = t.expr(*a, Want::Owned)?;
    let b = t.expr(*b, Want::Owned)?;
    Ok(Expr::bin(Bin::Eq, a, b))
}
