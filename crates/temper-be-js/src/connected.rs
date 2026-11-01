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
    /// As [`Helper`](Connection::Helper), with the receiver, a list,
    /// cast to a mutable array where it's read-only, as the helpers'
    /// declarations take it: `listedGet(ns, 0)`.
    ListHelper(&'static str),
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
    ("core.type List.get length()", Connection::Translate(length)),
    ("core.type List.get()", Connection::ListHelper("listedGet")),
    ("core.type List.toList()", Connection::Translate(same)),
    (
        "core.type List.toListBuilder()",
        Connection::Translate(copied),
    ),
    (
        "core.type ListBuilder.add()",
        Connection::Translate(list_add),
    ),
    (
        "core.type ListBuilder.addAll()",
        Connection::Translate(list_add_all),
    ),
    (
        "core.type ListBuilder.clear()",
        Connection::Helper("listBuilderClear"),
    ),
    (
        "core.type ListBuilder.constructor()",
        Connection::Translate(new_array),
    ),
    (
        "core.type ListBuilder.get length()",
        Connection::Translate(length),
    ),
    (
        "core.type ListBuilder.removeLast()",
        Connection::Helper("listBuilderRemoveLast"),
    ),
    (
        "core.type ListBuilder.reverse()",
        Connection::Helper("listBuilderReverse"),
    ),
    (
        "core.type ListBuilder.set()",
        Connection::Helper("listBuilderSet"),
    ),
    ("core.type ListBuilder.sort()", Connection::Translate(sort)),
    (
        "core.type ListBuilder.toList()",
        Connection::Helper("listBuilderToList"),
    ),
    (
        "core.type ListBuilder.toListBuilder()",
        Connection::Translate(copied),
    ),
    (
        "core.type Listed.filter()",
        Connection::ListHelper("listedFilter"),
    ),
    (
        "core.type Listed.get isEmpty()",
        Connection::Translate(list_is_empty),
    ),
    (
        "core.type Listed.get length()",
        Connection::Translate(length),
    ),
    (
        "core.type Listed.get()",
        Connection::ListHelper("listedGet"),
    ),
    (
        "core.type Listed.getOr()",
        Connection::ListHelper("listedGetOr"),
    ),
    (
        "core.type Listed.join()",
        Connection::ListHelper("listedJoin"),
    ),
    (
        "core.type Listed.map()",
        Connection::ListHelper("listedMap"),
    ),
    ("core.type Listed.reduce()", Connection::Translate(reduce)),
    (
        "core.type Listed.reduceFrom()",
        Connection::ListHelper("listedReduceFrom"),
    ),
    (
        "core.type Listed.slice()",
        Connection::ListHelper("listedSlice"),
    ),
    (
        "core.type Listed.sorted()",
        Connection::ListHelper("listedSorted"),
    ),
    (
        "core.type Listed.toList()",
        Connection::Helper("listedToList"),
    ),
    (
        "core.type Listed.toListBuilder()",
        Connection::Translate(copied),
    ),
    (
        "core.type Map.constructor()",
        Connection::Translate(new_map),
    ),
    (
        "core.type MapBuilder.clear()",
        Connection::Translate(map_clear),
    ),
    (
        "core.type MapBuilder.constructor()",
        Connection::Translate(new_map_builder),
    ),
    (
        "core.type MapBuilder.remove()",
        Connection::Helper("mapBuilderRemove"),
    ),
    ("core.type MapBuilder.set()", Connection::Translate(map_set)),
    (
        "core.type Mapped.forEach()",
        Connection::Helper("mappedForEach"),
    ),
    ("core.type Mapped.get length()", Connection::Translate(size)),
    ("core.type Mapped.get()", Connection::Helper("mappedGet")),
    (
        "core.type Mapped.getOr()",
        Connection::Translate(mapped_get_or),
    ),
    ("core.type Mapped.has()", Connection::Translate(has)),
    ("core.type Mapped.keys()", Connection::Translate(keys)),
    (
        "core.type Mapped.toList()",
        Connection::Helper("mappedToList"),
    ),
    (
        "core.type Mapped.toMap()",
        Connection::Helper("mappedToMap"),
    ),
    (
        "core.type Mapped.toMapBuilder()",
        Connection::Translate(new_map_from),
    ),
    ("core.type Mapped.values()", Connection::Translate(values)),
    (
        "core.type Pair.constructor()",
        Connection::Helper("pairConstructor"),
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

fn one(t: &mut Translator<'_>, args: &[NodeId], what: &str) -> Result<Expr, NotYet> {
    match args {
        [x] => t.expr(*x),
        _ => Err(NotYet {
            what: format!("`{what}` with other than its receiver"),
        }),
    }
}

fn two(t: &mut Translator<'_>, args: &[NodeId], what: &str) -> Result<[Expr; 2], NotYet> {
    match args {
        [a, b] => {
            let v = t.in_order(&[*a, *b])?;
            let [a, b] = <[Expr; 2]>::try_from(v).unwrap_or_else(|_| unreachable!());
            Ok([a, b])
        }
        _ => Err(NotYet {
            what: format!("`{what}` with other than its receiver and one argument"),
        }),
    }
}

/// `b.sort(compare)`, as Kotlin's: JS's sort is stable.
fn sort(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [b, compare] = two(t, args, "sort")?;
    Ok(b.method("sort", vec![compare]))
}

/// `ls.reduce(accumulate)`, as Kotlin's.
fn reduce(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [ls, accumulate] = two(t, args, "reduce")?;
    Ok(ls.method("reduce", vec![accumulate]))
}

/// `x.length`.
fn length(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    Ok(one(t, args, "length")?.member("length"))
}

/// `!x.length`, as Kotlin's.
fn list_is_empty(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let x = one(t, args, "isEmpty")?;
    Ok(Expr::unary(Unary::Not, x.member("length")))
}

/// `x.slice()`, a new `ListBuilder` of a list's items.
fn copied(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    Ok(one(t, args, "toListBuilder")?.method("slice", Vec::new()))
}

/// `[]`, a new `ListBuilder`.
fn new_array(_: &mut Translator<'_>, _: &[NodeId]) -> Result<Expr, NotYet> {
    Ok(Expr::Array(Vec::new()))
}

/// `b.push(x)`, or with a place, `listBuilderAdd(b, x, at)`, as Kotlin's.
fn list_add(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    match args {
        [b, x] => {
            let [b, x] = two(t, &[*b, *x], "add")?;
            Ok(b.method("push", vec![x]))
        }
        [b, x, at] if matches!(t.node(*at), Node::Omitted {}) => {
            let [b, x] = two(t, &[*b, *x], "add")?;
            Ok(b.method("push", vec![x]))
        }
        [b, x, at] => {
            let v = t.in_order(&[*b, *x, *at])?;
            t.import("listBuilderAdd");
            Ok(Expr::name("listBuilderAdd").call(v))
        }
        _ => Err(NotYet {
            what: "`add` without its value".into(),
        }),
    }
}

/// `listBuilderAddAll(b, xs)`, with the place if given.
fn list_add_all(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (b, xs, at) = match args {
        [b, xs] => (*b, *xs, None),
        [b, xs, at] if matches!(t.node(*at), Node::Omitted {}) => (*b, *xs, None),
        [b, xs, at] => (*b, *xs, Some(*at)),
        _ => {
            return Err(NotYet {
                what: "`addAll` without its values".into(),
            });
        }
    };
    let b = t.expr(b)?;
    let mut v = vec![b, t.list_arg(xs)?];
    if let Some(at) = at {
        v.push(t.expr(at)?);
    }
    t.import("listBuilderAddAll");
    Ok(Expr::name("listBuilderAddAll").call(v))
}

/// `mapConstructor(entries)`, as Kotlin's, its pairs taken as the
/// entries they are (its declaration says a pair).
fn new_map(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let entries = one(t, args, "Map")?;
    t.import("mapConstructor");
    Ok(Expr::name("mapConstructor").call(vec![Expr::Cast("any".into(), Box::new(entries))]))
}

/// `new Map()`, where Kotlin's helper takes an argument it ignores.
fn new_map_builder(_: &mut Translator<'_>, _: &[NodeId]) -> Result<Expr, NotYet> {
    Ok(Expr::New(Box::new(Expr::name("Map")), Vec::new()))
}

/// `new Map(m)`, a new `MapBuilder` of a map's entries.
fn new_map_from(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let m = one(t, args, "toMapBuilder")?;
    Ok(Expr::New(Box::new(Expr::name("Map")), vec![m]))
}

/// `m.clear()`.
fn map_clear(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    Ok(one(t, args, "clear")?.method("clear", Vec::new()))
}

/// `m.set(k, v)`.
fn map_set(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let v = t.in_order(args)?;
    let [m, k, v] = <[Expr; 3]>::try_from(v).map_err(|_| NotYet {
        what: "`set` without a key and a value".into(),
    })?;
    Ok(m.method("set", vec![k, v]))
}

/// `m.size`.
fn size(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    Ok(one(t, args, "length")?.member("size"))
}

/// `m.has(k)`.
fn has(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [m, k] = two(t, args, "has")?;
    Ok(m.method("has", vec![k]))
}

/// `Object.freeze(Array.from(m.keys()))`, as Kotlin's.
fn keys(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let m = one(t, args, "keys")?;
    Ok(frozen_from(m.method("keys", Vec::new())))
}

/// `Object.freeze(Array.from(m.values()))`.
fn values(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let m = one(t, args, "values")?;
    Ok(frozen_from(m.method("values", Vec::new())))
}

fn frozen_from(e: Expr) -> Expr {
    let array = Expr::name("Array").method("from", vec![e]);
    Expr::name("Object").method("freeze", vec![array])
}

/// `mappedGetOr(m, k, fallback)`, as Kotlin's, where the value can't be
/// null; else `m.has(k) ? m.get(k) : fallback`, since the helper gives
/// the fallback for a key whose value is null.
fn mapped_get_or(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [_, k, fallback] = *args else {
        return Err(NotYet {
            what: "`getOr` without a key and a fallback".into(),
        });
    };
    let nullable = t.may_be_null(fallback);
    let ty = t.type_of(fallback)?;
    let v = t.in_order(args)?;
    let [m, key, fallback] = <[Expr; 3]>::try_from(v).unwrap_or_else(|_| unreachable!());
    if !nullable {
        t.import("mappedGetOr");
        return Ok(Expr::name("mappedGetOr").call(vec![m, key, fallback]));
    }
    let simple = matches!(
        t.node(k),
        Node::StringLit { .. } | Node::Int32Lit { .. } | Node::Local { .. }
    );
    if !simple || !matches!(m, Expr::Name(_)) {
        return Err(NotYet {
            what: "`getOr` of a nullable value with a key or map that isn't simple".into(),
        });
    }
    let got = Expr::Cast(ty, Box::new(m.clone().method("get", vec![key.clone()])));
    Ok(Expr::Cond {
        cond: Box::new(m.method("has", vec![key])),
        then: Box::new(got),
        els: Box::new(fallback),
    })
}

/// The `@temperlang/core` helpers [`TABLE`] names, and those its custom
/// output calls.
pub fn helpers() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = TABLE
        .iter()
        .filter_map(|(_, c)| match c {
            Connection::Helper(h) | Connection::ListHelper(h) => Some(*h),
            _ => None,
        })
        .collect();
    out.extend([
        "listBuilderAdd",
        "listBuilderAddAll",
        "mapConstructor",
        "mappedGetOr",
    ]);
    out
}
