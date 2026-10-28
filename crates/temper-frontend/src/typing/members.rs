//! Classes' types, the fourth slice of milestone 4: values of a class or
//! interface (`this`, `new C(…)`), its members' types, read as `c.x`,
//! called as `c.add(1)` or a static `C.make()`, or named bare in the
//! class, and what assigning a member or calling a constructor wants.
//!
//! A generic class's values are typed too (notes/typing.md, "Generic
//! types"): `new Box(1)` is solved as a generic call
//! ([`super::generic_calls`]), and a `Box<Int32>`'s members have `Int32`
//! for `T`.

use super::core_members::{CoreMember, CoreMemberKind, CoreType, core_class};
use super::signatures::OperatorMethod;
use crate::builtins::{Builtin, Function, TYPES};
use crate::core::{
    self, CLASS_AUTO_EQ, CLASS_INTERFACE, MEMBER_OPERATOR_EQ, MEMBER_PRIVATE, MEMBER_STATIC,
    ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK, ROLE_METHOD, ROLE_PARAM, ROLE_SETTER, class_parts,
};
use crate::library::{Level, Message};
use crate::scope::{ScopeKind, SymbolId, SymbolKind};
use crate::settle::{Fact, Settle};
use crate::types::{
    ANY_VALUE, ClassInfo, NEVER, Param, TypeData, TypeId, Types, UNKNOWN, builtin_supertype,
};
use temper_syntax::names::NameId;
use temper_syntax::tree::{NodeId, NodeKind};

/// A value's class, with what its type parameters are bound to.
pub(super) struct Receiver {
    class: NodeId,
    formals: Vec<SymbolId>,
    actuals: Vec<TypeId>,
}

impl Settle {
    /// The type of a class's or interface's values, unless it's generic.
    /// One named as a builtin type is core's declaration of it, as in
    /// Kotlin Temper's `core.temper`, so it's that type.
    pub(super) fn class_type(&mut self, class: NodeId) -> TypeId {
        let tree = &self.lib.tree;
        if tree.kind(class) != NodeKind::Class || !core::type_params(tree, class).is_empty() {
            return UNKNOWN;
        }
        if let Some(i) = TYPES.iter().position(|&t| t == self.member_text(class)) {
            return self.typing.types.builtin(i as u8);
        }
        let Some(symbol) = self.scopes.resolved(class_parts(tree, class).0) else {
            return UNKNOWN;
        };
        let info = (!self.typing.types.knows_class(symbol)).then(|| ClassInfo {
            ancestors: (self.ancestors(class).into_iter())
                .filter_map(|c| self.scopes.resolved(class_parts(&self.lib.tree, c).0))
                .collect(),
        });
        self.typing.types.class(symbol, info)
    }

    /// The type of `this`, if `symbol` is a class's synthetic `this`: in
    /// a generic class, as `Box<T>`, given its own type parameters.
    pub(super) fn this_type(&mut self, symbol: SymbolId) -> TypeId {
        let scope = self.scopes.get(self.scopes.symbol(symbol).owner);
        if scope.kind != ScopeKind::Class || scope.names.first().map(|&(_, b)| b) != Some(symbol) {
            return UNKNOWN;
        }
        self.class_or_own_type(scope.node)
    }

    /// A class's type as its body sees it: given its own type parameters
    /// if it's generic, as `Box<T>`.
    fn class_or_own_type(&mut self, class: NodeId) -> TypeId {
        let formals = self.own_formals(class);
        if formals.is_empty() {
            return self.class_type(class);
        }
        let args = formals.iter().map(|&f| self.type_param_type(f)).collect();
        self.generic_class_type(class, args)
    }

    /// A generic class's or interface's type given arguments, as
    /// `Box<Int32>`, unless it's unknown: core's builtin types aren't
    /// typed this way.
    pub(super) fn generic_class_type(&mut self, class: NodeId, args: Vec<TypeId>) -> TypeId {
        let tree = &self.lib.tree;
        let builtin = TYPES.contains(&self.member_text(class));
        let symbol = self.scopes.resolved(class_parts(tree, class).0);
        match symbol {
            Some(symbol) if !builtin && core::type_params(tree, class).len() == args.len() => {
                let base = self.typing.types.intern(TypeData::Class(symbol));
                self.typing.types.intern(TypeData::Applied { base, args })
            }
            _ => UNKNOWN,
        }
    }

    /// The class of a value of type `t`, with what its type parameters
    /// are bound to, as `T` to `Int32` for a `Box<Int32>`.
    fn receiver(&mut self, t: TypeId) -> Option<Receiver> {
        let (symbol, actuals) = match self.typing.types.get(t) {
            &TypeData::Class(symbol) => (symbol, Vec::new()),
            TypeData::Applied { base, args } => match *self.typing.types.get(*base) {
                TypeData::Class(symbol) => (symbol, args.clone()),
                _ => return None,
            },
            _ => return None,
        };
        let class = self.scopes.symbol(symbol).decl?;
        let formals = self.own_formals(class);
        (formals.len() == actuals.len()).then_some(Receiver {
            class,
            formals,
            actuals,
        })
    }

    /// A class's receiver as its static members, or a bare name in its
    /// body, see it: its type parameters as they're written, or unknown
    /// if `statics`, as static members can't use them.
    fn own_receiver(&mut self, class: NodeId, statics: bool) -> Receiver {
        let formals = self.own_formals(class);
        let actuals = match statics {
            true => vec![UNKNOWN; formals.len()],
            false => formals.iter().map(|&f| self.type_param_type(f)).collect(),
        };
        Receiver {
            class,
            formals,
            actuals,
        }
    }

    /// A member's type when read through `receiver`: with the receiver's
    /// type arguments put in for its class's type parameters. One
    /// inherited from a generic supertype has that supertype's type
    /// parameters unknown, as supertypes' arguments aren't typed yet.
    fn member_type_in(&mut self, m: NodeId, receiver: &Receiver) -> TypeId {
        let t = self.member_type(m);
        let owner = self.member_owner(m);
        self.receiver_view(t, owner, receiver)
    }

    /// A type in a member declared by `owner`, seen through `receiver`,
    /// as [`Settle::member_type_in`] sees it.
    fn receiver_view(&mut self, t: TypeId, owner: Option<NodeId>, receiver: &Receiver) -> TypeId {
        match owner {
            Some(owner) if owner == receiver.class => {
                (self.typing.types).substitute(t, &receiver.formals, &receiver.actuals)
            }
            Some(owner) => {
                let formals = self.own_formals(owner);
                self.typing.types.erase(t, &formals)
            }
            None => UNKNOWN,
        }
    }

    /// The class or interface declaring a member.
    fn member_owner(&self, m: NodeId) -> Option<NodeId> {
        let b = self.scopes.resolved(*self.lib.tree.children(m).first()?)?;
        Some(self.scopes.get(self.scopes.symbol(b).owner).node)
    }

    /// The class a call `new C(…)` makes, if it's one typing knows and
    /// isn't generic, whose calls are solved ([`super::generic_calls`]).
    pub(super) fn new_class(&self, args: &[NodeId]) -> Option<NodeId> {
        let class = self.type_decl(self.scopes.resolved(*args.first()?)?)?;
        let tree = &self.lib.tree;
        (tree.data(class) & CLASS_INTERFACE == 0 && core::type_params(tree, class).is_empty())
            .then_some(class)
    }

    /// The parameters of a class's constructor.
    pub(super) fn constructor_type(&mut self, class: NodeId) -> Vec<Param> {
        let params = core::constructor_params(&self.lib.tree, class);
        params.into_iter().map(|p| self.member_param(p)).collect()
    }

    /// A parameter of a method or constructor: Kotlin Temper types one
    /// with a default as nullable, as null means the default.
    pub(super) fn member_param(&mut self, param: NodeId) -> Param {
        let (_, ty, default) = core::let_parts(&self.lib.tree, param);
        let mut ty = ty.map_or(UNKNOWN, |t| self.eval_type(t));
        if default.is_some() {
            ty = self.typing.types.nullable(ty);
        }
        Param {
            ty,
            optional: default.is_some(),
        }
    }

    /// The type of `object.key` when read: a field's or getter's, or a
    /// method's signature.
    pub(super) fn dot_type(
        &mut self,
        object: NodeId,
        key: NodeId,
        forced: bool,
    ) -> Result<TypeId, Fact> {
        Ok(match self.dot_member(object, key, false, forced)? {
            Some(t) => t,
            None => (self.core_member_type(object, key, false, forced)?).unwrap_or(UNKNOWN),
        })
    }

    /// The type of a bare name for a member, in its class, when read.
    pub(super) fn bare_member_type(&mut self, symbol: SymbolId) -> TypeId {
        self.bare_member(symbol, false).unwrap_or(UNKNOWN)
    }

    /// The type of what a call calls, its callee's, or if that's a
    /// member, as `c.add` in `c.add(1)` or `add` in its class, of the
    /// member that names, a method before a field.
    pub(super) fn callee_type(&mut self, callee: NodeId, forced: bool) -> Result<TypeId, Fact> {
        if let Some(t) = self.called_member(callee, forced)? {
            return Ok(t);
        }
        if let Some((object, key)) = self.dot_parts(callee) {
            if let Some(t) = self.core_member_type(object, key, true, forced)? {
                return Ok(t);
            }
        }
        self.expr_type(callee, forced)
    }

    /// `object` and `key` of a callee `object.key`.
    fn dot_parts(&self, callee: NodeId) -> Option<(NodeId, NodeId)> {
        match (self.lib.tree.kind(callee), self.lib.tree.children(callee)) {
            (NodeKind::Call, &[dot, object, key])
                if self.scopes.resolved_builtin(dot) == Some(Builtin::Function(Function::Dot)) =>
            {
                Some((object, key))
            }
            _ => None,
        }
    }

    /// The builtin type a name names, as `String`.
    fn builtin_type_named(&self, node: NodeId) -> Option<u8> {
        match self.scopes.resolved_builtin(node) {
            Some(Builtin::Type(i)) if self.lib.tree.kind(node) == NodeKind::RightName => Some(i),
            _ => None,
        }
    }

    /// Whether a node is `T.key` for a builtin type `T`, a static member.
    pub(super) fn names_builtin_static(&self, node: NodeId) -> bool {
        self.dot_parts(node)
            .is_some_and(|(object, _)| self.builtin_type_named(object).is_some())
    }

    /// The type of `object.key` for a member of a builtin type, as core
    /// declares it ([`super::core_members`]): an instance's, or a static
    /// one's if `object` names the type, as `String.fromCodePoint`. A
    /// method first if `callee`, else a field or getter first.
    fn core_member_type(
        &mut self,
        object: NodeId,
        key: NodeId,
        callee: bool,
        forced: bool,
    ) -> Result<Option<TypeId>, Fact> {
        let Some(name) = self.lib.symbol(key) else {
            return Ok(None);
        };
        let name = self.lib.names.get(name).to_string();
        self.core_member_named(object, &name, callee, forced)
    }

    /// The type of what `object[…]` calls, `object.get` as Kotlin Temper
    /// desugars it, for a value of a class or a builtin type.
    pub(super) fn index_callee_type(
        &mut self,
        object: NodeId,
        forced: bool,
    ) -> Result<TypeId, Fact> {
        let t = self.expr_type(object, forced)?;
        if let Some(receiver) = self.receiver(t) {
            let Some(get) = self.lib.names.find("get") else {
                return Ok(UNKNOWN);
            };
            return Ok(match self.member_named(receiver.class, get, false, true) {
                Some(m) => self.member_type_in(m, &receiver),
                None => UNKNOWN,
            });
        }
        Ok((self.core_member_named(object, "get", true, forced)?).unwrap_or(UNKNOWN))
    }

    /// The type of `object.name` for a member of a builtin type, as
    /// [`Settle::core_member_type`] finds it, with a generic type's
    /// arguments put in for its type parameters, as `Int32` for `T` in a
    /// `List<Int32>`'s `get`. An instance's member may be inherited from
    /// the builtin generic type it extends, as `List`'s `isEmpty` from
    /// `Listed`.
    fn core_member_named(
        &mut self,
        object: NodeId,
        name: &str,
        callee: bool,
        forced: bool,
    ) -> Result<Option<TypeId>, Fact> {
        let (type_index, args, statics) = match self.builtin_type_named(object) {
            Some(i) => (i, Vec::new(), true),
            None => {
                let t = self.expr_type(object, forced)?;
                let types = &self.typing.types;
                match types.get(types.base(t)) {
                    &TypeData::Builtin(i) => (i, types.args(t).to_vec(), false),
                    _ => return Ok(None),
                }
            }
        };
        let mut type_name = match TYPES[type_index as usize] {
            "Int" => "Int32",
            other => other,
        };
        let named = |m: &&CoreMember| m.name == name && m.is_static == statics;
        let is_method = |m: &&CoreMember| matches!(m.kind, CoreMemberKind::Method { .. });
        let (class, member) = loop {
            let class = core_class(type_name);
            let members = &class.members;
            let picked = (members.iter().filter(named))
                .find(|m| is_method(m) == callee)
                .or_else(|| members.iter().find(named));
            if let Some(member) = picked {
                break (class, member);
            }
            match builtin_supertype(type_name) {
                Some(sup) if !statics => type_name = sup,
                _ => return Ok(None),
            }
        };
        // A static member can't use the type's parameters, nor can a
        // generic type given the wrong number of arguments.
        let args = match args.len() == class.type_params {
            true => args,
            false => vec![UNKNOWN; class.type_params],
        };
        let types = &mut self.typing.types;
        Ok(Some(match &member.kind {
            CoreMemberKind::Value(t) => core_type(types, t, &args),
            CoreMemberKind::Method {
                params,
                ret,
                throws,
            } => {
                let params = (params.iter())
                    .map(|(t, optional)| {
                        let ty = core_type(types, t, &args);
                        Param {
                            // Null means the default, as in Kotlin Temper.
                            ty: if *optional { types.nullable(ty) } else { ty },
                            optional: *optional,
                        }
                    })
                    .collect();
                let ret = core_type(types, ret, &args);
                types.intern(TypeData::Fn {
                    params,
                    ret,
                    throws: *throws,
                })
            }
        }))
    }

    /// The type of the member a callee names, if it names one.
    fn called_member(&mut self, callee: NodeId, forced: bool) -> Result<Option<TypeId>, Fact> {
        Ok((self.called_member_node(callee, forced)?)
            .map(|(m, receiver, _)| self.member_type_in(m, &receiver)))
    }

    /// The generic method a callee names, as `o.first` in `o.first(xs)`
    /// for `first<T>(items: List<T>): T`, with its signature as the
    /// receiver sees it, its own type parameters as written, and the
    /// receiver's type for an instance's method, which Kotlin Temper
    /// shows as its first parameter.
    pub(super) fn generic_method(
        &mut self,
        callee: NodeId,
        forced: bool,
    ) -> Result<Option<(NodeId, TypeId, Option<TypeId>)>, Fact> {
        let Some((m, receiver, this)) = self.called_member_node(callee, forced)? else {
            return Ok(None);
        };
        let tree = &self.lib.tree;
        let f = core::let_parts(tree, m).2.filter(|&f| {
            tree.data(m) & ROLE_MASK == ROLE_METHOD
                && tree.kind(f) == NodeKind::Fn
                && !core::type_params(tree, f).is_empty()
        });
        let Some(f) = f else {
            return Ok(None);
        };
        let sig = self.method_signature(f);
        let owner = self.member_owner(m);
        let sig = self.receiver_view(sig, owner, &receiver);
        Ok(Some((f, sig, this)))
    }

    /// The member a callee names, if it names one, with its receiver and,
    /// for an instance's member, the receiver's type.
    fn called_member_node(
        &mut self,
        callee: NodeId,
        forced: bool,
    ) -> Result<Option<(NodeId, Receiver, Option<TypeId>)>, Fact> {
        let tree = &self.lib.tree;
        match (tree.kind(callee), tree.children(callee)) {
            (NodeKind::RightName, _) => Ok(self
                .scopes
                .resolved(callee)
                .filter(|&b| self.scopes.symbol(b).kind == SymbolKind::Member)
                .and_then(|b| self.bare_member_node(b, true))),
            (NodeKind::Call, &[dot, object, key])
                if self.scopes.resolved_builtin(dot) == Some(Builtin::Function(Function::Dot)) =>
            {
                self.dot_member_node(object, key, true, forced)
            }
            _ => Ok(None),
        }
    }

    /// The type of the member `object.key` names, as
    /// [`Settle::member_named`] picks it: a static one if `object` names a
    /// class, as `C.make`.
    fn dot_member(
        &mut self,
        object: NodeId,
        key: NodeId,
        callee: bool,
        forced: bool,
    ) -> Result<Option<TypeId>, Fact> {
        Ok((self.dot_member_node(object, key, callee, forced)?)
            .map(|(m, receiver, _)| self.member_type_in(m, &receiver)))
    }

    /// The member `object.key` names, as [`Settle::dot_member`] picks it,
    /// with its receiver and, for an instance's member, `object`'s type.
    fn dot_member_node(
        &mut self,
        object: NodeId,
        key: NodeId,
        callee: bool,
        forced: bool,
    ) -> Result<Option<(NodeId, Receiver, Option<TypeId>)>, Fact> {
        let Some(name) = self.lib.symbol(key) else {
            return Ok(None);
        };
        if self.lib.tree.kind(object) == NodeKind::RightName {
            if let Some(class) = (self.scopes.resolved(object)).and_then(|b| self.type_decl(b)) {
                let receiver = self.own_receiver(class, true);
                return Ok(
                    (self.member_named(class, name, true, callee)).map(|m| (m, receiver, None))
                );
            }
        }
        let t = self.expr_type(object, forced)?;
        let Some(receiver) = self.receiver(t) else {
            return Ok(None);
        };
        Ok(
            (self.member_named(receiver.class, name, false, callee))
                .map(|m| (m, receiver, Some(t))),
        )
    }

    /// The type of the member a bare name in a class means.
    fn bare_member(&mut self, symbol: SymbolId, callee: bool) -> Option<TypeId> {
        let (m, receiver, _) = self.bare_member_node(symbol, callee)?;
        Some(self.member_type_in(m, &receiver))
    }

    /// The member a bare name in a class means, as [`Settle::bare_member`]
    /// picks it, with its receiver and, for an instance's member, the
    /// type of `this`.
    fn bare_member_node(
        &mut self,
        symbol: SymbolId,
        callee: bool,
    ) -> Option<(NodeId, Receiver, Option<TypeId>)> {
        let symbol = *self.scopes.symbol(symbol);
        let class = self.scopes.get(symbol.owner).node;
        let is_static = self.lib.tree.data(symbol.decl?) & MEMBER_STATIC != 0;
        let m = self.member_named(class, symbol.name, is_static, callee)?;
        let receiver = self.own_receiver(class, is_static);
        let this = (!is_static).then(|| self.class_or_own_type(class));
        Some((m, receiver, this))
    }

    /// Reports calling a method that a class inherits more than once
    /// ([`Settle::tied_methods`]), as Kotlin Temper does, where its
    /// solver can't pick one: if more than one fits the arguments, as far
    /// as typing knows them. Returns whether the callee is such a method.
    pub(super) fn check_tie(&mut self, call: NodeId, callee: NodeId, args: &[NodeId]) -> bool {
        let tree = &self.lib.tree;
        let (NodeKind::Call, &[dot, object, key]) = (tree.kind(callee), tree.children(callee))
        else {
            return false;
        };
        let is_dot = self.scopes.resolved_builtin(dot) == Some(Builtin::Function(Function::Dot));
        let this = self.known_type(object);
        let (true, Some(name), &TypeData::Class(symbol)) =
            (is_dot, self.lib.symbol(key), self.typing.types.get(this))
        else {
            return false;
        };
        let Some(class) = self.scopes.symbol(symbol).decl else {
            return false;
        };
        let tied = self.tied_methods(class, name);
        if tied.len() < 2 {
            return false;
        }
        let arg_types: Vec<TypeId> = args.iter().map(|&a| self.known_type(a)).collect();
        let sigs: Vec<TypeId> = tied.iter().map(|&(_, m)| self.member_type(m)).collect();
        let types = &self.typing.types;
        if !arg_types.iter().chain(&sigs).all(|&t| types.is_known(t)) {
            return true;
        }
        let fits = |sig: TypeId| match types.get(sig) {
            TypeData::Fn { params, .. } => {
                let required = params.iter().filter(|p| !p.optional).count();
                (required..=params.len()).contains(&args.len())
                    && (arg_types.iter().zip(params))
                        .all(|(&a, p)| types.is_subtype(a, p.ty) != Some(false))
            }
            _ => false,
        };
        if sigs.iter().filter(|&&sig| fits(sig)).count() < 2 {
            return true;
        }
        let mut inputs = vec![self.show(this)];
        inputs.extend(arg_types.iter().map(|&t| self.show(t)));
        let among: Vec<String> = (tied.iter().zip(&sigs))
            .map(|(&(owner, _), &sig)| {
                let mut params = vec![self.member_text(owner).to_string()];
                let TypeData::Fn {
                    params: ps, ret, ..
                } = self.typing.types.get(sig)
                else {
                    unreachable!("checked by fits");
                };
                params.extend(ps.iter().map(|p| self.show(p.ty)));
                format!("({}) -> {}", params.join(", "), self.show(*ret))
            })
            .collect();
        let message = Message::NoCalleeMatches(inputs.join(", "), among.join(", "));
        let span = self.lib.tree.span(call);
        self.lib.report(Level::Error, message, span);
        true
    }

    /// What assigning a member wants, by the assignment's target: a bare
    /// name in its class or `object.key`.
    pub(super) fn member_target_type(&mut self, target: NodeId) -> TypeId {
        let tree = &self.lib.tree;
        let children = tree.children(target).to_vec();
        let (members, receiver) = match (tree.kind(target), children.as_slice()) {
            (NodeKind::RightName, _) => {
                let Some(b) = self.scopes.resolved(target) else {
                    return UNKNOWN;
                };
                let symbol = *self.scopes.symbol(b);
                let Some(decl) = symbol.decl.filter(|_| symbol.kind == SymbolKind::Member) else {
                    return UNKNOWN;
                };
                let class = self.scopes.get(symbol.owner).node;
                let is_static = self.lib.tree.data(decl) & MEMBER_STATIC != 0;
                let receiver = self.own_receiver(class, is_static);
                (self.members_named(class, symbol.name, is_static), receiver)
            }
            (NodeKind::Call, &[dot, object, key])
                if self.scopes.resolved_builtin(dot) == Some(Builtin::Function(Function::Dot)) =>
            {
                let object_type = self.known_type(object);
                let (Some(name), Some(receiver)) =
                    (self.lib.symbol(key), self.receiver(object_type))
                else {
                    return UNKNOWN;
                };
                (self.members_named(receiver.class, name, false), receiver)
            }
            _ => return UNKNOWN,
        };
        let tree = &self.lib.tree;
        let field = members.iter().copied().find(|&m| {
            matches!(tree.data(m) & ROLE_MASK, 0 | ROLE_PARAM) && tree.data(m) & MEMBER_STATIC == 0
        });
        if let Some(field) = field {
            return self.member_type_in(field, &receiver);
        }
        let setter = members
            .iter()
            .find(|&&m| tree.data(m) & ROLE_MASK == ROLE_SETTER);
        let Some(&setter) = setter else {
            return UNKNOWN;
        };
        let param = (core::let_parts(tree, setter).2)
            .and_then(|f| core::fn_parts(tree, f).0.first().copied());
        match param {
            Some(param) => {
                let t = self.member_param(param).ty;
                let owner = self.member_owner(setter);
                self.receiver_view(t, owner, &receiver)
            }
            None => UNKNOWN,
        }
    }

    /// A class's member named `name`, a method if `callee`, else a field
    /// or getter, or else the other. None if there's more than one method
    /// of that name, which Kotlin Temper allows for overloads, or if the
    /// class inherits more than one ([`Settle::tied_methods`]).
    fn member_named(
        &mut self,
        class: NodeId,
        name: NameId,
        statics: bool,
        callee: bool,
    ) -> Option<NodeId> {
        let members = self.members_named(class, name, statics);
        let tree = &self.lib.tree;
        let role = |m: NodeId| tree.data(m) & ROLE_MASK;
        let mut methods = members.iter().copied().filter(|&m| role(m) == ROLE_METHOD);
        let method = methods.next();
        if methods.next().is_some() {
            return None;
        }
        let read =
            (members.iter().copied()).find(|&m| matches!(role(m), 0 | ROLE_PARAM | ROLE_GETTER));
        let picked = if callee {
            method.or(read)
        } else {
            read.or(method)
        };
        if picked.is_some()
            && picked == method
            && !statics
            && self.tied_methods(class, name).len() > 1
        {
            return None;
        }
        picked
    }

    /// The methods named `name` that a class inherits, unless it declares
    /// one itself, as Kotlin Temper finds them (`FilterOutMaskedMembers.kt`):
    /// leaving out one that another overrides, and one with the same
    /// signature as one nearer the class (fewer `extends` away). More than
    /// one left is a tie, which Kotlin reports where it's called as "No
    /// callee matches".
    pub(super) fn tied_methods(&mut self, class: NodeId, name: NameId) -> Vec<(NodeId, NodeId)> {
        let tree = &self.lib.tree;
        let named = |m: NodeId| {
            tree.kind(m) == NodeKind::Let
                && self.lib.name(tree.children(m)[0]) == Some(name)
                && tree.data(m) & (MEMBER_STATIC | MEMBER_PRIVATE) == 0
        };
        if class_parts(tree, class).2.iter().any(|&m| named(m)) {
            return Vec::new();
        }
        // Supertypes breadth-first, with how far each is.
        let mut found = vec![(class, 0)];
        let mut next = 0;
        while next < found.len() {
            let (c, depth) = found[next];
            for sup in self.super_decls(c) {
                if !found.iter().any(|&(f, _)| f == sup) {
                    found.push((sup, depth + 1));
                }
            }
            next += 1;
        }
        let mut candidates: Vec<(NodeId, NodeId, u32)> = Vec::new();
        for &(owner, depth) in &found[1..] {
            let members = class_parts(&self.lib.tree, owner).2;
            let method = members
                .iter()
                .copied()
                .find(|&m| named(m) && self.lib.tree.data(m) & ROLE_MASK == ROLE_METHOD);
            if let Some(method) = method {
                candidates.push((owner, method, depth));
            }
        }
        let ancestors: Vec<Vec<NodeId>> = (candidates.iter())
            .map(|&(owner, ..)| self.ancestors(owner))
            .collect();
        let overridden = |i: usize| {
            (0..candidates.len()).any(|j| j != i && ancestors[j].contains(&candidates[i].0))
        };
        let candidates: Vec<(NodeId, NodeId, u32)> = (0..candidates.len())
            .filter(|&i| !overridden(i))
            .map(|i| candidates[i])
            .collect();
        let types: Vec<TypeId> = (candidates.iter())
            .map(|&(_, m, _)| self.member_type(m))
            .collect();
        let known = |t: TypeId| self.typing.types.is_known(t);
        let deeper_twin = |i: usize| {
            (0..candidates.len()).any(|j| {
                types[i] == types[j] && known(types[i]) && candidates[j].2 < candidates[i].2
            })
        };
        let mut tied: Vec<(NodeId, NodeId)> = (0..candidates.len())
            .filter(|&i| !deeper_twin(i))
            .map(|i| (candidates[i].0, candidates[i].1))
            .collect();
        // Kotlin lists them depth-first, in `extends` order.
        let mut order = Vec::new();
        self.depth_first(class, &mut order);
        tied.sort_by_key(|&(owner, _)| order.iter().position(|&c| c == owner));
        tied
    }

    /// A class and its supertypes, depth-first, each once.
    fn depth_first(&self, class: NodeId, out: &mut Vec<NodeId>) {
        if out.contains(&class) {
            return;
        }
        out.push(class);
        for sup in self.super_decls(class) {
            self.depth_first(sup, out);
        }
    }

    /// The method marked with `flag` ([`MEMBER_OPERATOR_EQ`] or
    /// [`core::MEMBER_OPERATOR_CMP`]) that an operator calls for a value of
    /// `class`: its own, or else the nearest inherited one, breadth-first;
    /// failing that, the one `@auto("==")` makes.
    pub(super) fn operator_method(&self, class: NodeId, flag: u32) -> OperatorMethod {
        let tree = &self.lib.tree;
        let ancestors = self.ancestors(class);
        let lineage = || std::iter::once(class).chain(ancestors.iter().copied());
        for owner in lineage() {
            let members = class_parts(tree, owner).2;
            let marked = members.iter().copied().find(|&m| {
                tree.data(m) & (flag | MEMBER_STATIC) == flag
                    && tree.data(m) & ROLE_MASK == ROLE_METHOD
            });
            if let Some(method) = marked {
                return OperatorMethod::Method { owner, method };
            }
        }
        let auto_eq = |c: NodeId| flag == MEMBER_OPERATOR_EQ && tree.data(c) & CLASS_AUTO_EQ != 0;
        let unknown_super =
            lineage().any(|c| class_parts(tree, c).1.len() != self.super_decls(c).len());
        if unknown_super || ancestors.iter().any(|&c| auto_eq(c)) {
            OperatorMethod::Unsure
        } else if auto_eq(class) {
            OperatorMethod::AutoEq
        } else {
            OperatorMethod::Builtin
        }
    }

    /// A class's members named `name`: its instances' own and inherited
    /// ones, or if `statics`, its own static ones. Not constructors.
    fn members_named(&self, class: NodeId, name: NameId, statics: bool) -> Vec<NodeId> {
        let Some(scope) = self.scopes.of_node(class) else {
            return Vec::new();
        };
        let tree = &self.lib.tree;
        (self.scopes.get(scope).names.iter())
            .filter(|&&(n, _)| n == name)
            .filter_map(|&(_, b)| self.scopes.symbol(b).decl)
            .filter(|&m| {
                tree.kind(m) == NodeKind::Let
                    && (tree.data(m) & MEMBER_STATIC != 0) == statics
                    && tree.data(m) & ROLE_MASK != ROLE_CONSTRUCTOR
            })
            .collect()
    }

    /// A member's type when read: a field's, a getter's result, or a
    /// method's signature.
    pub(super) fn member_type(&mut self, m: NodeId) -> TypeId {
        let tree = &self.lib.tree;
        let data = tree.data(m);
        let (_, ty, init) = core::let_parts(tree, m);
        match (data & ROLE_MASK, ty, init) {
            (0 | ROLE_PARAM | ROLE_GETTER, Some(ty), _) => self.eval_type(ty),
            (ROLE_GETTER, None, Some(f)) => self.declared_result(f).unwrap_or(UNKNOWN),
            (ROLE_METHOD, None, Some(f)) => self.method_type(f),
            _ => UNKNOWN,
        }
    }

    /// A method's type, from its signature, with its own type parameters
    /// unknown, as references and unsolved calls see it.
    fn method_type(&mut self, f: NodeId) -> TypeId {
        let t = self.method_signature(f);
        let formals = self.own_formals(f);
        self.typing.types.erase(t, &formals)
    }

    /// A method's signature, with its parameters as [`Settle::member_param`]
    /// types them, and its own type parameters as written.
    fn method_signature(&mut self, f: NodeId) -> TypeId {
        let t = self.fn_signature(f);
        let TypeData::Fn { ret, throws, .. } = *self.typing.types.get(t) else {
            return t;
        };
        let params = core::fn_parts(&self.lib.tree, f).0;
        let params = (params.into_iter())
            .map(|p| match self.lib.tree.kind(p) {
                NodeKind::Let => self.member_param(p),
                _ => Param {
                    ty: UNKNOWN,
                    optional: false,
                },
            })
            .collect();
        self.typing.types.intern(TypeData::Fn {
            params,
            ret,
            throws,
        })
    }
}

/// A type in a builtin type's member's signature, with `args` put in for
/// the type's own type parameters. A generic type that typing doesn't
/// take arguments for, as `Never<T>`, is unknown.
fn core_type(types: &mut Types, t: &CoreType, args: &[TypeId]) -> TypeId {
    match t {
        CoreType::Unknown => UNKNOWN,
        CoreType::Builtin(i) => types.builtin(*i),
        CoreType::Param(i) => args.get(*i as usize).copied().unwrap_or(UNKNOWN),
        CoreType::Applied(base, xs) => {
            let base = types.builtin(*base);
            if base == NEVER || base == ANY_VALUE || types.is_final(base) {
                return UNKNOWN;
            }
            let args = xs.iter().map(|x| core_type(types, x, args)).collect();
            types.intern(TypeData::Applied { base, args })
        }
        CoreType::Nullable(inner) => {
            let inner = core_type(types, inner, args);
            types.nullable(inner)
        }
        CoreType::Fn(params, ret) => {
            let params = (params.iter())
                .map(|p| Param {
                    ty: core_type(types, p, args),
                    optional: false,
                })
                .collect();
            let ret = core_type(types, ret, args);
            types.intern(TypeData::Fn {
                params,
                ret,
                throws: false,
            })
        }
    }
}
