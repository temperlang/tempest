//! The connection tables: how core's connected declarations and types are
//! written, by qname, with Kotlin's strings as keys (notes/tmpl/names.md,
//! "Connected declarations"). Each entry matches what Kotlin's Java
//! backend gives for it (its `JavaSupportNetwork.kt`).

use crate::NotYet;
use crate::translate::Translator;
use crate::tree::Expr;
use temper_tmpl::{Node, NodeId, Scalar, TypeData, TypeId};

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
        "core.type Float64.sign()",
        Connection::Static("Math", "signum"),
    ),
    (
        "core.type Float64.sqrt()",
        Connection::Static("Math", "sqrt"),
    ),
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
    ("core.type List.get length()", Connection::Method("size")),
    (
        "core.type List.get()",
        Connection::Static("Core", "listGet"),
    ),
    ("core.type List.toList()", Connection::Translate(same)),
    (
        "core.type List.toListBuilder()",
        Connection::Translate(new_array_list),
    ),
    (
        "core.type ListBuilder.add()",
        Connection::Translate(list_add),
    ),
    (
        "core.type ListBuilder.addAll()",
        Connection::Static("Core", "listAddAll"),
    ),
    ("core.type ListBuilder.clear()", Connection::Method("clear")),
    (
        "core.type ListBuilder.constructor()",
        Connection::Translate(new_array_list),
    ),
    (
        "core.type ListBuilder.get length()",
        Connection::Method("size"),
    ),
    (
        "core.type ListBuilder.removeLast()",
        Connection::Static("Core", "listRemoveLast"),
    ),
    (
        "core.type ListBuilder.reverse()",
        Connection::Static("Collections", "reverse"),
    ),
    ("core.type ListBuilder.set()", Connection::Method("set")),
    (
        "core.type ListBuilder.sort()",
        Connection::Translate(list_sort),
    ),
    (
        "core.type ListBuilder.toList()",
        Connection::Translate(copy_of),
    ),
    (
        "core.type ListBuilder.toListBuilder()",
        Connection::Translate(new_array_list),
    ),
    (
        "core.type Listed.filter()",
        Connection::Translate(list_filter),
    ),
    (
        "core.type Listed.get isEmpty()",
        Connection::Method("isEmpty"),
    ),
    ("core.type Listed.get length()", Connection::Method("size")),
    (
        "core.type Listed.get()",
        Connection::Static("Core", "listGet"),
    ),
    (
        "core.type Listed.getOr()",
        Connection::Static("Core", "listGetOr"),
    ),
    ("core.type Listed.join()", Connection::Translate(list_join)),
    ("core.type Listed.map()", Connection::Translate(list_map)),
    (
        "core.type Listed.reduce()",
        Connection::Translate(listed_reduce),
    ),
    (
        "core.type Listed.reduceFrom()",
        Connection::Translate(listed_reduce_from),
    ),
    (
        "core.type Listed.slice()",
        Connection::Static("Core", "listSlice"),
    ),
    (
        "core.type Listed.sorted()",
        Connection::Translate(list_sorted),
    ),
    ("core.type Listed.toList()", Connection::Translate(copy_of)),
    (
        "core.type Listed.toListBuilder()",
        Connection::Translate(new_array_list),
    ),
    (
        "core.type Map.constructor()",
        Connection::Static("Core", "mapConstructor"),
    ),
    ("core.type MapBuilder.clear()", Connection::Method("clear")),
    (
        "core.type MapBuilder.constructor()",
        Connection::Translate(new_linked_hash_map),
    ),
    (
        "core.type MapBuilder.remove()",
        Connection::Static("Core", "mapBuilderRemove"),
    ),
    ("core.type MapBuilder.set()", Connection::Method("put")),
    (
        "core.type Mapped.forEach()",
        Connection::Translate(mapped_for_each),
    ),
    ("core.type Mapped.get length()", Connection::Method("size")),
    (
        "core.type Mapped.get()",
        Connection::Static("Core", "mappedGet"),
    ),
    (
        "core.type Mapped.getOr()",
        Connection::Method("getOrDefault"),
    ),
    ("core.type Mapped.has()", Connection::Method("containsKey")),
    ("core.type Mapped.keys()", Connection::Translate(keys)),
    (
        "core.type Mapped.toList()",
        Connection::Static("Core", "mappedToList"),
    ),
    (
        "core.type Mapped.toMap()",
        Connection::Static("Core", "mappedToMap"),
    ),
    (
        "core.type Mapped.toMapBuilder()",
        Connection::Translate(new_linked_hash_map),
    ),
    ("core.type Mapped.values()", Connection::Translate(values)),
    (
        "core.type Pair.constructor()",
        Connection::Translate(new_entry),
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
pub const TYPES: &[(&str, TypeConnection)] = &[
    (
        "core.type Console",
        TypeConnection {
            name: "Console",
            import: "temper.core.Core.Console",
        },
    ),
    (
        "core.type List",
        TypeConnection {
            name: "List",
            import: "java.util.List",
        },
    ),
    (
        "core.type ListBuilder",
        TypeConnection {
            name: "List",
            import: "java.util.List",
        },
    ),
    (
        "core.type Listed",
        TypeConnection {
            name: "List",
            import: "java.util.List",
        },
    ),
    (
        "core.type Map",
        TypeConnection {
            name: "Map",
            import: "java.util.Map",
        },
    ),
    (
        "core.type MapBuilder",
        TypeConnection {
            name: "Map",
            import: "java.util.Map",
        },
    ),
    (
        "core.type Mapped",
        TypeConnection {
            name: "Map",
            import: "java.util.Map",
        },
    ),
    (
        "core.type Pair",
        TypeConnection {
            name: "Entry",
            import: "java.util.Map.Entry",
        },
    ),
];

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

fn one(t: &mut Translator<'_>, args: &[NodeId], what: &str) -> Result<Expr, NotYet> {
    match args {
        [x] => t.expr(*x),
        _ => Err(NotYet {
            what: format!("`{what}` with other than its receiver"),
        }),
    }
}

/// The receiver itself: `List.toList()`.
fn same(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    one(t, args, "toList")
}

/// `new ArrayList<>()`, or of a list's items, `new ArrayList<>(x)`.
fn new_array_list(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let args = match args {
        [] => Vec::new(),
        _ => vec![one(t, args, "toListBuilder")?],
    };
    t.import("java.util.ArrayList");
    Ok(Expr::New("ArrayList<>".into(), args))
}

/// `new LinkedHashMap<>()`, which keeps its keys' order as Temper's maps
/// do, or of a map's entries, `new LinkedHashMap<>(m)`.
fn new_linked_hash_map(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let args = match args {
        [] => Vec::new(),
        _ => vec![one(t, args, "toMapBuilder")?],
    };
    t.import("java.util.LinkedHashMap");
    Ok(Expr::New("LinkedHashMap<>".into(), args))
}

/// `List.copyOf(x)`, as Kotlin's, but `Core.listCopyOf(x)` where an item
/// may be null, which `List.copyOf` rejects.
fn copy_of(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let nullable = args.first().is_some_and(|&x| t.items_may_be_null(x));
    let x = one(t, args, "toList")?;
    Ok(if nullable {
        t.import("temper.core.Core");
        Expr::name("Core").method("listCopyOf", vec![x])
    } else {
        t.import("java.util.List");
        Expr::name("List").method("copyOf", vec![x])
    })
}

/// `b.add(x)`, or with a place, `b.add(at, x)`.
fn list_add(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (b, x, at) = match args {
        [b, x] => (*b, *x, None),
        [b, x, at] if matches!(t.node(*at), Node::Omitted {}) => (*b, *x, None),
        [b, x, at] => (*b, *x, Some(*at)),
        _ => {
            return Err(NotYet {
                what: "`add` without its value".into(),
            });
        }
    };
    let b = t.expr(b)?;
    let x = t.expr(x)?;
    Ok(match at {
        None => b.method("add", vec![x]),
        Some(at) => {
            let at = t.expr(at)?;
            b.method("add", vec![at, x])
        }
    })
}

/// `List.copyOf(m.keySet())`: keys are never null.
fn keys(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let m = one(t, args, "keys")?;
    t.import("java.util.List");
    Ok(Expr::name("List").method("copyOf", vec![m.method("keySet", Vec::new())]))
}

/// `Core.listCopyOf(m.values())`, whose values may be null.
fn values(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let m = one(t, args, "values")?;
    t.import("temper.core.Core");
    Ok(Expr::name("Core").method("listCopyOf", vec![m.method("values", Vec::new())]))
}

/// A pair, `new SimpleImmutableEntry<>(key, value)`, as Kotlin's.
fn new_entry(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [k, v] = args else {
        return Err(NotYet {
            what: "a `Pair` of other than a key and a value".into(),
        });
    };
    let k = t.expr(*k)?;
    let v = t.expr(*v)?;
    t.import("java.util.AbstractMap.SimpleImmutableEntry");
    Ok(Expr::New("SimpleImmutableEntry<>".into(), vec![k, v]))
}

/// How `Core`'s list helpers name a kind of item or result: `Int`,
/// `Double`, `Bool`, or `Obj`.
fn kind_of(t: &Translator<'_>, ty: TypeId) -> &'static str {
    match t.program().types.get(ty) {
        TypeData::Scalar(Scalar::Int32) => "Int",
        TypeData::Scalar(Scalar::Float64) => "Double",
        TypeData::Scalar(Scalar::Boolean) => "Bool",
        _ => "Obj",
    }
}

/// A list's item type.
fn item_of(t: &Translator<'_>, list: NodeId) -> TypeId {
    let p = t.program();
    p.types
        .args(p.nodes.ty(list))
        .first()
        .copied()
        .unwrap_or(TypeId::NONE)
}

/// A function value's result type.
fn result_of(t: &Translator<'_>, f: NodeId) -> TypeId {
    let p = t.program();
    (temper_tmpl::analysis::fun_signature(p, p.nodes.ty(f)))
        .and_then(|s| p.types.signature(s).map(|s| s.result))
        .unwrap_or(TypeId::NONE)
}

/// A function value for a helper whose parameter's interface is `want`'s
/// (its simple name), or the value's own where `None`: adapted where they
/// differ, as `f::applyAsInt`, unless it's a lambda or method reference,
/// which Java types by where it goes.
fn fn_arg(t: &mut Translator<'_>, f: NodeId, want: Option<&str>) -> Result<Expr, NotYet> {
    let e = t.expr(f)?;
    let Some(want) = want else {
        return Ok(e);
    };
    if matches!(e, Expr::Lambda(..) | Expr::MethodRef(..)) {
        return Ok(e);
    }
    let have = t.sam(t.node_ty(f))?;
    Ok(if have.class == want {
        e
    } else {
        Expr::MethodRef(Box::new(e), have.method.to_string())
    })
}

/// `Core.NAME(args)`, imported.
fn core_call(t: &mut Translator<'_>, name: String, args: Vec<Expr>) -> Expr {
    t.import("temper.core.Core");
    Expr::name("Core").method(name, args)
}

fn list_and_fn(args: &[NodeId], what: &str) -> Result<(NodeId, NodeId), NotYet> {
    match args {
        [ls, f] => Ok((*ls, *f)),
        _ => Err(NotYet {
            what: format!("`{what}` with other than a list and a function"),
        }),
    }
}

/// `Core.listMapIntToObj(ls, f)` and the like, by the item's and the
/// result's kinds, as Kotlin's.
fn list_map(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (ls, f) = list_and_fn(args, "map")?;
    let item = kind_of(t, item_of(t, ls));
    let item = if item == "Bool" { "Obj" } else { item };
    let result = kind_of(t, result_of(t, f));
    let list = t.expr(ls)?;
    let f = fn_arg(t, f, None)?;
    Ok(core_call(
        t,
        format!("listMap{item}To{result}"),
        vec![list, f],
    ))
}

/// `Core.listFilterInt(ls, f)` and the like, by the item's kind.
fn list_filter(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (ls, f) = list_and_fn(args, "filter")?;
    let item = kind_of(t, item_of(t, ls));
    let list = t.expr(ls)?;
    let f = fn_arg(t, f, None)?;
    Ok(core_call(t, format!("listFilter{item}"), vec![list, f]))
}

/// `Core.listJoinInt(ls, separator, f)` and the like, by the item's kind.
fn list_join(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [ls, sep, f] = args else {
        return Err(NotYet {
            what: "`join` with other than a list, a separator, and a function".into(),
        });
    };
    let item = kind_of(t, item_of(t, *ls));
    let list = t.expr(*ls)?;
    let sep = t.expr(*sep)?;
    let f = fn_arg(t, *f, None)?;
    Ok(core_call(t, format!("listJoin{item}"), vec![list, sep, f]))
}

/// `Core.listSortedInt(ls, compare)` for `Int`s, else `Core.listSorted`,
/// whose comparator is a `ToIntBiFunction`.
fn list_sorted(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (ls, f) = list_and_fn(args, "sorted")?;
    let int = kind_of(t, item_of(t, ls)) == "Int";
    let list = t.expr(ls)?;
    let f = fn_arg(t, f, (!int).then_some("ToIntBiFunction"))?;
    let name = if int { "listSortedInt" } else { "listSorted" };
    Ok(core_call(t, name.into(), vec![list, f]))
}

/// `Core.listSortInt(b, compare)` for `Int`s, else `Core.listSort`.
fn list_sort(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (ls, f) = list_and_fn(args, "sort")?;
    let int = kind_of(t, item_of(t, ls)) == "Int";
    let list = t.expr(ls)?;
    let f = fn_arg(t, f, (!int).then_some("ToIntBiFunction"))?;
    let name = if int { "listSortInt" } else { "listSort" };
    Ok(core_call(t, name.into(), vec![list, f]))
}

/// `Core.listedReduceInt(ls, f)` and the like, by the item's kind; an
/// object's reducer is a `BinaryOperator`, a boolean's Core's
/// `BooleanBiPredicate`.
fn listed_reduce(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (ls, f) = list_and_fn(args, "reduce")?;
    let item = kind_of(t, item_of(t, ls));
    let want = match item {
        "Obj" => Some("BinaryOperator"),
        "Bool" => Some("BooleanBiPredicate"),
        _ => None,
    };
    let list = t.expr(ls)?;
    let f = fn_arg(t, f, want)?;
    Ok(core_call(t, format!("listedReduce{item}"), vec![list, f]))
}

/// `Core.listedReduceIntToInt(ls, initial, f)` and the like, by the
/// item's and the initial value's kinds, where both are the same kind;
/// Core's others take reducers of its own.
fn listed_reduce_from(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [ls, initial, f] = args else {
        return Err(NotYet {
            what: "`reduceFrom` with other than a list, a value, and a function".into(),
        });
    };
    let item = kind_of(t, item_of(t, *ls));
    let to = kind_of(t, t.node_ty(*initial));
    if item != to || (item == "Obj" && item_of(t, *ls) != t.node_ty(*initial)) {
        return Err(NotYet {
            what: "`reduceFrom` to another type than the items'".into(),
        });
    }
    let want = match item {
        "Obj" => Some("BinaryOperator"),
        "Bool" => Some("BooleanBiPredicate"),
        _ => None,
    };
    let list = t.expr(*ls)?;
    let initial = t.expr(*initial)?;
    let f = fn_arg(t, *f, want)?;
    Ok(core_call(
        t,
        format!("listedReduce{item}To{to}"),
        vec![list, initial, f],
    ))
}

/// `Core.mappedForEach(m, f)`.
fn mapped_for_each(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let (m, f) = list_and_fn(args, "forEach")?;
    let map = t.expr(m)?;
    let f = fn_arg(t, f, Some("BiConsumer"))?;
    Ok(core_call(t, "mappedForEach".into(), vec![map, f]))
}
