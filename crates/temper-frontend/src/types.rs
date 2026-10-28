//! Static types, interned: each [`TypeId`] stands for one [`TypeData`],
//! so comparing types is comparing ids.
//!
//! Only some types are known well enough to reject a value: the final
//! builtin types ([`Types::is_final`]: `Boolean`, `Int32`, `Int64`,
//! `Float64`, `String` and `Void`), `AnyValue`, null, function types, and
//! classes and interfaces, whose supertypes typing registers
//! ([`Types::class`]), type parameters, which are subtypes of their bounds
//! only, and generic types given arguments, as `List<Int>`, as far as their
//! arguments tell them apart (notes/typing.md, "Generic types"). Anything
//! else may have subtypes that typing doesn't know about yet, so a value is
//! never rejected for it ([`Types::is_subtype`] answers `None`).
//! [`UNKNOWN`] stands for a type not worked out, and never causes an error
//! either.

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
    /// A type parameter, as `T` in `let f<T>(x: T)` or `class Box<T>`, by
    /// the symbol its declaration declares; its bound is in [`Types`].
    Param(SymbolId),
    /// A generic type given arguments, as `List<Int>` or `Box<String>`:
    /// `base` is a [`TypeData::Builtin`] or [`TypeData::Class`].
    Applied { base: TypeId, args: Vec<TypeId> },
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
    /// Type parameters' bounds, [`UNKNOWN`] where not worked out.
    bounds: HashMap<SymbolId, TypeId>,
}

/// Builtin generic types and the generic types they extend, with the
/// same type arguments, as `class List<T> extends Listed<T>` in core; and
/// the final types that are `MapKey`s of themselves, as `class String
/// extends MapKey<String>`. Core's `Equatable` isn't a builtin type yet.
const BUILTIN_SUPERTYPES: [(&str, &str); 9] = [
    ("List", "Listed"),
    ("ListBuilder", "Listed"),
    ("Map", "Mapped"),
    ("MapBuilder", "Mapped"),
    ("SafeGenerator", "Generator"),
    ("ValueResult", "GeneratorResult"),
    ("DoneResult", "GeneratorResult"),
    ("Int32", "MapKey"),
    ("String", "MapKey"),
];

/// The builtin generic type that builtin generic type `name` extends with
/// the same type arguments, as `Listed` for `List`, if any.
pub fn builtin_supertype(name: &str) -> Option<&'static str> {
    (BUILTIN_SUPERTYPES.iter())
        .find(|&&(sub, _)| sub == name && !WELL_KNOWN.contains(&sub))
        .map(|&(_, sup)| sup)
}

/// Builtin generic types that core declares as classes, which no class
/// extends; core marks the first four `@mayDowncastTo(false)`.
const BUILTIN_GENERIC_CLASSES: [&str; 10] = [
    "List",
    "ListBuilder",
    "Map",
    "MapBuilder",
    "Deque",
    "DoneResult",
    "Pair",
    "Promise",
    "PromiseBuilder",
    "ValueResult",
];

/// How many of [`BUILTIN_GENERIC_CLASSES`] are `@mayDowncastTo(false)`.
const NO_DOWNCAST: usize = 4;

/// The builtin generic types' type parameters, by index, that core marks
/// `@out` or `@in`, which Kotlin Temper honors and tempest doesn't
/// (notes/typing.md, "Type parameters are invariant").
const BUILTIN_VARIANT_PARAMS: [(&str, usize); 13] = [
    ("DoneResult", 0),
    ("Generator", 0),
    ("GeneratorResult", 0),
    ("List", 0),
    ("Map", 0),
    ("Map", 1),
    ("Mapped", 0),
    ("Never", 0),
    ("Pair", 0),
    ("Pair", 1),
    ("Promise", 0),
    ("SafeGenerator", 0),
    ("ValueResult", 0),
];

/// How deep [`Types::is_subtype`] looks, through type parameters' bounds
/// and type arguments, before giving up.
const MAX_SUBTYPE_DEPTH: u32 = 16;

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
            bounds: HashMap::new(),
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

    /// A type parameter's type. Its bound is set by
    /// [`Types::set_bound`], unknown until then.
    pub fn param(&mut self, symbol: SymbolId) -> TypeId {
        self.bounds.entry(symbol).or_insert(UNKNOWN);
        self.intern(TypeData::Param(symbol))
    }

    /// Whether [`Types::param`] has been asked for a type parameter.
    pub fn knows_param(&self, symbol: SymbolId) -> bool {
        self.bounds.contains_key(&symbol)
    }

    /// Sets a type parameter's bound: `AnyValue` if it has none. A bound
    /// that's a type parameter, as `U` in `<T extends U>`, is left unknown
    /// for now, so bounds can't lead around in a circle.
    pub fn set_bound(&mut self, symbol: SymbolId, bound: TypeId) {
        let bound = match self.get(bound) {
            TypeData::Param(_) => UNKNOWN,
            _ => bound,
        };
        self.bounds.insert(symbol, bound);
    }

    /// What a type is known to be a subtype of: a type parameter's bound,
    /// or the type itself.
    pub fn upper(&self, t: TypeId) -> TypeId {
        match self.get(t) {
            TypeData::Param(symbol) => self.bounds.get(symbol).copied().unwrap_or(UNKNOWN),
            _ => t,
        }
    }

    /// `t` with the type parameters `formals` unknown, as a generic
    /// function's signature is seen where its calls aren't solved.
    pub fn erase(&mut self, t: TypeId, formals: &[SymbolId]) -> TypeId {
        self.substitute(t, formals, &vec![UNKNOWN; formals.len()])
    }

    /// `t` with each of the type parameters `formals` replaced by the
    /// type at its index in `actuals`, as a generic call's solution gives
    /// them.
    pub fn substitute(&mut self, t: TypeId, formals: &[SymbolId], actuals: &[TypeId]) -> TypeId {
        if formals.is_empty() {
            return t;
        }
        match self.get(t).clone() {
            TypeData::Param(symbol) => match formals.iter().position(|&f| f == symbol) {
                Some(i) => actuals[i],
                None => t,
            },
            TypeData::Nullable(inner) => {
                let inner = self.substitute(inner, formals, actuals);
                self.nullable(inner)
            }
            TypeData::Applied { base, args } => {
                let args = (args.iter())
                    .map(|&a| self.substitute(a, formals, actuals))
                    .collect();
                self.intern(TypeData::Applied { base, args })
            }
            TypeData::Fn {
                params,
                ret,
                throws,
            } => {
                let params = (params.iter())
                    .map(|p| Param {
                        ty: self.substitute(p.ty, formals, actuals),
                        optional: p.optional,
                    })
                    .collect();
                let ret = self.substitute(ret, formals, actuals);
                self.intern(TypeData::Fn {
                    params,
                    ret,
                    throws,
                })
            }
            _ => t,
        }
    }

    /// Whether `t` mentions one of the type parameters `formals`.
    pub fn mentions(&self, t: TypeId, formals: &[SymbolId]) -> bool {
        match self.get(t) {
            TypeData::Param(symbol) => formals.contains(symbol),
            TypeData::Nullable(inner) => self.mentions(*inner, formals),
            TypeData::Applied { args, .. } => args.iter().any(|&a| self.mentions(a, formals)),
            TypeData::Fn { params, ret, .. } => {
                params.iter().any(|p| self.mentions(p.ty, formals)) || self.mentions(*ret, formals)
            }
            _ => false,
        }
    }

    /// Solves the type parameters `formals` from a value of type `actual`
    /// passed where a `pattern` is wanted, part by part: `T` from `Int32`,
    /// `List<T>` from `List<Int32>` or `ListBuilder<Int32>`, `T?` from
    /// `Int32?` or `Int32`, and function types' parameters and results.
    /// Each solution goes into `actuals` at the formal's index; one solved
    /// two different ways goes into `conflicts` too. Only fully known
    /// types are solutions, and not null, `Never`, `Void` or a nullable
    /// type for a bare `T`, whose bound is never nullable.
    pub fn unify(
        &self,
        pattern: TypeId,
        actual: TypeId,
        formals: &[SymbolId],
        actuals: &mut [TypeId],
        conflicts: &mut [bool],
    ) {
        use TypeData::*;
        match (self.get(pattern), self.get(actual)) {
            (Param(symbol), _) => {
                let Some(i) = formals.iter().position(|f| f == symbol) else {
                    return;
                };
                if matches!(self.get(actual), Never | Null | Nullable(_))
                    || actual == VOID
                    || !self.is_known(actual)
                {
                    return;
                }
                match actuals[i] {
                    UNKNOWN => actuals[i] = actual,
                    solved if solved != actual => conflicts[i] = true,
                    _ => {}
                }
            }
            (Nullable(p), Nullable(a)) => self.unify(*p, *a, formals, actuals, conflicts),
            (Nullable(p), _) => self.unify(*p, actual, formals, actuals, conflicts),
            (Applied { base: p, args: x }, Applied { base: a, args: y })
                if (p == a || self.builtin_extends(*a, *p)) && x.len() == y.len() =>
            {
                for (&x, &y) in x.iter().zip(y) {
                    self.unify(x, y, formals, actuals, conflicts);
                }
            }
            (
                Fn {
                    params: x, ret: p, ..
                },
                Fn {
                    params: y, ret: a, ..
                },
            ) if x.len() == y.len() => {
                for (x, y) in x.iter().zip(y) {
                    self.unify(x.ty, y.ty, formals, actuals, conflicts);
                }
                self.unify(*p, *a, formals, actuals, conflicts);
            }
            _ => {}
        }
    }

    /// `t` as the generic type `wanted`'s base, if `t`'s base is a builtin
    /// type extending it with the same arguments, as `Listed<T>` for a
    /// `List<T>` where a `Listed<String>` is wanted; else `t`.
    pub fn as_supertype(&mut self, t: TypeId, wanted: TypeId) -> TypeId {
        let (base, sup) = (self.base(t), self.base(wanted));
        if base == sup || !self.builtin_extends(base, sup) || self.args(t).is_empty() {
            return t;
        }
        let args = self.args(t).to_vec();
        self.intern(TypeData::Applied { base: sup, args })
    }

    /// Whether a type mentions a type parameter or type arguments.
    pub fn is_generic(&self, t: TypeId) -> bool {
        match self.get(t) {
            TypeData::Param(_) | TypeData::Applied { .. } => true,
            TypeData::Nullable(t) => self.is_generic(*t),
            TypeData::Fn { params, ret, .. } => {
                params.iter().any(|p| self.is_generic(p.ty)) || self.is_generic(*ret)
            }
            _ => false,
        }
    }

    /// A builtin type's name, if `t` is one.
    pub fn builtin_name(&self, t: TypeId) -> Option<&'static str> {
        match self.get(t) {
            TypeData::Builtin(i) => Some(TYPES[*i as usize]),
            _ => None,
        }
    }

    /// Whether `t`, or a generic type's base, is a builtin class, which no
    /// class extends: a final builtin type or one of core's generic
    /// classes, as `List`.
    pub fn is_builtin_class(&self, t: TypeId) -> bool {
        let base = self.base(t);
        self.is_final(base)
            || (self.builtin_name(base)).is_some_and(|n| BUILTIN_GENERIC_CLASSES.contains(&n))
    }

    /// Whether core marks `t`, or a generic type's base, `@mayDowncastTo(false)`:
    /// the final builtin types other than `Void`, and `List`, `Map` and
    /// their builders.
    pub fn no_downcast(&self, t: TypeId) -> bool {
        let base = self.base(t);
        (self.is_final(base) && base != VOID)
            || (self.builtin_name(base))
                .is_some_and(|n| BUILTIN_GENERIC_CLASSES[..NO_DOWNCAST].contains(&n))
    }

    /// Whether core marks a builtin generic type's `index`th type
    /// parameter `@out` or `@in`.
    pub fn is_builtin_variant(&self, base: TypeId, index: usize) -> bool {
        (self.builtin_name(base)).is_some_and(|n| BUILTIN_VARIANT_PARAMS.contains(&(n, index)))
    }

    /// A generic type's base, as `List` for `List<Int>`, or `t` itself.
    pub fn base(&self, t: TypeId) -> TypeId {
        match self.get(t) {
            TypeData::Applied { base, .. } => *base,
            _ => t,
        }
    }

    /// A generic type's arguments, none for another type.
    pub fn args(&self, t: TypeId) -> &[TypeId] {
        match self.get(t) {
            TypeData::Applied { args, .. } => args,
            _ => &[],
        }
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
            TypeData::Applied { base, args } => {
                self.is_known(*base) && args.iter().all(|&a| self.is_known(a))
            }
            TypeData::Fn { params, ret, .. } => {
                params.iter().all(|p| self.is_known(p.ty)) && self.is_known(*ret)
            }
            _ => true,
        }
    }

    /// Whether a value of type `s` may be used as a `t`, if that's known.
    /// One generic type given arguments is another's only if they're the
    /// same, but reported as not only where an argument sets them apart
    /// under any variance (notes/typing.md, "Generic types"), and one
    /// function type is a subtype of another only if they're the same. A
    /// type parameter is a subtype of its bound. A class or
    /// interface is a subtype of what it extends, and nothing else, even
    /// if a value of an interface may be of a class that extends another
    /// type too: an `is` check narrows it to that. A function may be one
    /// of a `@fun` interface.
    pub fn is_subtype(&self, s: TypeId, t: TypeId) -> Option<bool> {
        self.subtype(s, t, 0)
    }

    fn subtype(&self, s: TypeId, t: TypeId, depth: u32) -> Option<bool> {
        use TypeData::*;
        if depth > MAX_SUBTYPE_DEPTH {
            return None;
        }
        let depth = depth + 1;
        // Values of these types are never null, nor of another such type.
        let definite = |x: TypeId| self.is_final(x) || matches!(self.get(x), Fn { .. });
        let is_class = |x: TypeId| matches!(self.get(x), Class(_) | Param(_) | Applied { .. });
        match (self.get(s), self.get(t)) {
            _ if s == t => Some(true),
            (Unknown, _) | (_, Unknown) => None,
            (Never, _) => Some(true),
            (Null, Nullable(_)) => Some(true),
            (Nullable(a), Nullable(b)) => self.subtype(*a, *b, depth),
            (_, Nullable(b)) => self.subtype(s, *b, depth),
            // Unless its bound is nullable, `T` isn't.
            (Null | Nullable(_), Param(_))
                if self.subtype(NULL, self.upper(t), depth) != Some(false) =>
            {
                None
            }
            (Null | Nullable(_), _) => {
                (definite(t) || is_class(t) || t == ANY_VALUE).then_some(false)
            }
            // A type parameter is a subtype of its bound only, and nothing
            // but itself and `Never` is a subtype of it.
            (Param(_), _) => match (self.upper(s), self.subtype(self.upper(s), t, depth)) {
                // What an `AnyValue` may hold, a `T` needn't.
                (ANY_VALUE, None) => Some(false),
                (_, known) => known,
            },
            (_, Param(_)) => Some(false),
            _ if t == ANY_VALUE => (s != VOID && (definite(s) || is_class(s))).then_some(true),
            (Applied { base: a, args: x }, Applied { base: b, args: y }) => {
                // Arguments neither of which is a subtype of the other set
                // types apart, whatever the parameter's variance.
                let unrelated = |(&x, &y): (&TypeId, &TypeId)| {
                    self.subtype(x, y, depth) == Some(false)
                        && self.subtype(y, x, depth) == Some(false)
                };
                let apart = || x.len() == y.len() && x.iter().zip(y).any(unrelated);
                match (self.get(*a), self.get(*b)) {
                    // The same arguments of a supertype fit under any variance.
                    _ if a != b && self.builtin_extends(*a, *b) && x == y => Some(true),
                    _ if a == b || self.builtin_extends(*a, *b) => apart().then_some(false),
                    // Builtin types extend only the builtin types they're
                    // known to, and no user's class or interface.
                    (Builtin(_), _) => Some(false),
                    _ => None,
                }
            }
            (Applied { base, .. }, Class(_)) if matches!(self.get(*base), Builtin(_)) => {
                Some(false)
            }
            (Applied { .. }, _) if definite(t) => Some(false),
            // `String` is a `MapKey<String>`.
            (_, Applied { base, args }) if definite(s) => match self.builtin_extends(s, *base) {
                true if args[..] == [s] => Some(true),
                true => (args.len() == 1)
                    .then(|| self.subtype(s, args[0], depth))?
                    .filter(|&is| !is),
                false => Some(false),
            },
            (Applied { .. }, _) | (_, Applied { .. }) => None,
            (Class(a), Class(b)) => Some(self.classes.get(a)?.ancestors.contains(b)),
            (Class(_), _) if self.is_final(t) => Some(false),
            (_, Class(_)) if self.is_final(s) => Some(false),
            (Fn { .. }, Fn { .. }) => None,
            _ => (definite(s) && definite(t)).then_some(false),
        }
    }

    /// Whether builtin type `sub` extends builtin type `sup`, both bases
    /// of generic types, and so takes the same type arguments, or is
    /// a final type that's a `MapKey` of itself, as core declares them.
    fn builtin_extends(&self, sub: TypeId, sup: TypeId) -> bool {
        let (TypeData::Builtin(sub), TypeData::Builtin(sup)) = (self.get(sub), self.get(sup))
        else {
            return false;
        };
        let (sub, sup) = (TYPES[*sub as usize], TYPES[*sup as usize]);
        BUILTIN_SUPERTYPES.contains(&(sub, sup))
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
            TypeData::Class(symbol) | TypeData::Param(symbol) => class_name(*symbol),
            TypeData::Applied { base, args } => {
                let args: Vec<String> = args.iter().map(|&a| self.display(a, class_name)).collect();
                format!("{}<{}>", self.display(*base, class_name), args.join(", "))
            }
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

    #[test]
    fn builtin_supertypes_are_cores() {
        let core = include_str!("../../../vendor/temper/core/core.temper");
        for (sub, sup) in BUILTIN_SUPERTYPES {
            let declared = core.lines().any(|line| {
                let line = line.trim_start_matches("export ");
                let declares = ["class ", "interface ", "sealed interface "]
                    .iter()
                    .any(|kind| line.starts_with(&format!("{kind}{sub}")));
                declares && line.contains(&format!(" extends {sup}<"))
            });
            assert!(declared, "{sub} extends {sup}");
        }
    }

    #[test]
    fn builtin_generics_are_cores() {
        let core = include_str!("../../../vendor/temper/core/core.temper");
        let mut classes = Vec::new();
        let mut no_downcast = Vec::new();
        let mut variant = Vec::new();
        let mut previous = "";
        for line in core.lines() {
            let decl = line.trim_start_matches("export ");
            let decl = decl.trim_start_matches("/*sealed*/ ");
            let decl = decl.trim_start_matches("sealed ");
            let (is_class, rest) =
                match (decl.strip_prefix("class "), decl.strip_prefix("interface ")) {
                    (Some(rest), _) => (true, rest),
                    (_, Some(rest)) => (false, rest),
                    _ => {
                        previous = line;
                        continue;
                    }
                };
            let Some((name, params)) = rest.split_once('<') else {
                previous = line;
                continue;
            };
            if !line.starts_with("export") || !name.chars().all(char::is_alphanumeric) {
                previous = line;
                continue;
            }
            // The parameters, split at commas outside nested `<…>`.
            let (mut depth, mut pieces) = (0, vec![String::new()]);
            for c in params.chars() {
                match c {
                    '>' if depth == 0 => break,
                    ',' if depth == 0 => pieces.push(String::new()),
                    _ => {
                        depth += i32::from(c == '<') - i32::from(c == '>');
                        pieces.last_mut().unwrap().push(c);
                    }
                }
            }
            for (index, piece) in pieces.iter().enumerate() {
                let piece = piece.trim_start();
                if piece.starts_with("@out ") || piece.starts_with("@in ") {
                    variant.push((name, index));
                }
            }
            if is_class {
                classes.push(name);
                if previous.starts_with("@mayDowncastTo(false)") {
                    no_downcast.push(name);
                }
            }
            previous = line;
        }
        let sorted = |mut v: Vec<&'static str>| {
            v.sort();
            v
        };
        assert_eq!(sorted(classes), sorted(BUILTIN_GENERIC_CLASSES.to_vec()));
        assert_eq!(
            sorted(no_downcast),
            sorted(BUILTIN_GENERIC_CLASSES[..NO_DOWNCAST].to_vec())
        );
        variant.sort();
        assert_eq!(variant, BUILTIN_VARIANT_PARAMS.to_vec());
    }

    #[test]
    fn generic_subtypes() {
        let mut types = Types::new();
        let list = TYPES.iter().position(|&t| t == "List").unwrap();
        let list = types.builtin(list as u8);
        let applied = |types: &mut Types, arg| {
            types.intern(TypeData::Applied {
                base: list,
                args: vec![arg],
            })
        };
        let ints = applied(&mut types, INT32);
        let strings = applied(&mut types, STRING);
        let values = applied(&mut types, ANY_VALUE);
        assert_eq!(types.is_subtype(ints, strings), Some(false));
        // `List<Int32>` may be a `List<AnyValue>`, as `List` is `@out`
        // in Kotlin Temper.
        assert_eq!(types.is_subtype(ints, values), None);
        assert_eq!(types.is_subtype(ints, ANY_VALUE), Some(true));
        assert_eq!(types.is_subtype(ints, STRING), Some(false));
        assert_eq!(types.display(ints, &|_| String::new()), "List<Int32>");
        // A type parameter is a subtype of its bound only.
        let t = types.param(SymbolId(5));
        types.set_bound(SymbolId(5), ANY_VALUE);
        let u = types.param(SymbolId(6));
        assert_eq!(types.is_subtype(t, ANY_VALUE), Some(true));
        assert_eq!(types.is_subtype(t, INT32), Some(false));
        assert_eq!(types.is_subtype(INT32, t), Some(false));
        assert_eq!(types.is_subtype(NULL, t), Some(false));
        // `u`'s bound isn't known.
        assert_eq!(types.is_subtype(u, INT32), None);
        assert_eq!(types.is_subtype(u, t), None);
        let ts = applied(&mut types, t);
        assert_eq!(types.is_subtype(ts, ints), Some(false));
        // Builtin generic types extend those core says they do.
        let index = |name: &str| TYPES.iter().position(|&t| t == name).unwrap() as u8;
        let (listed, builder, map_key) = (
            types.builtin(index("Listed")),
            types.builtin(index("ListBuilder")),
            types.builtin(index("MapKey")),
        );
        let listed_strings = types.intern(TypeData::Applied {
            base: listed,
            args: vec![STRING],
        });
        let listed_ints = types.intern(TypeData::Applied {
            base: listed,
            args: vec![INT32],
        });
        let built_ints = types.intern(TypeData::Applied {
            base: builder,
            args: vec![INT32],
        });
        assert_eq!(types.is_subtype(ints, listed_ints), Some(true));
        assert_eq!(types.is_subtype(ints, listed_strings), Some(false));
        assert_eq!(types.is_subtype(built_ints, ints), Some(false));
        assert_eq!(types.is_subtype(listed_ints, ints), Some(false));
        let string_key = types.intern(TypeData::Applied {
            base: map_key,
            args: vec![STRING],
        });
        let int_key = types.intern(TypeData::Applied {
            base: map_key,
            args: vec![INT32],
        });
        assert_eq!(types.is_subtype(STRING, string_key), Some(true));
        assert_eq!(types.is_subtype(STRING, int_key), Some(false));
        assert_eq!(types.is_subtype(FLOAT64, int_key), Some(false));
        assert_eq!(
            types.erase(ts, &[SymbolId(5)]),
            types.intern(TypeData::Applied {
                base: list,
                args: vec![UNKNOWN]
            })
        );
    }
}
