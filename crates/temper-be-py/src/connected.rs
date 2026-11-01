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
    /// A call of a function another Python module has, imported from it:
    /// the module, then the name.
    Import(&'static str, &'static str),
    /// The receiver itself.
    Identity,
    /// Custom output: the expression, and the `temper_core` names it
    /// needs imported, or another module's, as `typing.cast`.
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
        "core.type Float64.sign()",
        Connection::Runtime("float64_sign"),
    ),
    (
        "core.type Float64.sqrt()",
        Connection::Import("math", "sqrt"),
    ),
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
    ("core.type List.get length()", Connection::Builtin("len")),
    ("core.type List.get()", Connection::Runtime("list_get")),
    ("core.type List.toList()", Connection::Identity),
    (
        "core.type List.toListBuilder()",
        Connection::Builtin("list"),
    ),
    (
        "core.type ListBuilder.add()",
        Connection::Translate(list_builder_add),
    ),
    (
        "core.type ListBuilder.addAll()",
        Connection::Runtime("list_builder_add_all"),
    ),
    ("core.type ListBuilder.clear()", Connection::Method("clear")),
    (
        "core.type ListBuilder.constructor()",
        Connection::Translate(empty_list),
    ),
    (
        "core.type ListBuilder.get length()",
        Connection::Builtin("len"),
    ),
    (
        "core.type ListBuilder.removeLast()",
        Connection::Method("pop"),
    ),
    (
        "core.type ListBuilder.reverse()",
        Connection::Method("reverse"),
    ),
    (
        "core.type ListBuilder.set()",
        Connection::Runtime("list_builder_set"),
    ),
    (
        "core.type ListBuilder.sort()",
        Connection::Translate(list_builder_sort),
    ),
    (
        "core.type ListBuilder.toList()",
        Connection::Builtin("tuple"),
    ),
    (
        "core.type ListBuilder.toListBuilder()",
        Connection::Builtin("list"),
    ),
    (
        "core.type Listed.filter()",
        Connection::Runtime("list_filter"),
    ),
    (
        "core.type Listed.get isEmpty()",
        Connection::Translate(is_empty),
    ),
    ("core.type Listed.get length()", Connection::Builtin("len")),
    ("core.type Listed.get()", Connection::Runtime("list_get")),
    (
        "core.type Listed.getOr()",
        Connection::Runtime("list_get_or"),
    ),
    ("core.type Listed.join()", Connection::Runtime("list_join")),
    ("core.type Listed.map()", Connection::Runtime("list_map")),
    (
        "core.type Listed.reduce()",
        Connection::Runtime("listed_reduce"),
    ),
    (
        "core.type Listed.reduceFrom()",
        Connection::Runtime("listed_reduce_from"),
    ),
    (
        "core.type Listed.slice()",
        Connection::Runtime("list_slice"),
    ),
    (
        "core.type Listed.sorted()",
        Connection::Runtime("listed_sorted"),
    ),
    (
        "core.type Listed.toList()",
        Connection::Runtime("listed_to_list"),
    ),
    (
        "core.type Listed.toListBuilder()",
        Connection::Builtin("list"),
    ),
    (
        "core.type Map.constructor()",
        Connection::Runtime("map_constructor"),
    ),
    ("core.type MapBuilder.clear()", Connection::Method("clear")),
    (
        "core.type MapBuilder.constructor()",
        Connection::Translate(empty_dict),
    ),
    ("core.type MapBuilder.remove()", Connection::Method("pop")),
    (
        "core.type MapBuilder.set()",
        Connection::Runtime("map_builder_set"),
    ),
    (
        "core.type Mapped.forEach()",
        Connection::Runtime("mapped_for_each"),
    ),
    ("core.type Mapped.get length()", Connection::Builtin("len")),
    ("core.type Mapped.get()", Connection::Translate(subscript)),
    ("core.type Mapped.getOr()", Connection::Method("get")),
    ("core.type Mapped.has()", Connection::Runtime("mapped_has")),
    (
        "core.type Mapped.keys()",
        Connection::Translate(mapped_keys),
    ),
    (
        "core.type Mapped.toList()",
        Connection::Runtime("mapped_to_list"),
    ),
    (
        "core.type Mapped.toListBuilder()",
        Connection::Runtime("mapped_to_list_builder"),
    ),
    (
        "core.type Mapped.toMap()",
        Connection::Runtime("mapped_to_map"),
    ),
    (
        "core.type Mapped.toMapBuilder()",
        Connection::Builtin("dict"),
    ),
    (
        "core.type Mapped.values()",
        Connection::Translate(mapped_values),
    ),
    ("core.type Pair.constructor()", Connection::Runtime("Pair")),
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

/// `[]`, an empty `ListBuilder`.
fn empty_list(_: Vec<Option<Expr>>) -> Translated {
    (Expr::List(Vec::new()), &[])
}

/// `{}`, an empty `MapBuilder`.
fn empty_dict(_: Vec<Option<Expr>>) -> Translated {
    (Expr::EmptyDict, &[])
}

/// `list_builder_sort(cast('list[Any]', b), compare)`: the runtime's
/// annotation takes a `list`, which a `ListBuilder`, annotated
/// `MutableSequence`, isn't to mypy (a Kotlin bug: its output fails `mypy
/// --strict` there).
fn list_builder_sort(args: Vec<Option<Expr>>) -> Translated {
    let mut args = args.into_iter().flatten();
    let (b, compare) = (
        args.next().unwrap_or(Expr::None),
        args.next().unwrap_or(Expr::None),
    );
    let b = Expr::name("cast").call(vec![Expr::Str("list[Any]".into()), b]);
    (
        Expr::name("list_builder_sort").call(vec![b, compare]),
        &["list_builder_sort", "typing.cast", "typing.Any"],
    )
}

/// `b.append(x)`, or `list_builder_add(b, x, at)` with a place, as
/// Kotlin's.
fn list_builder_add(mut args: Vec<Option<Expr>>) -> Translated {
    let b = arg(&mut args, 0).unwrap_or(Expr::None);
    let x = arg(&mut args, 1).unwrap_or(Expr::None);
    match arg(&mut args, 2) {
        None => (b.attr("append").call(vec![x]), &[]),
        Some(at) => (
            Expr::name("list_builder_add").call(vec![b, x, at]),
            &["list_builder_add"],
        ),
    }
}

/// `m[k]`, which raises `KeyError`, a bubble, for a missing key.
fn subscript(mut args: Vec<Option<Expr>>) -> Translated {
    let m = arg(&mut args, 0).unwrap_or(Expr::None);
    let k = arg(&mut args, 1).unwrap_or(Expr::None);
    (Expr::Index(Box::new(m), Box::new(k)), &[])
}

/// `list(m.keys())`, as Kotlin's.
fn mapped_keys(mut args: Vec<Option<Expr>>) -> Translated {
    let m = arg(&mut args, 0).unwrap_or(Expr::None);
    (
        Expr::name("list").call(vec![m.attr("keys").call(Vec::new())]),
        &[],
    )
}

/// `list(m.values())`, as Kotlin's.
fn mapped_values(mut args: Vec<Option<Expr>>) -> Translated {
    let m = arg(&mut args, 0).unwrap_or(Expr::None);
    (
        Expr::name("list").call(vec![m.attr("values").call(Vec::new())]),
        &[],
    )
}
