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
//! function literals' types, just goes unchecked. Builtin operators' and
//! `console.log`'s signatures are in [`signatures`], builtin types'
//! members', read from core, in
//! [`core_members`], what may bubble where it can't, in [`bubbles`],
//! classes' and their members' types, in [`members`], named `@fun`
//! interfaces' rules, nominal, in [`fun_interfaces`], run-time type
//! checks' errors, in [`rtti`], generic functions' calls, solved per
//! call, in [`generic_calls`], and the type a value's place wants, which
//! types list literals, in [`targets`], and decides object literals'
//! classes, in [`object_literals`]. Before
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
mod connected;
mod core_members;
mod exprs;
mod fields_first;
mod fun_interfaces;
mod generic_calls;
mod initializers;
mod members;
mod object_literals;
mod overrides;
mod resolve;
mod rtti;
mod signatures;
mod targets;

pub(crate) use bubbles::fold_int;
pub use exprs::{FileCoverage, coverage};
pub use resolve::Resolved;

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
    /// Builtin types' generic methods' own type parameters, as `O` in
    /// `Listed.map<O>`, by the declaring type and the member's index.
    core_formals: HashMap<(&'static str, usize), Vec<SymbolId>>,
    /// Each node's parent, under the files' roots, else [`NO_PARENT`].
    parents: Vec<NodeId>,
    /// Calls whose trailing block typing places other than last
    /// ([`Settle::placed_params`]), by the call: the position among the
    /// values the call passes (a constructor's class first, without type
    /// arguments) that the block takes, with null passed for skipped
    /// parameters before it, which take their defaults.
    pub trailing_blocks: HashMap<NodeId, usize>,
    /// Temporaries that hold object literals' values, by their `let`s,
    /// with their uses, whose places want what the values' places do.
    literal_temporaries: HashMap<NodeId, NodeId>,
    /// The type parameter of a list literal's signature, `listT` in `<listT
    /// extends AnyValue>(listT, listT) -> List<listT>`, once one needs it.
    list_formal: Option<SymbolId>,
    /// The type aliases being read, against cycles.
    aliases: Vec<NodeId>,
    /// The locals being typed from their initializers, against cycles,
    /// as `c = c + 1` ([`initializers`]).
    initializing: Vec<NodeId>,
    /// What each call, operator and member read resolves to, where
    /// typing knows ([`resolve`]).
    resolved: HashMap<NodeId, Resolved>,
    /// `==` and `!=` uses with nothing to call but identity, with why.
    identity_eqs: HashMap<NodeId, &'static str>,
    /// What each call of a generic function, method, or class's
    /// constructor binds its type parameters to, by the call: the callee's
    /// own, or for a constructor, its class's, unknown where unsolved.
    type_args: HashMap<NodeId, Vec<TypeId>>,
}

const UNSET: TypeId = TypeId(u32::MAX);
const NO_PARENT: NodeId = NodeId(u32::MAX);

impl Typing {
    /// An expression's type, if typing worked it out.
    pub fn expr(&self, node: NodeId) -> Option<TypeId> {
        self.exprs
            .get(node.0 as usize)
            .copied()
            .filter(|&t| t != UNSET)
    }

    /// What a generic call binds its callee's type parameters to (a
    /// constructor's, its class's), if typing solved it.
    pub fn type_args(&self, call: NodeId) -> Option<&[TypeId]> {
        self.type_args.get(&call).map(Vec::as_slice)
    }

    /// What a call, operator or member read resolves to, if typing knows.
    pub fn resolved(&self, node: NodeId) -> Option<&Resolved> {
        self.resolved.get(&node)
    }

    /// Why an `==` or `!=` would compare identities, if it would, having
    /// no equality to call (notes/typing.md, "Resolutions").
    pub fn identity_eq(&self, node: NodeId) -> Option<&'static str> {
        self.identity_eqs.get(&node).copied()
    }

    fn set_expr(&mut self, node: NodeId, t: TypeId) {
        let i = node.0 as usize;
        if i >= self.exprs.len() {
            self.exprs.resize(i + 1, UNSET);
        }
        self.exprs[i] = t;
    }

    /// The node a node is a child of, if it's under a file's root.
    fn parent(&self, node: NodeId) -> Option<NodeId> {
        (self.parents.get(node.0 as usize).copied()).filter(|&p| p != NO_PARENT)
    }

    fn set_parent(&mut self, node: NodeId, parent: NodeId) {
        let i = node.0 as usize;
        if i >= self.parents.len() {
            self.parents.resize(i + 1, NO_PARENT);
        }
        self.parents[i] = parent;
    }

    /// Whether nodes' parents are noted, as once typing has begun.
    fn has_parents(&self) -> bool {
        !self.parents.is_empty()
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
    /// A field's initializer, reported as a setter's argument, but solved
    /// from the type wanted, as Kotlin Temper does.
    Field,
}

impl Settle {
    /// Queues inferring the type of each declaration that takes its
    /// initializer's, under the files' roots, and notes each node's parent,
    /// for the type its place wants ([`targets`]).
    pub(crate) fn queue_typing(&mut self) {
        self.find_flow_facts();
        self.typing.parents = vec![NO_PARENT; self.lib.tree.len()];
        let mut stack: Vec<NodeId> = self.lib.files.iter().map(|f| f.root).collect();
        while let Some(node) = stack.pop() {
            for &child in self.lib.tree.children(node) {
                self.typing.set_parent(child, node);
            }
            stack.extend(self.lib.tree.children(node).iter().rev());
            if self.lib.tree.kind(node) != NodeKind::Let
                || !(self.is_value_decl(node) || self.is_field(node))
            {
                continue;
            }
            let (_, ty, init) = core::let_parts(&self.lib.tree, node);
            if ty.is_none() && init.is_some_and(|i| self.lib.tree.kind(i) != NodeKind::Fn) {
                self.queue_type(node);
            }
        }
        self.queue_object_literals();
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

    /// Whether a `Let` declares a class's field, static or not, which
    /// may take its initializer's type, as `static srgb = new Space()`.
    fn is_field(&self, node: NodeId) -> bool {
        let name = self.lib.tree.children(node)[0];
        self.lib.tree.data(node) & ROLE_MASK == 0
            && (self.scopes.resolved(name))
                .is_some_and(|b| self.scopes.symbol(b).kind == SymbolKind::Member)
    }

    /// Waits for a member's type, if it's a field's that its initializer
    /// gives ([`Settle::is_field`]), unless `forced`.
    fn wait_for_field(&self, m: NodeId, forced: bool) -> Result<(), Fact> {
        let tree = &self.lib.tree;
        let waits = !forced
            && tree.kind(m) == NodeKind::Let
            && tree.data(m) & ROLE_MASK == 0
            && !self.typing.decls.contains_key(&m)
            && matches!(core::let_parts(tree, m), (_, None, Some(init)) if tree.kind(init) != NodeKind::Fn)
            && self.is_field(m);
        if waits {
            return Err(Fact::Type(m));
        }
        Ok(())
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
            // A function literal's parameter, as `x` in `{ x => x + 1 }`,
            // typed from the literal's place, which a forced step may not
            // know all of; or a local declared without a value, typed
            // from what's assigned to it first ([`initializers`]).
            (None, None) => {
                let t = match self.untyped_param_type(decl, forced)? {
                    Some(t) => t,
                    None => self.initializers_type(decl, forced)?.unwrap_or(UNKNOWN),
                };
                if forced {
                    return Ok(t);
                }
                t
            }
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
            // One that jumps away before its end never completes, as `{
            // return -1; }`, whose last statement is a `void`.
            NodeKind::Block if children.iter().any(|&c| self.jumps_from(c, node)) => NEVER,
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
            NodeKind::Fn => self.literal_type(node, forced)?,
            NodeKind::Call => self.call_type(node, forced)?,
            // `await p`'s value is the `T` of `p`'s `Promise<T>`.
            NodeKind::Await => match first {
                Some(promise) => {
                    let t = self.expr_type(promise, forced)?;
                    match self.typing.types.get(t) {
                        TypeData::Applied { args, .. } if args.len() == 1 => args[0],
                        _ => UNKNOWN,
                    }
                }
                None => UNKNOWN,
            },
            _ => UNKNOWN,
        };
        let t = match self.lib.tree.kind(node) {
            NodeKind::Fn | NodeKind::RightName | NodeKind::Call => {
                self.target_fun(node, t, forced)?
            }
            _ => t,
        };
        if !forced {
            self.typing.set_expr(node, t);
        }
        Ok(t)
    }

    /// Whether a statement jumps away from the block it's in: `return`,
    /// `continue`, a failure, or `break`, unless to that block's end.
    fn jumps_from(&self, statement: NodeId, block: NodeId) -> bool {
        let tree = &self.lib.tree;
        match tree.kind(statement) {
            NodeKind::Return | NodeKind::Continue | NodeKind::Fail => true,
            NodeKind::Break => tree.data(statement) != block.0,
            _ => false,
        }
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
            SymbolKind::Member => {
                if let Some(decl) = symbol.decl {
                    self.wait_for_field(decl, forced)?;
                }
                self.bare_member_type(b)
            }
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
        let types = &mut self.typing.types;
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
        if self.literal_undecided(node) {
            return if forced {
                Ok(UNKNOWN)
            } else {
                Err(Fact::ObjectLiteral(node))
            };
        }
        if self.is_list_literal(node) {
            return self.list_type(node, forced);
        }
        if self.list_of_parts(node).is_some() {
            return self.list_of_type(node, forced);
        }
        if let Some(mut solved) = self.solve_call(node, forced)? {
            let t = self.solved_type(&mut solved, forced)?;
            let actuals = solved.actuals().to_vec();
            self.typing.type_args.insert(node, actuals);
            return Ok(t);
        }
        match self.scopes.resolved_builtin(callee) {
            Some(Builtin::Function(f)) => return self.builtin_call_type(f, args, forced),
            Some(_) => return Ok(UNKNOWN),
            None => {}
        }
        if self.is_console_log(callee) {
            return Ok(VOID);
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
            Ignore => VOID,
            // `doPure { … }` gives its block's result.
            DoPure => match args {
                [f] if self.lib.tree.kind(*f) == NodeKind::Fn => {
                    let t = self.expr_type(*f, forced)?;
                    match *self.typing.types.get(t) {
                        TypeData::Fn { ret, .. } => ret,
                        _ => UNKNOWN,
                    }
                }
                _ => UNKNOWN,
            },
            Empty => match crate::builtins::TYPES.iter().position(|&t| t == "Empty") {
                Some(i) => self.typing.types.builtin(i as u8),
                None => UNKNOWN,
            },
            Pow => FLOAT64,
            As if args.len() == 2 => self.eval_type(args[1]),
            New => match self.new_class(args) {
                Some(class) => self.class_type(class),
                None => self.builtin_new_type(args),
            },
            // `char'a'`, a code point, and `raw"a\b"`.
            Char => INT32,
            Raw => STRING,
            Dot if args.len() == 2 => self.dot_type(args[0], args[1], forced)?,
            // `a[i]` calls `a.get(i)`.
            Index if !args.is_empty() => {
                let get = self.index_callee_type(args[0], forced)?;
                match *self.typing.types.get(get) {
                    TypeData::Fn { ret, .. } => ret,
                    _ => UNKNOWN,
                }
            }
            _ => match self.builtin_variant(f, args, forced)? {
                Some((sig, _)) => sig.ret,
                None => self.exact_variant(f, args, forced)?,
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
    /// written. A generator's call makes a generator: `SafeGenerator<Y>`
    /// for `GeneratorResult<Y>`, unless it has optional parameters.
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
            // Kotlin doesn't check one with optional parameters against
            // the function type its place wants, so neither does tempest.
            Some(t) if !typed.iter().any(|p| p.optional) => (self.generator_made(t), false),
            _ => (UNKNOWN, false),
        };
        self.typing.types.intern(TypeData::Fn {
            params: typed,
            ret,
            throws,
        })
    }

    /// What a generator's factory makes, `SafeGenerator<Y>`, for its
    /// literal's `GeneratorResult<Y>`, or unknown for another type.
    fn generator_made(&mut self, out_type: NodeId) -> TypeId {
        let t = self.eval_type(out_type);
        let types = &mut self.typing.types;
        let index = |name: &str| crate::builtins::TYPES.iter().position(|&t| t == name);
        let (Some(result), Some(made)) = (index("GeneratorResult"), index("SafeGenerator")) else {
            return UNKNOWN;
        };
        let result = types.builtin(result as u8);
        match types.get(t) {
            TypeData::Applied { base, args } if *base == result && args.len() == 1 => {
                let args = args.clone();
                let base = types.builtin(made as u8);
                types.intern(TypeData::Applied { base, args })
            }
            _ => UNKNOWN,
        }
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
                        None => self.alias_type(b),
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

    /// The type a type alias names, as `TestCase` in `export let TestCase
    /// = Pair<TestName, TestFun>`: a top-level `let` whose initializer is
    /// a type's name, or a type given arguments, nullable, or a function
    /// type. Not one that names itself, even through others.
    fn alias_type(&mut self, symbol: SymbolId) -> TypeId {
        let tree = &self.lib.tree;
        let s = self.scopes.symbol(symbol);
        let Some(decl) = s.decl.filter(|&d| {
            s.kind == SymbolKind::TopLevel && !s.mutable && tree.kind(d) == NodeKind::Let
        }) else {
            return UNKNOWN;
        };
        let (_, None, Some(init)) = core::let_parts(tree, decl) else {
            return UNKNOWN;
        };
        let is_type = tree.kind(init) == NodeKind::RightName
            || tree.kind(init) == NodeKind::Call
                && matches!(
                    self.scopes.resolved_builtin(tree.children(init)[0]),
                    Some(Builtin::Function(
                        Function::Angle | Function::Nullable | Function::FnType
                    ))
                );
        if !is_type || self.typing.aliases.contains(&decl) {
            return UNKNOWN;
        }
        self.typing.aliases.push(decl);
        let t = self.eval_type(init);
        self.typing.aliases.pop();
        t
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
    /// `Never<T>` is `Never` for now; not a class given the wrong number
    /// of them.
    fn applied_type(&mut self, base: NodeId, args: &[NodeId]) -> TypeId {
        let Some(b) = self.scopes.resolved(base) else {
            return UNKNOWN;
        };
        let base = match self.scopes.symbol(b).kind {
            SymbolKind::Builtin(Builtin::Type(i)) => {
                let t = self.typing.types.builtin(i);
                // `Never<T>` is the bottom type until it carries its `T`.
                if t == NEVER && args.len() == 1 {
                    return NEVER;
                }
                if t == NEVER || t == ANY_VALUE || self.typing.types.is_final(t) {
                    return UNKNOWN;
                }
                t
            }
            _ => {
                let Some(class) = self.type_decl(b) else {
                    return UNKNOWN;
                };
                if core::type_params(&self.lib.tree, class).len() != args.len() {
                    return UNKNOWN;
                }
                match self.class_symbol(class) {
                    Some(symbol) => self.typing.types.intern(TypeData::Class(symbol)),
                    None => return UNKNOWN,
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
        self.check_fields_first();
        self.type_all_exprs();
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
                match self.declared_result(f) {
                    Some(t) => self.check_value(t, value, Site::Return(node)),
                    None => self.check_literal_return(f, value),
                }
            }
            (NodeKind::Fn, ..) => {
                let body = core::fn_parts(&self.lib.tree, node).2;
                if let (Some(t), Some(body)) = (self.declared_result(node), body) {
                    if t != VOID && self.falls_off(body) {
                        self.report_falls_off(t, body);
                    }
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
                self.check_literal_result(node);
                if self.may_not_bubble(node) {
                    self.check_bubbles(node);
                }
            }
            (NodeKind::Call, ..) => {
                self.check_call(node);
                self.record_trailing_block(node);
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
            (NodeKind::Class, ..) => {
                self.check_overrides(node);
                self.check_fun_interface(node);
                self.check_connected_class(node);
            }
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
                self.check_value(t, init, Site::Field);
            }
            if matches!(kind, SymbolKind::TopLevel | SymbolKind::Local) {
                self.check_not_void(init, init);
                if ty.is_none() {
                    self.check_untyped_fun_literal(node, init);
                }
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

    /// Reports a function literal without a return type whose result
    /// isn't what the function type its place wants gives, as Kotlin
    /// Temper does: "Expected subtype of Int32, but got String", for `{ x
    /// => "a" }` where a `fn (Int32): Int32` is wanted, at the result,
    /// where Kotlin's position is empty, at the start of the body. The
    /// result is the body's last expression, as one value, so an `if`'s
    /// branches are joined. Not where `Void` is wanted, which
    /// Kotlin reports but for some builtin methods, as `forEach`, nor for
    /// a result that's null, `Void` or never returns, which it reports
    /// with odd types.
    fn check_literal_result(&mut self, f: NodeId) {
        let (_, out_type, body) = core::fn_parts(&self.lib.tree, f);
        let (None, Some(body)) = (out_type, body) else {
            return;
        };
        let Some(t) = self.fn_result(f, false).unwrap_or(None) else {
            return;
        };
        let mut result = body;
        while self.lib.tree.kind(result) == NodeKind::Block {
            match self.lib.tree.children(result).last() {
                Some(&last) => result = last,
                None => break,
            }
        }
        let s = self.known_type(result);
        if t == VOID || matches!(s, NULL | VOID | NEVER) || !self.misfits(s, t, result) {
            return;
        }
        let message = Message::ExpectedSubType(self.show(t), self.show(s));
        let span = self.lib.tree.span(result);
        self.lib.report(Level::Error, message, span);
    }

    /// Reports a value returned from a function literal without a return
    /// type that isn't the result its place wants, as Kotlin Temper does:
    /// "Expected subtype of Int32, but got String", for `fn (x) { return
    /// "a"; }` where a `fn (Int32): Int32` is wanted, at the value, where
    /// Kotlin's position is empty, just after the `return`; a bare
    /// `return` gives `Void`, at the `return`, and null is shown as the
    /// nullable result wanted. A value that never returns, as `panic()`,
    /// fits.
    fn check_literal_return(&mut self, f: NodeId, value: NodeId) {
        if core::fn_parts(&self.lib.tree, f).1.is_some() {
            return;
        }
        let Some(t) = self.fn_result(f, false).unwrap_or(None) else {
            return;
        };
        let s = self.known_type(value);
        if s == NEVER || !self.misfits(s, t, value) {
            return;
        }
        let got = match s {
            NULL => self.typing.types.nullable(t),
            _ => s,
        };
        let message = Message::ExpectedSubType(self.show(t), self.show(got));
        let span = self.lib.tree.span(value);
        self.lib.report(Level::Error, message, span);
    }

    /// Whether a body can reach its end without a value, as Kotlin Temper
    /// sees it: where its result would be an empty block, as an `if`'s
    /// missing `else`.
    fn falls_off(&self, node: NodeId) -> bool {
        let children = self.lib.tree.children(node);
        match self.lib.tree.kind(node) {
            NodeKind::Block => children.last().is_none_or(|&last| self.falls_off(last)),
            NodeKind::If if children.len() == 3 => {
                children[1..].iter().any(|&branch| self.falls_off(branch))
            }
            _ => false,
        }
    }

    /// Reports a body that can reach its end where a `t` is wanted, as
    /// Kotlin Temper does, as `Void` assigned to the result, at its
    /// closing brace: "Cannot assign to String from Void". `to_tmpl`
    /// fails there.
    fn report_falls_off(&mut self, t: TypeId, body: NodeId) {
        let span = self.lib.tree.span(body);
        let span = Span {
            start: span.end.saturating_sub(1),
            end: span.end,
        };
        let message = Message::CannotAssign(self.show(t), self.show(VOID));
        self.lib.report(Level::Error, message, span);
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
            // `a[i]` calls `a.get(i)`.
            Some(Builtin::Function(Function::Index)) if !args.is_empty() => {
                let t = self.known_index_callee_type(args[0]);
                match self.typing.types.get(t) {
                    TypeData::Fn { params, .. } => params.clone(),
                    _ => return,
                }
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
        // A constructor's class, or what's indexed, comes first.
        let args = match self.scopes.resolved_builtin(callee) {
            Some(_) => &args[1..],
            None => args,
        };
        // Not yet named arguments, nor type arguments.
        if (args.iter()).any(|&a| matches!(self.lib.value(a), Some(Value::Symbol(_)))) {
            return;
        }
        let required = params.iter().filter(|p| !p.optional).count();
        if args.len() < required || args.len() > params.len() {
            let span = self.lib.tree.span(node);
            let message = Message::ArityMismatch(params.len());
            self.lib.report(Level::Error, message, span);
        }
        let placed = self.placed_params(&params, args);
        for (&arg, &i) in args.iter().zip(&placed) {
            let Some(param) = params.get(i) else {
                continue;
            };
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
        // Kotlin Temper solves a generic call or a list literal from the
        // type wanted, and reports its arguments instead; not for a call's
        // argument, where it misses the error.
        let solves_wanted = !matches!(site, Site::Arg);
        if solves_wanted && self.is_list_literal(value) && self.check_list_wanted(value, t) {
            return;
        }
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
        if solves_wanted && self.check_generic_wanted(value, t) {
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
        let span = match site {
            Site::Assign(n) | Site::Return(n) => self.lib.tree.span(n),
            _ => value_span,
        };
        if self.check_fun_adapter(s, t, value, span) {
            return;
        }
        let (to, from) = (self.show(t), self.show(s));
        let (message, span) = match site {
            Site::Init | Site::Result => (Message::CannotAssign(to, from), value_span),
            Site::Assign(n) | Site::Return(n) => {
                (Message::CannotAssign(to, from), self.lib.tree.span(n))
            }
            Site::Arg | Site::Field => (Message::ExpectedSubType(to, from), value_span),
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
    fn is_mismatch(&mut self, s: TypeId, t: TypeId) -> bool {
        let types = &mut self.typing.types;
        types.is_known(s) && types.is_known(t) && types.is_subtype(s, t) == Some(false)
    }

    /// A type once declarations' types are known: any still waited on
    /// was never queued, so is unknown.
    fn known_type(&mut self, node: NodeId) -> TypeId {
        self.expr_type(node, false).unwrap_or(UNKNOWN)
    }

    /// What `object[…]` calls, as [`Settle::index_callee_type`] finds it.
    fn known_index_callee_type(&mut self, object: NodeId) -> TypeId {
        self.index_callee_type(object, false).unwrap_or(UNKNOWN)
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
