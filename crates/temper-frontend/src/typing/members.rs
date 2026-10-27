//! Classes' types, the fourth slice of milestone 4: values of a class or
//! interface (`this`, `new C(…)`), its members' types, read as `c.x`,
//! called as `c.add(1)` or a static `C.make()`, or named bare in the
//! class, and what assigning a member or calling a constructor wants.
//!
//! Not yet: a class or interface with type parameters, whose values and
//! members are unknown until generics are typed; an abstract method's
//! parameters, which aren't kept, so a call of one has a type but its
//! arguments aren't checked.

use crate::builtins::{Builtin, Function, TYPES};
use crate::core::{
    self, CLASS_INTERFACE, MEMBER_PRIVATE, MEMBER_STATIC, ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK,
    ROLE_METHOD, ROLE_PARAM, ROLE_SETTER, class_parts,
};
use crate::library::{Level, Message};
use crate::scope::{ScopeKind, SymbolId, SymbolKind};
use crate::settle::{Fact, Settle};
use crate::types::{ClassInfo, Param, TypeData, TypeId, UNKNOWN};
use temper_syntax::names::NameId;
use temper_syntax::tree::{NodeId, NodeKind};

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
            interface: tree.data(class) & CLASS_INTERFACE != 0,
            ancestors: (self.ancestors(class).into_iter())
                .filter_map(|c| self.scopes.resolved(class_parts(&self.lib.tree, c).0))
                .collect(),
        });
        self.typing.types.class(symbol, info)
    }

    /// The type of `this`, if `symbol` is a class's synthetic `this`.
    pub(super) fn this_type(&mut self, symbol: SymbolId) -> TypeId {
        let scope = self.scopes.get(self.scopes.symbol(symbol).owner);
        if scope.kind != ScopeKind::Class || scope.names.first().map(|&(_, b)| b) != Some(symbol) {
            return UNKNOWN;
        }
        self.class_type(scope.node)
    }

    /// The class a call `new C(…)` makes, if it's one typing knows.
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
    fn member_param(&mut self, param: NodeId) -> Param {
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
            Some(m) => self.member_type(m),
            None => UNKNOWN,
        })
    }

    /// The type of a bare name for a member, in its class, when read.
    pub(super) fn bare_member_type(&mut self, symbol: SymbolId) -> TypeId {
        match self.bare_member(symbol, false) {
            Some(m) => self.member_type(m),
            None => UNKNOWN,
        }
    }

    /// The type of what a call calls, its callee's, or if that's a
    /// member, as `c.add` in `c.add(1)` or `add` in its class, of the
    /// member that names, a method before a field.
    pub(super) fn callee_type(&mut self, callee: NodeId, forced: bool) -> Result<TypeId, Fact> {
        match self.called_member(callee, forced)? {
            Some(m) => Ok(self.member_type(m)),
            None => self.expr_type(callee, forced),
        }
    }

    /// The member a callee names, if it names one.
    pub(super) fn called_member(
        &mut self,
        callee: NodeId,
        forced: bool,
    ) -> Result<Option<NodeId>, Fact> {
        let tree = &self.lib.tree;
        match (tree.kind(callee), tree.children(callee)) {
            (NodeKind::RightName, _) => Ok(self
                .scopes
                .resolved(callee)
                .filter(|&b| self.scopes.symbol(b).kind == SymbolKind::Member)
                .and_then(|b| self.bare_member(b, true))),
            (NodeKind::Call, &[dot, object, key])
                if self.scopes.resolved_builtin(dot) == Some(Builtin::Function(Function::Dot)) =>
            {
                self.dot_member(object, key, true, forced)
            }
            _ => Ok(None),
        }
    }

    /// What a call of an abstract method gives, whose parameters aren't
    /// kept.
    pub(super) fn abstract_result(&mut self, m: NodeId) -> Option<TypeId> {
        let tree = &self.lib.tree;
        match core::let_parts(tree, m) {
            (_, Some(ty), None) if tree.data(m) & ROLE_MASK == ROLE_METHOD => {
                Some(self.eval_type(ty))
            }
            _ => None,
        }
    }

    /// The member `object.key` names, as [`Settle::member_named`] picks
    /// it: a static one if `object` names a class, as `C.make`.
    fn dot_member(
        &mut self,
        object: NodeId,
        key: NodeId,
        callee: bool,
        forced: bool,
    ) -> Result<Option<NodeId>, Fact> {
        let Some(name) = self.lib.symbol(key) else {
            return Ok(None);
        };
        if self.lib.tree.kind(object) == NodeKind::RightName {
            if let Some(class) = (self.scopes.resolved(object)).and_then(|b| self.type_decl(b)) {
                return Ok(self.member_named(class, name, true, callee));
            }
        }
        let t = self.expr_type(object, forced)?;
        Ok(match *self.typing.types.get(t) {
            TypeData::Class(symbol) => (self.scopes.symbol(symbol).decl)
                .and_then(|class| self.member_named(class, name, false, callee)),
            _ => None,
        })
    }

    /// The member a bare name in a class means.
    fn bare_member(&mut self, symbol: SymbolId, callee: bool) -> Option<NodeId> {
        let symbol = *self.scopes.symbol(symbol);
        let class = self.scopes.get(symbol.owner).node;
        let is_static = self.lib.tree.data(symbol.decl?) & MEMBER_STATIC != 0;
        self.member_named(class, symbol.name, is_static, callee)
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
        let members = match (tree.kind(target), children.as_slice()) {
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
                self.members_named(class, symbol.name, is_static)
            }
            (NodeKind::Call, &[dot, object, key])
                if self.scopes.resolved_builtin(dot) == Some(Builtin::Function(Function::Dot)) =>
            {
                let object_type = self.known_type(object);
                let (Some(name), &TypeData::Class(symbol)) =
                    (self.lib.symbol(key), self.typing.types.get(object_type))
                else {
                    return UNKNOWN;
                };
                let Some(class) = self.scopes.symbol(symbol).decl else {
                    return UNKNOWN;
                };
                self.members_named(class, name, false)
            }
            _ => return UNKNOWN,
        };
        let tree = &self.lib.tree;
        let field = members.iter().copied().find(|&m| {
            matches!(tree.data(m) & ROLE_MASK, 0 | ROLE_PARAM) && tree.data(m) & MEMBER_STATIC == 0
        });
        if let Some(field) = field {
            return core::let_parts(tree, field)
                .1
                .map_or(UNKNOWN, |t| self.eval_type(t));
        }
        let setter = members
            .iter()
            .find(|&&m| tree.data(m) & ROLE_MASK == ROLE_SETTER)
            .and_then(|&m| core::let_parts(tree, m).2)
            .and_then(|f| core::fn_parts(tree, f).0.first().copied());
        match setter {
            Some(param) => self.member_param(param).ty,
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
    fn member_type(&mut self, m: NodeId) -> TypeId {
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

    /// A method's signature, with its parameters as [`Settle::member_param`]
    /// types them.
    fn method_type(&mut self, f: NodeId) -> TypeId {
        let t = self.fn_type(f);
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
