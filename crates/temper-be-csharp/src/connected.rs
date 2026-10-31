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
    (
        "core.type Float64.toInt32()",
        Connection::Static("Float64", "ToInt"),
    ),
    (
        "core.type Float64.toInt32Unsafe()",
        Connection::Translate(cast_int),
    ),
    (
        "core.type Float64.toInt64()",
        Connection::Static("Float64", "ToInt64"),
    ),
    (
        "core.type Float64.toInt64Unsafe()",
        Connection::Translate(cast_long),
    ),
    (
        "core.type Float64.toString()",
        Connection::Translate(float_to_string),
    ),
    ("core.type Int32.max()", Connection::Static("Math", "Max")),
    ("core.type Int32.min()", Connection::Static("Math", "Min")),
    (
        "core.type Int32.toFloat64()",
        Connection::Translate(cast_double),
    ),
    (
        "core.type Int32.toInt64()",
        Connection::Translate(cast_long),
    ),
    (
        "core.type Int32.toString()",
        Connection::Translate(int_to_string),
    ),
    ("core.type Int64.max()", Connection::Static("Math", "Max")),
    ("core.type Int64.min()", Connection::Static("Math", "Min")),
    (
        "core.type Int64.toFloat64()",
        Connection::Static("Float64", "ToFloat64"),
    ),
    (
        "core.type Int64.toFloat64Unsafe()",
        Connection::Translate(cast_double),
    ),
    (
        "core.type Int64.toInt32()",
        Connection::Static("Core", "ToInt"),
    ),
    (
        "core.type Int64.toInt32Unsafe()",
        Connection::Translate(cast_int),
    ),
    (
        "core.type Int64.toString()",
        Connection::Translate(int_to_string),
    ),
    (
        "core.type String.get isEmpty()",
        Connection::Translate(is_empty),
    ),
    (
        "core.type String.toFloat64()",
        Connection::Translate(string_to_float),
    ),
    (
        "core.type String.toInt32()",
        Connection::Static("Core", "ToInt"),
    ),
    (
        "core.type String.toInt64()",
        Connection::Static("Core", "ToInt64"),
    ),
];

/// A helper method: its name, result, parameters (type and name), and
/// body.
pub struct Helper {
    pub name: &'static str,
    pub result: &'static str,
    pub params: &'static [(&'static str, &'static str)],
    pub body: &'static str,
}

/// Helpers that a module's class gets where it uses them, as `internal`
/// static methods: each name, result, parameters, and body. Kotlin's
/// runtime's `Float64.Format` and `Float64.ToFloat64(string)` format and
/// parse by the current culture (`1,5` and a `−` in `sv-SE`); these are
/// theirs by the invariant culture, and the parse takes no thousands
/// separators.
pub const HELPERS: &[Helper] = &[
    Helper {
        name: "FormatFloat64",
        result: "string",
        params: &[("double", "d")],
        body: "if (double.IsInfinity(d))
{
    return d < 0 ? \"-Infinity\" : \"Infinity\";
}
if (double.IsNaN(d))
{
    return \"NaN\";
}
string s = d.ToString(global::System.Globalization.CultureInfo.InvariantCulture);
int e = s.IndexOf('E', global::System.StringComparison.Ordinal);
string main = e == -1 ? s : s[..e];
string frac = main.Contains('.', global::System.StringComparison.Ordinal) ? \"\" : \".0\";
string exp = e == -1 ? \"\" : string.Concat(\"e\", s.AsSpan(e + 1));
return main + frac + exp;",
    },
    Helper {
        name: "ParseFloat64",
        result: "double",
        params: &[("string", "s")],
        body: "string trimmed = s.Trim();
if (trimmed.StartsWith('.') || trimmed.EndsWith('.'))
{
    throw new global::System.FormatException();
}
return trimmed switch
{
    \"Infinity\" => double.PositiveInfinity,
    \"-Infinity\" => double.NegativeInfinity,
    _ => double.Parse(
        trimmed,
        global::System.Globalization.NumberStyles.Float,
        global::System.Globalization.CultureInfo.InvariantCulture),
};",
    },
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

/// `(int)x`, `(long)x`, and `(double)x`: conversions C# writes as casts,
/// as Kotlin's.
fn cast(t: &mut Translator<'_>, args: &[NodeId], ty: &str) -> Result<Expr, NotYet> {
    let [x] = args else {
        return Err(NotYet {
            what: format!("a conversion to {ty} without its receiver"),
        });
    };
    Ok(Expr::Cast(ty.into(), Box::new(t.expr(*x)?)))
}

fn cast_int(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    cast(t, args, "int")
}

fn cast_long(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    cast(t, args, "long")
}

fn cast_double(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    cast(t, args, "double")
}

/// The module's `FormatFloat64(x)` ([`HELPERS`]), where Kotlin's is the
/// runtime's `Float64.Format(x)`, by the current culture.
fn float_to_string(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [x] = args else {
        return Err(NotYet {
            what: "`Float64.toString` without its receiver".into(),
        });
    };
    let x = t.expr(*x)?;
    Ok(t.helper("FormatFloat64").call(vec![x]))
}

/// The module's `ParseFloat64(s)` ([`HELPERS`]), where Kotlin's is the
/// runtime's `Float64.ToFloat64(s)`, by the current culture.
fn string_to_float(t: &mut Translator<'_>, args: &[NodeId]) -> Result<Expr, NotYet> {
    let [s] = args else {
        return Err(NotYet {
            what: "`String.toFloat64` without its receiver".into(),
        });
    };
    let s = t.expr(*s)?;
    Ok(t.helper("ParseFloat64").call(vec![s]))
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
