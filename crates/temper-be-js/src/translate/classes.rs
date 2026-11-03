//! Classes and interfaces, as Kotlin's JS backend writes them: each a JS
//! class extending `type(...)` from `@temperlang/core`, a mixin of its
//! supertypes that copies their methods onto its prototype, and which
//! `instanceof` checks. A field is stored privately, as `#name`, with a
//! getter, and a setter if it's a `var`, unless it's private; private
//! methods, getters, and setters are `#name` too. A class's constructor
//! calls `super()` first. Static fields are plain, since the module's
//! init block assigns them, and so is everything of an interface's, since
//! the mixin copies its methods to classes where its `#` names don't
//! reach.
//!
//! Where a class inherits more than one default body for a method,
//! getter, or setter, it forwards to the one Temper picks, as Kotlin's
//! does: the nearest, breadth-first, and among equally near ones, the
//! first in `extends` order (`C.prototype.a.call(this)`).
//!
//! For TypeScript's checker, an interface declares its abstract members
//! as fields with their types, initialized to an `undefined` cast to
//! `any`, which nothing stores: nothing makes an interface's instance,
//! and the mixin copies only its prototype. (TypeScript 7 no longer reads
//! a declaration without a value, `/** @type {number} */ this.area;`.)
//! TypeScript doesn't see what the mixin copies, so an upcast or downcast
//! to a type with members the value's type doesn't declare is cast
//! through `unknown`.

use super::*;
use crate::names;
use temper_tmpl::Span;

impl Translator<'_> {
    /// Writes a class or interface, after its supertypes in this module
    /// that aren't written yet.
    pub(super) fn class_after_supers(
        &mut self,
        item: NodeId,
        done: &mut HashSet<DeclId>,
        body: &mut Vec<Stmt>,
    ) -> R<()> {
        let p = self.p;
        let (Node::Class { decl, .. } | Node::Interface { decl, .. }) = p.nodes.get(item) else {
            return Ok(());
        };
        // A `@fun` interface is only its function type.
        if p.decls.get(decl).facts.contains(temper_tmpl::Facts::FUN) {
            return Ok(());
        }
        if !done.insert(decl) {
            return Ok(());
        }
        for &t in p.decls.supers(decl) {
            let up = nominal_decl(p, t).and_then(|up| self.class_nodes.get(&up).copied());
            if let Some(node) = up {
                if self.home_is_here(node) {
                    self.class_after_supers(node, done, body)?;
                }
            }
        }
        if temper_tmpl::host::uses_host(p, decl, super::NAMES) {
            body.push(self.host_class(item, decl)?);
        } else {
            body.push(self.class(item)?);
        }
        Ok(())
    }

    fn home_is_here(&self, node: NodeId) -> bool {
        match self.p.nodes.get(node) {
            Node::Class { decl, .. } | Node::Interface { decl, .. } => {
                home(self.p, decl) == Some(self.here)
            }
            _ => false,
        }
    }

    /// A class or interface, by its name, imported if it's another
    /// module's.
    pub(super) fn class_ref(&mut self, decl: DeclId) -> Expr {
        Expr::name(self.top(decl))
    }

    /// Whether a member is `#name`: private (or protected, the same in a
    /// final class) to a class, and not a static field.
    fn hidden(&self, member: DeclId) -> bool {
        let p = self.p;
        let d = p.decls.get(member);
        (d.facts.contains(Facts::PRIVATE) || d.facts.contains(Facts::PROTECTED))
            && d.kind != DeclKind::StaticField
            && p.decls.get(d.owner).kind == DeclKind::Class
    }

    /// A member's name as its uses write it: `#name` if it's hidden.
    pub(super) fn member_name(&self, member: DeclId) -> String {
        let name = self.name(member);
        if self.hidden(member) {
            format!("#{name}")
        } else {
            name
        }
    }

    /// A field as `object.name`: its storage, `#name`, in its own class,
    /// and elsewhere its getter or setter.
    pub(super) fn field(&mut self, field: DeclId, object: NodeId) -> R<Expr> {
        let object = self.expr(object)?;
        // Core's, as a pair's `key`, by its own name.
        if by_connection(self.p, field) {
            return Ok(object.member(self.p.name(field)));
        }
        let name = self.name(field);
        Ok(if self.p.decls.get(field).owner == self.class {
            object.member(format!("#{name}"))
        } else {
            object.member(name)
        })
    }

    /// A call of a method, getter, or setter of the program's own: a
    /// getter is read as a property; a setter only assigns, as a
    /// statement ([`Translator::setter_call`]).
    pub(super) fn method_call(
        &mut self,
        method: DeclId,
        receiver: NodeId,
        args: &[NodeId],
    ) -> R<Expr> {
        let receiver = self.expr(receiver)?;
        let name = self.member_name(method);
        match self.p.decls.get(method).kind {
            DeclKind::Getter => Ok(receiver.member(name)),
            DeclKind::Setter => not_yet("a setter's call as a value"),
            _ => {
                let args = self.args(args)?;
                Ok(receiver.method(name, args))
            }
        }
    }

    /// A setter's call, `receiver.name = value;`, if `expr` is one.
    pub(super) fn setter_call(&mut self, expr: NodeId) -> R<Option<Stmt>> {
        let p = self.p;
        let Node::CallMethod {
            method,
            receiver,
            args,
            ..
        } = p.nodes.get(expr)
        else {
            return Ok(None);
        };
        if by_connection(p, method) || p.decls.get(method).kind != DeclKind::Setter {
            return Ok(None);
        }
        let [value] = p.nodes.list(args)[..] else {
            return not_yet("a setter's call without one value");
        };
        let target = self.expr(receiver)?.member(self.member_name(method));
        Ok(Some(Stmt::Assign {
            target,
            value: self.expr(value)?,
        }))
    }

    /// A value as another type: itself, where TypeScript sees it's one
    /// already, or cast there through `unknown`.
    pub(super) fn converted(&mut self, value: NodeId, to: TypeId) -> R<Expr> {
        let from = self.p.nodes.ty(value);
        let e = self.expr(value)?;
        if self.assignable(from, to) {
            return Ok(e);
        }
        let ty = self.ty(to)?;
        Ok(Expr::Cast(
            ty,
            Box::new(Expr::Cast("unknown".into(), Box::new(e))),
        ))
    }

    /// Whether TypeScript takes a value of type `from` as a `to`: the same
    /// class or interface, or an interface whose members TypeScript sees
    /// `from` declare, by name.
    fn assignable(&self, from: TypeId, to: TypeId) -> bool {
        let p = self.p;
        if from == to {
            return true;
        }
        // A type parameter is what its bounds are, for TypeScript too.
        if let TypeData::TypeParam { decl } = p.types.get(from) {
            return p.decls.supers(decl).iter().any(|&b| self.assignable(b, to));
        }
        let (Some(from), Some(to)) = (nominal_decl(p, from), nominal_decl(p, to)) else {
            return false;
        };
        // Core's lists as arrays, and maps as maps.
        let q = |d: DeclId| self.qname(d);
        if matches!(
            (q(from).as_str(), q(to).as_str()),
            (
                "core.type List" | "core.type ListBuilder",
                "core.type Listed"
            ) | ("core.type Map" | "core.type MapBuilder", "core.type Mapped")
                | (
                    "core.type StringIndex" | "core.type NoStringIndex",
                    "core.type StringIndexOption"
                )
        ) {
            return true;
        }
        if p.decls.get(to).kind != DeclKind::Interface {
            return false;
        }
        match (self.seen_members(from), self.seen_members(to)) {
            (Some(from), Some(to)) => to.is_subset(&from),
            _ => false,
        }
    }

    /// The instance members TypeScript sees a class or interface of the
    /// program's own declare, by name; `None` for another library's.
    fn seen_members(&self, decl: DeclId) -> Option<HashSet<String>> {
        let p = self.p;
        let &node = self.class_nodes.get(&decl)?;
        let (Node::Class { members, .. } | Node::Interface { members, .. }) = p.nodes.get(node)
        else {
            return None;
        };
        let mut names = HashSet::new();
        for &m in p.nodes.list(members) {
            let d = match p.nodes.get(m) {
                Node::Field { decl }
                | Node::Method { decl, .. }
                | Node::AbstractMethod { decl, .. } => decl,
                _ => continue,
            };
            if !self.hidden(d) && p.decls.get(d).kind != DeclKind::Constructor {
                names.insert(self.name(d));
            }
        }
        if p.decls.get(decl).kind == DeclKind::Class {
            names.extend(self.forwarded(decl).into_iter().map(|m| self.name(m)));
        } else {
            for (m, _) in self.inherited_abstract(decl, members) {
                if let Node::AbstractMethod { decl, .. } = p.nodes.get(m) {
                    names.insert(self.name(decl));
                }
            }
        }
        Some(names)
    }

    /// The inherited default bodies a class forwards to, each the one
    /// Temper picks where it inherits more than one: the nearest,
    /// breadth-first, and among equally near ones, the first in `extends`
    /// order.
    fn forwarded(&self, class: DeclId) -> Vec<DeclId> {
        let p = self.p;
        let mut own: HashSet<String> = HashSet::new();
        if let Some(&node) = self.class_nodes.get(&class) {
            if let Node::Class { members, .. } = p.nodes.get(node) {
                for &m in p.nodes.list(members) {
                    if let Node::Field { decl } | Node::Method { decl, .. } = p.nodes.get(m) {
                        own.insert(self.name(decl));
                    }
                }
            }
        }
        // By name and kind: the first found, and how many were.
        let mut found: Vec<((String, DeclKind), DeclId, usize)> = Vec::new();
        for up in ancestors(p, class) {
            let Some(&node) = self.class_nodes.get(&up) else {
                continue;
            };
            let Node::Interface { members, .. } = p.nodes.get(node) else {
                continue;
            };
            for &m in p.nodes.list(members) {
                let Node::Method { decl, .. } = p.nodes.get(m) else {
                    continue;
                };
                let key = (self.name(decl), p.decls.get(decl).kind);
                if own.contains(&key.0) {
                    continue;
                }
                match found.iter_mut().find(|(k, _, _)| *k == key) {
                    Some((_, _, count)) => *count += 1,
                    None => found.push((key, decl, 1)),
                }
            }
        }
        (found.into_iter())
            .filter(|&(_, _, count)| count > 1)
            .map(|(_, decl, _)| decl)
            .collect()
    }

    /// The abstract members an interface inherits from the program's own
    /// interfaces above it that aren't generic, and doesn't declare: which
    /// it declares again for TypeScript, which sees nothing through the
    /// runtime's `type(I)`, so `J extends I` has `I`'s properties.
    fn inherited_abstract(&self, iface: DeclId, members: temper_tmpl::Kids) -> Vec<(NodeId, bool)> {
        let p = self.p;
        let decl_of = |m: NodeId| match p.nodes.get(m) {
            Node::Field { decl }
            | Node::Method { decl, .. }
            | Node::AbstractMethod { decl, .. } => Some(decl),
            _ => None,
        };
        let mut names: HashSet<String> = (p.nodes.list(members).iter())
            .filter_map(|&m| decl_of(m))
            .map(|d| self.name(d))
            .collect();
        let mut out = Vec::new();
        for up in ancestors(p, iface) {
            if up == iface || !p.decls.type_params(up).is_empty() {
                continue;
            }
            let Some(&node) = self.class_nodes.get(&up) else {
                continue;
            };
            let Node::Interface { members, .. } = p.nodes.get(node) else {
                continue;
            };
            let mut adds = Vec::new();
            for &m in p.nodes.list(members) {
                match p.nodes.get(m) {
                    Node::AbstractMethod { decl, .. } if !names.contains(&self.name(decl)) => {
                        adds.push((m, self.name(decl)));
                    }
                    _ => {}
                }
            }
            // A getter and its setter share a name: both, then the name.
            for (m, name) in adds {
                out.push((m, true));
                names.insert(name);
            }
        }
        out
    }

    /// A class or interface.
    fn class(&mut self, item: NodeId) -> R<Stmt> {
        let p = self.p;
        let (Node::Class { decl, members } | Node::Interface { decl, members }) = p.nodes.get(item)
        else {
            return not_yet("a class that isn't one");
        };
        let outer = std::mem::replace(&mut self.class, decl);
        let body = self.class_body(decl, members);
        self.class = outer;
        let members = body?;
        self.imports.insert("type");
        let supers: Vec<Expr> = (p.decls.supers(decl).iter())
            .filter_map(|&t| nominal_decl(p, t))
            .map(|up| self.class_ref(up))
            .collect();
        let d = p.decls.get(decl);
        Ok(Stmt::Class {
            export: self.exported(decl),
            name: self.name(decl),
            name_span: d.name_span,
            source_name: p.name(decl).to_string(),
            templates: self.templates(decl)?,
            extends: Some(Expr::name("type").call(supers)),
            members,
        }
        .spanned(p.nodes.span(item)))
    }

    fn class_body(&mut self, class: DeclId, members: temper_tmpl::Kids) -> R<Vec<Member>> {
        let p = self.p;
        let interface = p.decls.get(class).kind == DeclKind::Interface;
        let (mut fields, mut constructor, mut accessors, mut methods) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        // An interface's abstract properties, declared once for a getter
        // and its setter.
        let mut declared = HashSet::new();
        let mut all: Vec<(NodeId, bool)> =
            p.nodes.list(members).iter().map(|&m| (m, false)).collect();
        if interface {
            all.extend(self.inherited_abstract(class, members));
        }
        for (m, inherited) in all {
            let span = if inherited {
                Span::UNKNOWN
            } else {
                p.nodes.span(m)
            };
            match p.nodes.get(m) {
                Node::Field { decl } => {
                    let d = p.decls.get(decl);
                    let name = self.name(decl);
                    let ty = self.ty(d.ty)?;
                    let storage = format!("#{name}");
                    fields.push(Member::Field {
                        span,
                        is_static: false,
                        name: storage.clone(),
                        ty: ty.clone(),
                        value: None,
                    });
                    if self.hidden(decl) {
                        continue;
                    }
                    let source_name = p.name(decl).to_string();
                    accessors.push(Member::Method {
                        span,
                        is_static: false,
                        kind: MethodKind::Getter,
                        name: name.clone(),
                        name_span: d.name_span,
                        source_name: source_name.clone(),
                        params: Vec::new(),
                        result: Some(ty.clone()),
                        body: vec![Stmt::Return(Some(Expr::This.member(storage.clone())))],
                    });
                    if d.facts.contains(Facts::VAR) {
                        accessors.push(Member::Method {
                            span,
                            is_static: false,
                            kind: MethodKind::Setter,
                            name,
                            name_span: d.name_span,
                            source_name,
                            params: vec![("value".into(), ty)],
                            result: None,
                            body: vec![Stmt::Assign {
                                target: Expr::This.member(storage),
                                value: Expr::name("value"),
                            }],
                        });
                    }
                }
                Node::StaticField { decl } => fields.push(Member::Field {
                    span,
                    is_static: true,
                    name: self.name(decl),
                    ty: self.ty(p.decls.get(decl).ty)?,
                    value: None,
                }),
                Node::AbstractMethod { decl, params } => {
                    let name = self.name(decl);
                    if !interface || !declared.insert(name.clone()) {
                        continue;
                    }
                    let ty = match (p.decls.get(decl).kind, p.signature(decl)) {
                        (DeclKind::Getter, Some(sig)) => self.ty(sig.result)?,
                        (DeclKind::Setter, Some(sig)) => match sig.params.first() {
                            Some(&t) => self.ty(t)?,
                            None => return not_yet("a setter without a value"),
                        },
                        (_, Some(sig)) => {
                            let result = self.ty(sig.result)?;
                            let ps: Vec<String> = (self.params(params)?.into_iter())
                                .map(|(n, t)| format!("{n}: {t}"))
                                .collect();
                            format!("({}) => {result}", ps.join(", "))
                        }
                        (_, None) => return not_yet("a method without a signature"),
                    };
                    // A value TypeScript takes as any type, which nothing
                    // stores, since nothing makes an interface's instance.
                    let nothing = Expr::Cast("any".into(), Box::new(Expr::Undefined));
                    fields.push(Member::Field {
                        span,
                        is_static: false,
                        name,
                        ty,
                        value: Some(nothing),
                    });
                }
                Node::Method { decl, params, body } => {
                    let kids = params;
                    let d = p.decls.get(decl);
                    let kind = match d.kind {
                        DeclKind::Constructor => MethodKind::Constructor,
                        DeclKind::Getter => MethodKind::Getter,
                        DeclKind::Setter => MethodKind::Setter,
                        _ => MethodKind::Method,
                    };
                    let (params, result, mut body) = self.function_parts(m, decl, params, body)?;
                    let name = if kind == MethodKind::Constructor {
                        body.insert(0, Stmt::Expr(Expr::name("super").call(Vec::new())));
                        "constructor".to_string()
                    } else {
                        self.member_name(decl)
                    };
                    // A constructor's name isn't its source's, nor is a
                    // made one's.
                    // A constructor that only calls `super()` is the one JS
                    // makes when there's none.
                    let trivial = kind == MethodKind::Constructor
                        && params.is_empty()
                        && body.len() == 1
                        && !self.hidden(decl);
                    let made = kind == MethodKind::Constructor
                        || d.facts.contains(Facts::AUTO)
                        || d.facts.contains(Facts::COMPILER_MADE);
                    let member = Member::Method {
                        span,
                        is_static: false,
                        kind,
                        name,
                        name_span: if made { Span::UNKNOWN } else { d.name_span },
                        source_name: p.name(decl).to_string(),
                        params,
                        result,
                        body,
                    };
                    if kind == MethodKind::Constructor {
                        if trivial {
                            continue;
                        }
                        if !self.hidden(decl) {
                            constructor.extend(self.factory(class, kids)?);
                        }
                        constructor.push(member);
                    } else {
                        methods.push(member);
                    }
                }
                Node::Function { decl, params, body } => {
                    let (params, result, body) = self.function_parts(m, decl, params, body)?;
                    methods.push(Member::Method {
                        span,
                        is_static: true,
                        kind: MethodKind::Method,
                        name: self.member_name(decl),
                        name_span: p.decls.get(decl).name_span,
                        source_name: p.name(decl).to_string(),
                        params,
                        result,
                        body,
                    });
                }
                other => return not_yet(format!("a {} member", other.kind().name())),
            }
        }
        if !interface {
            for winner in self.forwarded(class) {
                methods.push(self.forwarder(winner)?);
            }
        }
        fields.extend(constructor);
        fields.extend(accessors);
        fields.extend(methods);
        Ok(fields)
    }

    /// A public constructor's factory, as Kotlin's: with two or more
    /// parameters, `static new(props)`, which takes them by name, as the
    /// source names them (`C.new({ x: 1, y: 2 })`).
    fn factory(&mut self, class: DeclId, params: temper_tmpl::Kids) -> R<Option<Member>> {
        let p = self.p;
        let params = p.nodes.list(params);
        if params.len() < 2 {
            return Ok(None);
        }
        let props = Expr::name("props");
        let mut args = Vec::new();
        let mut types = Vec::new();
        for &param in params {
            let Node::Param { decl } = p.nodes.get(param) else {
                return not_yet("a constructor's parameter");
            };
            let d = p.decls.get(decl);
            let source = p.name(decl);
            let key = if !source.is_empty()
                && (source.chars().enumerate()).all(|(i, c)| (names::RULES.identifier)(c, i == 0))
            {
                source.to_string()
            } else {
                self.name(decl)
            };
            let optional = if d.facts.contains(Facts::OPTIONAL) {
                "?"
            } else {
                ""
            };
            types.push(format!("{key}{optional}: {}", self.ty(d.ty)?));
            args.push(props.clone().member(key));
        }
        let name = self.name(class);
        // A generic class's factory is generic itself, since a static
        // member can't name the class's type parameters.
        let mut ps: Vec<(String, String)> = (self.templates(class)?.into_iter())
            .map(|t| ("@template".to_string(), t))
            .collect();
        let uses: Vec<String> = (p.decls.type_params(class).iter())
            .map(|&t| p.name(t).to_string())
            .collect();
        let result = match uses.as_slice() {
            [] => name.clone(),
            _ => format!("{name}<{}>", uses.join(", ")),
        };
        ps.push(("props".into(), format!("{{ {} }}", types.join(", "))));
        Ok(Some(Member::Method {
            span: Span::UNKNOWN,
            is_static: true,
            kind: MethodKind::Method,
            name: "new".into(),
            name_span: Span::UNKNOWN,
            source_name: String::new(),
            params: ps,
            result: Some(result),
            body: vec![Stmt::Return(Some(Expr::New(
                Box::new(Expr::name(name)),
                args,
            )))],
        }))
    }

    /// A class's member that forwards to an inherited default body:
    /// `C.prototype.a.call(this, x)`, or for a getter or setter,
    /// `Reflect.get(C.prototype, "a", this)` or `Reflect.set(…)`.
    fn forwarder(&mut self, winner: DeclId) -> R<Member> {
        let p = self.p;
        let d = p.decls.get(winner);
        let name = self.name(winner);
        let Some(&node) = self.class_nodes.get(&d.owner) else {
            return not_yet("a forwarder to another library's interface");
        };
        let Node::Interface { members, .. } = p.nodes.get(node) else {
            return not_yet("a forwarder to a class");
        };
        let params = (p.nodes.list(members).iter())
            .find_map(|&m| match p.nodes.get(m) {
                Node::Method { decl, params, .. } if decl == winner => Some(params),
                _ => None,
            })
            .map(|params| self.params(params))
            .transpose()?
            .unwrap_or_default();
        let result = match p.signature(winner) {
            Some(sig) if sig.result != TypeId::VOID => Some(self.ty(sig.result)?),
            _ => None,
        };
        let prototype = self.class_ref(d.owner).member("prototype");
        let this = Expr::This;
        let (kind, body) = match d.kind {
            DeclKind::Getter => (
                MethodKind::Getter,
                vec![Stmt::Return(Some(Expr::name("Reflect").method(
                    "get",
                    vec![prototype, Expr::Str(name.clone()), this],
                )))],
            ),
            DeclKind::Setter => {
                let value = Expr::name(params.first().map_or("value", |(n, _)| n.as_str()));
                (
                    MethodKind::Setter,
                    vec![Stmt::Expr(Expr::name("Reflect").method(
                        "set",
                        vec![prototype, Expr::Str(name.clone()), value, this],
                    ))],
                )
            }
            _ => {
                let mut args = vec![this];
                args.extend(params.iter().map(|(n, _)| Expr::name(n.clone())));
                let call = prototype.member(name.clone()).method("call", args);
                let stmt = if result.is_some() {
                    Stmt::Return(Some(call))
                } else {
                    Stmt::Expr(call)
                };
                (MethodKind::Method, vec![stmt])
            }
        };
        Ok(Member::Method {
            span: Span::UNKNOWN,
            is_static: false,
            kind,
            name,
            name_span: Span::UNKNOWN,
            source_name: String::new(),
            params,
            result,
            body,
        })
    }
}
