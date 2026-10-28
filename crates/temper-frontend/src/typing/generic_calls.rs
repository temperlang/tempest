//! Calls of generic functions, solved per call (notes/typing.md, "Generic
//! calls"): `id(1)` for `let id<T>(x: T): T` binds `T` to `Int32` from
//! its argument, and `id<Int>(x)` binds it as written, so the call is an
//! `Int32`, and its arguments and `T`'s bound are checked as Kotlin
//! Temper's solver reports them. Only calls naming a function declared
//! by `let` are solved, not methods; the rest see a generic function's
//! own type parameters as unknown.

use crate::core;
use crate::library::{Level, Message};
use crate::scope::{SymbolId, SymbolKind};
use crate::settle::{Fact, Settle};
use crate::types::{ANY_VALUE, NULL, TypeData, TypeId, UNKNOWN, show_result};
use temper_syntax::tree::{NodeId, NodeKind, Span};

/// A call of a generic function, solved.
pub(super) struct Solved {
    /// The function's signature, with its type parameters as written.
    sig: TypeId,
    /// Its type parameters.
    formals: Vec<SymbolId>,
    /// What each type parameter binds to at this call, unknown where the
    /// call doesn't say.
    actuals: Vec<TypeId>,
    /// The call's arguments, without type arguments.
    args: Vec<NodeId>,
}

impl Settle {
    /// Solves a call of a generic function declared by `let`, as `id(1)`
    /// or `id<Int>(x)`: from its type arguments, if there are as many as
    /// the function has type parameters (else not at all), unless Kotlin
    /// Temper would fold the call; otherwise from its arguments' types
    /// ([`crate::types::Types::unify`]), leaving a type parameter that's
    /// solved two different ways unknown, as Kotlin may find a common
    /// supertype. A function argument, which may be a trailing block,
    /// isn't used yet. `None` if the call isn't of such a function.
    pub(super) fn solve_call(
        &mut self,
        call: NodeId,
        forced: bool,
    ) -> Result<Option<Solved>, Fact> {
        let children = self.lib.tree.children(call).to_vec();
        let Some((&callee, args)) = children.split_first() else {
            return Ok(None);
        };
        let (Some(f), Some((type_args, args))) =
            (self.generic_fn(callee), self.split_type_args(args))
        else {
            return Ok(None);
        };
        let sig = self.fn_signature(f);
        let formals = self.own_formals(f);
        let TypeData::Fn { params, .. } = self.typing.types.get(sig).clone() else {
            return Ok(None);
        };
        let mut actuals = vec![UNKNOWN; formals.len()];
        // Kotlin Temper folds a call of constants to its value before
        // checking it, so its type arguments go unchecked: `let s: String
        // = id<Int>("a")` passes, as the value is a `String`.
        if !type_args.is_empty() && !self.may_fold(call, 0) {
            if type_args.len() == formals.len() {
                for (actual, &t) in actuals.iter_mut().zip(&type_args) {
                    *actual = self.eval_type(t);
                }
            }
        } else {
            let mut conflicts = vec![false; formals.len()];
            for (&arg, param) in args.iter().zip(&params) {
                if self.lib.tree.kind(arg) == NodeKind::Fn {
                    continue;
                }
                let t = self.expr_type(arg, forced)?;
                let types = &self.typing.types;
                types.unify(param.ty, t, &formals, &mut actuals, &mut conflicts);
            }
            for (actual, conflict) in actuals.iter_mut().zip(conflicts) {
                if conflict {
                    *actual = UNKNOWN;
                }
            }
        }
        Ok(Some(Solved {
            sig,
            formals,
            actuals,
            args,
        }))
    }

    /// A solved call's type: its function's result with the solution
    /// put in, unknown where it isn't solved.
    pub(super) fn solved_result(&mut self, solved: &Solved) -> TypeId {
        let TypeData::Fn { ret, .. } = *self.typing.types.get(solved.sig) else {
            return UNKNOWN;
        };
        let types = &mut self.typing.types;
        types.substitute(ret, &solved.formals, &solved.actuals)
    }

    /// Checks a call of a generic function, as Kotlin Temper's solver
    /// does, at its positions: a type parameter bound to a type outside
    /// its bound, "Type formal <T extends Animal> cannot bind to Int32
    /// which does not fit upper bounds [Animal]", at the call; the wrong
    /// number of arguments, "No callee matches inputs [Int32] among [<T
    /// extends AnyValue>(T, Int32) -> T]", at the call; and arguments
    /// that don't fit the solved signature, "Actual arguments do not match
    /// signature: <T extends AnyValue>(T, Int32) -> T expected [String,
    /// Int32], but got [Int32, String]", from the first that doesn't to
    /// the last. Returns whether the call was of a generic function.
    pub(super) fn check_generic_call(&mut self, call: NodeId) -> bool {
        let Ok(Some(solved)) = self.solve_call(call, false) else {
            return false;
        };
        let span = self.lib.tree.span(call);
        for (&formal, &actual) in solved.formals.iter().zip(&solved.actuals) {
            let types = &mut self.typing.types;
            let param = types.param(formal);
            let bound = types.upper(param);
            let fits = types.is_subtype(actual, bound) != Some(false);
            if fits || types.mentions(bound, &solved.formals) || !types.is_known(bound) {
                continue;
            }
            let message = Message::TypeFormalBound(
                self.show_formal(formal),
                self.show(actual),
                self.show(bound),
            );
            self.lib.report(Level::Error, message, span);
            return true;
        }
        // Not yet where a trailing block goes.
        if (solved.args.iter()).any(|&a| self.lib.tree.kind(a) == NodeKind::Fn) {
            return true;
        }
        let TypeData::Fn { params, .. } = self.typing.types.get(solved.sig).clone() else {
            return true;
        };
        let arg_types: Vec<TypeId> = solved.args.iter().map(|&a| self.known_type(a)).collect();
        let required = params.iter().filter(|p| !p.optional).count();
        if !(required..=params.len()).contains(&solved.args.len()) {
            if arg_types.iter().all(|&t| self.typing.types.is_known(t)) {
                let inputs: Vec<String> = arg_types.iter().map(|&t| self.show(t)).collect();
                let among = self.show_generic_sig(&solved);
                let message = Message::NoCalleeMatches(inputs.join(", "), among);
                self.lib.report(Level::Error, message, span);
            }
            return true;
        }
        let wanted: Vec<TypeId> = (params.iter().take(solved.args.len()))
            .map(|p| {
                let types = &mut self.typing.types;
                types.substitute(p.ty, &solved.formals, &solved.actuals)
            })
            .collect();
        let unfit: Vec<usize> = (0..wanted.len())
            .filter(|&i| self.misfits(arg_types[i], wanted[i], solved.args[i]))
            .collect();
        let (Some(&first), Some(&last)) = (unfit.first(), unfit.last()) else {
            return true;
        };
        let span = Span {
            start: self.lib.tree.span(solved.args[first]).start,
            end: self.lib.tree.span(solved.args[last]).end,
        };
        // Kotlin's solver gives `null` the type wanted, made nullable.
        let got: Vec<String> = (arg_types.iter().zip(&wanted))
            .map(|(&t, &w)| match t {
                NULL => format!("{}?", self.show(w)),
                _ => self.show(t),
            })
            .collect();
        let expected: Vec<String> = wanted.iter().map(|&t| self.show(t)).collect();
        let message = Message::SignatureMismatch(
            self.show_generic_sig(&solved),
            format!("[{}]", expected.join(", ")),
            format!("[{}]", got.join(", ")),
        );
        self.lib.report(Level::Error, message, span);
        true
    }

    /// Whether a value of type `s` can't be passed where a `t` is wanted,
    /// as [`Settle::check_value`] judges it.
    fn misfits(&mut self, s: TypeId, t: TypeId, value: NodeId) -> bool {
        if s == NULL {
            return self.is_mismatch(NULL, t);
        }
        if !self.is_mismatch(s, t) {
            return false;
        }
        match *self.typing.types.get(s) {
            TypeData::Nullable(inner) if !self.is_mismatch(inner, t) => {
                t != ANY_VALUE && !self.may_fold(value, 0)
            }
            _ => true,
        }
    }

    /// The generic function a callee names: one declared by `let` (not
    /// `var`), with type parameters and no declared type.
    fn generic_fn(&self, callee: NodeId) -> Option<NodeId> {
        let tree = &self.lib.tree;
        if tree.kind(callee) != NodeKind::RightName {
            return None;
        }
        let symbol = self.scopes.symbol(self.scopes.resolved(callee)?);
        if !matches!(symbol.kind, SymbolKind::TopLevel | SymbolKind::Local) || symbol.mutable {
            return None;
        }
        let decl = symbol.decl.filter(|&d| tree.kind(d) == NodeKind::Let)?;
        let (_, ty, init) = core::let_parts(tree, decl);
        let f = init.filter(|&f| ty.is_none() && tree.kind(f) == NodeKind::Fn)?;
        (!core::type_params(tree, f).is_empty()).then_some(f)
    }

    /// A call's type arguments, as `Int` in `f<Int>(x)`, and its other
    /// arguments; `None` if it has other symbols among them.
    fn split_type_args(&self, args: &[NodeId]) -> Option<(Vec<NodeId>, Vec<NodeId>)> {
        let (mut type_args, mut values) = (Vec::new(), Vec::new());
        let mut rest = args;
        while let Some((&arg, more)) = rest.split_first() {
            rest = more;
            match self.lib.symbol(arg) {
                Some(key) if self.lib.names.get(key) == "typeArg" => {
                    let (&t, more) = rest.split_first()?;
                    type_args.push(t);
                    rest = more;
                }
                Some(_) => return None,
                None => values.push(arg),
            }
        }
        Some((type_args, values))
    }

    /// A type parameter with its bound, as Kotlin Temper writes it: `T
    /// extends AnyValue`.
    fn show_formal(&mut self, formal: SymbolId) -> String {
        let types = &mut self.typing.types;
        let param = types.param(formal);
        let bound = types.upper(param);
        format!("{} extends {}", self.show(param), self.show(bound))
    }

    /// A generic function's signature, as Kotlin Temper writes it: `<T
    /// extends AnyValue>(T, Int32) -> T`.
    fn show_generic_sig(&mut self, solved: &Solved) -> String {
        let formals: Vec<String> = (solved.formals.iter())
            .map(|&f| self.show_formal(f))
            .collect();
        let TypeData::Fn {
            params,
            ret,
            throws,
        } = self.typing.types.get(solved.sig).clone()
        else {
            return String::new();
        };
        let params: Vec<String> = params.iter().map(|p| self.show(p.ty)).collect();
        let ret = show_result(self.show(ret), throws);
        format!("<{}>({}) -> {ret}", formals.join(", "), params.join(", "))
    }
}
