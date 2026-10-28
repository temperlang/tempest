//! Static types, interned: each [`TypeId`] stands for one [`TypeData`],
//! so comparing types is comparing ids.
//!
//! Only some types are known well enough to reject a value: the final
//! builtin types ([`Types::is_final`]: `Boolean`, `Int32`, `Int64`,
//! `Float64`, `String` and `Void`), `AnyValue`, null, function types, and
//! classes and interfaces, whose supertypes typing registers
//! ([`Types::class`]). Anything else, such as a type parameter or a
//! generic type, may have subtypes that typing doesn't know about yet, so
//! a value is never rejected for it ([`Types::is_subtype`] answers
//! `None`). [`UNKNOWN`] stands for a type not worked out, and never causes
//! an error either.

use crate::builtins::TYPES;
use crate::scope::SymbolId;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

/// A type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeData {
    /// Not worked out: anything may be assigned to it or from it.
    Unknown,
    /// The type of what never completes, as `panic()` or `return`; a
    /// subtype of every type.
    Never,
    /// The type of `null`.
    Null,
    /// A builtin type, by its index in [`TYPES`]. `Int` is `Int32`.
    Builtin(u8),
    /// A class or interface without type parameters, by the symbol its
    /// declaration declares.
    Class(SymbolId),
    /// `T?`: a type or null. `T` isn't nullable itself.
    Nullable(TypeId),
    /// `fn (A, B): R`, or `fn (A, B): R throws Bubble` if `throws`. A
    /// parameter with a default is optional.
    Fn {
        params: Vec<Param>,
        ret: TypeId,
        throws: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Param {
    pub ty: TypeId,
    pub optional: bool,
}

pub const UNKNOWN: TypeId = TypeId(0);
pub const NEVER: TypeId = TypeId(1);
pub const NULL: TypeId = TypeId(2);
pub const VOID: TypeId = TypeId(3);
pub const BOOLEAN: TypeId = TypeId(4);
pub const INT32: TypeId = TypeId(5);
pub const INT64: TypeId = TypeId(6);
pub const FLOAT64: TypeId = TypeId(7);
pub const STRING: TypeId = TypeId(8);
pub const ANY_VALUE: TypeId = TypeId(9);

/// The builtin types with fixed ids, from [`VOID`] to [`ANY_VALUE`].
const WELL_KNOWN: [&str; 7] = [
    "Void", "Boolean", "Int32", "Int64", "Float64", "String", "AnyValue",
];

/// What subtyping needs to know of a class or interface.
#[derive(Clone, Debug)]
pub struct ClassInfo {
    /// Its supertypes, theirs and so on.
    pub ancestors: Vec<SymbolId>,
}

/// The interner.
#[derive(Debug)]
pub struct Types {
    data: Vec<TypeData>,
    ids: HashMap<TypeData, TypeId>,
    classes: HashMap<SymbolId, ClassInfo>,
}

impl Default for Types {
    fn default() -> Types {
        Types::new()
    }
}

impl Types {
    pub fn new() -> Types {
        let mut types = Types {
            data: Vec::new(),
            ids: HashMap::new(),
            classes: HashMap::new(),
        };
        for data in [TypeData::Unknown, TypeData::Never, TypeData::Null] {
            types.intern(data);
        }
        for name in WELL_KNOWN {
            let index = TYPES.iter().position(|&t| t == name).unwrap();
            types.intern(TypeData::Builtin(index as u8));
        }
        debug_assert_eq!(types.data.len() as u32, ANY_VALUE.0 + 1);
        types
    }

    pub fn intern(&mut self, data: TypeData) -> TypeId {
        if let Some(&id) = self.ids.get(&data) {
            return id;
        }
        let id = TypeId(self.data.len() as u32);
        self.data.push(data.clone());
        self.ids.insert(data, id);
        id
    }

    pub fn get(&self, id: TypeId) -> &TypeData {
        &self.data[id.0 as usize]
    }

    /// A class's or interface's type, with what subtyping needs to know
    /// of it, unless that's known already.
    pub fn class(&mut self, symbol: SymbolId, info: Option<ClassInfo>) -> TypeId {
        if let Some(info) = info {
            self.classes.insert(symbol, info);
        }
        self.intern(TypeData::Class(symbol))
    }

    /// Whether [`Types::class`] has been told about a class.
    pub fn knows_class(&self, symbol: SymbolId) -> bool {
        self.classes.contains_key(&symbol)
    }

    /// A builtin type by its index in [`TYPES`].
    pub fn builtin(&mut self, index: u8) -> TypeId {
        match TYPES[index as usize] {
            "Int" => INT32,
            "Never" => NEVER,
            _ => self.intern(TypeData::Builtin(index)),
        }
    }

    /// `t?`.
    pub fn nullable(&mut self, t: TypeId) -> TypeId {
        match self.get(t) {
            TypeData::Unknown | TypeData::Null | TypeData::Nullable(_) => t,
            TypeData::Never => NULL,
            _ => self.intern(TypeData::Nullable(t)),
        }
    }

    /// The type of a value that may come from either of two branches.
    pub fn join(&mut self, a: TypeId, b: TypeId) -> TypeId {
        match (self.get(a), self.get(b)) {
            _ if a == b => a,
            (TypeData::Unknown, _) | (_, TypeData::Unknown) => UNKNOWN,
            (TypeData::Never, _) => b,
            (_, TypeData::Never) => a,
            (TypeData::Null, _) => self.nullable(b),
            (_, TypeData::Null) => self.nullable(a),
            (TypeData::Nullable(x), _) if *x == b => a,
            (_, TypeData::Nullable(x)) if *x == a => b,
            _ => UNKNOWN,
        }
    }

    /// A final builtin type, which no other type extends.
    pub fn is_final(&self, t: TypeId) -> bool {
        (VOID.0..=STRING.0).contains(&t.0)
    }

    /// Whether every part of a type is worked out.
    pub fn is_known(&self, t: TypeId) -> bool {
        match self.get(t) {
            TypeData::Unknown => false,
            TypeData::Nullable(t) => self.is_known(*t),
            TypeData::Fn { params, ret, .. } => {
                params.iter().all(|p| self.is_known(p.ty)) && self.is_known(*ret)
            }
            _ => true,
        }
    }

    /// Whether a value of type `s` may be used as a `t`, if that's known.
    /// Type parameters are invariant (notes/typing.md), and one function
    /// type is a subtype of another only if they're the same. A class or
    /// interface is a subtype of what it extends, and nothing else, even
    /// if a value of an interface may be of a class that extends another
    /// type too: an `is` check narrows it to that. A function may be one
    /// of a `@fun` interface.
    pub fn is_subtype(&self, s: TypeId, t: TypeId) -> Option<bool> {
        use TypeData::*;
        // Values of these types are never null, nor of another such type.
        let definite = |x: TypeId| self.is_final(x) || matches!(self.get(x), Fn { .. });
        let is_class = |x: TypeId| matches!(self.get(x), Class(_));
        match (self.get(s), self.get(t)) {
            _ if s == t => Some(true),
            (Unknown, _) | (_, Unknown) => None,
            (Never, _) => Some(true),
            (Null, Nullable(_)) => Some(true),
            (Nullable(a), Nullable(b)) => self.is_subtype(*a, *b),
            (_, Nullable(b)) => self.is_subtype(s, *b),
            (Null | Nullable(_), _) => {
                (definite(t) || is_class(t) || t == ANY_VALUE).then_some(false)
            }
            _ if t == ANY_VALUE => (s != VOID && (definite(s) || is_class(s))).then_some(true),
            (Class(a), Class(b)) => Some(self.classes.get(a)?.ancestors.contains(b)),
            (Class(_), _) if self.is_final(t) => Some(false),
            (_, Class(_)) if self.is_final(s) => Some(false),
            (Fn { .. }, Fn { .. }) => None,
            _ => (definite(s) && definite(t)).then_some(false),
        }
    }

    /// A type as Kotlin Temper writes it in messages: `Int32`, `String?`,
    /// `fn (Int32): Boolean`, and `fn: Int32` with no parameters, but one
    /// that throws as Temper's syntax does, `fn (Int32): Int32 throws
    /// Bubble` ([`show_result`]). Classes are named by `class_name`.
    pub fn display(&self, t: TypeId, class_name: &dyn Fn(SymbolId) -> String) -> String {
        match self.get(t) {
            TypeData::Unknown => "?".into(),
            TypeData::Never => "Never".into(),
            TypeData::Null => "Null".into(),
            TypeData::Builtin(i) => TYPES[*i as usize].into(),
            TypeData::Class(symbol) => class_name(*symbol),
            TypeData::Nullable(inner) => match self.get(*inner) {
                TypeData::Fn { .. } => format!("({})?", self.display(*inner, class_name)),
                _ => format!("{}?", self.display(*inner, class_name)),
            },
            TypeData::Fn {
                params,
                ret,
                throws,
            } => {
                let params: Vec<String> = params
                    .iter()
                    .map(|p| self.display(p.ty, class_name))
                    .collect();
                let ret = show_result(self.display(*ret, class_name), *throws);
                if params.is_empty() {
                    format!("fn: {ret}")
                } else {
                    format!("fn ({}): {ret}", params.join(", "))
                }
            }
        }
    }
}

/// A result type, `ret`, as Temper's syntax writes it: `Int32 throws
/// Bubble` if it may bubble. Every message uses this, where Kotlin Temper
/// writes its internal `Result<Int32, Bubble>` or `(Int32 | Bubble)`.
pub fn show_result(ret: String, throws: bool) -> String {
    match throws {
        true => format!("{ret} throws Bubble"),
        false => ret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtypes() {
        let mut types = Types::new();
        let int_or_null = types.nullable(INT32);
        assert_eq!(types.is_subtype(INT32, int_or_null), Some(true));
        assert_eq!(types.is_subtype(NULL, int_or_null), Some(true));
        assert_eq!(types.is_subtype(int_or_null, INT32), Some(false));
        assert_eq!(types.is_subtype(INT32, FLOAT64), Some(false));
        assert_eq!(types.is_subtype(NEVER, STRING), Some(true));
        assert_eq!(types.is_subtype(STRING, ANY_VALUE), Some(true));
        assert_eq!(types.is_subtype(NULL, ANY_VALUE), Some(false));
        // AnyValue may hold an Int32.
        assert_eq!(types.is_subtype(ANY_VALUE, INT32), None);
        assert_eq!(types.is_subtype(UNKNOWN, STRING), None);
        // A class or interface is only what it extends.
        let interface = types.class(
            SymbolId(2),
            Some(ClassInfo {
                ancestors: Vec::new(),
            }),
        );
        let other = types.class(
            SymbolId(3),
            Some(ClassInfo {
                ancestors: Vec::new(),
            }),
        );
        let extends = types.class(
            SymbolId(4),
            Some(ClassInfo {
                ancestors: vec![SymbolId(2)],
            }),
        );
        assert_eq!(types.is_subtype(extends, interface), Some(true));
        assert_eq!(types.is_subtype(extends, other), Some(false));
        assert_eq!(types.is_subtype(interface, extends), Some(false));
        assert_eq!(types.is_subtype(interface, other), Some(false));
        assert_eq!(types.is_subtype(extends, STRING), Some(false));
        assert_eq!(types.is_subtype(STRING, interface), Some(false));
        assert_eq!(types.is_subtype(NULL, interface), Some(false));
        let f = types.intern(TypeData::Fn {
            params: vec![Param {
                ty: INT32,
                optional: false,
            }],
            ret: BOOLEAN,
            throws: false,
        });
        assert_eq!(types.is_subtype(f, INT32), Some(false));
        assert_eq!(types.display(f, &|_| String::new()), "fn (Int32): Boolean");
        let g = types.intern(TypeData::Fn {
            params: Vec::new(),
            ret: INT32,
            throws: true,
        });
        assert_eq!(
            types.display(g, &|_| String::new()),
            "fn: Int32 throws Bubble"
        );
        assert_eq!(types.join(NULL, INT32), int_or_null);
        assert_eq!(types.join(NEVER, STRING), STRING);
        assert_eq!(types.join(INT32, STRING), UNKNOWN);
    }
}
