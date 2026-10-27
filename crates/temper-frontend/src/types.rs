//! Static types, interned: each [`TypeId`] stands for one [`TypeData`],
//! so comparing types is comparing ids.
//!
//! Only some types are known well enough to reject a value: the final
//! builtin types ([`Types::is_final`]: `Boolean`, `Int32`, `Int64`,
//! `Float64`, `String` and `Void`), `AnyValue`, null, and function types.
//! Anything else, such as a class, a type parameter or a generic type, may
//! have subtypes that typing doesn't know about yet, so a value is never
//! rejected for it ([`Types::is_subtype`] answers `None`). [`UNKNOWN`]
//! stands for a type not worked out, and never causes an error either.

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
    /// A class or interface, by its symbol.
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

/// The interner.
#[derive(Debug)]
pub struct Types {
    data: Vec<TypeData>,
    ids: HashMap<TypeData, TypeId>,
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
    /// type is a subtype of another only if they're the same.
    pub fn is_subtype(&self, s: TypeId, t: TypeId) -> Option<bool> {
        use TypeData::*;
        // Values of these types are never null, nor of another such type.
        let definite = |x: TypeId| self.is_final(x) || matches!(self.get(x), Fn { .. });
        match (self.get(s), self.get(t)) {
            _ if s == t => Some(true),
            (Unknown, _) | (_, Unknown) => None,
            (Never, _) => Some(true),
            (Null, Nullable(_)) => Some(true),
            (Nullable(a), Nullable(b)) => self.is_subtype(*a, *b),
            (_, Nullable(b)) => self.is_subtype(s, *b),
            (Null | Nullable(_), _) => (definite(t) || t == ANY_VALUE).then_some(false),
            _ if t == ANY_VALUE => {
                (s != VOID && (definite(s) || matches!(self.get(s), Class(_)))).then_some(true)
            }
            (Fn { .. }, Fn { .. }) => None,
            _ => (definite(s) && definite(t)).then_some(false),
        }
    }

    /// A type as Kotlin Temper writes it in messages: `Int32`, `String?`,
    /// `fn (Int32): Boolean`, `fn: Int32` with no parameters, and `fn
    /// (Int32): (Int32 | Bubble)` for one that throws. Classes are named
    /// by `class_name`.
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
                let mut ret = self.display(*ret, class_name);
                if *throws {
                    ret = format!("({ret} | Bubble)");
                }
                if params.is_empty() {
                    format!("fn: {ret}")
                } else {
                    format!("fn ({}): {ret}", params.join(", "))
                }
            }
        }
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
        // AnyValue may hold an Int32, and a class may be anything's.
        assert_eq!(types.is_subtype(ANY_VALUE, INT32), None);
        let class = types.intern(TypeData::Class(SymbolId(1)));
        assert_eq!(types.is_subtype(STRING, class), None);
        assert_eq!(types.is_subtype(UNKNOWN, STRING), None);
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
        assert_eq!(types.display(g, &|_| String::new()), "fn: (Int32 | Bubble)");
        assert_eq!(types.join(NULL, INT32), int_or_null);
        assert_eq!(types.join(NEVER, STRING), STRING);
        assert_eq!(types.join(INT32, STRING), UNKNOWN);
    }
}
