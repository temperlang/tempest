//! The type table: every type interned, so equal types have equal ids
//! (notes/tmpl/types.md, Level 3).
//!
//! The scalar builtin types are fixed entries, the same in every program;
//! a function declaration's type is its signature; and a class or
//! interface is a non-generic `Nominal` type naming its declaration.

use crate::ids::{DeclId, TypeId};
use std::collections::HashMap;

/// A builtin type with no type parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scalar {
    Void,
    Boolean,
    Int32,
    Int64,
    Float64,
    String,
    AnyValue,
}

impl Scalar {
    pub const ALL: [Scalar; 7] = [
        Scalar::Void,
        Scalar::Boolean,
        Scalar::Int32,
        Scalar::Int64,
        Scalar::Float64,
        Scalar::String,
        Scalar::AnyValue,
    ];

    /// Its name, as text writes it.
    pub fn name(self) -> &'static str {
        match self {
            Scalar::Void => "Void",
            Scalar::Boolean => "Boolean",
            Scalar::Int32 => "Int32",
            Scalar::Int64 => "Int64",
            Scalar::Float64 => "Float64",
            Scalar::String => "String",
            Scalar::AnyValue => "AnyValue",
        }
    }
}

impl TypeId {
    pub const VOID: TypeId = TypeId(0);
    pub const BOOLEAN: TypeId = TypeId(1);
    pub const INT32: TypeId = TypeId(2);
    pub const INT64: TypeId = TypeId(3);
    pub const FLOAT64: TypeId = TypeId(4);
    pub const STRING: TypeId = TypeId(5);
    pub const ANY_VALUE: TypeId = TypeId(6);
}

/// A run of types in [`TypeTable::list`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeList {
    start: u32,
    len: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeData {
    Scalar(Scalar),
    /// A signature: parameter types, which are optional, the result, and
    /// whether it may bubble.
    Function {
        params: TypeList,
        result: TypeId,
        bubbles: bool,
    },
    /// A class or interface, by its declaration; no type arguments yet.
    /// Core's classes for scalars, such as `core.type Int32`, aren't
    /// `Nominal`: their declarations' type is the scalar.
    Nominal {
        decl: DeclId,
    },
    // Subset B: type arguments, TypeParam, Nullable, Never; subset C: Result.
}

/// A signature, as [`TypeTable::signature`] gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature<'a> {
    pub params: &'a [TypeId],
    /// Which parameters are optional, one per parameter.
    pub optional: &'a [bool],
    pub result: TypeId,
    pub bubbles: bool,
}

type FunctionKey = (Vec<TypeId>, Vec<bool>, TypeId, bool);

#[derive(Clone, Debug)]
pub struct TypeTable {
    data: Vec<TypeData>,
    lists: Vec<TypeId>,
    /// Beside `lists`: whether each parameter is optional.
    optional: Vec<bool>,
    functions: HashMap<FunctionKey, TypeId>,
    nominals: HashMap<DeclId, TypeId>,
}

impl Default for TypeTable {
    fn default() -> Self {
        TypeTable {
            data: Scalar::ALL.iter().map(|&s| TypeData::Scalar(s)).collect(),
            lists: Vec::new(),
            optional: Vec::new(),
            functions: HashMap::new(),
            nominals: HashMap::new(),
        }
    }
}

impl TypeTable {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn contains(&self, id: TypeId) -> bool {
        id.index() < self.data.len()
    }

    pub fn get(&self, id: TypeId) -> TypeData {
        self.data[id.index()]
    }

    pub fn list(&self, list: TypeList) -> &[TypeId] {
        &self.lists[list.start as usize..(list.start + list.len) as usize]
    }

    /// The scalar type with this name, as text writes it.
    pub fn scalar_named(name: &str) -> Option<TypeId> {
        let i = Scalar::ALL.iter().position(|s| s.name() == name)?;
        Some(TypeId(i as u32))
    }

    /// Interns a signature whose parameters are all required.
    pub fn function(&mut self, params: &[TypeId], result: TypeId, bubbles: bool) -> TypeId {
        self.function_with(params, &vec![false; params.len()], result, bubbles)
    }

    /// Interns a signature; `optional` says which parameters are, one per
    /// parameter (missing ones are required).
    pub fn function_with(
        &mut self,
        params: &[TypeId],
        optional: &[bool],
        result: TypeId,
        bubbles: bool,
    ) -> TypeId {
        let optional: Vec<bool> = (0..params.len())
            .map(|i| optional.get(i).copied().unwrap_or(false))
            .collect();
        let key = (params.to_vec(), optional, result, bubbles);
        if let Some(&id) = self.functions.get(&key) {
            return id;
        }
        let start = self.lists.len() as u32;
        self.lists.extend_from_slice(params);
        self.optional.extend_from_slice(&key.1);
        let id = TypeId(self.data.len() as u32);
        self.data.push(TypeData::Function {
            params: TypeList {
                start,
                len: params.len() as u32,
            },
            result,
            bubbles,
        });
        self.functions.insert(key, id);
        id
    }

    /// Interns a class's or interface's type.
    pub fn nominal(&mut self, decl: DeclId) -> TypeId {
        if let Some(&id) = self.nominals.get(&decl) {
            return id;
        }
        let id = TypeId(self.data.len() as u32);
        self.data.push(TypeData::Nominal { decl });
        self.nominals.insert(decl, id);
        id
    }

    /// A signature's parameters, which are optional, its result, and
    /// whether it may bubble.
    pub fn signature(&self, id: TypeId) -> Option<Signature<'_>> {
        match self.data.get(id.index())? {
            &TypeData::Function {
                params,
                result,
                bubbles,
            } => Some(Signature {
                params: self.list(params),
                optional: &self.optional
                    [params.start as usize..(params.start + params.len) as usize],
                result,
                bubbles,
            }),
            TypeData::Scalar(_) | TypeData::Nominal { .. } => None,
        }
    }

    /// Whether `id` here and `other` in another table are the same type.
    pub fn same(&self, id: TypeId, others: &TypeTable, other: TypeId) -> bool {
        if !self.contains(id) || !others.contains(other) {
            return id == other;
        }
        match (self.get(id), others.get(other)) {
            (TypeData::Scalar(a), TypeData::Scalar(b)) => a == b,
            (TypeData::Nominal { decl: a }, TypeData::Nominal { decl: b }) => a == b,
            (TypeData::Function { .. }, TypeData::Function { .. }) => {
                let (Some(x), Some(y)) = (self.signature(id), others.signature(other)) else {
                    return false;
                };
                x.bubbles == y.bubbles
                    && x.optional == y.optional
                    && x.params.len() == y.params.len()
                    && x.params
                        .iter()
                        .zip(y.params)
                        .all(|(&a, &b)| self.same(a, others, b))
                    && self.same(x.result, others, y.result)
            }
            _ => false,
        }
    }

    /// A type as text writes it, with `decl` writing a nominal type's
    /// declaration.
    pub fn show_with(&self, id: TypeId, decl: &dyn Fn(DeclId) -> String) -> String {
        if id.is_none() {
            return "(no type)".into();
        }
        match self.data.get(id.index()) {
            None => format!("(missing type {})", id.0),
            Some(TypeData::Scalar(s)) => s.name().into(),
            Some(&TypeData::Nominal { decl: d }) => decl(d),
            Some(TypeData::Function { .. }) => {
                let Some(sig) = self.signature(id) else {
                    return String::new();
                };
                let params: Vec<String> = sig
                    .params
                    .iter()
                    .zip(sig.optional)
                    .map(|(&t, &optional)| {
                        let ty = self.show_with(t, decl);
                        if optional { format!("_?: {ty}") } else { ty }
                    })
                    .collect();
                let throws = if sig.bubbles { " throws Bubble" } else { "" };
                let result = self.show_with(sig.result, decl);
                format!("fn ({}): {result}{throws}", params.join(", "))
            }
        }
    }

    /// A type as text writes it, for messages, with a nominal type's
    /// declaration as `#id`.
    pub fn show(&self, id: TypeId) -> String {
        self.show_with(id, &|d| format!("#{}", d.0))
    }
}
