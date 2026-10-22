//! The core constructs that macros reduce everything to
//! (notes/milestone-3.md), as node kinds of the shared tree, with their
//! layouts:
//!
//! | Kind | Children | Data |
//! |---|---|---|
//! | `Block` | statements; its value is the last one's | |
//! | `If` | condition, then, else (an empty block if none) | |
//! | `Loop` | body, then `next` if any, run after the body and on `continue` | [`LOOP_NEXT`] |
//! | `Break`, `Continue` | | the target `Block` or `Loop` |
//! | `Return` | value | the `Fn` |
//! | `Orelse` | body, fallback | |
//! | `Fail` | | |
//! | `Fn` | parameters (each a `Let`), return type if any, body | parameter count, [`FN_OUT_TYPE`] |
//! | `Let` | `LeftName`, type if any, initial value if any | [`LET_VAR`], [`LET_TYPE`], [`LET_INIT`] |
//! | `Assign` | `RightName` or `obj.name` (a `.` call), value | |
//! | `Call` | callee, arguments | |
//! | `Class` | `LeftName`, supertypes, members (each a `Let`) | supertype count, [`CLASS_INTERFACE`] |
//!
//! Plus names (`RightName` resolved to a symbol), `Value` and `Error`.
//!
//! A member is a `Let` whose data also has a role ([`ROLE_MASK`]) and maybe
//! [`MEMBER_PRIVATE`]. A field's value is its initializer, and a method's,
//! getter's, setter's or constructor's is its `Fn`, or none if abstract.

use temper_syntax::tree::{NodeId, Tree};

/// A `Let` that may be assigned again.
pub const LET_VAR: u32 = 1;
pub const LET_TYPE: u32 = 2;
pub const LET_INIT: u32 = 4;

/// A `Loop` with a part to run after each pass.
pub const LOOP_NEXT: u32 = 1;

/// A `Class` that's an interface.
pub const CLASS_INTERFACE: u32 = 1 << 16;

/// A member `Let`'s role: a field (0) or one of these.
pub const ROLE_MASK: u32 = 7 << 8;
/// A field declared in the class's header, a constructor parameter.
pub const ROLE_PARAM: u32 = 1 << 8;
pub const ROLE_METHOD: u32 = 2 << 8;
pub const ROLE_GETTER: u32 = 3 << 8;
pub const ROLE_SETTER: u32 = 4 << 8;
pub const ROLE_CONSTRUCTOR: u32 = 5 << 8;
/// A member only its class may use.
pub const MEMBER_PRIVATE: u32 = 1 << 11;

/// A `Class`'s name, supertypes and members.
pub fn class_parts(tree: &Tree, node: NodeId) -> (NodeId, &[NodeId], &[NodeId]) {
    let count = (tree.data(node) & 0xffff) as usize;
    let children = tree.children(node);
    (children[0], &children[1..1 + count], &children[1 + count..])
}

/// A `Fn` with a declared return type.
pub const FN_OUT_TYPE: u32 = 1 << 31;

/// A `Let`'s name, type and initial value.
pub fn let_parts(tree: &Tree, node: NodeId) -> (NodeId, Option<NodeId>, Option<NodeId>) {
    let flags = tree.data(node);
    let children = tree.children(node);
    let mut i = 1;
    let mut next = |present: bool| {
        present.then(|| {
            i += 1;
            children[i - 1]
        })
    };
    let ty = next(flags & LET_TYPE != 0);
    let init = next(flags & LET_INIT != 0);
    (children[0], ty, init)
}

/// A `Fn`'s parameters, return type and body.
pub fn fn_parts(tree: &Tree, node: NodeId) -> (Vec<NodeId>, Option<NodeId>, NodeId) {
    let data = tree.data(node);
    let count = (data & !FN_OUT_TYPE) as usize;
    let children = tree.children(node);
    let out_type = (data & FN_OUT_TYPE != 0).then(|| children[count]);
    (children[..count].to_vec(), out_type, *children.last().unwrap())
}
