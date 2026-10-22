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
//! | `Assign` | `RightName`, value | |
//! | `Call` | callee, arguments | |
//!
//! Plus names (`RightName` resolved to a symbol), `Value` and `Error`.

use temper_syntax::tree::{NodeId, Tree};

/// A `Let` that may be assigned again.
pub const LET_VAR: u32 = 1;
pub const LET_TYPE: u32 = 2;
pub const LET_INIT: u32 = 4;

/// A `Loop` with a part to run after each pass.
pub const LOOP_NEXT: u32 = 1;

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
