//! The key table: every builtin operator and member that tmpl calls by
//! key, with its signature (notes/tmpl/analyses.md, Level 3).
//!
//! Subset A's keys are the builtin types' operators and the members of
//! `Boolean`, `Int32`, `Int64`, `Float64`, and `String` that need no other
//! types, plus `console.log`. Each operator has one key per operand type,
//! never a generic one (notes/tmpl/nodes.md, "Equality"). Parameters with
//! defaults (`toString`'s radix) are left out until subset B brings
//! `Omitted`. Each backend's table will be checked against this one.

use crate::ids::TypeId;
use std::sync::OnceLock;

/// A builtin key, by its index in [`KEYS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u32);

/// A key's name and signature.
#[derive(Debug)]
pub struct KeyInfo {
    /// As text writes it after `%`: `Int32.add`, `console.log`.
    pub name: &'static str,
    pub params: &'static [TypeId],
    pub result: TypeId,
    /// Whether a call may bubble: written `^` in text.
    pub bubbles: bool,
    /// Whether it has no side effects (bubbling aside).
    pub pure: bool,
}

impl Key {
    /// The key with this name.
    pub fn named(name: &str) -> Option<Key> {
        static SORTED: OnceLock<Vec<u32>> = OnceLock::new();
        let sorted = SORTED.get_or_init(|| {
            let mut sorted: Vec<u32> = (0..KEYS.len() as u32).collect();
            sorted.sort_by_key(|&i| KEYS[i as usize].name);
            sorted
        });
        let i = sorted
            .binary_search_by_key(&name, |&i| KEYS[i as usize].name)
            .ok()?;
        Some(Key(sorted[i]))
    }

    /// Its entry, if it's a key at all.
    pub fn info(self) -> Option<&'static KeyInfo> {
        KEYS.get(self.0 as usize)
    }
}

const V: TypeId = TypeId::VOID;
const B: TypeId = TypeId::BOOLEAN;
const I: TypeId = TypeId::INT32;
const L: TypeId = TypeId::INT64;
const F: TypeId = TypeId::FLOAT64;
const S: TypeId = TypeId::STRING;

macro_rules! keys {
    ($($name:literal ($($param:ident),*) -> $result:ident $($mark:ident)?;)*) => {
        /// Every key, in a fixed order: a key's index is its id.
        pub const KEYS: &[KeyInfo] = &[$(
            key($name, &[$($param),*], $result, mark!($($mark)?)),
        )*];
    };
}

macro_rules! mark {
    () => {
        Mark::Plain
    };
    ($mark:ident) => {
        Mark::$mark
    };
}

/// How a key differs from a plain one, which is pure and never bubbles.
enum Mark {
    Plain,
    Bubbles,
    Impure,
}

const fn key(name: &'static str, params: &'static [TypeId], result: TypeId, mark: Mark) -> KeyInfo {
    KeyInfo {
        name,
        params,
        result,
        bubbles: matches!(mark, Mark::Bubbles),
        pure: !matches!(mark, Mark::Impure),
    }
}

keys! {
    "Boolean.not" (B) -> B;
    "Boolean.eq" (B, B) -> B;
    "Boolean.lt" (B, B) -> B;
    "Boolean.le" (B, B) -> B;
    "Boolean.gt" (B, B) -> B;
    "Boolean.ge" (B, B) -> B;
    "Boolean.cmp" (B, B) -> I;
    "Boolean.toString" (B) -> S;

    "Int32.add" (I, I) -> I;
    "Int32.sub" (I, I) -> I;
    "Int32.mul" (I, I) -> I;
    "Int32.div" (I, I) -> I Bubbles;
    "Int32.mod" (I, I) -> I Bubbles;
    "Int32.neg" (I) -> I;
    "Int32.eq" (I, I) -> B;
    "Int32.lt" (I, I) -> B;
    "Int32.le" (I, I) -> B;
    "Int32.gt" (I, I) -> B;
    "Int32.ge" (I, I) -> B;
    "Int32.cmp" (I, I) -> I;
    "Int32.bitAnd" (I, I) -> I;
    "Int32.bitOr" (I, I) -> I;
    "Int32.bitXor" (I, I) -> I;
    "Int32.bitNot" (I) -> I;
    "Int32.shl" (I, I) -> I;
    "Int32.shr" (I, I) -> I;
    "Int32.ushr" (I, I) -> I;
    "Int32.min" (I, I) -> I;
    "Int32.max" (I, I) -> I;
    "Int32.toFloat64" (I) -> F;
    "Int32.toInt64" (I) -> L;
    "Int32.toString" (I) -> S;

    "Int64.add" (L, L) -> L;
    "Int64.sub" (L, L) -> L;
    "Int64.mul" (L, L) -> L;
    "Int64.div" (L, L) -> L Bubbles;
    "Int64.mod" (L, L) -> L Bubbles;
    "Int64.neg" (L) -> L;
    "Int64.eq" (L, L) -> B;
    "Int64.lt" (L, L) -> B;
    "Int64.le" (L, L) -> B;
    "Int64.gt" (L, L) -> B;
    "Int64.ge" (L, L) -> B;
    "Int64.cmp" (L, L) -> I;
    "Int64.bitAnd" (L, L) -> L;
    "Int64.bitOr" (L, L) -> L;
    "Int64.bitXor" (L, L) -> L;
    "Int64.bitNot" (L) -> L;
    "Int64.shl" (L, I) -> L;
    "Int64.shr" (L, I) -> L;
    "Int64.ushr" (L, I) -> L;
    "Int64.min" (L, L) -> L;
    "Int64.max" (L, L) -> L;
    "Int64.toFloat64" (L) -> F Bubbles;
    "Int64.toFloat64Unsafe" (L) -> F;
    "Int64.toInt32" (L) -> I Bubbles;
    "Int64.toInt32Unsafe" (L) -> I;
    "Int64.toString" (L) -> S;

    "Float64.add" (F, F) -> F;
    "Float64.sub" (F, F) -> F;
    "Float64.mul" (F, F) -> F;
    "Float64.div" (F, F) -> F Bubbles;
    "Float64.mod" (F, F) -> F Bubbles;
    "Float64.neg" (F) -> F;
    "Float64.pow" (F, F) -> F;
    "Float64.eq" (F, F) -> B;
    "Float64.lt" (F, F) -> B;
    "Float64.le" (F, F) -> B;
    "Float64.gt" (F, F) -> B;
    "Float64.ge" (F, F) -> B;
    "Float64.cmp" (F, F) -> I;
    "Float64.min" (F, F) -> F;
    "Float64.max" (F, F) -> F;
    "Float64.abs" (F) -> F;
    "Float64.acos" (F) -> F;
    "Float64.asin" (F) -> F;
    "Float64.atan" (F) -> F;
    "Float64.atan2" (F, F) -> F;
    "Float64.ceil" (F) -> F;
    "Float64.cos" (F) -> F;
    "Float64.cosh" (F) -> F;
    "Float64.exp" (F) -> F;
    "Float64.expm1" (F) -> F;
    "Float64.floor" (F) -> F;
    "Float64.log" (F) -> F;
    "Float64.log10" (F) -> F;
    "Float64.log1p" (F) -> F;
    "Float64.round" (F) -> F;
    "Float64.sign" (F) -> F;
    "Float64.sin" (F) -> F;
    "Float64.sinh" (F) -> F;
    "Float64.sqrt" (F) -> F;
    "Float64.tan" (F) -> F;
    "Float64.tanh" (F) -> F;
    "Float64.toInt32" (F) -> I Bubbles;
    "Float64.toInt32Unsafe" (F) -> I;
    "Float64.toInt64" (F) -> L Bubbles;
    "Float64.toInt64Unsafe" (F) -> L;
    "Float64.toString" (F) -> S;

    "String.eq" (S, S) -> B;
    "String.lt" (S, S) -> B;
    "String.le" (S, S) -> B;
    "String.gt" (S, S) -> B;
    "String.ge" (S, S) -> B;
    "String.cmp" (S, S) -> I;
    "String.cat" (S, S) -> S;
    "String.isEmpty" (S) -> B;
    "String.toString" (S) -> S;
    "String.toFloat64" (S) -> F Bubbles;
    "String.toInt32" (S) -> I Bubbles;
    "String.toInt64" (S) -> L Bubbles;
    "String.fromCodePoint" (I) -> S Bubbles;

    "console.log" (S) -> V Impure;
}
