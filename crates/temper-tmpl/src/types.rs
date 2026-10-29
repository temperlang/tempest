//! The type table: every type interned, so equal types have equal ids
//! (notes/tmpl/types.md, Level 3).
//!
//! The scalar builtin types are fixed entries, the same in every program;
//! a function declaration's type is its signature.

use crate::ids::TypeId;
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
    /// A signature: parameter types, result, and whether it may bubble.
    Function {
        params: TypeList,
        result: TypeId,
        bubbles: bool,
    },
    // Subset B: Nominal, TypeParam, Nullable, Never; subset C: Result.
}

#[derive(Clone, Debug)]
pub struct TypeTable {
    data: Vec<TypeData>,
    lists: Vec<TypeId>,
    functions: HashMap<(Vec<TypeId>, TypeId, bool), TypeId>,
}

impl Default for TypeTable {
    fn default() -> Self {
        TypeTable {
            data: Scalar::ALL.iter().map(|&s| TypeData::Scalar(s)).collect(),
            lists: Vec::new(),
            functions: HashMap::new(),
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

    /// Interns a signature.
    pub fn function(&mut self, params: &[TypeId], result: TypeId, bubbles: bool) -> TypeId {
        let key = (params.to_vec(), result, bubbles);
        if let Some(&id) = self.functions.get(&key) {
            return id;
        }
        let start = self.lists.len() as u32;
        self.lists.extend_from_slice(params);
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

    /// A signature's parameters, result, and whether it may bubble.
    pub fn signature(&self, id: TypeId) -> Option<(&[TypeId], TypeId, bool)> {
        match self.data.get(id.index())? {
            &TypeData::Function {
                params,
                result,
                bubbles,
            } => Some((self.list(params), result, bubbles)),
            TypeData::Scalar(_) => None,
        }
    }

    /// Whether `id` here and `other` in another table are the same type.
    pub fn same(&self, id: TypeId, others: &TypeTable, other: TypeId) -> bool {
        if !self.contains(id) || !others.contains(other) {
            return id == other;
        }
        match (self.get(id), others.get(other)) {
            (TypeData::Scalar(a), TypeData::Scalar(b)) => a == b,
            (
                TypeData::Function {
                    params: p,
                    result: r,
                    bubbles: x,
                },
                TypeData::Function {
                    params: q,
                    result: s,
                    bubbles: y,
                },
            ) => {
                let (p, q) = (self.list(p), others.list(q));
                x == y
                    && p.len() == q.len()
                    && p.iter().zip(q).all(|(&a, &b)| self.same(a, others, b))
                    && self.same(r, others, s)
            }
            _ => false,
        }
    }

    /// A type as text writes it, for messages.
    pub fn show(&self, id: TypeId) -> String {
        if id.is_none() {
            return "(no type)".into();
        }
        match self.data.get(id.index()) {
            None => format!("(missing type {})", id.0),
            Some(TypeData::Scalar(s)) => s.name().into(),
            Some(&TypeData::Function {
                params,
                result,
                bubbles,
            }) => {
                let params: Vec<String> = self.list(params).iter().map(|&t| self.show(t)).collect();
                let throws = if bubbles { " throws Bubble" } else { "" };
                format!("fn ({}): {}{throws}", params.join(", "), self.show(result))
            }
        }
    }
}
