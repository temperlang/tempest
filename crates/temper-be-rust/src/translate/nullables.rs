//! Nullable values, `orelse`, and `bubble()`.
//!
//! `T?` is `Option<T>`, as Kotlin's (`String?` parameters are
//! `Option<impl ToArcString>`, mapped to `Option<Arc<String>>`, as
//! Kotlin's are): `null` is `None`, a value where a nullable one is
//! wanted `Some(x)`, a null check `x.is_none()` (`x.is_some()` negated),
//! and a value that a null check narrowed `x.unwrap()`, cloned first if
//! its place wants one of its own, else borrowed (`x.as_ref().unwrap()`);
//! an `if` on a null check of a local that reads it only narrowed is
//! `if let Some(x) = &x`, and one picking it or a fallback
//! `x.unwrap_or(d)`, as clippy wants.
//! `==` with a side that may be null compares the `Option`s, which Rust
//! does by value, or `x.is_none()` against `null`; `Float64`'s, which
//! orders NaN as Temper does, zips them.
//!
//! `orelse` runs its body where it may bubble: a single bubbling call
//! as the `Result` it returns, else a closure returning one, and takes
//! the fallback with `unwrap_or` (a literal or a name) or
//! `unwrap_or_else`; standing alone, `if body.is_err() { fallback }`.
//! Kotlin writes labeled blocks with a result local for each. A fallback
//! that bubbles or returns isn't yet, since it would run in a closure.

use super::{R, Translator, Want, not_yet};
use crate::ops::{self, Spelling};
use crate::tree::{Bin, Block, Expr, Stmt, Unary};
use temper_tmpl::analysis::walk;
use temper_tmpl::{DeclId, Flags, Node, NodeId, TypeId};

impl Translator<'_> {
    /// A nullable value narrowed to its type.
    pub(crate) fn non_null(&mut self, id: NodeId, value: NodeId, want: Want) -> R<Expr> {
        let ty = self.p.nodes.ty(id);
        if let Node::Local { local } = self.p.nodes.get(value) {
            if let Some(&copy) = self.narrowed.get(&local) {
                // Bound by `if let`: the value, or a reference to it.
                let name = Expr::path(self.name(local));
                return Ok(match want {
                    Want::Owned | Want::IntoArc if !copy => name.method("clone", Vec::new()),
                    _ => name,
                });
            }
        }
        if self.is_copy(ty) {
            return Ok(self
                .expr(value, Want::Borrowed)?
                .method("unwrap", Vec::new()));
        }
        Ok(match want {
            Want::Owned | Want::IntoArc => {
                self.expr(value, Want::Owned)?.method("unwrap", Vec::new())
            }
            Want::Borrowed | Want::Display => self
                .expr(value, Want::Borrowed)?
                .method("as_ref", Vec::new())
                .method("unwrap", Vec::new()),
        })
    }

    /// `left == right`, either maybe null.
    pub(crate) fn eq_nullable(
        &mut self,
        op: temper_tmpl::ops::Op,
        left: NodeId,
        right: NodeId,
    ) -> R<Expr> {
        let p = self.p;
        let null = |n: NodeId| matches!(p.nodes.get(n), Node::Null {});
        if null(left) && null(right) {
            return Ok(Expr::path("true"));
        }
        if null(right) {
            return Ok(self
                .expr(left, Want::Borrowed)?
                .method("is_none", Vec::new()));
        }
        if null(left) {
            return Ok(self
                .expr(right, Want::Borrowed)?
                .method("is_none", Vec::new()));
        }
        match ops::spelling(op) {
            Some(Spelling::Bin(Bin::Eq) | Spelling::StrBin(Bin::Eq)) => {
                let side = |t: &mut Self, n: NodeId| match p.nodes.get(n) {
                    Node::AsNullable { .. } => t.expr(n, Want::Owned),
                    _ => t.expr(n, Want::Borrowed),
                };
                let a = side(self, left)?;
                let b = side(self, right)?;
                Ok(Expr::bin(Bin::Eq, a, b))
            }
            Some(Spelling::FloatCmp(Some(Bin::Eq))) => {
                // `a.zip(b).map_or(a.is_none() && b.is_none(), |(a, b)| …)`,
                // for names, which may be read twice.
                let plain = |n: NodeId| matches!(p.nodes.get(n), Node::Local { .. });
                if !plain(left) || !plain(right) {
                    return not_yet("Float64's == of nullable values that aren't locals");
                }
                let a = self.expr(left, Want::Borrowed)?;
                let b = self.expr(right, Want::Borrowed)?;
                let both = Expr::bin(
                    Bin::And,
                    a.clone().method("is_none", Vec::new()),
                    b.clone().method("is_none", Vec::new()),
                );
                let eq = ops::apply(
                    Spelling::FloatCmp(Some(Bin::Eq)),
                    vec![Expr::path("a"), Expr::path("b")],
                )
                .unwrap_or_else(|| Expr::path("false"));
                let f = Expr::Closure(vec!["(a, b)".into()], Block::of(eq));
                Ok(a.method("zip", vec![b]).method("map_or", vec![both, f]))
            }
            _ => not_yet("== of nullable values of this type"),
        }
    }

    /// Whether anything under `id`, not `id` itself, may bubble, or
    /// returns, outside a nested `orelse`'s body.
    fn bubbles_below(&self, id: NodeId) -> bool {
        let p = self.p;
        let mut found = false;
        for &kid in p.nodes.kids(id) {
            walk(p, kid, &mut |n| {
                found |= p.nodes.flags(n).contains(Flags::MAY_BUBBLE)
                    || matches!(p.nodes.get(n), Node::Return { .. } | Node::Error { .. });
            });
        }
        found
    }

    /// Whether code may bubble, return, or jump anywhere in it, so it
    /// can't run in a closure.
    fn escapes(&self, id: NodeId) -> bool {
        let p = self.p;
        let mut found = false;
        walk(p, id, &mut |n| {
            found |= p.nodes.flags(n).contains(Flags::MAY_BUBBLE)
                || matches!(
                    p.nodes.get(n),
                    Node::Return { .. }
                        | Node::Error { .. }
                        | Node::Break { .. }
                        | Node::Continue { .. }
                );
        });
        found
    }

    /// An `orelse`'s body as a `Result`.
    fn attempt(&mut self, body: NodeId) -> R<Expr> {
        let outer = std::mem::replace(&mut self.bubbles, true);
        let result = self.attempt_inner(body);
        self.bubbles = outer;
        result
    }

    fn attempt_inner(&mut self, body: NodeId) -> R<Expr> {
        let p = self.p;
        if p.nodes.flags(body).contains(Flags::MAY_BUBBLE) && !self.bubbles_below(body) {
            if let Expr::Try(inner) = self.expr(body, Want::Owned)? {
                return Ok(*inner);
            }
        }
        let mut no_jumps = true;
        walk(p, body, &mut |n| {
            no_jumps &= !matches!(
                p.nodes.get(n),
                Node::Return { .. } | Node::Break { .. } | Node::Continue { .. }
            );
        });
        if !no_jumps {
            return not_yet("a jump out of an orelse's body");
        }
        let ty = p.nodes.ty(body);
        let mut stmts = Vec::new();
        // Whether a statement before the value may bubble, with a `?`.
        let mut stmts_bubble = false;
        let value = match p.nodes.get(body) {
            Node::BlockValue { stmts: kids, value } => {
                for &s in p.nodes.list(kids) {
                    walk(p, s, &mut |n| {
                        stmts_bubble |= p.nodes.flags(n).contains(Flags::MAY_BUBBLE);
                    });
                    self.stmt(s, &mut stmts)?;
                }
                value
            }
            _ => body,
        };
        let v = if self.p.nodes.ty(value) == TypeId::VOID && ty == TypeId::VOID {
            let v = self.expr(value, Want::Owned)?;
            stmts.push(Stmt::Expr(v));
            Expr::path("()")
        } else {
            self.expr(value, Want::Owned)?
        };
        // `Ok(e?)` is `e`, after statements too (clippy's
        // `needless_question_mark`); `e`'s type gives the closure's. With
        // no other `?`, a block will do (clippy's `redundant_closure_call`).
        let tail = match v {
            Expr::Try(e) if stmts.is_empty() => return Ok(*e),
            Expr::Try(e) if !stmts_bubble => return Ok(Expr::Block(Block::new(stmts, Some(*e)))),
            Expr::Try(e) => *e,
            v => Expr::path("Ok::<_, temper_core::Error>").call(vec![v]),
        };
        let closure = Expr::Closure(Vec::new(), Block::new(stmts, Some(tail)));
        Ok(closure.call(Vec::new()))
    }

    /// `body orelse fallback` as a value.
    pub(crate) fn or_else(&mut self, ty: TypeId, body: NodeId, fallback: NodeId) -> R<Expr> {
        if ty == TypeId::VOID {
            return not_yet("an orelse of type Void as a value");
        }
        if matches!(self.p.nodes.get(body), Node::Bubble { .. }) {
            return self.expr(fallback, Want::Owned);
        }
        if self.escapes(fallback) {
            return not_yet("an orelse whose fallback bubbles or jumps");
        }
        let attempt = self.attempt(body)?;
        let f = self.expr(fallback, Want::Owned)?;
        Ok(if simple(&f) {
            attempt.method("unwrap_or", vec![f])
        } else {
            let f = Expr::Closure(vec!["_".into()], Block::of(f));
            attempt.method("unwrap_or_else", vec![f])
        })
    }

    /// `body orelse fallback` standing alone: `if body.is_err() {
    /// fallback }`.
    pub(crate) fn or_else_stmt(
        &mut self,
        body: NodeId,
        fallback: NodeId,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        if matches!(self.p.nodes.get(body), Node::Bubble { .. }) {
            let f = self.expr(fallback, Want::Owned)?;
            out.push(Stmt::Expr(f));
            return Ok(());
        }
        let attempt = self.attempt(body)?;
        let mut then = Vec::new();
        match self.p.nodes.get(fallback) {
            Node::VoidLit {} => {
                out.push(Stmt::Let {
                    name: "_".into(),
                    mutable: false,
                    ty: None,
                    value: Some(attempt),
                });
                return Ok(());
            }
            _ => {
                let f = self.expr(fallback, Want::Owned)?;
                then.push(Stmt::Expr(f));
            }
        }
        out.push(Stmt::If {
            cond: attempt.method("is_err", Vec::new()),
            then: Block::new(then, None),
            els: None,
        });
        Ok(())
    }

    /// For an `if` on a null check of a local, `if (x == null) … else …`
    /// or `if (x != null) …`, whose branch where it isn't null reads it
    /// only narrowed and doesn't assign it: the local, and whether that
    /// branch is `then`. It's written `if let Some(x) = &x`, as clippy
    /// wants, rather than `x.unwrap()` after `x.is_none()`.
    pub(crate) fn if_let(
        &self,
        cond: NodeId,
        then: NodeId,
        els: Option<NodeId>,
    ) -> Option<(DeclId, bool)> {
        let p = self.p;
        let checked = |n: NodeId| match p.nodes.get(n) {
            Node::IsNull { value } => match p.nodes.get(value) {
                Node::Local { local } => Some(local),
                _ => None,
            },
            _ => None,
        };
        let (local, branch, narrowed_then) = match p.nodes.get(cond) {
            Node::IsNull { .. } => (checked(cond)?, els?, false),
            Node::Op { op, args, .. } if ops::spelling(op) == Some(Spelling::Unary(Unary::Not)) => {
                let [arg] = p.nodes.list(args) else {
                    return None;
                };
                (checked(*arg)?, then, true)
            }
            _ => return None,
        };
        if self.narrowed.contains_key(&local) {
            return None;
        }
        let (mut reads, mut narrowed, mut assigned) = (0, 0, false);
        walk(p, branch, &mut |n| match p.nodes.get(n) {
            Node::Local { local: l } if l == local => reads += 1,
            Node::NonNull { value } => {
                if matches!(p.nodes.get(value), Node::Local { local: l } if l == local) {
                    narrowed += 1;
                }
            }
            Node::Assign { local: l, .. } if l == local => assigned = true,
            _ => {}
        });
        (reads == narrowed && narrowed > 0 && !assigned).then_some((local, narrowed_then))
    }

    /// `let Some(x) = &x`, or `= x` for a `Copy` value.
    pub(crate) fn let_some(&self, local: DeclId) -> Expr {
        let name = self.name(local);
        let by = if self.is_copy(self.p.decls.get(local).ty) {
            ""
        } else {
            "&"
        };
        Expr::path(format!("let Some({name}) = {by}{name}"))
    }

    /// `x as T` where `x` is `T?`: `x.ok_or_else(temper_core::Error::new)?`.
    pub(crate) fn as_non_null(&mut self, value: NodeId) -> R<Expr> {
        let v = self.expr(value, Want::Owned)?;
        Ok(
            v.method("ok_or_else", vec![Expr::path("temper_core::Error::new")])
                .try_(),
        )
    }

    /// `!isNull(x)` as `x.is_some()`.
    pub(crate) fn is_some(&mut self, args: &[NodeId]) -> R<Option<Expr>> {
        if let [arg] = args {
            if let Node::IsNull { value } = self.p.nodes.get(*arg) {
                let v = self.expr(value, Want::Borrowed)?;
                return Ok(Some(v.method("is_some", Vec::new())));
            }
        }
        Ok(None)
    }
}

/// `if let Some(x) = x { x } else { d }` as `x.unwrap_or(d)`, as clippy
/// wants it (`&x` and `x.clone()` as `x.clone()…`), with
/// `unwrap_or_else(|| d)` where `d` isn't [`simple`]; else `e` as it is.
pub(crate) fn unwrap_or(e: Expr) -> Expr {
    let Expr::If(cond, then, els) = &e else {
        return e;
    };
    let Expr::Path(cond) = &**cond else {
        return e;
    };
    let Some(rest) = cond.strip_prefix("let Some(") else {
        return e;
    };
    let Some((name, scrutinee)) = rest.split_once(") = ") else {
        return e;
    };
    let by_ref = scrutinee.starts_with('&');
    let (Some(t), Some(d)) = (then.tail.as_deref(), els.tail.as_deref()) else {
        return e;
    };
    if !then.stmts.is_empty() || !els.stmts.is_empty() || !closure_safe(d) {
        return e;
    }
    let value = Expr::path(name);
    let fits = if by_ref {
        *t == value.clone().method("clone", Vec::new())
    } else {
        *t == value
    };
    if !fits {
        return e;
    }
    let x = if by_ref {
        value.method("clone", Vec::new())
    } else {
        value
    };
    if simple(d) {
        x.method("unwrap_or", vec![d.clone()])
    } else {
        let f = Expr::Closure(Vec::new(), Block::of(d.clone()));
        x.method("unwrap_or_else", vec![f])
    }
}

/// Whether an expression may run in a closure: it doesn't return, try,
/// or hold statements.
fn closure_safe(e: &Expr) -> bool {
    match e {
        Expr::Path(_) | Expr::Lit(_) | Expr::Str(_) => true,
        Expr::Call(f, args) => closure_safe(f) && args.iter().all(closure_safe),
        Expr::Method(r, _, args) => closure_safe(r) && args.iter().all(closure_safe),
        Expr::Macro(_, args) => args.iter().all(closure_safe),
        Expr::Bin(_, a, b) => closure_safe(a) && closure_safe(b),
        Expr::Unary(_, x) | Expr::Cast(x, _) | Expr::Field(x, _) => closure_safe(x),
        _ => false,
    }
}

/// A fallback cheap enough to compute whether it's needed or not: a
/// literal, a name, or `None`, as clippy's `unnecessary_lazy_evaluations`
/// wants them.
fn simple(e: &Expr) -> bool {
    match e {
        Expr::Lit(_) | Expr::Str(_) | Expr::Path(_) => true,
        Expr::Unary(Unary::Neg, x) => simple(x),
        _ => false,
    }
}
