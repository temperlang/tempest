//! Builtins' signatures, the third slice of milestone 4: the variants of
//! each operator as Kotlin Temper declares them (`BuiltinFuns.kt` and
//! `BuiltinOperatorSpecs.kt`), which one a call picks, as Kotlin's solver
//! does, and `console.log`'s. `<`, `<=`, `>` and `>=` take `<=>`'s
//! variants, as Kotlin desugars them to it.
//!
//! An operator with several variants reports arguments that don't fit the
//! one picked as "Actual arguments do not match signature"; one with a
//! single variant (`!`, `**` and `console.log`), each argument as "Expected
//! subtype", as for any function. A call is only checked when typing knows
//! every argument's type as Kotlin would, so an unknown type, a class or a
//! function never causes an error.

use super::Site;
use crate::builtins::{Builtin, Constant, Function};
use crate::library::{Level, Message};
use crate::scope::SymbolKind;
use crate::settle::{Fact, Settle};
use crate::types::{BOOLEAN, FLOAT64, INT32, INT64, NULL, STRING, TypeData, TypeId, VOID};
use temper_syntax::ast::Value;
use temper_syntax::tree::{NodeId, NodeKind, Span};

/// One variant of a builtin function.
#[derive(Debug)]
pub(super) struct Sig {
    pub params: &'static [TypeId],
    pub ret: TypeId,
    /// Whether it may bubble: its return type is `Result<ret, Bubble>`.
    pub bubbles: bool,
}

const fn sig(params: &'static [TypeId], ret: TypeId) -> Sig {
    Sig {
        params,
        ret,
        bubbles: false,
    }
}

const fn bubbly(params: &'static [TypeId], ret: TypeId) -> Sig {
    Sig {
        params,
        ret,
        bubbles: true,
    }
}

const ARITH: &[Sig] = &[
    sig(&[INT32, INT32], INT32),
    sig(&[INT64, INT64], INT64),
    sig(&[FLOAT64, FLOAT64], FLOAT64),
];
const NEGATE: &[Sig] = &[
    sig(&[INT32], INT32),
    sig(&[INT64], INT64),
    sig(&[FLOAT64], FLOAT64),
];
const DIVIDE: &[Sig] = &[
    bubbly(&[INT32, INT32], INT32),
    bubbly(&[INT64, INT64], INT64),
    bubbly(&[FLOAT64, FLOAT64], FLOAT64),
];
// `&` has a third variant, for intersection types, which no value fits.
const BITS: &[Sig] = &[sig(&[INT32, INT32], INT32), sig(&[INT64, INT64], INT64)];
const BIT_NOT: &[Sig] = &[sig(&[INT32], INT32), sig(&[INT64], INT64)];
const SHIFT: &[Sig] = &[sig(&[INT32, INT32], INT32), sig(&[INT64, INT32], INT64)];
const COMPARE: &[Sig] = &[
    sig(&[INT32, INT32], INT32),
    sig(&[INT64, INT64], INT32),
    sig(&[FLOAT64, FLOAT64], INT32),
    sig(&[STRING, STRING], INT32),
    sig(&[BOOLEAN, BOOLEAN], INT32),
];
const EQUAL: &[Sig] = &[
    sig(&[INT32, INT32], BOOLEAN),
    sig(&[INT64, INT64], BOOLEAN),
    sig(&[FLOAT64, FLOAT64], BOOLEAN),
    sig(&[STRING, STRING], BOOLEAN),
    sig(&[BOOLEAN, BOOLEAN], BOOLEAN),
];
const NOT: &[Sig] = &[sig(&[BOOLEAN], BOOLEAN)];
const POW: &[Sig] = &[sig(&[FLOAT64, FLOAT64], FLOAT64)];
/// `Console.log`'s, without `this`.
const LOG: Sig = sig(&[STRING], VOID);

/// A builtin function's variants for a number of arguments, if typed.
pub(super) fn variants(f: Function, arity: usize) -> &'static [Sig] {
    use Function::*;
    match (f, arity) {
        (Add | Sub, 1) => NEGATE,
        (Add | Sub | Mul, 2) => ARITH,
        (Div | Mod, 2) => DIVIDE,
        (BitAnd | BitOr | BitXor, 2) => BITS,
        (BitNot, 1) => BIT_NOT,
        (Shl | Shr | UShr, 2) => SHIFT,
        (Lt | Le | Gt | Ge | Cmp, 2) => COMPARE,
        (Eq | Ne, 2) => EQUAL,
        (Not, 1) => NOT,
        (Pow, 2) => POW,
        _ => &[],
    }
}

impl Settle {
    /// The variant of a builtin operator that a call picks, as Kotlin
    /// Temper's solver does, with its arguments' types, if typing knows
    /// them as Kotlin would: each a final builtin type, maybe nullable, or
    /// null. Going through the arguments in order, the solver drops, last
    /// first, each variant that the argument can't fit, until one is left.
    pub(super) fn builtin_variant(
        &mut self,
        f: Function,
        args: &[NodeId],
        forced: bool,
    ) -> Result<Option<(&'static Sig, Vec<TypeId>)>, Fact> {
        let variants = variants(f, args.len());
        if variants.len() < 2 {
            return Ok(None);
        }
        let mut types = Vec::with_capacity(args.len());
        for &arg in args {
            types.push(self.expr_type(arg, forced)?);
        }
        let is_eq = matches!(f, Function::Eq | Function::Ne);
        for (&arg, &t) in args.iter().zip(&types) {
            let base = match *self.typing.types.get(t) {
                TypeData::Nullable(inner) => inner,
                _ => t,
            };
            let known = t == NULL || self.typing.types.is_final(base);
            // Kotlin expands `==` with a nullable or `Void` side otherwise,
            // and folds a nullable constant to its value's type first; a
            // compiler temporary, as `x++` makes, gets other errors there.
            if !known
                || (is_eq && (t != base || t == NULL || base == VOID))
                || (t != base && self.may_fold(arg, 0))
                || self.is_synthetic(arg)
            {
                return Ok(None);
            }
        }
        let mut left: Vec<&Sig> = variants.iter().collect();
        'args: for (i, &t) in types.iter().enumerate() {
            for j in (0..left.len()).rev() {
                if left[j].params[i] != t {
                    left.remove(j);
                    if left.len() == 1 {
                        break 'args;
                    }
                }
            }
        }
        Ok((left.len() == 1).then(|| (left[0], types)))
    }

    /// Whether a node reads a compiler temporary.
    fn is_synthetic(&self, node: NodeId) -> bool {
        self.lib.tree.kind(node) == NodeKind::RightName
            && self
                .scopes
                .resolved(node)
                .is_some_and(|b| self.scopes.symbol(b).kind == SymbolKind::Synthetic)
    }

    /// Checks a call of a builtin function against its signatures.
    pub(super) fn check_builtin_call(&mut self, f: Function, args: &[NodeId]) {
        let variants = variants(f, args.len());
        if let [only] = variants {
            for (&arg, &param) in args.iter().zip(only.params) {
                self.check_value(param, arg, Site::Arg);
            }
            return;
        }
        let Ok(Some((sig, types))) = self.builtin_variant(f, args, false) else {
            return;
        };
        let bad: Vec<usize> = (0..args.len())
            .filter(|&i| types[i] != sig.params[i])
            .collect();
        let (Some(&first), Some(&last)) = (bad.first(), bad.last()) else {
            return;
        };
        let span = Span {
            start: self.lib.tree.span(args[first]).start,
            end: self.lib.tree.span(args[last]).end,
        };
        // Kotlin's solver gives `null` the type wanted, made nullable.
        let got = (types.iter().zip(sig.params))
            .map(|(&t, &param)| match t {
                NULL => format!("{}?", self.show(param)),
                _ => self.show(t),
            })
            .collect::<Vec<_>>();
        let expected = self.show_list(sig.params);
        let message = Message::SignatureMismatch(
            self.show_sig(sig),
            format!("[{expected}]"),
            format!("[{}]", got.join(", ")),
        );
        self.lib.report(Level::Error, message, span);
    }

    /// Checks `console.log(message)`, if the callee is that.
    pub(super) fn check_console_call(&mut self, call: NodeId, callee: NodeId, args: &[NodeId]) {
        let tree = &self.lib.tree;
        if tree.kind(callee) != NodeKind::Call {
            return;
        }
        let [dot, object, key] = *tree.children(callee) else {
            return;
        };
        let is_log = self.scopes.resolved_builtin(dot) == Some(Builtin::Function(Function::Dot))
            && self.scopes.resolved_builtin(object) == Some(Builtin::Value(Constant::Console))
            && self
                .lib
                .symbol(key)
                .is_some_and(|s| self.lib.names.get(s) == "log");
        if !is_log
            || args.iter().any(|&a| {
                tree.kind(a) == NodeKind::Fn || matches!(self.lib.value(a), Some(Value::Symbol(_)))
            })
        {
            return;
        }
        // Kotlin counts `this`.
        if args.len() != LOG.params.len() {
            let message = Message::ArityMismatch(LOG.params.len() + 1);
            self.lib
                .report(Level::Error, message, self.lib.tree.span(call));
        }
        for (&arg, &param) in args.iter().zip(LOG.params) {
            self.check_value(param, arg, Site::Arg);
        }
    }

    fn show_list(&self, types: &[TypeId]) -> String {
        let shown: Vec<String> = types.iter().map(|&t| self.show(t)).collect();
        shown.join(", ")
    }

    /// A signature as Kotlin Temper writes it: `(Int32, Int32) -> Int32`.
    fn show_sig(&self, sig: &Sig) -> String {
        let ret = self.show(sig.ret);
        if sig.bubbles {
            format!("({}) -> Result<{ret}, Bubble>", self.show_list(sig.params))
        } else {
            format!("({}) -> {ret}", self.show_list(sig.params))
        }
    }
}
