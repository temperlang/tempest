//! Static types, the first slice of milestone 4 (notes/milestone-4.md):
//! declared and inferred types of declarations as facts in the settle
//! loop, and the type errors Kotlin Temper reports for literals, locals,
//! parameters, calls of known functions, conditions and returns.
//!
//! Typing starts once expansion has settled, as a second round of the
//! loop: each declaration whose type comes from its initializer, as `let
//! x = f(1)`, is a work item that publishes [`Fact::Type`] once it knows
//! that type, waiting on the declarations its initializer reads. Declared
//! types, parameters' and functions' signatures need no waiting: they come
//! from annotations. A cycle, as `let a = b; let b = a`, is forced, leaving
//! the type unknown. Then a pass over each file checks what it can. Macros
//! that need types will interleave with this once there are some.
//!
//! An unknown type never causes an error, so what isn't typed yet, such as
//! generic methods' calls and generic builtin types' methods, just goes
//! unchecked. Builtin operators' and `console.log`'s signatures are in
//! [`signatures`], the final builtin types' members', read from core, in
//! [`core_members`], what may bubble where it can't, in [`bubbles`],
//! classes' and their members' types, in [`members`], run-time type
//! checks' errors, in [`rtti`], and generic functions' calls, solved per
//! call, in [`generic_calls`]. Before
//! inferring, flow facts ([`crate::flow`]) find the uses of locals that
//! null checks and `is` checks narrow, so typing a use of `a: String?`
//! after `if (a != null)` gives `String`, and of `i: I` after `if (i is
//! D)`, `D`.

use crate::builtins::{Builtin, Constant, Function};
use crate::core::{self, ROLE_MASK, ROLE_TYPE_PARAM};
use crate::library::{Level, Message};
use crate::scope::{SymbolId, SymbolKind};
use crate::settle::{Fact, Settle};
use crate::types::{
    ANY_VALUE, BOOLEAN, FLOAT64, INT32, INT64, NEVER, NULL, Param, STRING, TypeData, TypeId, Types,
    UNKNOWN, VOID,
};
use std::collections::{HashMap, HashSet};
use temper_syntax::ast::Value;
use temper_syntax::tree::{NodeId, NodeKind, Span};

mod bubbles;
mod core_members;
mod generic_calls;
mod members;
mod overrides;
mod rtti;
mod signatures;

/// What typing found: declarations' and expressions' types.
#[derive(Debug, Default)]
pub struct Typing {
    pub types: Types,
    /// Declarations' types, by their `Let`s.
    pub decls: HashMap<NodeId, TypeId>,
    /// Expressions' types, by node id, where worked out, else [`UNSET`].
    exprs: Vec<TypeId>,
    /// Uses of locals that flow facts show aren't null.
    pub(crate) non_null: HashSet<NodeId>,
    /// Uses of locals that flow facts show are of a type, by the type
    /// expression of the `is` check that says so.
    pub(crate) is_type: HashMap<NodeId, NodeId>,
    /// Declarations that are assigned after they're declared.
    pub(crate) assigned: HashSet<NodeId>,
}

const UNSET: TypeId = TypeId(u32::MAX);

impl Typing {
    /// An expression's type, if typing worked it out.
    pub fn expr(&self, node: NodeId) -> Option<TypeId> {
        self.exprs
            .get(node.0 as usize)
            .copied()
            .filter(|&t| t != UNSET)
    }

    fn set_expr(&mut self, node: NodeId, t: TypeId) {
        let i = node.0 as usize;
        if i >= self.exprs.len() {
            self.exprs.resize(i + 1, UNSET);
        }
        self.exprs[i] = t;
    }
}

/// Where a value is stored, for how a mismatch is reported.
#[derive(Clone, Copy)]
enum Site {
    /// A declaration's initializer.
    Init,
    /// An assignment, `a = v`.
    Assign(NodeId),
    /// `return v`.
    Return(NodeId),
    /// A function body's result.
    Result,
    /// A call's argument.
    Arg,
}

impl Settle {
    /// Queues inferring the type of each declaration that takes its
    /// initializer's, under the files' roots.
    pub(crate) fn queue_typing(&mut self) {
        self.find_flow_facts();
        let mut stack: Vec<NodeId> = self.lib.files.iter().map(|f| f.root).collect();
        while let Some(node) = stack.pop() {
            stack.extend(self.lib.tree.children(node).iter().rev());
            if self.lib.tree.kind(node) != NodeKind::Let || !self.is_value_decl(node) {
                continue;
            }
            let (_, ty, init) = core::let_parts(&self.lib.tree, node);
            if ty.is_none() && init.is_some_and(|i| self.lib.tree.kind(i) != NodeKind::Fn) {
                self.queue_type(node);
            }
        }
    }

    /// Whether a `Let` declares a local, parameter or top-level name, not
    /// a member or type parameter.
    fn is_value_decl(&self, node: NodeId) -> bool {
        let name = self.lib.tree.children(node)[0];
        self.scopes.resolved(name).is_some_and(|b| {
            matches!(
                self.scopes.symbol(b).kind,
                SymbolKind::TopLevel
                    | SymbolKind::Local
                    | SymbolKind::Parameter
                    | SymbolKind::Synthetic
            )
        })
    }

    /// The step that infers a declaration's type from its initializer.
    pub(crate) fn infer_decl(&mut self, decl: NodeId, forced: bool) -> Result<(), Fact> {
        let init = core::let_parts(&self.lib.tree, decl).2;
        let mut t = match init {
            Some(init) => self.expr_type(init, forced)?,
            None => UNKNOWN,
        };
        // Kotlin Temper would type these from later uses.
        if t == NULL || t == NEVER {
            t = UNKNOWN;
        }
        self.typing.decls.insert(decl, t);
        self.publish(Fact::Type(decl));
        Ok(())
    }

    /// A declaration's type: declared, a function's signature, or inferred
    /// from its initializer, waiting for that if it's not known yet, unless
    /// `forced`.
    fn decl_type(&mut self, decl: NodeId, forced: bool) -> Result<TypeId, Fact> {
        if let Some(&t) = self.typing.decls.get(&decl) {
            return Ok(t);
        }
        if self.lib.tree.kind(decl) != NodeKind::Let {
            return Ok(UNKNOWN);
        }
        let (_, ty, init) = core::let_parts(&self.lib.tree, decl);
        let t = match (ty, init) {
            (Some(ty), _) => self.eval_type(ty),
            (None, Some(f)) if self.lib.tree.kind(f) == NodeKind::Fn => {
                self.expr_type(f, forced)?
            }
            (None, Some(_)) if forced => return Ok(UNKNOWN),
            (None, Some(_)) => return Err(Fact::Type(decl)),
            (None, None) => UNKNOWN,
        };
        self.typing.decls.insert(decl, t);
        Ok(t)
    }

    /// An expression's type, waiting for the declarations it reads. Types
    /// worked out in a forced step aren't kept, as they may lack what it
    /// didn't wait for.
    fn expr_type(&mut self, node: NodeId, forced: bool) -> Result<TypeId, Fact> {
        if let Some(t) = self.typing.expr(node) {
            return Ok(t);
        }
        let tree = &self.lib.tree;
        let children = tree.children(node);
        let (first, last) = (children.first().copied(), children.last().copied());
        let before_last = children.len().checked_sub(2).map(|i| children[i]);
        let t = match tree.kind(node) {
            NodeKind::Value => match self.lib.value(node) {
                Some(Value::Int32(_)) => INT32,
                Some(Value::Int64(_)) => INT64,
                Some(Value::Float64(_)) => FLOAT64,
                Some(Value::String(_)) => STRING,
                Some(Value::Bool(_)) => BOOLEAN,
                Some(Value::Null) => NULL,
                Some(Value::Void) => VOID,
                _ => UNKNOWN,
            },
            NodeKind::RightName => self.name_type(node, forced)?,
            NodeKind::Block => match last {
                Some(last) => self.expr_type(last, forced)?,
                None => VOID,
            },
            NodeKind::If | NodeKind::Orelse => match (before_last, last) {
                (Some(a), Some(b)) => {
                    let a = self.expr_type(a, forced)?;
                    let b = self.expr_type(b, forced)?;
                    self.typing.types.join(a, b)
                }
                _ => UNKNOWN,
            },
            NodeKind::Let | NodeKind::Loop => VOID,
            NodeKind::Return | NodeKind::Break | NodeKind::Continue | NodeKind::Fail => NEVER,
            // An assignment's value is what it assigns; one to a member is
            // a setter's call.
            NodeKind::Assign => match (first.and_then(|t| self.local_target(t)), last) {
                (Some(_), Some(value)) => self.expr_type(value, forced)?,
                _ => UNKNOWN,
            },
            NodeKind::Fn => self.fn_type(node),
            NodeKind::Call => self.call_type(node, forced)?,
            _ => UNKNOWN,
        };
        if !forced {
            self.typing.set_expr(node, t);
        }
        Ok(t)
    }

    /// The declaration of a local, parameter or top-level name that an
    /// assignment targets, if that's what it targets.
    fn local_target(&self, target: NodeId) -> Option<NodeId> {
        if self.lib.tree.kind(target) != NodeKind::RightName {
            return None;
        }
        let symbol = self.scopes.symbol(self.scopes.resolved(target)?);
        match symbol.kind {
            SymbolKind::TopLevel | SymbolKind::Local | SymbolKind::Parameter => symbol.decl,
            _ => None,
        }
    }

    fn name_type(&mut self, node: NodeId, forced: bool) -> Result<TypeId, Fact> {
        let Some(b) = self.scopes.resolved(node) else {
            return Ok(UNKNOWN);
        };
        let symbol = *self.scopes.symbol(b);
        Ok(match symbol.kind {
            SymbolKind::Builtin(Builtin::Value(c)) => match c {
                Constant::True | Constant::False => BOOLEAN,
                Constant::Null => NULL,
                Constant::Void => VOID,
                Constant::NaN | Constant::Infinity => FLOAT64,
                Constant::TestServerPort(_) => INT32,
                Constant::Console => UNKNOWN,
            },
            SymbolKind::TopLevel
            | SymbolKind::Local
            | SymbolKind::Parameter
            | SymbolKind::Synthetic => match symbol.decl {
                Some(decl) => {
                    let t = self.decl_type(decl, forced)?;
                    self.narrowed(node, t)
                }
                None => self.this_type(b),
            },
            SymbolKind::Member => self.bare_member_type(b),
            _ => UNKNOWN,
        })
    }

    /// The type of a use of a declaration of type `t`, as flow facts
    /// narrow it: not null after a null check, and after `a is T`, a `T`,
    /// as in Kotlin Temper, unless `t` is a subtype of `T` already, which
    /// is more precise, or `T` isn't a subtype of `t`, an error.
    fn narrowed(&mut self, node: NodeId, t: TypeId) -> TypeId {
        if !self.typing.non_null.contains(&node) {
            return t;
        }
        let inner = match self.typing.types.get(t) {
            TypeData::Nullable(inner) => *inner,
            _ => t,
        };
        let Some(&ty) = self.typing.is_type.get(&node) else {
            return inner;
        };
        let target = self.eval_type(ty);
        let types = &self.typing.types;
        // A type parameter's value may be of any subtype of its bound.
        let upper = types.upper(inner);
        match (
            types.is_subtype(inner, target),
            types.is_subtype(target, upper),
        ) {
            (Some(true), _) => inner,
            // A check between unrelated types is an error of its own
            // (`rtti`), so uses keep their declared type.
            (_, Some(false)) => inner,
            _ => target,
        }
    }

    /// A call's type: a builtin's, or what a known function or method
    /// returns.
    fn call_type(&mut self, node: NodeId, forced: bool) -> Result<TypeId, Fact> {
        let children = self.lib.tree.children(node).to_vec();
        let Some((&callee, args)) = children.split_first() else {
            return Ok(UNKNOWN);
        };
        if let Some(solved) = self.solve_call(node, forced)? {
            return Ok(self.solved_result(&solved));
        }
        match self.scopes.resolved_builtin(callee) {
            Some(Builtin::Function(f)) => return self.builtin_call_type(f, args, forced),
            Some(_) => return Ok(UNKNOWN),
            None => {}
        }
        let callee_type = self.callee_type(callee, forced)?;
        Ok(match self.typing.types.get(callee_type) {
            TypeData::Fn { ret, .. } => *ret,
            _ => UNKNOWN,
        })
    }

    fn builtin_call_type(
        &mut self,
        f: Function,
        args: &[NodeId],
        forced: bool,
    ) -> Result<TypeId, Fact> {
        use Function::*;
        Ok(match f {
            Lt | Le | Gt | Ge | Eq | Ne | Not | Is => BOOLEAN,
            Cmp => INT32,
            Cat => STRING,
            Panic => NEVER,
            Pow => FLOAT64,
            As if args.len() == 2 => self.eval_type(args[1]),
            New => match self.new_class(args) {
                Some(class) => self.class_type(class),
                None => UNKNOWN,
            },
            Dot if args.len() == 2 => self.dot_type(args[0], args[1], forced)?,
            _ => match self.builtin_variant(f, args, forced)? {
                Some((sig, _)) => sig.ret,
                None => UNKNOWN,
            },
        })
    }

    /// A function's type, from its signature, with its own type
    /// parameters unknown, as references and unsolved calls see it.
    fn fn_type(&mut self, node: NodeId) -> TypeId {
        let t = self.fn_signature(node);
        let formals = self.own_formals(node);
        self.typing.types.erase(t, &formals)
    }

    /// A function's signature, with its own type parameters as they're
    /// written. A generator's call makes a generator, which isn't typed
    /// yet.
    fn fn_signature(&mut self, node: NodeId) -> TypeId {
        let tree = &self.lib.tree;
        let (params, out_type, _) = core::fn_parts(tree, node);
        let is_generator = !core::fn_supers(tree, node).is_empty();
        let mut typed = Vec::with_capacity(params.len());
        for param in params {
            if self.lib.tree.kind(param) != NodeKind::Let {
                typed.push(Param {
                    ty: UNKNOWN,
                    optional: false,
                });
                continue;
            }
            let (_, ty, default) = core::let_parts(&self.lib.tree, param);
            typed.push(Param {
                ty: ty.map_or(UNKNOWN, |t| self.eval_type(t)),
                optional: default.is_some(),
            });
        }
        let (ret, throws) = match out_type {
            Some(t) if !is_generator => (self.eval_type(t), self.is_throws(t)),
            _ => (UNKNOWN, false),
        };
        self.typing.types.intern(TypeData::Fn {
            params: typed,
            ret,
            throws,
        })
    }

    /// The type parameters a function declares, as `T` in `let f<T>(x:
    /// T)`. References to it, and calls that aren't solved
    /// ([`generic_calls`]), see them as unknown (notes/typing.md,
    /// "Generic calls").
    fn own_formals(&self, f: NodeId) -> Vec<SymbolId> {
        let tree = &self.lib.tree;
        (core::type_params(tree, f).iter())
            .filter_map(|&p| self.scopes.resolved(core::let_parts(tree, p).0))
            .collect()
    }

    /// The type a type expression names, as `Int`, `String?` or `fn
    /// (Int): Boolean`. `T throws Bubble` is `T` for now.
    fn eval_type(&mut self, node: NodeId) -> TypeId {
        let tree = &self.lib.tree;
        match tree.kind(node) {
            NodeKind::RightName => {
                let Some(b) = self.scopes.resolved(node) else {
                    return UNKNOWN;
                };
                let symbol = self.scopes.symbol(b);
                match symbol.kind {
                    SymbolKind::Builtin(Builtin::Type(i)) => self.typing.types.builtin(i),
                    SymbolKind::TypeParam => self.type_param_type(b),
                    _ => match self.type_decl(b) {
                        Some(class) => self.class_type(class),
                        None => UNKNOWN,
                    },
                }
            }
            NodeKind::Call => {
                let children = tree.children(node).to_vec();
                let Some((&callee, args)) = children.split_first() else {
                    return UNKNOWN;
                };
                match (self.scopes.resolved_builtin(callee), args) {
                    (Some(Builtin::Function(Function::Nullable)), [inner]) => {
                        let inner = self.eval_type(*inner);
                        self.typing.types.nullable(inner)
                    }
                    (Some(Builtin::Function(Function::Angle)), [base, args @ ..]) => {
                        self.applied_type(*base, args)
                    }
                    (Some(Builtin::Function(Function::Throws)), [pass, ..]) => {
                        self.eval_type(*pass)
                    }
                    (Some(Builtin::Function(Function::FnType)), [params @ .., ret]) => {
                        let params = params
                            .iter()
                            .map(|&p| Param {
                                ty: self.eval_type(p),
                                optional: false,
                            })
                            .collect();
                        let throws = self.is_throws(*ret);
                        let ret = self.eval_type(*ret);
                        self.typing.types.intern(TypeData::Fn {
                            params,
                            ret,
                            throws,
                        })
                    }
                    _ => UNKNOWN,
                }
            }
            _ => UNKNOWN,
        }
    }

    /// A type parameter's type, as `T` names it, with its bound: `AnyValue`
    /// or the type it `extends`. Not a class's synthetic parameter.
    fn type_param_type(&mut self, symbol: SymbolId) -> TypeId {
        let tree = &self.lib.tree;
        let Some(decl) = (self.scopes.symbol(symbol).decl).filter(|&d| {
            tree.kind(d) == NodeKind::Let && tree.data(d) & ROLE_MASK == ROLE_TYPE_PARAM
        }) else {
            return UNKNOWN;
        };
        if self.typing.types.knows_param(symbol) {
            return self.typing.types.param(symbol);
        }
        // Known before its bound, which may name it, as in `<T extends
        // MapKey<T>>`.
        let t = self.typing.types.param(symbol);
        let bound = match core::let_parts(tree, decl).1 {
            Some(bound) => self.eval_type(bound),
            None => ANY_VALUE,
        };
        self.typing.types.set_bound(symbol, bound);
        t
    }

    /// A generic type given arguments, as `List<Int>` or `Box<String>`.
    /// Not `Never<T>` yet, nor a class given the wrong number of them.
    fn applied_type(&mut self, base: NodeId, args: &[NodeId]) -> TypeId {
        let Some(b) = self.scopes.resolved(base) else {
            return UNKNOWN;
        };
        let base = match self.scopes.symbol(b).kind {
            SymbolKind::Builtin(Builtin::Type(i)) => {
                let t = self.typing.types.builtin(i);
                if t == NEVER || t == ANY_VALUE || self.typing.types.is_final(t) {
                    return UNKNOWN;
                }
                t
            }
            _ => {
                let tree = &self.lib.tree;
                let Some(class) = self.type_decl(b) else {
                    return UNKNOWN;
                };
                let symbol = self.scopes.resolved(core::class_parts(tree, class).0);
                match symbol {
                    Some(symbol) if core::type_params(tree, class).len() == args.len() => {
                        self.typing.types.intern(TypeData::Class(symbol))
                    }
                    _ => return UNKNOWN,
                }
            }
        };
        let args = args.iter().map(|&a| self.eval_type(a)).collect();
        self.typing.types.intern(TypeData::Applied { base, args })
    }

    // Checking.

    /// Reports type errors in each file, once declarations' types are
    /// known.
    pub(crate) fn check_types(&mut self) {
        let roots: Vec<NodeId> = self.lib.files.iter().map(|f| f.root).collect();
        for root in roots {
            self.check(root);
        }
    }

    /// Checks a node after its children, as Kotlin Temper's `TypeChecker`
    /// does, so deeper errors come first.
    fn check(&mut self, node: NodeId) {
        let count = self.lib.tree.children(node).len();
        for i in 0..count {
            let child = self.lib.tree.children(node)[i];
            self.check(child);
        }
        let first = self.lib.tree.children(node).first().copied();
        let last = self.lib.tree.children(node).last().copied();
        match (self.lib.tree.kind(node), first, last) {
            (NodeKind::Let, ..) => self.check_let(node),
            (NodeKind::Assign, Some(target), Some(value)) if count == 2 => {
                if let Some(decl) = self.local_target(target) {
                    let t = self.known_decl_type(decl);
                    self.check_value(t, value, Site::Assign(node));
                    self.check_not_void(value, node);
                } else {
                    // A setter's call, as Kotlin Temper reports it.
                    let t = self.member_target_type(target);
                    self.check_value(t, value, Site::Arg);
                    self.check_not_void(value, value);
                }
            }
            (NodeKind::Return, Some(value), _) => {
                let f = NodeId(self.lib.tree.data(node));
                if let Some(t) = self.declared_result(f) {
                    self.check_value(t, value, Site::Return(node));
                }
            }
            (NodeKind::Fn, ..) => {
                let body = core::fn_parts(&self.lib.tree, node).2;
                if let (Some(t), Some(body)) = (self.declared_result(node), body) {
                    let mut results = Vec::new();
                    self.results(body, &mut results);
                    for result in results {
                        // Kotlin Temper reports a Void result where one is
                        // wanted at odd positions; not yet here.
                        if self.known_type(result) != VOID {
                            self.check_value(t, result, Site::Result);
                        }
                    }
                }
                if self.may_not_bubble(node) {
                    self.check_bubbles(node);
                }
            }
            (NodeKind::Call, ..) => {
                self.check_call(node);
                let args = self
                    .lib
                    .tree
                    .children(node)
                    .get(1..)
                    .unwrap_or(&[])
                    .to_vec();
                for arg in args {
                    self.check_not_void(arg, arg);
                }
            }
            (NodeKind::Class, ..) => self.check_overrides(node),
            (NodeKind::If, Some(condition), _) => {
                let t = self.known_type(condition);
                if self.is_mismatch(t, BOOLEAN) && t != NULL {
                    let (expected, got) = (self.show(BOOLEAN), self.show(t));
                    let span = self.lib.tree.span(condition);
                    let message = Message::ExpectedValueOfType(expected, got);
                    self.lib.report(Level::Error, message, span);
                }
            }
            _ => {}
        }
    }

    fn check_let(&mut self, node: NodeId) {
        let (name, ty, init) = core::let_parts(&self.lib.tree, node);
        let Some(b) = self.scopes.resolved(name) else {
            return;
        };
        let kind = self.scopes.symbol(b).kind;
        let role = self.lib.tree.data(node) & core::ROLE_MASK;
        let wants_type = match kind {
            SymbolKind::TopLevel | SymbolKind::Local => ty.is_none(),
            SymbolKind::Member => role == core::ROLE_METHOD || role == core::ROLE_GETTER,
            _ => false,
        };
        if let Some(init) = init {
            let is_fn = self.lib.tree.kind(init) == NodeKind::Fn;
            let body = is_fn
                .then(|| core::fn_parts(&self.lib.tree, init).2)
                .flatten();
            let body = body.filter(|_| wants_type && self.lacks_return_type(init));
            if let Some(body) = body {
                let start = self.lib.tree.span(body).start;
                let span = Span { start, end: start };
                self.lib
                    .report(Level::Error, Message::MissingReturnType, span);
            }
            if ty.is_some() && matches!(kind, SymbolKind::TopLevel | SymbolKind::Local) {
                let t = self.known_decl_type(node);
                self.check_value(t, init, Site::Init);
            }
            // A field's initializer, which Kotlin Temper assigns as a
            // setter would.
            let is_field = role == 0 && self.lib.tree.data(node) & core::MEMBER_STATIC == 0;
            if let (Some(ty), SymbolKind::Member, true) = (ty, kind, is_field) {
                let t = self.eval_type(ty);
                self.check_value(t, init, Site::Arg);
            }
            if matches!(kind, SymbolKind::TopLevel | SymbolKind::Local) {
                self.check_not_void(init, init);
            }
            // These return `Void` without saying so.
            let is_void = role == core::ROLE_CONSTRUCTOR || role == core::ROLE_SETTER;
            if is_void && is_fn && core::fn_parts(&self.lib.tree, init).1.is_none() {
                self.check_bubbles(init);
            }
        }
    }

    /// A function with no return type that something else could give it:
    /// not a generator, nor a `for … of` body.
    fn lacks_return_type(&self, f: NodeId) -> bool {
        let data = self.lib.tree.data(f);
        data & (core::FN_OUT_TYPE | core::FN_LOOP_BODY) == 0
            && core::fn_supers(&self.lib.tree, f).is_empty()
    }

    /// A function's declared return type, unless it's a generator's.
    fn declared_result(&mut self, f: NodeId) -> Option<TypeId> {
        if self.lib.tree.kind(f) != NodeKind::Fn || !core::fn_supers(&self.lib.tree, f).is_empty() {
            return None;
        }
        let out_type = core::fn_parts(&self.lib.tree, f).1?;
        Some(self.eval_type(out_type))
    }

    /// The expressions whose values a body gives as its result.
    fn results(&self, node: NodeId, out: &mut Vec<NodeId>) {
        let children = self.lib.tree.children(node);
        match self.lib.tree.kind(node) {
            NodeKind::Block => {
                if let Some(&last) = children.last() {
                    self.results(last, out);
                }
            }
            NodeKind::If | NodeKind::Orelse if children.len() >= 2 => {
                for &branch in &children[children.len() - 2..] {
                    self.results(branch, out);
                }
            }
            _ => out.push(node),
        }
    }

    fn check_call(&mut self, node: NodeId) {
        let children = self.lib.tree.children(node).to_vec();
        let Some((&callee, args)) = children.split_first() else {
            return;
        };
        let params = match self.scopes.resolved_builtin(callee) {
            Some(Builtin::Function(Function::New)) => match self.new_class(args) {
                Some(class) => self.constructor_type(class),
                None => {
                    self.check_generic_call(node);
                    return;
                }
            },
            Some(Builtin::Function(f @ (Function::Is | Function::As))) => {
                return self.check_rtti(node, f, args);
            }
            Some(Builtin::Function(f)) => return self.check_builtin_call(f, args),
            Some(_) => return,
            None if self.check_tie(node, callee, args) => return,
            None if self.check_generic_call(node) => return,
            None => {
                self.check_console_call(node, callee, args);
                let callee_type = self.known_callee_type(callee);
                match self.typing.types.get(callee_type) {
                    TypeData::Fn { params, .. } => params.clone(),
                    _ => return,
                }
            }
        };
        // A constructor's class comes first.
        let args = match self.scopes.resolved_builtin(callee) {
            Some(_) => &args[1..],
            None => args,
        };
        // Not yet where a trailing block goes, nor type arguments.
        let tree = &self.lib.tree;
        if args.iter().any(|&a| {
            tree.kind(a) == NodeKind::Fn || matches!(self.lib.value(a), Some(Value::Symbol(_)))
        }) {
            return;
        }
        let required = params.iter().filter(|p| !p.optional).count();
        if args.len() < required || args.len() > params.len() {
            let span = self.lib.tree.span(node);
            let message = Message::ArityMismatch(params.len());
            self.lib.report(Level::Error, message, span);
        }
        for (&arg, param) in args.iter().zip(&params) {
            // A default applies for null.
            if !(param.optional && self.known_type(arg) == NULL) {
                self.check_value(param.ty, arg, Site::Arg);
            }
        }
    }

    /// Reports a `Void` value stored in a declaration or passed as an
    /// argument, as `let x = b()` for `b(): Void` or `c(void)`, at `site`,
    /// unless an error is reported there already, as Kotlin Temper
    /// reports one error per position.
    fn check_not_void(&mut self, value: NodeId, site: NodeId) {
        if self.known_type(value) != VOID {
            return;
        }
        let span = self.lib.tree.span(site);
        let reported =
            (self.lib.diagnostics.iter()).any(|d| d.level == Level::Error && d.span == span);
        if !reported {
            self.lib.report(Level::Error, Message::VoidAsValue, span);
        }
    }

    /// Reports storing `value` where a `t` is wanted, if it can't be one.
    fn check_value(&mut self, t: TypeId, value: NodeId, site: Site) {
        let s = self.known_type(value);
        let value_span = self.lib.tree.span(value);
        if s == NULL {
            if self.is_mismatch(NULL, t) {
                let t_or_null = self.typing.types.nullable(t);
                let message = Message::ExpectedSubType(self.show(t), self.show(t_or_null));
                self.lib.report(Level::Error, message, value_span);
            }
            return;
        }
        if !self.is_mismatch(s, t) {
            return;
        }
        // Kotlin Temper solves a generic call from the type wanted, there,
        // and reports its arguments instead; not for an argument, where
        // it misses the error.
        if !matches!(site, Site::Arg) && self.check_generic_wanted(value, t) {
            return;
        }
        // Kotlin Temper lets a nullable value be an `AnyValue`, and folds
        // constants first, so `s` in `let s: String? = "a"` is a `String`
        // there.
        if let TypeData::Nullable(inner) = *self.typing.types.get(s) {
            if !self.is_mismatch(inner, t) && (t == ANY_VALUE || self.may_fold(value, 0)) {
                return;
            }
        }
        let (to, from) = (self.show(t), self.show(s));
        let (message, span) = match site {
            Site::Init | Site::Result => (Message::CannotAssign(to, from), value_span),
            Site::Assign(n) | Site::Return(n) => {
                (Message::CannotAssign(to, from), self.lib.tree.span(n))
            }
            Site::Arg => (Message::ExpectedSubType(to, from), value_span),
        };
        self.lib.report(Level::Error, message, span);
    }

    /// Whether Kotlin Temper might work out an expression's value while
    /// compiling, as it does for literals, `let`s of them, and calls of
    /// functions with such arguments, when it can: anything that doesn't
    /// read a parameter, an instance's member, or a `var` that's assigned,
    /// may be.
    fn may_fold(&self, node: NodeId, depth: u32) -> bool {
        let tree = &self.lib.tree;
        if depth > 8 {
            return true;
        }
        match tree.kind(node) {
            NodeKind::Value | NodeKind::Fn => true,
            NodeKind::RightName => {
                let Some(b) = self.scopes.resolved(node) else {
                    return true;
                };
                let symbol = self.scopes.symbol(b);
                match (symbol.kind, symbol.decl) {
                    (SymbolKind::Parameter, _) => false,
                    // An instance's member reads `this`.
                    (SymbolKind::Member, Some(decl)) => tree.data(decl) & core::MEMBER_STATIC != 0,
                    (_, Some(decl)) if tree.kind(decl) == NodeKind::Let => {
                        match core::let_parts(tree, decl).2 {
                            _ if symbol.mutable && self.typing.assigned.contains(&decl) => false,
                            Some(init) => self.may_fold(init, depth + 1),
                            None => true,
                        }
                    }
                    _ => true,
                }
            }
            // A builtin type's static member, as `String.fromCodePoint`.
            NodeKind::Call if self.names_builtin_static(node) => true,
            NodeKind::Call | NodeKind::Block | NodeKind::If | NodeKind::Orelse => tree
                .children(node)
                .iter()
                .all(|&child| self.may_fold(child, depth + 1)),
            NodeKind::Let => core::let_parts(tree, node)
                .2
                .is_none_or(|init| self.may_fold(init, depth + 1)),
            _ => false,
        }
    }

    /// Whether a value of type `s` can't be a `t`, both fully known.
    fn is_mismatch(&self, s: TypeId, t: TypeId) -> bool {
        let types = &self.typing.types;
        types.is_known(s) && types.is_known(t) && types.is_subtype(s, t) == Some(false)
    }

    /// A type once declarations' types are known: any still waited on
    /// was never queued, so is unknown.
    fn known_type(&mut self, node: NodeId) -> TypeId {
        self.expr_type(node, false).unwrap_or(UNKNOWN)
    }

    /// What a callee calls, as [`Settle::callee_type`] finds it.
    fn known_callee_type(&mut self, callee: NodeId) -> TypeId {
        self.callee_type(callee, false).unwrap_or(UNKNOWN)
    }

    fn known_decl_type(&mut self, decl: NodeId) -> TypeId {
        self.decl_type(decl, false).unwrap_or(UNKNOWN)
    }

    /// A type as messages show it.
    fn show(&self, t: TypeId) -> String {
        let class_name = |b| {
            let name = self.scopes.symbol(b).name;
            self.lib.names.get(name).to_string()
        };
        self.typing.types.display(t, &class_name)
    }
}
