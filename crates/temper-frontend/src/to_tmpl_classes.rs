//! Classes and interfaces in tmpl's translation (subset B's first step;
//! notes/tmpl/roadmap.md): declarations with their fields, a
//! constructor (the class's own, or one made from its header and field
//! initializers), methods, getters and setters, static fields and
//! functions, and `@operator` methods, among them the `eq` that
//! `@auto("==")` makes (a real member since the frontend generates it);
//! and in bodies, `this`, members read, called, and assigned, `new`,
//! `is`, identity for `==` with no method to call, and the conversions
//! that subtyping needs: `Upcast` where a value goes where a supertype is
//! wanted, and `Downcast` where an `is` check narrowed a local.
//!
//! Not yet: generic classes, connected members, parameters' defaults,
//! a function or method as a value, supertypes from core (as
//! `Stringable`), and static fields assigned after their initializer.

use super::{Translator, Tx};
use crate::builtins::Function;
use crate::core::{
    self, CLASS_INTERFACE, LET_INIT, LET_VAR, MEMBER_CONNECTED, MEMBER_OPERATOR_CMP,
    MEMBER_OPERATOR_EQ, MEMBER_PRIVATE, MEMBER_PROTECTED, MEMBER_STATIC, ROLE_CONSTRUCTOR,
    ROLE_GETTER, ROLE_MASK, ROLE_METHOD, ROLE_PARAM, ROLE_SETTER, ROLE_TYPE_PARAM,
};
use crate::scope::{SymbolId, SymbolKind};
use crate::types::{self as ft, TypeData as FTypeData};
use temper_syntax::tree::{NodeId as FNode, NodeKind};
use temper_tmpl::analysis::is_subtype;
use temper_tmpl::ops::Op;
use temper_tmpl::{DeclId, DeclKind, Facts, NodeId, TypeId};

/// What a class member is in tmpl.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Member {
    /// A field of a class, or a static field.
    Field(DeclId),
    /// A method, getter, or setter.
    Method(DeclId),
    /// A property of an interface: its getter, and its setter if `var`.
    Property(DeclId, Option<DeclId>),
    /// A static function.
    Static(DeclId),
}

impl Translator<'_> {
    // Declaring.

    /// Declares the classes and interfaces among a module's top levels,
    /// so any module's code may name them.
    pub(super) fn declare_classes(&mut self, module: DeclId, top_levels: &[FNode]) -> Tx<()> {
        for &node in top_levels {
            if self.kind(node) != NodeKind::Class {
                continue;
            }
            if !core::type_params(&self.s.library.tree, node).is_empty() {
                return self.not_yet(node, "a generic class");
            }
            let interface = self.data(node) & CLASS_INTERFACE != 0;
            let kind = if interface {
                DeclKind::Interface
            } else {
                DeclKind::Class
            };
            let name_node = self.children(node)[0];
            let name = self.name_of(name_node).to_string();
            let decl = self.b.declare_type(kind, &name, module);
            let exported = (self.s.scopes.resolved(name_node))
                .is_some_and(|s| self.s.scopes.symbol(s).exported);
            if exported {
                self.b.add_facts(decl, Facts::EXPORTED);
            }
            self.decl_spans(decl, node);
            self.classes.insert(node, decl);
            self.class_order.push(node);
        }
        Ok(())
    }

    /// The class or interface a symbol names, following imports: a `let C =
    /// C'` an import makes.
    pub(super) fn class_node(&self, symbol: SymbolId) -> Option<FNode> {
        let mut symbol = symbol;
        for _ in 0..8 {
            let decl = self.s.scopes.symbol(symbol).decl?;
            match self.kind(decl) {
                NodeKind::Class => return Some(decl),
                NodeKind::Let if self.data(decl) & ROLE_MASK == 0 => {
                    let init = core::let_parts(&self.s.library.tree, decl).2?;
                    if self.kind(init) != NodeKind::RightName {
                        return None;
                    }
                    symbol = self.s.scopes.resolved(init)?;
                }
                _ => return None,
            }
        }
        None
    }

    /// A class's or interface's type, by its node: the library's own, or
    /// another library's.
    pub(super) fn class_type(&mut self, class: FNode, at: FNode) -> Tx<TypeId> {
        if let Some(&d) = self.classes.get(&class) {
            return Ok(self.b.nominal(d));
        }
        let name = self.children(class)[0];
        let Some(symbol) = self.s.scopes.resolved(name) else {
            return self.not_yet(at, "a class or interface type");
        };
        match self.external_type(symbol, at)? {
            Some(d) => Ok(self.b.nominal(d)),
            None => self.not_yet(at, "a class or interface type"),
        }
    }

    /// Declares a class's members, and sets its supertypes.
    pub(super) fn declare_members(&mut self, class: FNode) -> Tx<()> {
        let decl = self.classes[&class];
        let interface = self.data(class) & CLASS_INTERFACE != 0;
        let (_, supers, members) = core::class_parts(&self.s.library.tree, class);
        let (supers, members) = (supers.to_vec(), members.to_vec());
        let mut types = Vec::new();
        for sup in supers {
            let name = match self.kind(sup) {
                NodeKind::RightName => sup,
                _ => return self.not_yet(sup, "a generic supertype"),
            };
            let node = self
                .s
                .scopes
                .resolved(name)
                .and_then(|s| self.class_node(s));
            let Some(node) = node else {
                return self.not_yet(sup, "a builtin supertype");
            };
            types.push(self.class_type(node, sup)?);
        }
        self.b.set_supers(decl, &types);
        let class_ty = self.b.nominal(decl);
        let mut ctor_params = Vec::new();
        let mut explicit = None;
        for m in members {
            if self.kind(m) != NodeKind::Let {
                return self.not_yet(m, "a class member of another kind");
            }
            let data = self.data(m);
            let role = data & ROLE_MASK;
            let is_static = data & MEMBER_STATIC != 0;
            if data & MEMBER_CONNECTED != 0 {
                return self.not_yet(m, "a connected member");
            }
            if role == ROLE_TYPE_PARAM {
                return self.not_yet(m, "a generic class");
            }
            let name_node = self.children(m)[0];
            let name = self.name_of(name_node).to_string();
            let mut facts = Facts::NONE;
            if data & MEMBER_PRIVATE != 0 {
                facts = facts | Facts::PRIVATE;
            }
            if data & MEMBER_PROTECTED != 0 {
                facts = facts | Facts::PROTECTED;
            }
            let member = match (role, is_static) {
                (0 | ROLE_PARAM, false) => {
                    let ty = self.member_ty(m)?;
                    if role == ROLE_PARAM {
                        if core::let_parts(&self.s.library.tree, m).2.is_some() {
                            return self.not_yet(m, "a parameter's default");
                        }
                        ctor_params.push(m);
                    }
                    if interface {
                        let sig = self.b.function_type(&[], ty, false);
                        let get = self.b.declare(DeclKind::Getter, &name, decl, sig);
                        self.b.add_facts(get, facts);
                        self.decl_spans(get, m);
                        let set = if data & LET_VAR != 0 {
                            let sig = self.b.function_type(&[ty], TypeId::VOID, false);
                            let set = self.b.declare(DeclKind::Setter, &name, decl, sig);
                            self.b.add_facts(set, facts);
                            self.decl_spans(set, m);
                            self.b.set_param_names(set, &["value"]);
                            Some(set)
                        } else {
                            None
                        };
                        Member::Property(get, set)
                    } else {
                        let d = self.b.declare(DeclKind::Field, &name, decl, ty);
                        if data & LET_VAR != 0 {
                            facts = facts | Facts::VAR;
                        }
                        self.b.add_facts(d, facts);
                        self.decl_spans(d, m);
                        Member::Field(d)
                    }
                }
                (0, true) => {
                    let ty = self.member_ty(m)?;
                    let d = self.b.declare(DeclKind::StaticField, &name, decl, ty);
                    if data & LET_VAR != 0 {
                        facts = facts | Facts::VAR;
                    }
                    self.b.add_facts(d, facts);
                    self.decl_spans(d, m);
                    Member::Field(d)
                }
                (ROLE_CONSTRUCTOR, false) => {
                    explicit = Some(m);
                    continue;
                }
                (ROLE_METHOD | ROLE_GETTER | ROLE_SETTER, _) => {
                    let Some(f) = core::let_parts(&self.s.library.tree, m).2 else {
                        return self.not_yet(m, "a member of another kind");
                    };
                    if !core::type_params(&self.s.library.tree, f).is_empty() {
                        return self.not_yet(m, "a generic method");
                    }
                    let sig = if role == ROLE_SETTER {
                        self.setter_signature(m, f)?
                    } else {
                        self.signature(f)?
                    };
                    let kind = match (role, is_static) {
                        (_, true) if role != ROLE_METHOD => {
                            return self.not_yet(m, "a static getter or setter");
                        }
                        (_, true) => DeclKind::Function,
                        (ROLE_GETTER, _) => DeclKind::Getter,
                        (ROLE_SETTER, _) => DeclKind::Setter,
                        _ => DeclKind::Method,
                    };
                    let d = self.b.declare(kind, &name, decl, sig);
                    if data & MEMBER_OPERATOR_EQ != 0 {
                        facts = facts | Facts::OPERATOR_EQ;
                    }
                    if data & MEMBER_OPERATOR_CMP != 0 {
                        facts = facts | Facts::OPERATOR_CMP;
                    }
                    // The `eq` `@auto("==")` makes has no name of its own
                    // in the source.
                    let text = self
                        .s
                        .library
                        .source_text(self.s.library.tree.span(name_node));
                    if data & MEMBER_OPERATOR_EQ != 0 && text != name {
                        facts = facts | Facts::AUTO;
                    }
                    self.b.add_facts(d, facts);
                    self.decl_spans(d, m);
                    if is_static {
                        Member::Static(d)
                    } else {
                        Member::Method(d)
                    }
                }
                _ => return self.not_yet(m, "a class member of another kind"),
            };
            self.members.insert(m, member);
        }
        if !interface {
            // The constructor: the class's own, or one taking the header's
            // fields.
            let params = match explicit {
                Some(c) => {
                    let Some(f) = core::let_parts(&self.s.library.tree, c).2 else {
                        return self.not_yet(c, "a constructor's form");
                    };
                    core::fn_parts(&self.s.library.tree, f).0
                }
                None => ctor_params,
            };
            let mut types = Vec::new();
            let mut names = Vec::new();
            for &p in &params {
                if core::let_parts(&self.s.library.tree, p).2.is_some() {
                    return self.not_yet(p, "a parameter's default");
                }
                types.push(match self.members.get(&p) {
                    Some(&Member::Field(f)) => self.b.program().decls.get(f).ty,
                    _ => self.decl_ty(p)?,
                });
                let name = core::let_parts(&self.s.library.tree, p).0;
                names.push(self.name_of(name).to_string());
            }
            let sig = self.b.function_type(&types, class_ty, false);
            let ctor = self
                .b
                .declare(DeclKind::Constructor, "constructor", decl, sig);
            if let Some(c) = explicit {
                let data = self.data(c);
                if data & MEMBER_PRIVATE != 0 {
                    self.b.add_facts(ctor, Facts::PRIVATE);
                }
                if data & MEMBER_PROTECTED != 0 {
                    self.b.add_facts(ctor, Facts::PROTECTED);
                }
                self.decl_spans(ctor, c);
            } else {
                self.decl_spans(ctor, class);
            }
            self.ctors.insert(decl, (ctor, explicit, params));
        }
        Ok(())
    }

    /// A setter's signature, `fn (T): Void`, its parameter typed as
    /// written, or as its property's getter or field.
    fn setter_signature(&mut self, m: FNode, f: FNode) -> Tx<TypeId> {
        let params = core::fn_parts(&self.s.library.tree, f).0;
        let [param] = params[..] else {
            return self.not_yet(m, "a setter's form");
        };
        let t = match self.s.typing.decls.get(&param).copied() {
            Some(t) if t != ft::UNKNOWN => self.ty(t, param)?,
            _ => {
                let other = self.accessor_named(m, false);
                match other.and_then(|o| self.s.typing.decls.get(&o).copied()) {
                    Some(t) => self.ty(t, m)?,
                    None => return self.not_yet(m, "an expression of unknown type"),
                }
            }
        };
        Ok(self.b.function_type(&[t], TypeId::VOID, false))
    }

    /// A field's or property's type.
    fn member_ty(&mut self, m: FNode) -> Tx<TypeId> {
        match self.s.typing.decls.get(&m).copied() {
            Some(t) => self.ty(t, m),
            None => self.not_yet(m, "an expression of unknown type"),
        }
    }

    // Translating.

    /// A class or interface as a module item, with its static fields'
    /// initializers added to `init`.
    pub(super) fn class_item(&mut self, class: FNode, init: &mut Vec<NodeId>) -> Tx<NodeId> {
        let decl = self.classes[&class];
        let members = core::class_parts(&self.s.library.tree, class).2.to_vec();
        let outer = self.this_class.replace((class, decl));
        let mut nodes = Vec::new();
        let mut ctor_done = false;
        for m in members {
            let member = self.members.get(&m).copied();
            match member {
                None => {
                    // The constructor, at its place, or where the header's
                    // fields are.
                    if !ctor_done && self.data(m) & ROLE_MASK == ROLE_CONSTRUCTOR {
                        nodes.push(self.constructor(class, decl)?);
                        ctor_done = true;
                    }
                }
                Some(Member::Field(d)) => {
                    if self.b.program().decls.get(d).kind == DeclKind::StaticField {
                        let node = self.b.static_field(d);
                        nodes.push(self.own(node, m));
                        if let Some(value) = core::let_parts(&self.s.library.tree, m).2 {
                            let ty = self.b.program().decls.get(d).ty;
                            let owner = std::mem::replace(&mut self.owner, self.module);
                            let value = self.expr(value);
                            self.owner = owner;
                            let value = self.fit(value?, ty);
                            let stmt = self.b.init_static(d, value);
                            init.push(self.own(stmt, m));
                        }
                    } else {
                        let node = self.b.field(d);
                        nodes.push(self.own(node, m));
                    }
                }
                Some(Member::Property(get, set)) => {
                    let node = self.b.abstract_method(get, &[]);
                    nodes.push(self.own(node, m));
                    if let Some(set) = set {
                        let ty = self.b.program().decls.get(get).ty;
                        let ty = self.b.program().types.signature(ty).unwrap().result;
                        let value = self.b.declare(DeclKind::Param, "value", set, ty);
                        self.decl_spans(value, m);
                        let param = self.b.param(value);
                        let param = self.own(param, m);
                        let node = self.b.abstract_method(set, &[param]);
                        nodes.push(self.own(node, m));
                    }
                }
                Some(Member::Method(d) | Member::Static(d)) => {
                    let f = core::let_parts(&self.s.library.tree, m).2.unwrap();
                    let is_static = matches!(member, Some(Member::Static(_)));
                    if is_static {
                        self.this_class = None;
                    }
                    let node = match core::fn_parts(&self.s.library.tree, f).2 {
                        Some(_) => {
                            let (params, body) = self.function_parts(f, d)?;
                            if is_static {
                                self.b.function(d, &params, body)
                            } else {
                                self.b.method(d, &params, body)
                            }
                        }
                        None => {
                            let params = self.bodiless_params(f, d)?;
                            self.b.abstract_method(d, &params)
                        }
                    };
                    self.this_class = Some((class, decl));
                    nodes.push(self.own(node, m));
                }
            }
        }
        if !ctor_done && self.ctors.contains_key(&decl) {
            // Kotlin puts the made constructor after the fields; so do we.
            let ctor = self.constructor(class, decl)?;
            let at = nodes
                .iter()
                .rposition(|&n| {
                    matches!(
                        self.b.program().nodes.get(n),
                        temper_tmpl::Node::Field { .. }
                    )
                })
                .map_or(0, |i| i + 1);
            nodes.insert(at, ctor);
        }
        self.this_class = outer;
        let item = self.b.class(decl, &nodes);
        Ok(self.own(item, class))
    }

    /// A bodiless method's parameters, declared.
    fn bodiless_params(&mut self, f: FNode, decl: DeclId) -> Tx<Vec<NodeId>> {
        let params = core::fn_parts(&self.s.library.tree, f).0;
        let types: Vec<TypeId> =
            (self.b.program().signature(decl)).map_or_else(Vec::new, |s| s.params.to_vec());
        let mut nodes = Vec::new();
        for (i, p) in params.into_iter().enumerate() {
            let name = core::let_parts(&self.s.library.tree, p).0;
            let name = self.name_of(name).to_string();
            let ty = match types.get(i) {
                Some(&t) => t,
                None => self.decl_ty(p)?,
            };
            let d = self.b.declare(DeclKind::Param, &name, decl, ty);
            self.decl_spans(d, p);
            self.decls.insert(p, d);
            let param = self.b.param(d);
            nodes.push(self.own(param, p));
        }
        Ok(nodes)
    }

    /// The class's constructor: its parameters, then its header's fields
    /// set from them if it has no constructor of its own, then the
    /// other fields' initializers, in order, then its own constructor's
    /// body, as the interpreter runs them.
    fn constructor(&mut self, class: FNode, decl: DeclId) -> Tx<NodeId> {
        let (ctor, explicit, params) = self.ctors[&decl].clone();
        let class_ty = self.b.nominal(decl);
        let (owner, result) = (
            std::mem::replace(&mut self.owner, ctor),
            std::mem::replace(&mut self.result, TypeId::VOID),
        );
        let types: Vec<TypeId> = self.b.program().signature(ctor).unwrap().params.to_vec();
        let mut param_nodes = Vec::new();
        let mut param_decls = Vec::new();
        for (i, &p) in params.iter().enumerate() {
            let name = core::let_parts(&self.s.library.tree, p).0;
            let name = self.name_of(name).to_string();
            let d = self.b.declare(DeclKind::Param, &name, ctor, types[i]);
            if explicit.is_some() && self.data(p) & LET_VAR != 0 {
                self.b.add_facts(d, Facts::VAR);
            }
            self.decl_spans(d, p);
            if explicit.is_some() {
                self.decls.insert(p, d);
            }
            let param = self.b.param(d);
            param_nodes.push(self.own(param, p));
            param_decls.push(d);
        }
        let mut stmts = Vec::new();
        let members = core::class_parts(&self.s.library.tree, class).2.to_vec();
        for m in members {
            let data = self.data(m);
            if data & MEMBER_STATIC != 0 {
                continue;
            }
            let Some(Member::Field(field)) = self.members.get(&m).copied() else {
                continue;
            };
            let value = match data & ROLE_MASK {
                ROLE_PARAM if explicit.is_none() => {
                    let i = params.iter().position(|&p| p == m).unwrap();
                    let read = self.b.local(param_decls[i]);
                    self.made(read, m)
                }
                0 if data & LET_INIT != 0 => {
                    let init = core::let_parts(&self.s.library.tree, m).2.unwrap();
                    let ty = self.b.program().decls.get(field).ty;
                    let value = self.expr(init)?;
                    self.fit(value, ty)
                }
                _ => continue,
            };
            let this = self.b.this(class_ty);
            let this = self.made(this, m);
            let set = self.b.set_field(field, this, value);
            stmts.push(self.made(set, m));
        }
        if let Some(c) = explicit {
            let f = core::let_parts(&self.s.library.tree, c).2.unwrap();
            if let Some(body) = core::fn_parts(&self.s.library.tree, f).2 {
                self.stmts(body, &mut stmts)?;
            }
        }
        let at = explicit.unwrap_or(class);
        let block = self.b.block(&stmts);
        let block = self.made(block, at);
        let node = self.b.method(ctor, &param_nodes, block);
        self.owner = owner;
        self.result = result;
        Ok(self.made(node, at))
    }

    // Conversions.

    /// `value` as `want`: itself if it has that type, else upcast to it if
    /// it's a subtype, or downcast if it's a supertype that typing found
    /// narrowed.
    pub(super) fn fit(&mut self, value: NodeId, want: TypeId) -> NodeId {
        let have = self.b.program().nodes.ty(value);
        if have == want || want.is_none() || have.is_none() {
            return value;
        }
        let span = self.b.program().nodes.span(value);
        let made = if is_subtype(self.b.program(), have, want) {
            self.b.upcast(want, value)
        } else if is_subtype(self.b.program(), want, have) {
            // Typing accepted it, so flow facts (an `is` check) narrowed it.
            self.b.downcast(want, value)
        } else {
            return value;
        };
        self.b.set_span(made, span, true);
        made
    }

    /// Each argument fit to its parameter's type.
    pub(super) fn fit_args(&mut self, callee: DeclId, args: Vec<NodeId>) -> Vec<NodeId> {
        let params: Vec<TypeId> =
            (self.b.program().signature(callee)).map_or_else(Vec::new, |s| s.params.to_vec());
        args.into_iter()
            .enumerate()
            .map(|(i, a)| match params.get(i) {
                Some(&t) => self.fit(a, t),
                None => a,
            })
            .collect()
    }

    /// A local's read, narrowed: downcast where typing says an `is` check
    /// made it a subtype of its declared type.
    pub(super) fn narrowed(&mut self, read: NodeId, node: FNode) -> Tx<NodeId> {
        let Some(t) = self.s.typing.expr(node) else {
            return Ok(read);
        };
        if !matches!(self.s.typing.types.get(t), FTypeData::Class(_)) {
            return Ok(read);
        }
        let want = self.ty(t, node)?;
        let have = self.b.program().nodes.ty(read);
        if want != have && is_subtype(self.b.program(), want, have) {
            let made = self.b.downcast(want, read);
            return Ok(self.made(made, node));
        }
        Ok(read)
    }

    // Members in bodies.

    /// `this`, as the class whose member is being translated.
    pub(super) fn this(&mut self, at: FNode) -> Tx<NodeId> {
        let Some((_, decl)) = self.this_class else {
            return self.not_yet(at, "this outside a class");
        };
        let ty = self.b.nominal(decl);
        let made = self.b.this(ty);
        Ok(self.own(made, at))
    }

    /// Whether a name is the synthetic `this` of a class.
    pub(super) fn is_this(&self, node: FNode) -> bool {
        let Some(symbol) = self.s.scopes.resolved(node) else {
            return false;
        };
        let s = self.s.scopes.symbol(symbol);
        s.kind == SymbolKind::Synthetic
            && self.s.library.names.get(s.name).starts_with("this")
            && self.s.scopes.get(s.owner).names.first().map(|&(_, b)| b) == Some(symbol)
            && self.kind(self.s.scopes.get(s.owner).node) == NodeKind::Class
    }

    /// The member a class's `Let` is in tmpl, declaring another
    /// library's on first use.
    pub(super) fn member(&mut self, m: FNode, at: FNode) -> Tx<Member> {
        match self.members.get(&m) {
            Some(&member) => Ok(member),
            None => self.not_yet(at, "another library's class member"),
        }
    }

    /// The class (or interface) node that declares a member `Let`.
    fn owner_class(&self, m: FNode) -> Option<FNode> {
        let name = self.children(m).first().copied()?;
        let symbol = self.s.scopes.resolved(name)?;
        let scope = self.s.scopes.symbol(symbol).owner;
        let node = self.s.scopes.get(scope).node;
        (self.kind(node) == NodeKind::Class).then_some(node)
    }

    /// Another member named like `m` of `m`'s class, for reading (a field
    /// or getter) or writing (a field or setter).
    fn accessor_named(&self, m: FNode, write: bool) -> Option<FNode> {
        let found = self.accessor(m, write);
        (found != m).then_some(found)
    }

    /// A member named like `m` of `m`'s class for reading (a field or
    /// getter) or writing (a field or setter), preferring `m` itself.
    fn accessor(&self, m: FNode, write: bool) -> FNode {
        let role = |n: FNode| self.data(n) & ROLE_MASK;
        let fits = |n: FNode| match role(n) {
            0 | ROLE_PARAM => true,
            ROLE_GETTER => !write,
            ROLE_SETTER => write,
            _ => false,
        };
        if fits(m) {
            return m;
        }
        let Some(class) = self.owner_class(m) else {
            return m;
        };
        let name = self.s.library.name(self.children(m)[0]);
        let members = core::class_parts(&self.s.library.tree, class).2;
        (members.iter().copied())
            .find(|&o| self.s.library.name(self.children(o)[0]) == name && fits(o))
            .unwrap_or(m)
    }

    /// Reads a member of `object` (or of `this`, if `None`): a field, a
    /// getter's value, or a static field.
    pub(super) fn read_member(&mut self, m: FNode, object: Option<FNode>, at: FNode) -> Tx<NodeId> {
        let m = self.accessor(m, false);
        let member = self.member(m, at)?;
        let made = match member {
            Member::Field(d) if self.b.program().decls.get(d).kind == DeclKind::StaticField => {
                self.b.get_static(d)
            }
            Member::Field(d) => {
                let object = self.receiver(object, d, at)?;
                self.b.get_field(d, object)
            }
            Member::Method(d) if self.b.program().decls.get(d).kind == DeclKind::Getter => {
                let object = self.receiver(object, d, at)?;
                self.b.call_method(d, object, &[])
            }
            Member::Property(get, _) => {
                let object = self.receiver(object, get, at)?;
                self.b.call_method(get, object, &[])
            }
            _ => return self.not_yet(at, "a method as a value"),
        };
        Ok(self.own(made, at))
    }

    /// The receiver for a member: `object`, or `this`, fit to the
    /// member's owner's type.
    fn receiver(&mut self, object: Option<FNode>, member: DeclId, at: FNode) -> Tx<NodeId> {
        let value = match object {
            Some(o) => self.expr(o)?,
            None => self.this(at)?,
        };
        let owner = self.b.program().decls.get(member).owner;
        let ty = self.b.program().decls.get(owner).ty;
        Ok(self.fit(value, ty))
    }

    /// Calls a method, or a static function, of `object` (or `this`).
    pub(super) fn call_member(
        &mut self,
        m: FNode,
        object: Option<FNode>,
        args: &[FNode],
        at: FNode,
    ) -> Tx<NodeId> {
        let member = self.member(m, at)?;
        let callee = match member {
            Member::Method(d) | Member::Static(d) => d,
            _ => return self.not_yet(at, "a call of a field's value"),
        };
        let count = (self.b.program().signature(callee)).map_or(0, |s| s.params.len());
        if count != args.len() {
            return self.not_yet(at, "an argument left out");
        }
        let mut values = Vec::new();
        let made = match member {
            Member::Method(d) => {
                let receiver = self.receiver(object, d, at)?;
                for &a in args {
                    values.push(self.expr(a)?);
                }
                let values = self.fit_args(d, values);
                self.b.call_method(d, receiver, &values)
            }
            Member::Static(d) => {
                for &a in args {
                    values.push(self.expr(a)?);
                }
                let values = self.fit_args(d, values);
                self.b.call(d, &values)
            }
            _ => return self.not_yet(at, "a call of a field's value"),
        };
        Ok(self.own(made, at))
    }

    /// Assigns a member of `object` (or `this`): a field, or a setter's
    /// call.
    pub(super) fn assign_member(
        &mut self,
        m: FNode,
        object: Option<FNode>,
        value: FNode,
        at: FNode,
    ) -> Tx<NodeId> {
        let m = self.accessor(m, true);
        let member = self.member(m, at)?;
        match member {
            Member::Field(d) if self.b.program().decls.get(d).kind == DeclKind::Field => {
                let object = self.receiver(object, d, at)?;
                let ty = self.b.program().decls.get(d).ty;
                let value = self.expr(value)?;
                let value = self.fit(value, ty);
                Ok(self.b.set_field(d, object, value))
            }
            Member::Method(d) if self.b.program().decls.get(d).kind == DeclKind::Setter => {
                let object = self.receiver(object, d, at)?;
                let value = self.expr(value)?;
                let value = self.fit_args(d, vec![value]);
                let call = self.b.call_method(d, object, &value);
                let call = self.own(call, at);
                Ok(self.b.expr_statement(call))
            }
            Member::Property(_, Some(set)) => {
                let object = self.receiver(object, set, at)?;
                let value = self.expr(value)?;
                let value = self.fit_args(set, vec![value]);
                let call = self.b.call_method(set, object, &value);
                let call = self.own(call, at);
                Ok(self.b.expr_statement(call))
            }
            _ => self.not_yet(at, "an assignment to a static field"),
        }
    }

    /// `new C(args)`.
    pub(super) fn new_object(&mut self, class: FNode, args: &[FNode], at: FNode) -> Tx<NodeId> {
        let Some(&decl) = self.classes.get(&class) else {
            return self.not_yet(at, "another library's constructor");
        };
        let Some(&(ctor, ..)) = self.ctors.get(&decl) else {
            return self.not_yet(at, "a constructor");
        };
        let count = self
            .b
            .program()
            .signature(ctor)
            .map_or(0, |s| s.params.len());
        if args.len() != count {
            return self.not_yet(at, "an argument left out");
        }
        let mut values = Vec::new();
        for &a in args {
            if self.s.library.value(a) == Some(&temper_syntax::ast::Value::Null) {
                return self.not_yet(a, "a parameter's default");
            }
            values.push(self.expr(a)?);
        }
        let values = self.fit_args(ctor, values);
        let made = self.b.new_object(ctor, &values);
        Ok(self.own(made, at))
    }

    /// `a == b` (or `!=`, or a comparison) by a class's `@operator`
    /// method.
    pub(super) fn operator_call(
        &mut self,
        node: FNode,
        f: Function,
        method: FNode,
        args: &[FNode],
    ) -> Tx<NodeId> {
        let [left, right] = *args else {
            return self.not_yet(node, "an operator's arguments");
        };
        let member = self.member(method, node)?;
        let Member::Method(d) = member else {
            return self.not_yet(node, "an operator's method");
        };
        let receiver = self.receiver(Some(left), d, node)?;
        let arg = self.expr(right)?;
        let arg = self.fit_args(d, vec![arg]);
        let call = self.b.call_method(d, receiver, &arg);
        let call = self.own(call, node);
        let op = match f {
            Function::Eq | Function::Cmp => return Ok(call),
            Function::Ne => {
                let not = Op::named("Boolean.not").unwrap();
                let made = self.b.op(not, &[call]);
                return Ok(self.made(made, node));
            }
            Function::Lt => "Int32.lt",
            Function::Le => "Int32.le",
            Function::Gt => "Int32.gt",
            Function::Ge => "Int32.ge",
            _ => return self.not_yet(node, "an operator by a method"),
        };
        let zero = self.b.int32(0);
        let zero = self.made(zero, node);
        let made = self.b.op(Op::named(op).unwrap(), &[call, zero]);
        Ok(self.made(made, node))
    }

    /// `a == b` or `a != b` comparing objects that have no `==` method:
    /// identity.
    pub(super) fn identity(&mut self, node: FNode, f: Function, args: &[FNode]) -> Tx<NodeId> {
        let [left, right] = *args else {
            return self.not_yet(node, "an operator's arguments");
        };
        let nominal = |t: Option<ft::TypeId>, s: &Self| {
            t.is_some_and(|t| matches!(s.s.typing.types.get(t), FTypeData::Class(_)))
        };
        if !nominal(self.s.typing.expr(left), self) || !nominal(self.s.typing.expr(right), self) {
            return self.not_yet(node, "== by identity on other types");
        }
        let (a, b) = (self.expr(left)?, self.expr(right)?);
        let made = self.b.identical(a, b);
        let made = self.own(made, node);
        if f == Function::Ne {
            let not = Op::named("Boolean.not").unwrap();
            let made = self.b.op(not, &[made]);
            return Ok(self.made(made, node));
        }
        Ok(made)
    }

    /// `x is C`.
    pub(super) fn is_check(&mut self, node: FNode, args: &[FNode]) -> Tx<NodeId> {
        let [value, ty] = *args else {
            return self.not_yet(node, "an is check's form");
        };
        let class = self.s.scopes.resolved(ty).and_then(|s| self.class_node(s));
        let Some(class) = class else {
            return self.not_yet(node, "an is check of another type");
        };
        let ty = self.class_type(class, node)?;
        let value = self.expr(value)?;
        let made = self.b.is(ty, value);
        Ok(self.own(made, node))
    }

    /// `x as C`, checked, bubbling if it fails.
    pub(super) fn as_cast(&mut self, node: FNode, args: &[FNode]) -> Tx<NodeId> {
        let [value, _] = *args else {
            return self.not_yet(node, "an as cast's form");
        };
        let ty = self.expr_ty(node)?;
        if temper_tmpl::analysis::nominal_decl(self.b.program(), ty).is_none() {
            return self.not_yet(node, "an as cast to another type");
        }
        let value = self.expr(value)?;
        let made = self.b.as_cast(ty, value);
        Ok(self.own(made, node))
    }

    /// Whether a `Let` is a class member.
    pub(super) fn is_member(&self, decl: FNode) -> bool {
        self.kind(decl) == NodeKind::Let && self.owner_class(decl).is_some()
    }
}

impl Translator<'_> {
    /// The class whose member a `Let` is.
    pub(super) fn member_class(&self, m: FNode) -> Option<FNode> {
        self.owner_class(m)
    }

    /// The object of `object.member`, or `None` where it names a class,
    /// for a static member.
    pub(super) fn static_object(&self, object: FNode) -> Option<FNode> {
        let names_class = self.kind(object) == NodeKind::RightName
            && (self.s.scopes.resolved(object)).is_some_and(|s| self.class_node(s).is_some())
            && self
                .s
                .typing
                .expr(object)
                .is_none_or(|t| !matches!(self.s.typing.types.get(t), FTypeData::Class(_)));
        (!names_class).then_some(object)
    }

    /// The object and member `Let` of an assignment's target, `object.x`:
    /// what the object's class (or a class named, for a static) has
    /// named `x`, own or inherited.
    pub(super) fn assigned_member(&self, target: FNode) -> Option<(FNode, FNode)> {
        let [dot, object, key] = *self.children(target) else {
            return None;
        };
        if self.function_of(dot) != Some(Function::Dot) {
            return None;
        }
        let name = self.s.library.symbol(key)?;
        let class = match self
            .s
            .typing
            .expr(object)
            .map(|t| self.s.typing.types.get(t))
        {
            Some(&FTypeData::Class(symbol)) => self.class_node(symbol)?,
            _ => self.class_node(self.s.scopes.resolved(object)?)?,
        };
        let scope = self.s.scopes.of_node(class)?;
        let found = (self.s.scopes.get(scope).names.iter())
            .filter(|&&(n, _)| n == name)
            .filter_map(|&(_, b)| self.s.scopes.symbol(b).decl)
            .find(|&m| self.kind(m) == NodeKind::Let)?;
        Some((object, found))
    }
}
