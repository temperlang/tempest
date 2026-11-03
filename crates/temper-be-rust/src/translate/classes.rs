//! Classes and interfaces, as Kotlin's Rust backend writes them.
//!
//! A class `Rect` is a cloneable handle, `pub struct Rect(Arc<RectStruct>)`,
//! over its fields, with its constructor `Rect::new`, its methods, getters
//! (`area()`), and setters (`set_doubled(…)`) as methods, a getter for each
//! public field of an exported class (`w()`, and `set_count(…)` for a
//! `var`), its statics as getters (`Rect::unit()`) over `OnceLock`s
//! (`RECT__UNIT`), and its static functions as associated functions. An
//! exported class whose public constructor takes two or more arguments
//! also gets a builder, `builders::RectBuilder { w, h }.build()`. An
//! interface `Shape` is a trait, `ShapeTrait`, with its default methods,
//! and a handle on a trait object, `pub struct Shape(Arc<dyn ShapeTrait>)`,
//! which also implements the trait and the traits above it; `Shape::new`
//! makes one of a class's instance (an upcast). Each class implements the
//! traits of every interface above it, a method another interface
//! overrides (a diamond's) forwarding to the override that Kotlin picks:
//! the first found breadth-first. `temper_core`'s `impl_any_value_trait!`
//! macros give `is` and `as` (`temper_core::is::<Square>(q)`).
//!
//! Where this differs from Kotlin's: a class's plain `toString()` also
//! makes it `Display`, and its `@operator("==")` method `PartialEq`, so
//! `==` compares with it; only `var` fields are locked (`RwLock`), each
//! on its own, and read through a method, so no lock is held across a
//! call (a field read in an expression that also calls something, as `n +
//! peer.bump(this) + n`, can't deadlock); a non-exported class gets no
//! getters for its fields, since its module reads them directly; a
//! method an interface gives a class is called by its trait
//! (`CTrait::a(&d)`), where Kotlin also gives the class a private method
//! that does; the constructor builds its instance in one expression where
//! it can; and what Kotlin always writes but a type that isn't exported
//! may not use (`new`, `clone_boxed`) is `#[allow(dead_code)]`, where
//! Kotlin's output allows every warning.

use super::{Params, R, Translator, Want, not_yet};
use crate::names::static_name;
use crate::tree::{Bin, Block, Expr, Item, Member, Stmt, Unary};
use std::collections::{HashMap, HashSet};
use temper_tmpl::analysis::{ancestors, home, is_plain_to_string, nominal_decl, walk};
use temper_tmpl::naming::snake_case;
use temper_tmpl::qname::{is_external, library_of};
use temper_tmpl::{DeclId, DeclKind, Facts, Kids, Module, Node, NodeId, Program, TypeId};

/// What `this` is in the code being translated.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum This {
    /// Not in a method.
    #[default]
    None,
    /// A class's method: `self`, a `&Class`.
    Class,
    /// An interface's default method: `self`, a `&Self` of its trait.
    Trait(DeclId),
    /// A constructor after its instance is made: a local of that name.
    Local(String),
}

/// Every class and interface node of the program, by declaration.
pub(crate) fn index(p: &Program) -> HashMap<DeclId, NodeId> {
    let mut out = HashMap::new();
    for module in &p.modules {
        for &item in &module.items {
            if let Node::Class { decl, .. } | Node::Interface { decl, .. } = p.nodes.get(item) {
                out.insert(decl, item);
            }
        }
    }
    out
}

/// The visibility of what a type that isn't exported may not use, as
/// [`Translator::plan_accessors`] says.
const UNUSED: &str = "#[allow(dead_code)]\n";
const UNUSED_PUB: &str = "#[allow(dead_code)]\npub";

/// A member's kind for overriding: a getter, a setter, or a method.
fn slot(kind: DeclKind) -> DeclKind {
    match kind {
        DeclKind::Field => DeclKind::Getter,
        k => k,
    }
}

impl<'a> Translator<'a> {
    fn kind(&self, decl: DeclId) -> DeclKind {
        self.p.decls.get(decl).kind
    }

    fn facts(&self, decl: DeclId) -> Facts {
        self.p.decls.get(decl).facts
    }

    /// Whether a declaration is the program's own, not another library's
    /// or core's.
    pub(crate) fn own(&self, decl: DeclId) -> bool {
        !is_external(self.p, decl)
    }

    /// A class or interface member's Rust name: a setter's is `set_` and
    /// its property's.
    pub(crate) fn member_name(&self, decl: DeclId) -> String {
        let name = self.name(decl);
        match self.kind(decl) {
            DeclKind::Setter => format!("set_{}", name.trim_start_matches("r#")),
            _ => name,
        }
    }

    /// A class's or interface's member nodes.
    fn member_nodes(&self, class: DeclId) -> &'a [NodeId] {
        let p = self.p;
        match self.classes.get(&class).map(|&n| p.nodes.get(n)) {
            Some(Node::Class { members, .. } | Node::Interface { members, .. }) => {
                p.nodes.list(members)
            }
            _ => &[],
        }
    }

    /// A member node's declaration, and whether it has a body.
    fn member_decl(&self, member: NodeId) -> Option<(DeclId, bool)> {
        match self.p.nodes.get(member) {
            Node::Field { decl } | Node::StaticField { decl } => Some((decl, true)),
            Node::Method { decl, .. } | Node::Function { decl, .. } => Some((decl, true)),
            Node::AbstractMethod { decl, .. } => Some((decl, false)),
            _ => None,
        }
    }

    /// The instance member of `class` that fills the slot of `member`
    /// (a method, getter, or setter, by its source name): the class's own
    /// method, or a field for a getter, or a `var` field for a setter.
    fn provider(&self, class: DeclId, member: DeclId) -> Option<DeclId> {
        let name = self.p.name(member);
        let kind = self.kind(member);
        self.member_nodes(class).iter().find_map(|&m| {
            let (d, body) = self.member_decl(m)?;
            let dk = self.kind(d);
            let fits = match dk {
                DeclKind::Field => {
                    kind == DeclKind::Getter
                        || (kind == DeclKind::Setter && self.facts(d).contains(Facts::VAR))
                }
                DeclKind::Method | DeclKind::Getter | DeclKind::Setter => dk == kind,
                _ => false,
            };
            (fits && body && self.p.name(d) == name).then_some(d)
        })
    }

    /// The default body that fills `member`'s slot in a class: the first
    /// interface above it, breadth-first, as Kotlin picks, with a method
    /// of the slot's name and kind that has a body.
    fn winner(&self, class: DeclId, member: DeclId) -> Option<DeclId> {
        let name = self.p.name(member);
        let kind = slot(self.kind(member));
        ancestors(self.p, class).into_iter().find(|&i| {
            self.member_nodes(i).iter().any(|&m| {
                matches!(self.p.nodes.get(m), Node::Method { decl, .. }
                    if self.kind(decl) == kind && self.p.name(decl) == name)
            })
        })
    }

    /// A class or interface's own instance members, by Rust name.
    fn instance_names(&self, class: DeclId) -> Vec<String> {
        self.member_nodes(class)
            .iter()
            .filter_map(|&m| match self.p.nodes.get(m) {
                Node::Method { decl, .. } | Node::AbstractMethod { decl, .. }
                    if self.kind(decl) != DeclKind::Constructor =>
                {
                    Some(self.member_name(decl))
                }
                _ => None,
            })
            .collect()
    }

    /// A type's trait: `ShapeTrait`, by its path from here.
    fn trait_path(&self, iface: DeclId) -> R<String> {
        Ok(format!("{}Trait", self.top_path(iface)?))
    }

    /// The type a class's statics are named for: `RECT` for `Rect`.
    fn static_of(&self, decl: DeclId) -> String {
        let owner = self.p.decls.get(decl).owner;
        format!(
            "{}__{}",
            static_name(&snake_case(self.p.name(owner))),
            static_name(&self.name(decl))
        )
    }

    /// Plans which fields' getters and setters a module needs: those of
    /// the public fields of an exported class, or of any class where the
    /// library has host code, which may call them (`hidden.i()`), and a `var`'s getter
    /// wherever it's read, which reads it without holding its lock past
    /// the read. And which classes it constructs, which interfaces'
    /// handles it makes, and which interfaces' default methods use
    /// `clone_boxed`: Kotlin's output always has `new` and `clone_boxed`,
    /// which in a type that isn't exported may go unused, and so are
    /// marked `#[allow(dead_code)]`.
    pub(crate) fn plan_accessors(&mut self, module: &Module) {
        let p = self.p;
        let mut roots: Vec<NodeId> = module.items.clone();
        roots.push(module.init);
        roots.extend(&module.tests);
        // Upcasts that are receivers, which calls leave out.
        let mut receivers = HashSet::new();
        for &root in &roots {
            walk(p, root, &mut |id| {
                if let Node::CallMethod { receiver, .. } = p.nodes.get(id) {
                    let mut r = receiver;
                    while let Node::Upcast { value } = p.nodes.get(r) {
                        receivers.insert(r);
                        r = value;
                    }
                }
            });
        }
        for &root in &roots {
            walk(p, root, &mut |id| match p.nodes.get(id) {
                Node::GetField { field, .. } if self.facts(field).contains(Facts::VAR) => {
                    self.getters.insert(field);
                }
                Node::New { ctor, .. } => {
                    self.constructed.insert(p.decls.get(ctor).owner);
                }
                Node::Upcast { .. } if !receivers.contains(&id) => {
                    if let Some(to) = nominal_decl(p, p.nodes.ty(id)) {
                        self.wrapped.insert(to);
                    }
                }
                _ => {}
            });
        }
        // Interfaces whose default methods need `this` as a value, by
        // `clone_boxed`.
        for &item in &module.items {
            let Node::Interface { decl: iface, .. } = p.nodes.get(item) else {
                continue;
            };
            let mut thises = Vec::new();
            let mut receiving = HashSet::new();
            walk(p, item, &mut |id| match p.nodes.get(id) {
                Node::This {} => thises.push(id),
                Node::CallMethod { receiver, .. } => {
                    receiving.insert(receiver);
                }
                _ => {}
            });
            if thises.iter().any(|t| !receiving.contains(t)) {
                self.boxed.insert(iface);
            }
        }
        let host = !temper_tmpl::host::files(p, &[".rs"]).is_empty();
        for &item in &module.items {
            let Node::Class { decl: class, .. } = p.nodes.get(item) else {
                continue;
            };
            let reachable = host || self.facts(class).contains(Facts::EXPORTED);
            for iface in ancestors(p, class) {
                self.wrapped.insert(iface);
            }
            for &m in self.member_nodes(class) {
                let Node::Field { decl } = p.nodes.get(m) else {
                    continue;
                };
                let public = !self.is_private(decl);
                if reachable && public {
                    self.getters.insert(decl);
                    if self.facts(decl).contains(Facts::VAR) {
                        self.setters.insert(decl);
                    }
                }
            }
            // Fields that implement an interface's getters and setters.
            for iface in ancestors(p, class) {
                for &m in self.member_nodes(iface) {
                    let Some((d, _)) = self.member_decl(m) else {
                        continue;
                    };
                    if let Some(f) = self.provider(class, d) {
                        if self.kind(f) == DeclKind::Field {
                            match self.kind(d) {
                                DeclKind::Getter => self.getters.insert(f),
                                _ => self.setters.insert(f),
                            };
                        }
                    }
                }
            }
        }
    }

    fn is_private(&self, decl: DeclId) -> bool {
        let f = self.facts(decl);
        f.contains(Facts::PRIVATE) || f.contains(Facts::PROTECTED)
    }

    fn member_vis(&self, decl: DeclId) -> &'static str {
        if self.is_private(decl) { "" } else { "pub" }
    }

    /// A trait method's parameters and result, with concrete types:
    /// its parameters written out (`&self` first), their names, and its
    /// result.
    fn trait_sig(
        &mut self,
        decl: DeclId,
        params: Kids,
    ) -> R<(Vec<String>, Vec<String>, Option<String>)> {
        let p = self.p;
        let Some(sig) = p.signature(decl) else {
            return not_yet("a method without a signature");
        };
        if !p.decls.type_params(decl).is_empty() {
            return not_yet("a generic method of an interface");
        }
        let mut ps = vec!["&self".to_string()];
        let mut names = Vec::new();
        for &param in p.nodes.list(params) {
            let Node::Param { decl: d } = p.nodes.get(param) else {
                continue;
            };
            let name = self.name(d);
            ps.push(format!("{name}: {}", self.ty(p.decls.get(d).ty)?));
            names.push(name);
        }
        let result = self.result(sig.result, sig.bubbles)?;
        Ok((ps, names, result))
    }

    /// A result type, a `Result` if it bubbles.
    fn result(&mut self, ty: TypeId, bubbles: bool) -> R<Option<String>> {
        let result = if ty == TypeId::VOID {
            None
        } else {
            Some(self.ty(ty)?)
        };
        Ok(if bubbles {
            Some(format!(
                "temper_core::Result<{}>",
                result.unwrap_or_else(|| "()".into())
            ))
        } else {
            result
        })
    }

    /// A member's trait signature, from its node.
    fn member_sig(&mut self, member: NodeId) -> R<(Vec<String>, Vec<String>, Option<String>)> {
        match self.p.nodes.get(member) {
            Node::Method { decl, params, .. } | Node::AbstractMethod { decl, params } => {
                self.trait_sig(decl, params)
            }
            _ => not_yet("a trait member that isn't a method"),
        }
    }

    /// A method's member of an `impl`: its body translated with `this`
    /// as given.
    fn method_member(&mut self, item: NodeId, this: This, style: Params) -> R<Member> {
        let (Node::Method { decl, params, body } | Node::Function { decl, params, body }) =
            self.p.nodes.get(item)
        else {
            return not_yet("a method that isn't one");
        };
        let (generics, _) = self.type_params(decl)?;
        if style == Params::Trait && !generics.is_empty() {
            return not_yet("a generic method of an interface");
        }
        let outer = std::mem::replace(&mut self.this, this);
        let made = self.callable(item, decl, params, body, style);
        self.this = outer;
        let (params, result, body) = made?;
        let vis = if style == Params::Trait {
            ""
        } else {
            self.member_vis(decl)
        };
        Ok(Member::Fn {
            vis,
            name: format!("{}{generics}", self.member_name(decl)),
            params,
            result,
            body: Some(body),
        })
    }

    /// A static field's `OnceLock`, and its getter.
    fn static_field(&mut self, decl: DeclId) -> R<(Item, Member)> {
        if self.facts(decl).contains(Facts::VAR) {
            return not_yet("a static `var`");
        }
        let ty = self.ty(self.p.decls.get(decl).ty)?;
        self.uses.insert("std::sync::OnceLock");
        let stat = self.static_of(decl);
        let get = self.initialized(
            Block::of(Expr::path(&stat).method("get", Vec::new())),
            false,
        );
        let read = if self.is_copy(self.p.decls.get(decl).ty) {
            Expr::unary(Unary::Deref, get)
        } else {
            get.method("clone", Vec::new())
        };
        Ok((
            Item::Static {
                vis: "",
                name: stat,
                ty: format!("OnceLock<{ty}>"),
                value: Expr::path("OnceLock::new").call(Vec::new()),
            },
            Member::Fn {
                vis: self.member_vis(decl),
                name: self.name(decl),
                params: Vec::new(),
                result: Some(ty),
                body: Some(Block::of(read)),
            },
        ))
    }

    /// A class: its fields' struct, its handle, its statics, its
    /// methods, its traits, and its builder, if it has one.
    pub(crate) fn class(&mut self, node: NodeId) -> R<Vec<Item>> {
        let p = self.p;
        let Node::Class { decl: class, .. } = p.nodes.get(node) else {
            return not_yet("a class that isn't one");
        };
        let name = self.name(class);
        let vis = self.vis(class);
        let (gdecl, guse) = self.type_params(class)?;
        let where_ = self.extra_bounds(class)?;
        self.check_params_used(class)?;
        self.uses.insert("std::sync::Arc");
        let mut fields = Vec::new();
        let mut statics = Vec::new();
        let mut members = Vec::new();
        let mut accessors = Vec::new();
        let mut eq_hook = None;
        let mut display = false;
        for &m in self.member_nodes(class) {
            match p.nodes.get(m) {
                Node::Field { decl } => {
                    let ty = p.decls.get(decl).ty;
                    let rust_ty = self.ty(ty)?;
                    let var = self.facts(decl).contains(Facts::VAR);
                    let field = self.name(decl);
                    let stored = if var {
                        self.uses.insert("std::sync::RwLock");
                        format!("RwLock<{rust_ty}>")
                    } else {
                        rust_ty.clone()
                    };
                    fields.push(("", field.clone(), stored));
                    let fvis = self.member_vis(decl);
                    if self.getters.contains(&decl) {
                        let place = Expr::path("self").field("0").field(&field);
                        let read = if var {
                            let guard = place
                                .method("read", Vec::new())
                                .method("unwrap", Vec::new());
                            if self.is_copy(ty) {
                                Expr::unary(Unary::Deref, guard)
                            } else {
                                guard.method("clone", Vec::new())
                            }
                        } else if self.is_copy(ty) {
                            place
                        } else {
                            place.method("clone", Vec::new())
                        };
                        accessors.push(Member::Fn {
                            vis: fvis,
                            name: field.clone(),
                            params: vec!["&self".into()],
                            result: Some(rust_ty.clone()),
                            body: Some(Block::of(read)),
                        });
                    }
                    if self.setters.contains(&decl) {
                        let place = Expr::path("self").field("0").field(&field);
                        let write = Expr::unary(
                            Unary::Deref,
                            place
                                .method("write", Vec::new())
                                .method("unwrap", Vec::new()),
                        );
                        let mut stmts = Vec::new();
                        let param = if ty == TypeId::STRING {
                            self.uses.insert("temper_core::ToArcString");
                            stmts.push(Stmt::Let {
                                name: "value".into(),
                                mutable: false,
                                ty: None,
                                value: Some(
                                    Expr::path("value").method("to_arc_string", Vec::new()),
                                ),
                            });
                            "value: impl ToArcString".to_string()
                        } else {
                            format!("value: {rust_ty}")
                        };
                        stmts.push(Stmt::Assign(write, Expr::path("value")));
                        accessors.push(Member::Fn {
                            vis: fvis,
                            name: format!("set_{}", field.trim_start_matches("r#")),
                            params: vec!["&self".into(), param],
                            result: None,
                            body: Some(Block::new(stmts, None)),
                        });
                    }
                }
                Node::StaticField { decl } => {
                    let (stat, getter) = self.static_field(decl)?;
                    statics.push(stat);
                    members.push(getter);
                }
                Node::Method { decl, .. } if self.kind(decl) == DeclKind::Constructor => {
                    members.push(self.constructor(class, m)?);
                    self.builder(class, decl)?;
                }
                Node::Method { decl, .. } => {
                    if is_plain_to_string(p, decl) && !self.bubbles_decl(decl) {
                        display = true;
                    }
                    let mut member = self.method_member(m, This::Class, Params::Method)?;
                    if self.is_eq_hook(class, decl) {
                        eq_hook = Some(decl);
                        // Kotlin's API has `eq`, and it's `PartialEq`'s too.
                        if let Member::Fn { vis, name, .. } = &mut member {
                            if name == "eq" && self.facts(class).contains(Facts::EXPORTED) {
                                *vis = "#[allow(clippy::should_implement_trait)]\npub";
                            }
                        }
                    }
                    members.push(member);
                }
                Node::Function { .. } => {
                    members.push(self.method_member(m, This::None, Params::Free)?);
                }
                other => return not_yet(format!("a {} in a class", other.kind().name())),
            }
        }
        members.extend(accessors);
        let mut items = vec![
            Item::Struct {
                attrs: Vec::new(),
                vis: "",
                name: format!("{name}Struct{gdecl}"),
                fields,
            },
            Item::TupleStruct {
                attrs: vec!["#[derive(Clone)]".into()],
                vis,
                name: format!("{name}{gdecl}"),
                types: vec![format!("Arc<{name}Struct{guse}>")],
            },
        ];
        items.extend(statics);
        items.push(Item::Impl {
            head: format!("{gdecl} {name}{guse}").trim_start().to_string(),
            items: members,
        });
        if display {
            items.push(Item::Impl {
                head: format!("{gdecl} std::fmt::Display for {name}{guse}")
                    .trim_start()
                    .to_string(),
                items: vec![Member::Fn {
                    vis: "",
                    name: "fmt".into(),
                    params: vec!["&self".into(), "f: &mut std::fmt::Formatter<'_>".into()],
                    result: Some("std::fmt::Result".into()),
                    body: Some(Block::of(Expr::path("f").method(
                        "write_str",
                        vec![Expr::unary(
                            Unary::Ref,
                            Expr::path("self").method("to_string", Vec::new()),
                        )],
                    ))),
                }],
            });
        }
        if let Some(eq) = eq_hook {
            let call = Expr::path(format!("{name}::{}", self.name(eq))).call(vec![
                Expr::path("self"),
                Expr::path("other").method("clone", Vec::new()),
            ]);
            items.push(Item::Impl {
                head: format!("{gdecl} PartialEq for {name}{guse}")
                    .trim_start()
                    .to_string(),
                items: vec![Member::Fn {
                    vis: "",
                    name: "eq".into(),
                    params: vec!["&self".into(), format!("other: &{name}{guse}")],
                    result: Some("bool".into()),
                    body: Some(Block::of(call)),
                }],
            });
        }
        let ups = ancestors(p, class);
        for &iface in &ups {
            items.push(self.class_trait_impl(class, iface)?);
        }
        let mut targets = Vec::new();
        for &iface in &ups {
            let (t, _, saved) = self.super_type(class, iface)?;
            self.subst = saved;
            targets.push(t);
        }
        items.push(Item::Macro(Expr::Macro(
            "temper_core::impl_any_value_trait".into(),
            vec![
                Expr::path(format!("{name}{guse}")),
                Expr::path(format!("[{}]{where_}", targets.join(", "))),
            ],
        )));
        Ok(items)
    }

    fn bubbles_decl(&self, decl: DeclId) -> bool {
        self.p.signature(decl).is_some_and(|s| s.bubbles)
    }

    /// Whether a class's method is its `==`, which Rust's `PartialEq`
    /// calls: marked `@operator("==")` (or made by `@auto("==")`), taking
    /// one of the class, giving a `Boolean`, without bubbling.
    fn is_eq_hook(&self, class: DeclId, decl: DeclId) -> bool {
        let d = self.p.decls.get(decl);
        d.kind == DeclKind::Method
            && d.facts.contains(Facts::OPERATOR_EQ)
            && !self.is_private(decl)
            && self.p.signature(decl).is_some_and(|s| {
                !s.bubbles
                    && s.result == TypeId::BOOLEAN
                    && s.params.len() == 1
                    && nominal_decl(self.p, s.params[0]) == Some(class)
            })
    }

    /// Whether a class's `==` is Rust's `==`.
    pub(crate) fn eq_hook(&self, method: DeclId) -> bool {
        let owner = self.p.decls.get(method).owner;
        self.p.decls.contains(owner)
            && self.kind(owner) == DeclKind::Class
            && self.own(method)
            && self.is_eq_hook(owner, method)
    }

    /// Whether a method is a plain `toString()` that's its class's
    /// `Display`.
    pub(crate) fn display_hook(&self, method: DeclId) -> bool {
        self.own(method) && is_plain_to_string(self.p, method) && !self.bubbles_decl(method)
    }

    /// `impl ShapeTrait for Rect`: `clone_boxed`, and each of the
    /// interface's members the class fills, or that a default elsewhere
    /// overrides.
    fn class_trait_impl(&mut self, class: DeclId, iface: DeclId) -> R<Item> {
        let (gdecl, guse) = self.type_params(class)?;
        let name = format!("{}{guse}", self.name(class));
        let (iname, iargs, saved) = self.super_type(class, iface)?;
        let made = self.class_trait_items(class, iface, &name, &iname);
        self.subst = saved;
        let items = made?;
        let path = self.top_path(iface)?;
        Ok(Item::Impl {
            head: format!("{gdecl} {path}Trait{iargs} for {name}")
                .trim_start()
                .to_string(),
            items,
        })
    }

    /// A class or interface seen as `iface`, one of its supertypes: its
    /// type as written (`Holder<T>`), its arguments (`<T>`), and the
    /// substitution as it was before this put the interface's type
    /// parameters in, which the caller restores.
    pub(crate) fn super_type(
        &mut self,
        class: DeclId,
        iface: DeclId,
    ) -> R<(String, String, HashMap<DeclId, TypeId>)> {
        let p = self.p;
        let saved = self.subst.clone();
        let ty = p.decls.get(class).ty;
        let Some(args) = temper_tmpl::analysis::as_super(p, ty, iface) else {
            return not_yet("a supertype whose type arguments aren't interned");
        };
        let mut texts = Vec::new();
        for &a in &args {
            texts.push(self.ty(a)?);
        }
        for (&tp, &a) in p.decls.type_params(iface).iter().zip(&args) {
            self.subst.insert(tp, a);
        }
        let iargs = if texts.is_empty() {
            String::new()
        } else {
            format!("<{}>", texts.join(", "))
        };
        Ok((format!("{}{iargs}", self.top_path(iface)?), iargs, saved))
    }

    /// Fails unless each of a generic class's type parameters is in a
    /// field's type, as Rust needs.
    fn check_params_used(&mut self, class: DeclId) -> R<()> {
        let p = self.p;
        for &tp in p.decls.type_params(class) {
            let tp_ty = p.decls.get(tp).ty;
            let used = self
                .member_nodes(class)
                .iter()
                .any(|&m| match p.nodes.get(m) {
                    Node::Field { decl } => mentions(p, p.decls.get(decl).ty, tp_ty),
                    _ => false,
                });
            if !used {
                return not_yet("a class type parameter that no field's type has");
            }
        }
        Ok(())
    }

    /// The members of `impl ShapeTrait for Rect`.
    fn class_trait_items(
        &mut self,
        class: DeclId,
        iface: DeclId,
        name: &str,
        iname: &str,
    ) -> R<Vec<Member>> {
        let iname = iname.to_string();
        let mut items = vec![Member::Fn {
            vis: "",
            name: "clone_boxed".into(),
            params: vec!["&self".into()],
            result: Some(iname.clone()),
            body: Some(Block::of(
                Expr::path(format!("{}::new", self.top_path(iface)?))
                    .call(vec![Expr::path("self").method("clone", Vec::new())]),
            )),
        }];
        for &m in self.member_nodes(iface) {
            if !matches!(
                self.p.nodes.get(m),
                Node::Method { .. } | Node::AbstractMethod { .. }
            ) {
                continue;
            }
            let Some((d, _)) = self.member_decl(m) else {
                continue;
            };
            let (params, names, result) = self.member_sig(m)?;
            let args: Vec<Expr> = names.iter().map(Expr::path).collect();
            let rust = self.member_name(d);
            let call = if self.provider(class, d).is_some() {
                Expr::path("self").method(&rust, args)
            } else {
                match self.winner(class, d) {
                    Some(w) if w == iface => continue,
                    Some(w) => {
                        let mut all = vec![Expr::path("self")];
                        all.extend(args);
                        Expr::path(format!("{}::{rust}", self.trait_path(w)?)).call(all)
                    }
                    None => {
                        return not_yet(format!("{name} without {} of {iname}", self.p.name(d)));
                    }
                }
            };
            items.push(Member::Fn {
                vis: "",
                name: rust,
                params,
                result,
                body: Some(Block::of(call)),
            });
        }
        Ok(items)
    }

    /// An interface: its trait, its handle, which implements the trait
    /// and those above it, and its statics.
    pub(crate) fn interface(&mut self, node: NodeId) -> R<Vec<Item>> {
        let p = self.p;
        let Node::Interface { decl: iface, .. } = p.nodes.get(node) else {
            return not_yet("an interface that isn't one");
        };
        let bare = self.name(iface);
        let vis = self.vis(iface);
        let (gdecl, guse) = self.type_params(iface)?;
        let where_ = self.extra_bounds(iface)?;
        let name = format!("{bare}{guse}");
        self.uses.insert("std::sync::Arc");
        let mut bounds: Vec<String> = [
            "temper_core::AsAnyValue",
            "temper_core::AnyValueTrait",
            "Send",
            "Sync",
        ]
        .map(String::from)
        .to_vec();
        for &s in p.decls.supers(iface) {
            if let Some(up) = nominal_decl(p, s) {
                let (_, args, saved) = self.super_type(iface, up)?;
                self.subst = saved;
                bounds.push(format!("{}{args}", self.trait_path(up)?));
            }
        }
        let exported = self.facts(iface).contains(Facts::EXPORTED);
        let mut trait_items = vec![Member::Fn {
            vis: if exported || self.boxed.contains(&iface) {
                ""
            } else {
                UNUSED
            },
            name: "clone_boxed".into(),
            params: vec!["&self".into()],
            result: Some(name.clone()),
            body: None,
        }];
        let mut statics = Vec::new();
        let mut own = vec![Member::Fn {
            vis: if exported || self.wrapped.contains(&iface) {
                "pub"
            } else {
                UNUSED_PUB
            },
            name: "new".into(),
            params: vec![format!("selfish: impl {bare}Trait{guse} + 'static")],
            result: Some(name.clone()),
            body: Some(Block::of(Expr::path(&bare).call(vec![
                Expr::path("Arc::new").call(vec![Expr::path("selfish")]),
            ]))),
        }];
        for &m in self.member_nodes(iface) {
            match p.nodes.get(m) {
                Node::AbstractMethod { decl, .. } => {
                    let (params, _, result) = self.member_sig(m)?;
                    trait_items.push(Member::Fn {
                        vis: "",
                        name: self.member_name(decl),
                        params,
                        result,
                        body: None,
                    });
                }
                Node::Method { .. } => {
                    trait_items.push(self.method_member(m, This::Trait(iface), Params::Trait)?);
                }
                Node::StaticField { decl } => {
                    let (stat, getter) = self.static_field(decl)?;
                    statics.push(stat);
                    own.push(getter);
                }
                Node::Function { .. } => {
                    own.push(self.method_member(m, This::None, Params::Free)?);
                }
                other => return not_yet(format!("a {} in an interface", other.kind().name())),
            }
        }
        let mut items = vec![
            Item::Trait {
                vis,
                name: format!("{bare}Trait{gdecl}"),
                bounds,
                items: trait_items,
            },
            Item::TupleStruct {
                attrs: vec!["#[derive(Clone)]".into()],
                vis,
                name: format!("{bare}{gdecl}"),
                types: vec![format!("Arc<dyn {bare}Trait{guse}>")],
            },
        ];
        items.extend(statics);
        items.push(Item::Impl {
            head: format!("{gdecl} {name}").trim_start().to_string(),
            items: own,
        });
        let mut traits = vec![iface];
        traits.extend(ancestors(p, iface));
        for t in traits {
            let (_, args, saved) = self.super_type(iface, t)?;
            let made = self.forwarding_impl(&name, t, &gdecl, &args);
            self.subst = saved;
            items.push(made?);
        }
        items.push(Item::Macro(Expr::Macro(
            "temper_core::impl_any_value_trait_for_interface".into(),
            vec![Expr::path(format!("{name}{where_}"))],
        )));
        items.push(Item::Impl {
            head: format!("{gdecl} std::ops::Deref for {name}")
                .trim_start()
                .to_string(),
            items: vec![
                Member::Type {
                    name: "Target".into(),
                    ty: format!("dyn {bare}Trait{guse}"),
                },
                Member::Fn {
                    vis: "",
                    name: "deref".into(),
                    params: vec!["&self".into()],
                    result: Some("&Self::Target".into()),
                    body: Some(Block::of(Expr::unary(
                        Unary::Ref,
                        Expr::unary(Unary::Deref, Expr::path("self").field("0")),
                    ))),
                },
            ],
        });
        Ok(items)
    }

    /// `impl ATrait for B`, on an interface's handle: each of `A`'s
    /// members, `clone_boxed` first, calls the object's.
    fn forwarding_impl(
        &mut self,
        name: &str,
        iface: DeclId,
        generics: &str,
        args: &str,
    ) -> R<Item> {
        let tpath = self.trait_path(iface)?;
        let object = || {
            Expr::unary(
                Unary::Ref,
                Expr::unary(Unary::Deref, Expr::path("self").field("0")),
            )
        };
        let mut items = vec![Member::Fn {
            vis: "",
            name: "clone_boxed".into(),
            params: vec!["&self".into()],
            result: Some(format!("{}{args}", self.top_path(iface)?)),
            body: Some(Block::of(
                Expr::path(format!("{tpath}::clone_boxed")).call(vec![object()]),
            )),
        }];
        for &m in self.member_nodes(iface) {
            let (Node::Method { decl, .. } | Node::AbstractMethod { decl, .. }) =
                self.p.nodes.get(m)
            else {
                continue;
            };
            let (params, names, result) = self.member_sig(m)?;
            let mut args = vec![object()];
            args.extend(names.iter().map(Expr::path));
            let rust = self.member_name(decl);
            items.push(Member::Fn {
                vis: "",
                name: rust.clone(),
                params,
                result,
                body: Some(Block::of(Expr::path(format!("{tpath}::{rust}")).call(args))),
            });
        }
        Ok(Item::Impl {
            head: format!("{generics} {tpath}{args} for {name}")
                .trim_start()
                .to_string(),
            items,
        })
    }

    /// A class's constructor, `new`: it sets each field, in order, from
    /// values that read only parameters, locals, and fields already set,
    /// then makes the instance, then runs the rest of its body with it.
    /// The frontend's fields-first rule guarantees that every field is
    /// set before any other use of `this`, and by every normal return;
    /// fields set in branches or loops, or with other statements between
    /// them, are still not yet here.
    fn constructor(&mut self, class: DeclId, item: NodeId) -> R<Member> {
        let p = self.p;
        let Node::Method { decl, params, body } = p.nodes.get(item) else {
            return not_yet("a constructor that isn't a method");
        };
        let Some(sig) = p.signature(decl) else {
            return not_yet("a constructor without a signature");
        };
        let name = self.name(class);
        self.begin(item, sig.bubbles);
        let mut stmts = Vec::new();
        let mut ps = Vec::new();
        self.params_into(item, params, Params::Free, &mut ps, &mut stmts)?;
        let fields: Vec<DeclId> = (self.member_nodes(class).iter())
            .filter_map(|&m| match p.nodes.get(m) {
                Node::Field { decl } => Some(decl),
                _ => None,
            })
            .collect();
        let body_stmts: &[NodeId] = match p.nodes.get(body) {
            Node::Block { stmts } => p.nodes.list(stmts),
            _ => &[],
        };
        // Its parameters' defaults first, which can't read `this`.
        let defaults = temper_tmpl::analysis::defaults(p, body).len();
        for &s in &body_stmts[..defaults] {
            let mut this = false;
            walk(p, s, &mut |id| {
                this |= matches!(p.nodes.get(id), Node::This {})
            });
            if this {
                return not_yet("a constructor's default that reads `this`");
            }
            self.stmt(s, &mut stmts)?;
        }
        let body_stmts = &body_stmts[defaults..];
        // The fields set first, in order, each once.
        let mut set: Vec<(DeclId, NodeId)> = Vec::new();
        let mut read: HashSet<DeclId> = HashSet::new();
        let mut rest = body_stmts.len();
        for (i, &s) in body_stmts.iter().enumerate() {
            let Node::SetField {
                field,
                object,
                value,
            } = p.nodes.get(s)
            else {
                rest = i;
                break;
            };
            let set_already = |f: DeclId| set.iter().any(|&(g, _)| g == f);
            let mut fits = matches!(p.nodes.get(object), Node::This {}) && !set_already(field);
            // `this` only as the object of a field already set.
            let (mut reads, mut thises) = (Vec::new(), 0);
            walk(p, value, &mut |id| match p.nodes.get(id) {
                Node::GetField { field: g, object }
                    if matches!(p.nodes.get(object), Node::This {}) =>
                {
                    reads.push(g);
                }
                Node::This {} => thises += 1,
                _ => {}
            });
            fits &= thises == reads.len() && reads.iter().all(|&g| set_already(g));
            if !fits {
                rest = i;
                break;
            }
            read.extend(reads);
            set.push((field, value));
        }
        if set.len() != fields.len() || fields.iter().any(|f| !set.iter().any(|(g, _)| g == f)) {
            return not_yet(format!(
                "a constructor of {name} that doesn't set each field first"
            ));
        }
        // Names taken in the constructor, for locals of fields.
        let mut taken: HashSet<String> = HashSet::new();
        walk(p, item, &mut |id| {
            if let Node::Param { decl } | Node::Let { local: decl, .. } = p.nodes.get(id) {
                taken.insert(self.name(decl));
            }
        });
        let mut lits = Vec::new();
        for (field, value) in set {
            let fname = self.name(field);
            let ty = p.decls.get(field).ty;
            // A parameter read only here moves into its field.
            let moved = match p.nodes.get(value) {
                Node::Local { local } => {
                    let mut reads = 0;
                    walk(p, item, &mut |id| {
                        if matches!(p.nodes.get(id), Node::Local { local: l } if l == local) {
                            reads += 1;
                        }
                    });
                    (reads == 1).then(|| Expr::path(self.name(local)))
                }
                _ => None,
            };
            let mut e = match moved {
                Some(e) => e,
                None => self.expr(value, Want::Owned)?,
            };
            if read.contains(&field) {
                let mut local = fname.clone();
                while taken.contains(&local) {
                    local.push('_');
                }
                taken.insert(local.clone());
                stmts.push(Stmt::Let {
                    name: local.clone(),
                    mutable: false,
                    ty: super::numeric_literal(&e)
                        .then(|| self.ty(ty))
                        .transpose()?,
                    value: Some(e),
                });
                self.ctor_fields.insert(field, local.clone());
                e = if self.is_copy(ty) {
                    Expr::path(local)
                } else {
                    Expr::path(local).method("clone", Vec::new())
                };
            }
            if self.facts(field).contains(Facts::VAR) {
                e = Expr::path("RwLock::new").call(vec![e]);
            }
            lits.push(match e {
                Expr::Path(ref x) if *x == fname => (fname, None),
                e => (fname, Some(e)),
            });
        }
        self.ctor_fields.clear();
        let made = Expr::path(&name).call(vec![
            Expr::path("Arc::new").call(vec![Expr::Struct(format!("{name}Struct"), lits)]),
        ]);
        let tail = if rest == body_stmts.len() {
            made
        } else {
            let mut local = "selfish".to_string();
            while taken.contains(&local) {
                local.push('_');
            }
            stmts.push(Stmt::Let {
                name: local.clone(),
                mutable: false,
                ty: None,
                value: Some(made),
            });
            let outer = std::mem::replace(&mut self.this, This::Local(local.clone()));
            let mut done = Ok(());
            for &s in &body_stmts[rest..] {
                done = self.stmt(s, &mut stmts);
                if done.is_err() {
                    break;
                }
            }
            self.this = outer;
            done?;
            Expr::path(local)
        };
        let tail = if self.bubbles {
            Expr::path("Ok").call(vec![tail])
        } else {
            tail
        };
        let result = self.result(sig.result, false)?;
        let result = if self.bubbles {
            result.map(|r| format!("temper_core::Result<{r}>"))
        } else {
            result
        };
        let exported = self.facts(class).contains(Facts::EXPORTED);
        let vis = match self.member_vis(decl) {
            // Kotlin's API has `new()` without `Default`.
            "pub" if exported && ps.is_empty() && !self.bubbles => {
                "#[allow(clippy::new_without_default)]\npub"
            }
            vis if exported || self.constructed.contains(&class) => vis,
            "pub" => UNUSED_PUB,
            _ => UNUSED,
        };
        Ok(Member::Fn {
            vis,
            name: "new".into(),
            params: ps,
            result,
            body: Some(Block::new(stmts, Some(tail))),
        })
    }

    /// A builder, as Kotlin's, for an exported class whose public
    /// constructor takes two or more arguments: `RectBuilder { w, h }`,
    /// whose `build()` calls `Rect::new`.
    fn builder(&mut self, class: DeclId, ctor: DeclId) -> R<()> {
        let p = self.p;
        if !self.facts(class).contains(Facts::EXPORTED) || self.is_private(ctor) {
            return Ok(());
        }
        let Some(node) = self
            .member_nodes(class)
            .iter()
            .copied()
            .find(|&m| matches!(p.nodes.get(m), Node::Method { decl, .. } if decl == ctor))
        else {
            return Ok(());
        };
        let Node::Method { params, .. } = p.nodes.get(node) else {
            return Ok(());
        };
        let params = p.nodes.list(params);
        if params.len() < 2 {
            return Ok(());
        }
        // A generic class's builder takes its type parameters, as
        // Kotlin's: `ChangeBuilder<T> { … }`.
        let (gdecl, guse) = self.type_params(class)?;
        if !self.extra_bounds(class)?.is_empty() {
            return not_yet("a builder for a class with extra bounds");
        }
        let name = self.name(class);
        let any_optional = has_optional(p, params);
        let mut fields = Vec::new();
        let mut optional = Vec::new();
        let mut args = Vec::new();
        for &param in params {
            let Node::Param { decl } = p.nodes.get(param) else {
                continue;
            };
            let pname = self.name(decl);
            let ty = p.decls.get(decl).ty;
            if p.decls.get(decl).facts.contains(Facts::OPTIONAL) {
                let inner = self.ty(p.types.non_null(ty))?;
                optional.push((pname.clone(), self.ty(ty)?, inner));
                args.push(Expr::path("self").field(pname));
            } else {
                fields.push(("pub", pname.clone(), self.ty(ty)?));
                let selfish = if !any_optional {
                    Expr::path("self")
                } else {
                    Expr::path("self").field("selfish")
                };
                args.push(selfish.field(pname));
            }
        }
        let result = self.result(TypeId::VOID, self.bubbles_decl(ctor))?;
        let result = match result {
            Some(_) => format!("temper_core::Result<{name}{guse}>"),
            None => format!("{name}{guse}"),
        };
        if !optional.is_empty() {
            if !gdecl.is_empty() {
                return not_yet("a builder with options for a generic class");
            }
            return self.builder_with_options(&name, fields, optional, args, result);
        }
        self.builders.push(Item::Struct {
            attrs: vec!["#[derive(Clone)]".into()],
            vis: "pub",
            name: format!("{name}Builder{gdecl}"),
            fields,
        });
        self.builders.push(Item::Impl {
            head: format!("{gdecl} {name}Builder{guse}")
                .trim_start()
                .to_string(),
            items: vec![Member::Fn {
                vis: "pub",
                name: "build".into(),
                params: vec!["self".into()],
                result: Some(result),
                body: Some(Block::of(Expr::path(format!("{name}::new")).call(args))),
            }],
        });
        Ok(())
    }

    /// A builder for a constructor with optional parameters, as Kotlin's:
    /// `RectBuilder { w }` for the required ones, whose `options()` gives
    /// `RectOptions`, with a method to set each optional one (`.h(2)`),
    /// and `build()` on either. Kotlin passes the required ones first;
    /// these keep the constructor's order.
    fn builder_with_options(
        &mut self,
        name: &str,
        fields: Vec<(&'static str, String, String)>,
        optional: Vec<(String, String, String)>,
        args: Vec<Expr>,
        result: String,
    ) -> R<()> {
        let (builder, options) = (format!("{name}Builder"), format!("{name}Options"));
        let required = !fields.is_empty();
        self.builders.push(Item::Struct {
            attrs: vec!["#[derive(Clone)]".into()],
            vis: "pub",
            name: builder.clone(),
            fields,
        });
        // Kotlin keeps the builder in the options even without required
        // fields, which clippy then finds never read.
        let mut option_fields = Vec::new();
        if required {
            option_fields.push(("", "selfish".to_string(), builder.clone()));
        }
        option_fields.extend(optional.iter().map(|(n, t, _)| ("", n.clone(), t.clone())));
        self.builders.push(Item::Struct {
            attrs: vec!["#[derive(Clone)]".into()],
            vis: "pub",
            name: options.clone(),
            fields: option_fields,
        });
        let mut inits = Vec::new();
        if required {
            inits.push(("selfish".to_string(), None));
        }
        inits.extend(
            optional
                .iter()
                .map(|(n, ..)| (n.clone(), Some(Expr::path("None")))),
        );
        let selfish = if required { "selfish" } else { "_selfish" };
        let mut items = vec![Member::Fn {
            vis: "pub",
            name: "new".into(),
            params: vec![format!("{selfish}: {builder}")],
            result: Some("Self".into()),
            body: Some(Block::of(Expr::Struct("Self".into(), inits))),
        }];
        for (n, _, inner) in &optional {
            let set = Stmt::Assign(
                Expr::path("self").field(n.clone()),
                Expr::path("Some").call(vec![Expr::path(n.clone())]),
            );
            items.push(Member::Fn {
                vis: "pub",
                name: n.clone(),
                params: vec!["mut self".into(), format!("{n}: {inner}")],
                result: Some("Self".into()),
                body: Some(Block::new(vec![set], Some(Expr::path("self")))),
            });
        }
        items.push(Member::Fn {
            vis: "pub",
            name: "build".into(),
            params: vec!["self".into()],
            result: Some(result.clone()),
            body: Some(Block::of(Expr::path(format!("{name}::new")).call(args))),
        });
        self.builders.push(Item::Impl {
            head: options.clone(),
            items,
        });
        let build = Expr::path("self")
            .method("options", Vec::new())
            .method("build", Vec::new());
        self.builders.push(Item::Impl {
            head: builder,
            items: vec![
                Member::Fn {
                    vis: "pub",
                    name: "build".into(),
                    params: vec!["self".into()],
                    result: Some(result),
                    body: Some(Block::of(build)),
                },
                Member::Fn {
                    vis: "pub",
                    name: "options".into(),
                    params: vec!["self".into()],
                    result: Some(options.clone()),
                    body: Some(Block::of(
                        Expr::path(format!("{options}::new")).call(vec![Expr::path("self")]),
                    )),
                },
            ],
        });
        Ok(())
    }

    /// `this` as its place wants it.
    pub(crate) fn this(&mut self, want: Want) -> R<Expr> {
        let owned = matches!(want, Want::Owned | Want::IntoArc);
        Ok(match self.this.clone() {
            This::None => return not_yet("`this` outside a method"),
            This::Class if owned => Expr::path("self").method("clone", Vec::new()),
            This::Class => Expr::path("self"),
            This::Trait(iface) if owned => {
                Expr::path(format!("{}::clone_boxed", self.trait_path(iface)?))
                    .call(vec![Expr::path("self")])
            }
            This::Trait(_) => Expr::path("self"),
            This::Local(name) if owned => Expr::path(name).method("clone", Vec::new()),
            This::Local(name) => Expr::path(name),
        })
    }

    /// Reads an instance field: a field of the class being constructed
    /// from its local; else of a class of this module directly, or a
    /// `var` through its getter, which holds its lock only for the read;
    /// else through its getter.
    pub(crate) fn get_field(
        &mut self,
        field: DeclId,
        object: NodeId,
        ty: TypeId,
        want: Want,
    ) -> R<Expr> {
        let p = self.p;
        let copy = self.is_copy(ty);
        let owned = matches!(want, Want::Owned | Want::IntoArc);
        // A pair's, a tuple's.
        if is_external(p, field) && self.type_qname(p.nodes.ty(object)) == "core.type Pair" {
            let index = if p.name(field) == "key" { "0" } else { "1" };
            let e = self.expr(object, Want::Borrowed)?.field(index);
            return Ok(if owned && !copy {
                e.method("clone", Vec::new())
            } else {
                e
            });
        }
        if matches!(p.nodes.get(object), Node::This {}) {
            if let Some(local) = self.ctor_fields.get(&field) {
                let e = Expr::path(local.clone());
                return Ok(if owned && !copy {
                    e.method("clone", Vec::new())
                } else {
                    e
                });
            }
        }
        let obj = self.expr(object, Want::Borrowed)?;
        let name = self.name(field);
        let here = home(p, field) == Some(self.here);
        if !here || self.facts(field).contains(Facts::VAR) {
            return Ok(obj.method(name, Vec::new()));
        }
        let e = obj.field("0").field(name);
        Ok(if owned && !copy {
            e.method("clone", Vec::new())
        } else {
            e
        })
    }

    /// Writes an instance field: of a class of this module under its
    /// lock, else through its setter.
    pub(crate) fn set_field(&mut self, field: DeclId, object: NodeId, value: NodeId) -> R<Stmt> {
        let p = self.p;
        let value = self.expr(value, Want::Owned)?;
        let obj = self.expr(object, Want::Borrowed)?;
        if home(p, field) != Some(self.here) {
            let name = format!("set_{}", self.name(field).trim_start_matches("r#"));
            return Ok(Stmt::Expr(obj.method(name, vec![value])));
        }
        if !self.facts(field).contains(Facts::VAR) {
            return not_yet("setting a `let` field after its constructor makes the instance");
        }
        let place = obj.field("0").field(self.name(field));
        let write = place
            .method("write", Vec::new())
            .method("unwrap", Vec::new());
        Ok(Stmt::Assign(Expr::unary(Unary::Deref, write), value))
    }

    /// `RECT__UNIT.set(…)`, in the init block.
    pub(crate) fn init_static(&mut self, target: DeclId, value: NodeId) -> R<Stmt> {
        if home(self.p, target) != Some(self.here) {
            return not_yet("initializing another module's static");
        }
        let v = self.expr(value, Want::Owned)?;
        let fail = Expr::Closure(
            vec!["_".into()],
            Block::of(Expr::Macro("panic".into(), Vec::new())),
        );
        Ok(Stmt::Expr(
            Expr::path(self.static_of(target))
                .method("set", vec![v])
                .method("unwrap_or_else", vec![fail]),
        ))
    }

    /// A static's getter: `Rect::unit()`.
    pub(crate) fn get_static(&mut self, target: DeclId) -> R<Expr> {
        if is_external(self.p, target) && library_of(self.p, target) == "core" {
            // `String.begin`, `Float64.pi`, and the like.
            return self.connected(target, &[], Want::Owned);
        }
        let owner = self.p.decls.get(target).owner;
        Ok(
            Expr::path(format!("{}::{}", self.top_path(owner)?, self.name(target)))
                .call(Vec::new()),
        )
    }

    /// A call of a static function, `C::say_hi()`.
    pub(crate) fn static_fn_path(&self, callee: DeclId) -> R<Option<String>> {
        let owner = self.p.decls.get(callee).owner;
        if !self.p.decls.contains(owner)
            || !matches!(self.kind(owner), DeclKind::Class | DeclKind::Interface)
        {
            return Ok(None);
        }
        Ok(Some(format!(
            "{}::{}",
            self.top_path(owner)?,
            self.name(callee)
        )))
    }

    /// A call of a method of the program's own: on a class, its method;
    /// on an interface, its trait's, as a method where that's
    /// unambiguous, else by the trait's path. An upcast receiver is the
    /// value as it is, since its class implements the trait.
    pub(crate) fn call_method(
        &mut self,
        method: DeclId,
        receiver: NodeId,
        args: &[NodeId],
    ) -> R<Expr> {
        let p = self.p;
        let rust = self.member_name(method);
        if !p.is_virtual(method) {
            if self.eq_hook(method) {
                if let [other] = args {
                    let a = self.expr(receiver, Want::Borrowed)?;
                    let b = self.expr(*other, Want::Borrowed)?;
                    return Ok(Expr::bin(Bin::Eq, a, b));
                }
            }
            let recv = self.expr(receiver, Want::Borrowed)?;
            let args = self.args(method, args)?;
            return Ok(recv.method(rust, args));
        }
        let iface = p.decls.get(method).owner;
        let mut recv_node = receiver;
        while let Node::Upcast { value } = p.nodes.get(recv_node) {
            recv_node = value;
        }
        let params: Vec<TypeId> =
            (p.signature(method)).map_or_else(Vec::new, |s| s.params.to_vec());
        let mut xs = Vec::new();
        for (i, &a) in args.iter().enumerate() {
            if matches!(p.nodes.get(a), Node::Omitted {}) {
                xs.push(self.omitted(params.get(i).copied())?);
                continue;
            }
            if params.get(i).is_some_and(|&t| self.is_nullable_string(t)) {
                xs.push(self.nullable_string_arg(a)?);
                continue;
            }
            xs.push(self.expr(a, Want::Owned)?);
        }
        let recv = self.expr(recv_node, Want::Borrowed)?;
        let ty = nominal_decl(p, p.nodes.ty(recv_node));
        let plain =
            home(p, iface) == Some(self.here) && ty.is_some_and(|t| self.unambiguous(t, &rust));
        if plain {
            return Ok(recv.method(rust, xs));
        }
        let recv = match recv {
            Expr::Path(ref s) if s == "self" => recv,
            recv => Expr::unary(Unary::Ref, recv),
        };
        let mut all = vec![recv];
        all.extend(xs);
        Ok(Expr::path(format!("{}::{rust}", self.trait_path(iface)?)).call(all))
    }

    /// Whether `x.name(…)` on a value of class or interface `ty` calls
    /// exactly one method: the class's own, or one trait's.
    fn unambiguous(&self, ty: DeclId, name: &str) -> bool {
        let mut traits = Vec::new();
        match self.kind(ty) {
            DeclKind::Class => {
                if self.instance_names(ty).iter().any(|n| n == name) {
                    return true;
                }
            }
            DeclKind::Interface => traits.push(ty),
            _ => return false,
        }
        traits.extend(ancestors(self.p, ty));
        let defining = traits
            .iter()
            .filter(|&&t| self.instance_names(t).iter().any(|n| n == name))
            .count();
        defining == 1 && !matches!(name, "clone_boxed")
    }

    /// `Shape::new(r)`: a class's instance, or an interface's, as an
    /// interface above it.
    pub(crate) fn upcast(&mut self, id: NodeId, value: NodeId) -> R<Expr> {
        let p = self.p;
        let to = p.nodes.ty(id);
        let from = p.nodes.ty(value);
        if from == to {
            return self.expr(value, Want::Owned);
        }
        match self.type_qname(to).as_str() {
            // A string index as an option, `Some(i)`; no index, `None`.
            "core.type StringIndexOption" => {
                if self.string_index(from) == Some(false) {
                    if let Node::GetStatic { .. } = p.nodes.get(value) {
                        return Ok(Expr::path("None"));
                    }
                    let v = self.expr(value, Want::Owned)?;
                    return Ok(
                        Expr::path("temper_core::string::cast_none_as_index_option").call(vec![v])
                    );
                }
                let v = self.expr(value, Want::Owned)?;
                return Ok(Expr::path("Some").call(vec![v]));
            }
            "core.type Listed" => {
                let v = self.expr(value, Want::Owned)?;
                return Ok(Expr::path("temper_core::Listed::new").call(vec![v]));
            }
            "core.type Mapped" => {
                let v = self.expr(value, Want::Owned)?;
                return Ok(Expr::path("temper_core::Mapped::new").call(vec![v]));
            }
            _ => {}
        }
        // A type parameter as its class bound: `Into` it.
        if let (temper_tmpl::TypeData::TypeParam { .. }, Some(class)) = (
            p.types.get(from),
            nominal_decl(p, to).filter(|&d| self.kind(d) == DeclKind::Class),
        ) {
            let _ = class;
            let ty = self.ty(to)?;
            let v = self.expr(value, Want::Owned)?;
            return Ok(Expr::path(format!("Into::<{ty}>::into")).call(vec![v]));
        }
        let Some(iface) = nominal_decl(p, to).filter(|&d| self.kind(d) == DeclKind::Interface)
        else {
            return not_yet(format!("an upcast to {}", p.show_type(to)));
        };
        let v = self.expr(value, Want::Owned)?;
        Ok(Expr::path(format!("{}::new", self.top_path(iface)?)).call(vec![v]))
    }

    /// Whether a type is core's `StringIndex` (true) or `NoStringIndex`
    /// (false), which an `Option<usize>` is checked or cast to.
    pub(crate) fn string_index(&self, ty: TypeId) -> Option<bool> {
        match self.type_qname(ty).as_str() {
            "core.type StringIndex" => Some(true),
            "core.type NoStringIndex" => Some(false),
            _ => None,
        }
    }

    /// `temper_core::cast::<Square>(q)`, an `Option`; for a string index,
    /// `temper_core::string::cast_as_index(i)`, as Kotlin's.
    pub(crate) fn cast(&mut self, id: NodeId, value: NodeId) -> R<Expr> {
        if let Some(index) = self.string_index(self.p.nodes.ty(id)) {
            let v = self.expr(value, Want::Owned)?;
            let f = if index {
                "temper_core::string::cast_as_index"
            } else {
                "temper_core::string::cast_as_no_index"
            };
            return Ok(Expr::path(f).call(vec![v]));
        }
        let ty = self.ty(self.p.nodes.ty(id))?;
        let v = self.expr(value, Want::Owned)?;
        Ok(Expr::path(format!("temper_core::cast::<{ty}>")).call(vec![v]))
    }

    /// `temper_core::is::<Square>(q)`.
    pub(crate) fn is(&mut self, ty: TypeId, value: NodeId) -> R<Expr> {
        if let Some(index) = self.string_index(ty) {
            // `i.is_some()` or `i.is_none()`, as Kotlin's.
            let v = self.expr(value, Want::Borrowed)?;
            let name = if index { "is_some" } else { "is_none" };
            return Ok(v.method(name, Vec::new()));
        }
        let ty = self.ty(ty)?;
        let v = self.expr(value, Want::Owned)?;
        Ok(Expr::path(format!("temper_core::is::<{ty}>")).call(vec![v]))
    }

    /// Whether two values are the same object, by `temper_core`'s
    /// `ptr_id`, which an interface's handle takes from its object.
    pub(crate) fn identical(&mut self, left: NodeId, right: NodeId) -> R<Expr> {
        let id = |t: &mut Translator<'a>, n: NodeId| -> R<Expr> {
            let e = t.expr(n, Want::Borrowed)?;
            Ok(Expr::path("temper_core::AnyValueTrait::ptr_id")
                .call(vec![Expr::unary(Unary::Ref, e)]))
        };
        let a = id(self, left)?;
        let b = id(self, right)?;
        Ok(Expr::bin(Bin::Eq, a, b))
    }

    /// Adds the module refs of classes' and interfaces' methods to what
    /// functions use.
    pub(crate) fn class_refs(p: &Program, module: &Module) -> Vec<DeclId> {
        let mut out = Vec::new();
        for &item in &module.items {
            if let Node::Class { .. } | Node::Interface { .. } = p.nodes.get(item) {
                out.extend(temper_tmpl::analysis::module_refs(p, item));
            }
        }
        out
    }
}

/// Whether type `ty` is or has `part`.
fn mentions(p: &Program, ty: TypeId, part: TypeId) -> bool {
    ty == part
        || match p.types.get(ty) {
            temper_tmpl::TypeData::Nominal { args, .. } => {
                p.types.list(args).iter().any(|&a| mentions(p, a, part))
            }
            temper_tmpl::TypeData::Nullable(inner) => mentions(p, inner, part),
            // A function type's parameters and result: Rust's `dyn Fn`
            // uses its type parameters.
            temper_tmpl::TypeData::Function { .. } => p.types.signature(ty).is_some_and(|s| {
                (s.params.iter().chain([&s.result])).any(|&t| mentions(p, t, part))
            }),
            _ => false,
        }
}

/// Whether any of a constructor's parameters is optional.
fn has_optional(p: &Program, params: &[NodeId]) -> bool {
    params.iter().any(|&param| {
        matches!(p.nodes.get(param), Node::Param { decl } if p.decls.get(decl).facts.contains(Facts::OPTIONAL))
    })
}
