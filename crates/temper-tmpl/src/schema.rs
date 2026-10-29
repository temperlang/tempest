//! The schema: one table declaring every node kind, its family, where its
//! type comes from, and its fields, from which `schema!` generates
//! [`Kind`], its metadata, and [`Node`] with [`Nodes::get`].
//!
//! Each field says where it's stored:
//!
//! - `a` or `b`: a payload slot, holding an id or a literal's bits;
//!   `ab`: both, for 64-bit literals;
//! - `flag`: the node's [`Flags::MAY_BUBBLE`] bit;
//! - `one`, `opt`, or `many`, then a [`Place`]: children, in order. A kind
//!   has at most one `opt` or `many` field, which takes the children that
//!   the `one` fields leave.
//!
//! Subset A (notes/tmpl/nodes.md, Level 3) only, so far.

use crate::builtins::Key;
use crate::ids::{DeclId, NodeId, StrId, TypeId};
use crate::nodes::{Flags, Kids, Nodes};
use crate::profile::Features;

/// What a kind of node is, which says where it may go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Defines a declaration: functions, parameters, and module `let`s.
    Decl,
    /// No value and no type.
    Stmt,
    /// A typed value.
    Expr,
}

/// Where a kind's type comes from: the local rules that text relies on
/// to leave types out (notes/tmpl/formats.md, Level 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeRule {
    /// Untyped: statements and declarations.
    None,
    /// Always this type: literals, `&&`, and `||`.
    Fixed(TypeId),
    /// Its declaration's type (the declaration in `a`).
    Decl,
    /// Its callee's signature's result (the function in `a`).
    Callee,
    /// Its builtin key's result.
    Key,
    /// Written in text, as `(e): T`.
    Written,
}

/// Where a field is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    A(Payload),
    B(Payload),
    AB(Payload),
    Flag,
    Kid(Arity, Place),
}

/// What a payload slot holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Payload {
    Decl,
    Str,
    Key,
    Bool,
    Int32,
    Int64,
    Float64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arity {
    One,
    Opt,
    Many,
}

/// What a child's place takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A `Param` node.
    Param,
    /// A `Block` node.
    Block,
    /// Any statement.
    Stmt,
    /// Any expression.
    Expr,
}

/// A field of a kind, as the schema declares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub slot: Slot,
}

/// A field's Rust type, and how it's decoded from the columns.
pub trait FieldValue: Sized {
    /// What it is as a payload; `None` for children.
    const PAYLOAD: Option<Payload> = None;
    fn from_slots(_a: u32, _b: u32) -> Self {
        unreachable!("not a payload")
    }
    fn from_kids(_nodes: &Nodes, _kids: Kids) -> Self {
        unreachable!("not a child")
    }
}

macro_rules! payload {
    ($($t:ty = $p:ident, |$a:ident, $b:ident| $e:expr;)*) => {$(
        impl FieldValue for $t {
            const PAYLOAD: Option<Payload> = Some(Payload::$p);
            fn from_slots($a: u32, $b: u32) -> Self {
                $e
            }
        }
    )*};
}

payload! {
    DeclId = Decl, |a, _b| DeclId(a);
    StrId = Str, |a, _b| StrId(a);
    Key = Key, |a, _b| Key(a);
    bool = Bool, |a, _b| a != 0;
    i32 = Int32, |a, _b| a as i32;
    i64 = Int64, |a, b| (a as u64 | ((b as u64) << 32)) as i64;
    f64 = Float64, |a, b| f64::from_bits(a as u64 | ((b as u64) << 32));
}

impl FieldValue for NodeId {
    fn from_kids(nodes: &Nodes, kids: Kids) -> Self {
        nodes.list(kids)[0]
    }
}

impl FieldValue for Option<NodeId> {
    fn from_kids(nodes: &Nodes, kids: Kids) -> Self {
        nodes.list(kids).first().copied()
    }
}

impl FieldValue for Kids {
    fn from_kids(_nodes: &Nodes, kids: Kids) -> Self {
        kids
    }
}

/// A 64-bit literal's payload slots.
pub(crate) fn split_bits(bits: u64) -> (u32, u32) {
    (bits as u32, (bits >> 32) as u32)
}

/// Where a field of type `$t` is stored, from its slot in the table.
macro_rules! slot {
    ($t:ty, a) => {
        Slot::A(payload::<$t>())
    };
    ($t:ty, b) => {
        Slot::B(payload::<$t>())
    };
    ($t:ty, ab) => {
        Slot::AB(payload::<$t>())
    };
    ($t:ty, flag) => {
        Slot::Flag
    };
    ($t:ty, one $place:ident) => {
        Slot::Kid(Arity::One, Place::$place)
    };
    ($t:ty, opt $place:ident) => {
        Slot::Kid(Arity::Opt, Place::$place)
    };
    ($t:ty, many $place:ident) => {
        Slot::Kid(Arity::Many, Place::$place)
    };
}

const fn payload<T: FieldValue>() -> Payload {
    match T::PAYLOAD {
        Some(p) => p,
        None => panic!("not a payload type"),
    }
}

macro_rules! decode {
    ($nodes:ident, $id:ident, $parts:ident, a) => {
        FieldValue::from_slots($nodes.slots($id).0, 0)
    };
    ($nodes:ident, $id:ident, $parts:ident, b) => {
        FieldValue::from_slots($nodes.slots($id).1, 0)
    };
    ($nodes:ident, $id:ident, $parts:ident, ab) => {
        FieldValue::from_slots($nodes.slots($id).0, $nodes.slots($id).1)
    };
    ($nodes:ident, $id:ident, $parts:ident, flag) => {
        $nodes.flags($id).contains(Flags::MAY_BUBBLE)
    };
    ($nodes:ident, $id:ident, $parts:ident, $arity:ident) => {
        FieldValue::from_kids($nodes, $parts.next().expect("malformed node"))
    };
}

macro_rules! schema {
    ($(
        $(#[doc = $doc:literal])*
        $kind:ident($family:ident, $rule:expr $(, $feature:ident)?) {
            $($field:ident: $t:ty = $slot:ident $($place:ident)?),* $(,)?
        }
    )*) => {
        /// A node's kind.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Kind {
            $($(#[doc = $doc])* $kind,)*
        }

        impl Kind {
            pub const ALL: &[Kind] = &[$(Kind::$kind),*];

            pub fn name(self) -> &'static str {
                match self {
                    $(Kind::$kind => stringify!($kind),)*
                }
            }

            pub fn family(self) -> Family {
                match self {
                    $(Kind::$kind => Family::$family,)*
                }
            }

            pub fn type_rule(self) -> TypeRule {
                match self {
                    $(Kind::$kind => $rule,)*
                }
            }

            /// The profile features a program needs to hold this kind.
            pub fn needs(self) -> Features {
                match self {
                    $(Kind::$kind => Features::NONE $(.with(Features::$feature))?,)*
                }
            }

            pub fn fields(self) -> &'static [Field] {
                match self {
                    $(Kind::$kind => {
                        const FIELDS: &[Field] = &[$(Field {
                            name: stringify!($field),
                            slot: slot!($t, $slot $($place)?),
                        }),*];
                        FIELDS
                    })*
                }
            }
        }

        /// A node, decoded by [`Nodes::get`].
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Node {
            $($(#[doc = $doc])* $kind { $($field: $t),* },)*
        }

        impl Node {
            pub fn kind(&self) -> Kind {
                match self {
                    $(Node::$kind { .. } => Kind::$kind,)*
                }
            }
        }

        impl Nodes {
            /// Decodes a node's fields. Panics if its children don't fit
            /// its kind, which the validator's S1 rules out.
            pub fn get(&self, id: NodeId) -> Node {
                let nodes = self;
                #[allow(unused_mut, unused_variables)]
                let mut parts = nodes.parts(id).expect("malformed node");
                match nodes.kind(id) {
                    $(Kind::$kind => Node::$kind {
                        $($field: decode!(nodes, id, parts, $slot)),*
                    },)*
                }
            }
        }
    };
}

use TypeRule::{Callee, Decl as DeclType, Fixed, None as Untyped, Written};

schema! {
    // Declarations, each defining the declaration in `a`.

    /// A function: its parameters, then its body.
    Function(Decl, Untyped) {
        decl: DeclId = a,
        params: Kids = many Param,
        body: NodeId = one Block,
    }
    /// A function's parameter.
    Param(Decl, Untyped) { decl: DeclId = a }
    /// A module `let` or `var`; its initializer is an `InitLet`.
    ModuleLet(Decl, Untyped) { decl: DeclId = a }

    // Statements.

    /// Statements in order, in a scope of their own.
    Block(Stmt, Untyped) { stmts: Kids = many Stmt }
    /// Declares a local, maybe without an initializer.
    Let(Stmt, Untyped) { local: DeclId = a, init: Option<NodeId> = opt Expr }
    /// Assigns a local.
    Assign(Stmt, Untyped) { local: DeclId = a, value: NodeId = one Expr }
    /// Assigns a module `var`.
    SetModuleLet(Stmt, Untyped) { target: DeclId = a, value: NodeId = one Expr }
    /// Initializes a module `let`; only in an init block.
    InitLet(Stmt, Untyped) { target: DeclId = a, value: NodeId = one Expr }
    If(Stmt, Untyped) {
        cond: NodeId = one Expr,
        then: NodeId = one Block,
        els: Option<NodeId> = opt Block,
    }
    /// A loop, declaring its label.
    While(Stmt, Untyped) {
        label: DeclId = a,
        cond: NodeId = one Expr,
        body: NodeId = one Block,
    }
    /// A loop whose body runs before its condition, declaring its label.
    DoWhile(Stmt, Untyped) {
        label: DeclId = a,
        body: NodeId = one Block,
        cond: NodeId = one Expr,
    }
    Break(Stmt, Untyped) { label: DeclId = a }
    Continue(Stmt, Untyped) { label: DeclId = a }
    /// Has a value exactly when the function's result isn't `Void`.
    Return(Stmt, Untyped) { value: Option<NodeId> = opt Expr }
    ExprStatement(Stmt, Untyped) { expr: NodeId = one Expr }

    // Expressions.

    BoolLit(Expr, Fixed(TypeId::BOOLEAN)) { value: bool = a }
    Int32Lit(Expr, Fixed(TypeId::INT32)) { value: i32 = a }
    Int64Lit(Expr, Fixed(TypeId::INT64)) { value: i64 = ab }
    Float64Lit(Expr, Fixed(TypeId::FLOAT64)) { value: f64 = ab }
    StringLit(Expr, Fixed(TypeId::STRING)) { value: StrId = a }
    VoidLit(Expr, Fixed(TypeId::VOID)) {}
    /// Reads a local or parameter.
    Local(Expr, DeclType) { local: DeclId = a }
    /// Reads a module `let`.
    GetModuleLet(Expr, DeclType) { target: DeclId = a }
    CallFunction(Expr, Callee) {
        callee: DeclId = a,
        may_bubble: bool = flag,
        args: Kids = many Expr,
    }
    /// An operator or member of a builtin type, by key.
    Builtin(Expr, TypeRule::Key) {
        key: Key = a,
        may_bubble: bool = flag,
        args: Kids = many Expr,
    }
    And(Expr, Fixed(TypeId::BOOLEAN)) { left: NodeId = one Expr, right: NodeId = one Expr }
    Or(Expr, Fixed(TypeId::BOOLEAN)) { left: NodeId = one Expr, right: NodeId = one Expr }
    IfValue(Expr, Written) {
        cond: NodeId = one Expr,
        then: NodeId = one Expr,
        els: NodeId = one Expr,
    }
    /// Statements, then a value: statements inside an expression.
    BlockValue(Expr, Written, STATEMENT_EXPRESSIONS) {
        stmts: Kids = many Stmt,
        value: NodeId = one Expr,
    }
    /// Code the frontend couldn't settle; fails when run. Valid anywhere,
    /// with any type.
    Error(Expr, Written) { message: StrId = a }
}
