//! The names every module starts with: macros, functions, values and types.
//! Macro expansion is in `macros`, and what functions do at run time in
//! `interp`.

/// A builtin, by what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    Macro(Macro),
    Function(Function),
    Value(Constant),
    /// A type, by its name in [`TYPES`]. Types aren't checked yet.
    Type(u8),
}

/// Builtins that rewrite their call, needing only its syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Macro {
    Let,
    /// `@decorator target`.
    Decorate,
    Fn,
    If,
    While,
    For,
    Do,
    Return,
    Break,
    Continue,
    Bubble,
    Assign,
    And,
    Or,
    Orelse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Function {
    Add,
    /// Subtraction, or negation with one argument.
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    /// Concatenates its arguments' strings, for `"a${b}"`.
    Cat,
    Panic,
    /// `obj.name`, a property, or as a callee, a method.
    Dot,
    /// A function type, `fn (A): B`, kept for later type checking.
    FnType,
    /// `A throws B`, a type that may fail.
    Throws,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Constant {
    True,
    False,
    Null,
    Console,
}

/// Type names, resolved but not checked yet.
pub const TYPES: &[&str] = &[
    "AnyValue", "Boolean", "Bubble", "Float64", "Int", "Int32", "Int64", "List", "ListBuilder",
    "Listed", "Map", "MapBuilder", "Mapped", "Never", "NoResult", "String", "StringBuilder", "Void",
];

/// Names and their builtins, apart from types.
pub const BUILTINS: &[(&str, Builtin)] = {
    use Builtin::{Function as F, Macro as M, Value as V};
    &[
        ("let", M(Macro::Let)),
        ("@", M(Macro::Decorate)),
        ("fn", M(Macro::Fn)),
        ("if", M(Macro::If)),
        ("while", M(Macro::While)),
        ("for", M(Macro::For)),
        ("do", M(Macro::Do)),
        ("return", M(Macro::Return)),
        ("break", M(Macro::Break)),
        ("continue", M(Macro::Continue)),
        ("bubble", M(Macro::Bubble)),
        ("=", M(Macro::Assign)),
        ("&&", M(Macro::And)),
        ("||", M(Macro::Or)),
        ("orelse", M(Macro::Orelse)),
        ("+", F(Function::Add)),
        ("-", F(Function::Sub)),
        ("*", F(Function::Mul)),
        ("/", F(Function::Div)),
        ("%", F(Function::Mod)),
        ("<", F(Function::Lt)),
        ("<=", F(Function::Le)),
        (">", F(Function::Gt)),
        (">=", F(Function::Ge)),
        ("==", F(Function::Eq)),
        ("!=", F(Function::Ne)),
        ("!", F(Function::Not)),
        ("&", F(Function::BitAnd)),
        ("|", F(Function::BitOr)),
        ("^", F(Function::BitXor)),
        ("<<", F(Function::Shl)),
        (">>", F(Function::Shr)),
        ("panic", F(Function::Panic)),
        (".", F(Function::Dot)),
        ("throws", F(Function::Throws)),
        ("true", V(Constant::True)),
        ("false", V(Constant::False)),
        ("null", V(Constant::Null)),
        ("console", V(Constant::Console)),
    ]
};

impl Builtin {
    /// The builtin with a name, if any.
    pub fn named(name: &str) -> Option<Builtin> {
        if let Some(&(_, b)) = BUILTINS.iter().find(|(n, _)| *n == name) {
            return Some(b);
        }
        TYPES.iter().position(|&t| t == name).map(|i| Builtin::Type(i as u8))
    }

    /// The name it's known by.
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Type(i) => TYPES[i as usize],
            Builtin::Function(Function::Cat) => "cat",
            Builtin::Function(Function::FnType) => "FnType",
            b => BUILTINS.iter().find(|(_, x)| *x == b).map_or("?", |(n, _)| n),
        }
    }
}
