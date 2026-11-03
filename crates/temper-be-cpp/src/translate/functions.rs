//! Function values in C++, as Kotlin's C++ backend writes them: a
//! function type, or a `@fun` interface, is a `std::function`, a
//! parameter of one a `const &`; a literal is a lambda that captures by
//! value, `[=](int32_t x) -> int32_t { return f(f(x)); }`, or with more
//! statements, an `auto` local declared before the statement it's in; a
//! reference to a function names it; and a call of a value calls it.
//!
//! Safer than Kotlin's, which captures a local the lambda assigns by
//! reference (`[=, &count]`), so a closure that outlives its function
//! writes a dead stack slot: a local that a lambda captures and that's
//! assigned is a cell, a `std::shared_ptr` the lambda copies, read as
//! `*count` and written as `*count = …`. And `this` in a lambda is the
//! member's `self`, a `shared_ptr` the lambda copies, never the raw
//! pointer.

use super::*;
use temper_tmpl::Kids;
use temper_tmpl::analysis::walk;

impl Translator<'_> {
    /// A `std::function` from a signature: `std::function<int32_t(int32_t)>`.
    pub(crate) fn function_type(&self, ty: TypeId) -> R<String> {
        let Some(sig) = temper_tmpl::analysis::fun_signature(self.p, ty) else {
            return not_yet("a std::function for another type");
        };
        let Some(s) = self.p.types.signature(sig) else {
            return not_yet("a function type without a signature");
        };
        if s.optional.contains(&true) {
            return not_yet("a function type with optional parameters");
        }
        let mut texts = Vec::new();
        for &t in s.params {
            texts.push(self.ty(t)?);
        }
        Ok(format!(
            "std::function<{}({})>",
            self.ty(s.result)?,
            texts.join(", ")
        ))
    }

    /// Whether a local is a cell: captured by a lambda and assigned.
    pub(crate) fn is_cell(&self, decl: DeclId) -> bool {
        self.cells.contains(&decl)
    }

    /// A function literal: a one-line lambda where its body is one
    /// statement, else an `auto` local of it declared before the
    /// statement it's in.
    pub(crate) fn literal(&mut self, decl: DeclId, params: Kids, body: NodeId) -> R<Expr> {
        let p = self.p;
        let Some(sig) = p.signature(decl) else {
            return not_yet("a function literal without a signature");
        };
        let result = self.ty(sig.result)?;
        // What the body reads; a parameter it doesn't is unnamed.
        let mut used = std::collections::HashSet::new();
        walk(p, body, &mut |id| {
            if let Node::Local { local } = p.nodes.get(id) {
                used.insert(local);
            }
        });
        let mut ps = Vec::new();
        for &param in p.nodes.list(params) {
            if let Node::Param { decl } = p.nodes.get(param) {
                let name = match used.contains(&decl) {
                    true => self.name(decl),
                    false => String::new(),
                };
                ps.push((self.ty(p.decls.get(decl).ty)?, name));
            }
        }
        let captures = if p.decls.captures(decl).is_empty() && !uses_this(p, body) {
            String::new()
        } else {
            "=".to_string()
        };
        // The function around it, whose state its own replaces.
        let saved = (
            std::mem::take(&mut self.loops),
            std::mem::take(&mut self.labels),
            std::mem::take(&mut self.pending),
            self.deferred,
            self.in_function,
        );
        self.deferred = 0;
        self.in_function = true;
        self.begin(body);
        let stmts = self.stmts(body).map(reachable);
        let (loops, labels, pending, deferred, in_function) = saved;
        self.loops = loops;
        self.labels = labels;
        self.pending = pending;
        self.deferred = deferred;
        self.in_function = in_function;
        let mut stmts = stmts?;
        if sig.result == TypeId::VOID && stmts.last() == Some(&Stmt::Return(None)) {
            stmts.pop();
        }
        if stmts.len() <= 1 {
            return Ok(Expr::Lambda {
                captures,
                params: ps,
                result,
                body: stmts,
            });
        }
        // The naming pass gave it a name of its own in the function.
        let name = self.names.get(decl).to_string();
        self.pending.push(Stmt::LambdaLocal {
            name: name.clone(),
            captures,
            params: ps,
            result,
            body: stmts,
        });
        Ok(Expr::name(name))
    }

    /// A declared function or static method by its name, which converts
    /// to a `std::function`: `addOne`, `Scaler::triple`, where Kotlin's
    /// names a static that isn't there.
    pub(crate) fn function_ref(&self, target: DeclId) -> R<Expr> {
        if !self.p.decls.type_params(target).is_empty() {
            return not_yet("a generic function as a value");
        }
        Ok(Expr::name(self.top_name(target)))
    }
}

/// Whether a literal's body uses `this`.
pub(crate) fn uses_this(p: &Program, body: NodeId) -> bool {
    let mut found = false;
    walk(p, body, &mut |id| {
        found |= matches!(p.nodes.get(id), Node::This {})
    });
    found
}
