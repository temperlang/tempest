//! Parameters' defaults in [`to_tmpl`](super) (notes/tmpl/nodes.md,
//! "Parameters' defaults"). A parameter with a default is optional and
//! typed `T?`, since null means its default, as in Kotlin Temper; its
//! function's body starts with `let x = coalesce(x', default)` for it, in
//! parameter order, and the body reads that local. A call leaves out a
//! trailing optional parameter as `Omitted`, and passes a value of `T` as
//! `T?`.

use super::{Translator, Tx};
use crate::core;
use temper_syntax::tree::NodeId as FNode;
use temper_tmpl::{DeclId, DeclKind, NodeId, TypeId};

impl Translator<'_> {
    /// A parameter's default, if it has one.
    pub(super) fn param_default(&self, p: FNode) -> Option<FNode> {
        core::let_parts(&self.s.library.tree, p).2
    }

    /// The defaults of `params` (each a parameter's `Let`, its
    /// declaration, and the type the body reads it as), as the `let`s a
    /// body starts with, in order. Each default sees the parameters
    /// before it with their defaults applied, so the body's reads, and
    /// later defaults', go to its local from here on.
    pub(super) fn defaults(
        &mut self,
        params: &[(FNode, DeclId, TypeId)],
        out: &mut Vec<NodeId>,
    ) -> Tx<()> {
        for &(p, param, ty) in params {
            let Some(default) = self.param_default(p) else {
                continue;
            };
            let value = self.expr(default)?;
            let value = self.fit(value, ty);
            let read = self.b.local(param);
            let read = self.made(read, p);
            let init = self.b.coalesce(read, value);
            let init = self.made(init, p);
            let name = self.b.program().name(param).to_string();
            let local = self.b.declare(DeclKind::Local, &name, self.owner, ty);
            self.decl_spans(local, p);
            self.decls.insert(p, local);
            let stmt = self.b.let_stmt(local, Some(init));
            out.push(self.made(stmt, p));
        }
        Ok(())
    }

    /// The type a body reads a parameter with a default as: its declared
    /// type (or its default's), which its signature makes nullable.
    pub(super) fn default_ty(&mut self, p: FNode) -> Tx<TypeId> {
        self.decl_ty(p)
    }

    /// `Omitted` for each parameter after the `given` arguments, of a
    /// callee whose parameters (with type parameters put in) are `params`,
    /// if each is optional.
    pub(super) fn omit_rest(
        &mut self,
        params: &[TypeId],
        optional: &[bool],
        given: usize,
        at: FNode,
    ) -> Tx<Vec<NodeId>> {
        if given > params.len() || optional.get(given..).is_none_or(|o| o.contains(&false)) {
            return self.not_yet(at, "an argument left out");
        }
        let mut out = Vec::new();
        for &ty in &params[given..] {
            let omitted = self.b.omitted(ty);
            out.push(self.made(omitted, at));
        }
        Ok(out)
    }

    /// [`Translator::omit_rest`] for a call of `callee`, with the type
    /// parameters the call binds put in.
    pub(super) fn omit_rest_of(
        &mut self,
        callee: DeclId,
        receiver: Option<NodeId>,
        type_args: &[TypeId],
        given: usize,
        at: FNode,
    ) -> Tx<Vec<NodeId>> {
        let Some(sig) = self.b.program().signature(callee) else {
            return self.not_yet(at, "a call without a signature");
        };
        let (params, optional) = (sig.params.to_vec(), sig.optional.to_vec());
        if given == params.len() {
            return Ok(Vec::new());
        }
        let receiver = receiver.map(|r| self.b.program().nodes.ty(r));
        let (from, to) =
            temper_tmpl::analysis::bindings(self.b.program(), callee, receiver, type_args);
        let params: Vec<TypeId> = (params.into_iter())
            .map(|p| self.b.program_mut().types.substitute(p, &from, &to))
            .collect();
        self.omit_rest(&params, &optional, given, at)
    }
}
