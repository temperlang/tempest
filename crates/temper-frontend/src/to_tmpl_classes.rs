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
    self, CLASS_CONNECTED, CLASS_INTERFACE, FN_NO_BODY, LET_INIT, LET_VAR, MEMBER_CONNECTED,
    MEMBER_OPERATOR_CMP, MEMBER_OPERATOR_EQ, MEMBER_PRIVATE, MEMBER_PROTECTED, MEMBER_STATIC,
    ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK, ROLE_METHOD, ROLE_PARAM, ROLE_SETTER,
    ROLE_TYPE_PARAM,
};
use crate::scope::{SymbolId, SymbolKind};
use crate::types::{self as ft, TypeData as FTypeData};
use temper_syntax::tree::{NodeId as FNode, NodeKind};
use temper_tmpl::analysis::is_subtype;
use temper_tmpl::ops::Op;
use temper_tmpl::{DeclId, DeclKind, Facts, Node, NodeId, TypeId};

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
            if self.data(node) & crate::core::CLASS_FUN != 0 {
                self.b.add_facts(decl, Facts::FUN);
            }
            self.decl_spans(decl, node);
            self.classes.insert(node, decl);
            self.class_order.push(node);
        }
        // Type parameters once every class is declared, as their bounds may
        // name any of them.
        for &node in top_levels {
            let Some(&decl) = self.classes.get(&node) else {
                continue;
            };
            let name_node = self.children(node)[0];
            let formals = (self.s.scopes.resolved(name_node))
                .and_then(|s| self.s.typing.types.class_info(s))
                .map(|info| info.formals.clone())
                .unwrap_or_default();
            if !formals.is_empty() {
                self.declare_formals(&formals, decl, node)?;
            }
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
        let span = self.s.library.tree.span(class);
        if self.data(class) & CLASS_CONNECTED != 0 && !self.s.library.in_std(span.start) {
            // A user's `@connected` class with its Temper implementation,
            // which backends without host code for it translate (one
            // without, all bodiless methods, is "not yet", below):
            // connected as a function is (`declare_top_level`).
            match (self.s.library.connected.get(&class)).and_then(|c| c.backends.as_ref()) {
                None => self.b.add_facts(decl, Facts::CONNECTED),
                Some(listed) if listed.is_empty() => {}
                Some(listed) => {
                    let listed: Vec<&str> = listed.iter().map(String::as_str).collect();
                    self.b.set_connected(decl, &listed);
                }
            }
        }
        let (_, supers, members) = core::class_parts(&self.s.library.tree, class);
        let (supers, members) = (supers.to_vec(), members.to_vec());
        let mut types = Vec::new();
        let written = (self.s.scopes.resolved(self.children(class)[0]))
            .and_then(|s| self.s.typing.types.class_info(s))
            .map(|info| info.supers.clone())
            .unwrap_or_default();
        for (i, sup) in supers.into_iter().enumerate() {
            let name = match self.kind(sup) {
                NodeKind::RightName => sup,
                _ => match written.get(i) {
                    // `extends Sup<T>`, as typing read it.
                    Some(&t) if self.s.typing.types.is_known(t) => {
                        types.push(self.ty(t, sup)?);
                        continue;
                    }
                    _ => return self.not_yet(sup, "a generic supertype"),
                },
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
        let class_ty = self.b.program().decls.get(decl).ty;
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
                // Declared with the class (`declare_classes`).
                continue;
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
                    // Only a connected class's methods (#633) have no body.
                    if !interface && self.data(f) & FN_NO_BODY != 0 {
                        return self.not_yet(m, "a connected class");
                    }
                    let generic = !core::type_params(&self.s.library.tree, f).is_empty();
                    let sig = if role == ROLE_SETTER || generic {
                        TypeId::NONE
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
                    if generic {
                        self.declare_type_params(f, d)?;
                    }
                    if role == ROLE_SETTER {
                        let sig = self.setter_signature(m, f)?;
                        self.b.set_decl_type(d, sig);
                    } else if generic {
                        let sig = self.signature(f)?;
                        self.b.set_decl_type(d, sig);
                    }
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
            let mut optional = Vec::new();
            let mut names = Vec::new();
            for &p in &params {
                let ty = match self.members.get(&p) {
                    Some(&Member::Field(f)) => self.b.program().decls.get(f).ty,
                    _ => self.decl_ty(p)?,
                };
                // Null means a parameter's default, so callers may pass it.
                let has_default = self.param_default(p).is_some();
                types.push(if has_default { self.b.nullable(ty) } else { ty });
                optional.push(has_default);
                let name = core::let_parts(&self.s.library.tree, p).0;
                names.push(self.name_of(name).to_string());
            }
            let sig = self
                .b
                .function_type_with(&types, &optional, class_ty, false);
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
            if self.param_default(p).is_some() {
                self.b.add_facts(d, Facts::OPTIONAL);
            }
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
        let class_ty = self.b.program().decls.get(decl).ty;
        let (owner, result) = (
            std::mem::replace(&mut self.owner, ctor),
            std::mem::replace(&mut self.result, TypeId::VOID),
        );
        let types: Vec<TypeId> = self.b.program().signature(ctor).unwrap().params.to_vec();
        let mut param_nodes = Vec::new();
        let mut param_decls = Vec::new();
        let mut defaulted = Vec::new();
        for (i, &p) in params.iter().enumerate() {
            let name = core::let_parts(&self.s.library.tree, p).0;
            let name = self.name_of(name).to_string();
            let d = self.b.declare(DeclKind::Param, &name, ctor, types[i]);
            let has_default = self.param_default(p).is_some();
            if explicit.is_some() && self.data(p) & LET_VAR != 0 {
                if has_default {
                    return self.not_yet(p, "a var parameter's default");
                }
                self.b.add_facts(d, Facts::VAR);
            }
            if has_default {
                self.b.add_facts(d, Facts::OPTIONAL);
                let inner = match self.members.get(&p) {
                    Some(&Member::Field(f)) if explicit.is_none() => {
                        self.b.program().decls.get(f).ty
                    }
                    _ => self.default_ty(p)?,
                };
                defaulted.push((p, d, inner));
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
        // A header's field's `Let` is the field, not its parameter, so its
        // default's local is found by position.
        self.defaults(&defaulted, &mut stmts)?;
        let mut locals = param_decls.clone();
        for (i, &p) in params.iter().enumerate() {
            if self.param_default(p).is_some() {
                locals[i] = self.decls[&p];
                if explicit.is_none() {
                    self.decls.remove(&p);
                }
            }
        }
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
                    let read = self.b.local(locals[i]);
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
        if have.is_none() && !want.is_none() {
            // `null`, `bubble()`, or `panic()`, typed by its place.
            self.b.retype(value, want);
            return value;
        }
        if have == want || want.is_none() || have.is_none() {
            return value;
        }
        let span = self.b.program().nodes.span(value);
        let types = &self.b.program().types;
        let (have_null, want_null) = (types.is_nullable(have), types.is_nullable(want));
        if want_null && !have_null {
            // A narrowed read where its nullable type is wanted again.
            if let Node::NonNull { value: read } = self.b.program().nodes.get(value) {
                if self.b.program().nodes.ty(read) == want {
                    return read;
                }
            }
            let types = &self.b.program().types;
            let inner = types.non_null(want);
            let value = self.fit(value, inner);
            if self.b.program().nodes.ty(value) != inner {
                return value;
            }
            let made = self.b.as_nullable(value);
            self.b.set_span(made, span, true);
            return made;
        }
        if have_null && !want_null {
            // Typing accepted it, so flow facts (a null check) narrowed it.
            let made = self.b.non_null(value);
            self.b.set_span(made, span, true);
            return self.fit(made, want);
        }
        // A function value where another `@fun` interface or function type
        // with the same signature is wanted: a literal or reference takes
        // the type wanted, as typing gives it; else a conversion.
        let (have_sig, want_sig) = (self.b.fun_signature(have), self.b.fun_signature(want));
        if have_sig.is_some() && have_sig == want_sig {
            let kind = self.b.program().nodes.kind(value);
            if matches!(
                kind,
                temper_tmpl::Kind::FunctionLiteral | temper_tmpl::Kind::FunctionRef
            ) {
                self.b.retype(value, want);
                return value;
            }
            let fun = |p: &temper_tmpl::Program, t: TypeId| {
                temper_tmpl::analysis::nominal_decl(p, t)
                    .is_some_and(|d| p.decls.get(d).facts.contains(Facts::FUN))
            };
            let p = self.b.program();
            let made = match (fun(p, have), fun(p, want)) {
                (false, true) => self.b.to_fun(want, value),
                (true, false) => self.b.from_fun(want, value),
                (true, true) => {
                    let plain = have_sig.unwrap();
                    let inner = self.b.from_fun(plain, value);
                    self.b.set_span(inner, span, true);
                    self.b.to_fun(want, inner)
                }
                (false, false) => return value,
            };
            self.b.set_span(made, span, true);
            return made;
        }
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
        self.fit_args_with(callee, None, &[], args)
    }

    /// Each argument fit to its parameter's type, with the type parameters
    /// the call binds put in: its type arguments, and the receiver's.
    pub(super) fn fit_args_with(
        &mut self,
        callee: DeclId,
        receiver: Option<NodeId>,
        type_args: &[TypeId],
        args: Vec<NodeId>,
    ) -> Vec<NodeId> {
        let params: Vec<TypeId> =
            (self.b.program().signature(callee)).map_or_else(Vec::new, |s| s.params.to_vec());
        let receiver = receiver.map(|r| self.b.program().nodes.ty(r));
        let (from, to) =
            temper_tmpl::analysis::bindings(self.b.program(), callee, receiver, type_args);
        let params: Vec<TypeId> = (params.into_iter())
            .map(|p| self.b.program_mut().types.substitute(p, &from, &to))
            .collect();
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
        let have = self.b.program().nodes.ty(read);
        let read = match self.s.typing.types.get(t) {
            FTypeData::Nullable(_) => return Ok(read),
            _ if self.b.program().types.is_nullable(have) && self.s.typing.types.is_known(t) => {
                // A null check narrowed it.
                let made = self.b.non_null(read);
                self.made(made, node)
            }
            _ => read,
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
        let ty = self.b.program().decls.get(decl).ty;
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
            Member::Static(d) => return self.function_ref(d, at),
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
        // A generic class's or interface's: any subtype, with any type
        // arguments, after a null check's narrowing.
        let generic = !self.b.program().decls.type_params(owner).is_empty();
        if generic {
            let have = self.b.program().nodes.ty(value);
            let value = if self.b.program().types.is_nullable(have) {
                let made = self.b.non_null(value);
                self.made(made, at)
            } else {
                value
            };
            let have = self.b.program().nodes.ty(value);
            if temper_tmpl::analysis::as_super(self.b.program(), have, owner).is_some() {
                return Ok(value);
            }
        }
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
            Member::Field(_) | Member::Property(..) => {
                // A field's or property's function value, called.
                let value = self.read_member(m, object, m)?;
                return self.call_value_of(at, value, args);
            }
        };
        let count = (self.b.program().signature(callee)).map_or(0, |s| s.params.len());
        if count < args.len() {
            return self.not_yet(at, "an argument too many");
        }
        let mut values = Vec::new();
        let made = match member {
            Member::Method(d) => {
                let receiver = self.receiver(object, d, at)?;
                let type_args = self.call_type_args(at, d)?;
                let rest = self.omit_rest_of(d, Some(receiver), &type_args, args.len(), at)?;
                for &a in args {
                    values.push(self.expr(a)?);
                }
                let mut values = self.fit_args_with(d, Some(receiver), &type_args, values);
                values.extend(rest);
                self.b.call_method_generic(d, &type_args, receiver, &values)
            }
            Member::Static(d) => {
                let type_args = self.call_type_args(at, d)?;
                let rest = self.omit_rest_of(d, None, &type_args, args.len(), at)?;
                for &a in args {
                    values.push(self.expr(a)?);
                }
                let mut values = self.fit_args_with(d, None, &type_args, values);
                values.extend(rest);
                self.b.call_generic(d, &type_args, &values)
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
                let have = Some(self.b.program().nodes.ty(object));
                let (from, to) = temper_tmpl::analysis::bindings(self.b.program(), d, have, &[]);
                let ty = self.b.program_mut().types.substitute(ty, &from, &to);
                let value = self.expr(value)?;
                let value = self.fit(value, ty);
                Ok(self.b.set_field(d, object, value))
            }
            Member::Method(d) if self.b.program().decls.get(d).kind == DeclKind::Setter => {
                let object = self.receiver(object, d, at)?;
                let value = self.expr(value)?;
                let value = self.fit_args_with(d, Some(object), &[], vec![value]);
                let call = self.b.call_method(d, object, &value);
                let call = self.own(call, at);
                Ok(self.b.expr_statement(call))
            }
            Member::Property(_, Some(set)) => {
                let object = self.receiver(object, set, at)?;
                let value = self.expr(value)?;
                let value = self.fit_args_with(set, Some(object), &[], vec![value]);
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
        let type_args = self.call_type_args(at, ctor)?;
        let rest = self.omit_rest_of(ctor, None, &type_args, args.len(), at)?;
        let mut values = Vec::new();
        for &a in args {
            values.push(self.expr(a)?);
        }
        let mut values = self.fit_args_with(ctor, None, &type_args, values);
        values.extend(rest);
        let made = self.b.new_generic(ctor, &type_args, &values);
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
        self.compared(node, f, call)
    }

    /// An operator's value from a call of its `==` or `<=>` method:
    /// `eq`'s result, or `compareTo`'s compared with 0.
    pub(super) fn compared(&mut self, node: FNode, f: Function, call: NodeId) -> Tx<NodeId> {
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
            // `x is T` for `x: T?`, where `T` is builtin: a null check.
            if let Some(crate::builtins::Builtin::Type(i)) = self.s.scopes.resolved_builtin(ty) {
                let name = match crate::builtins::TYPES[i as usize] {
                    "Int" => "Int32",
                    name => name,
                };
                let t = temper_tmpl::TypeTable::scalar_named(name).unwrap_or(TypeId::NONE);
                let value = self.expr(value)?;
                let have = self.b.program().nodes.ty(value);
                if self.b.program().types.is_nullable(have)
                    && self.b.program().types.non_null(have) == t
                {
                    let check = self.b.is_null(value);
                    let check = self.made(check, node);
                    let not = Op::named("Boolean.not").unwrap();
                    let made = self.b.op(not, &[check]);
                    return Ok(self.own(made, node));
                }
                // `x is StringIndex`, one of core's classes.
                let core = self.core_decl(crate::builtins::TYPES[i as usize]);
                let core = core.filter(|&d| {
                    let decl = self.b.program().decls.get(d);
                    decl.kind == DeclKind::Class && self.b.program().decls.type_params(d).is_empty()
                });
                if let Some(d) = core {
                    let ty = self.b.nominal(d);
                    let made = self.b.is(ty, value);
                    return Ok(self.own(made, node));
                }
            }
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
        let value = self.expr(value)?;
        let have = self.b.program().nodes.ty(value);
        // `x as T` for `x: T?` checks that it isn't null.
        let null_check = self.b.program().types.non_null(have) == ty && have != ty;
        if !null_check && temper_tmpl::analysis::nominal_decl(self.b.program(), ty).is_none() {
            return self.not_yet(node, "an as cast to another type");
        }
        // `x as I` for an `x` already an `I`, which can't fail.
        let made = if !null_check && temper_tmpl::analysis::is_subtype(self.b.program(), have, ty) {
            self.b.upcast(ty, value)
        } else {
            self.b.as_cast(ty, value)
        };
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
