//! Building programs: the frontend's way in, and the text reader's.
//!
//! Children come first, so a child's id is always lower than its
//! parent's. Each method gives its node the type the local rules say
//! (a literal's, a name's declaration's, a call's signature's result) and
//! marks calls that may bubble, so callers pass a type only where no rule
//! gives one. The builder never fails: whether the result makes sense
//! (argument types, scopes, assignments, returns) is the validator's to
//! say.
//!
//! ```
//! use temper_tmpl::{Builder, DeclKind, Facts, Profile, TypeId, ops::Op, text, validate};
//!
//! let mut b = Builder::new("example", "0.1.0");
//! let module = b.module("example");
//! let sig = b.function_type(&[TypeId::INT32], TypeId::INT32, false);
//! let twice = b.declare(DeclKind::Function, "twice", module, sig);
//! b.add_facts(twice, Facts::EXPORTED);
//! let n = b.declare(DeclKind::Param, "n", twice, TypeId::INT32);
//! let param = b.param(n);
//! let (read, two) = (b.local(n), b.int32(2));
//! let product = b.op(Op::named("Int32.mul").unwrap(), &[read, two]);
//! let ret = b.return_stmt(Some(product));
//! let body = b.block(&[ret]);
//! let function = b.function(twice, &[param], body);
//! b.add_item(module, function);
//! let init = b.block(&[]);
//! b.set_init(module, init);
//! let program = b.finish();
//!
//! assert!(validate(&program, &Profile::full()).is_empty());
//! assert!(text::write(&program).contains("export fn twice#1(n#2: Int32): Int32 {"));
//! ```

use crate::decls::{DeclKind, Facts};
use crate::ids::{DeclId, FileId, NodeId, SourceFile, Span, TypeId};
use crate::nodes::Flags;
use crate::ops::Op;
use crate::qname::{core_scalar, qname_library};
use crate::schema::{Kind, split_bits};
use crate::{Library, Module, Program, Uses};

/// Builds a [`Program`].
pub struct Builder {
    p: Program,
}

impl Builder {
    pub fn new(library: &str, version: &str) -> Builder {
        Builder {
            p: Program {
                library: Library {
                    name: library.into(),
                    version: version.into(),
                },
                profile: None,
                uses: Vec::new(),
                modules: Vec::new(),
                nodes: Default::default(),
                types: Default::default(),
                decls: Default::default(),
                strings: Default::default(),
                files: Vec::new(),
                has_spans: false,
            },
        }
    }

    pub fn finish(self) -> Program {
        self.p
    }

    /// The program so far.
    pub fn program(&self) -> &Program {
        &self.p
    }

    pub fn program_mut(&mut self) -> &mut Program {
        &mut self.p
    }

    /// Names the profile the program is lowered for.
    pub fn set_profile(&mut self, profile: &str) {
        self.p.profile = Some(profile.into());
    }

    // Types and declarations.

    /// A function's signature, all its parameters required.
    pub fn function_type(&mut self, params: &[TypeId], result: TypeId, bubbles: bool) -> TypeId {
        self.p.types.function(params, result, bubbles)
    }

    /// A function's signature; `optional` says which parameters are.
    pub fn function_type_with(
        &mut self,
        params: &[TypeId],
        optional: &[bool],
        result: TypeId,
        bubbles: bool,
    ) -> TypeId {
        self.p
            .types
            .function_with(params, optional, result, bubbles)
    }

    /// A class's or interface's type.
    pub fn nominal(&mut self, decl: DeclId) -> TypeId {
        self.p.types.nominal(decl)
    }

    /// A generic class's or interface's type with type arguments.
    pub fn applied(&mut self, decl: DeclId, args: &[TypeId]) -> TypeId {
        self.p.types.applied(decl, args)
    }

    /// Declares a type parameter of `owner`, with its bounds; call
    /// [`Builder::set_type_params`] with all of `owner`'s, in order.
    pub fn declare_type_param(&mut self, name: &str, owner: DeclId, bounds: &[TypeId]) -> DeclId {
        let decl = self.declare(DeclKind::TypeParam, name, owner, TypeId::NONE);
        let ty = self.p.types.type_param(decl);
        self.p.decls.get_mut(decl).ty = ty;
        self.p.decls.set_supers(decl, bounds);
        decl
    }

    /// Sets a declaration's type, as a function's signature once its type
    /// parameters, which the signature mentions, are declared.
    pub fn set_decl_type(&mut self, decl: DeclId, ty: TypeId) {
        self.p.decls.get_mut(decl).ty = ty;
    }

    /// A generic declaration's type parameters, in order. A class's or
    /// interface's type becomes itself applied to them.
    pub fn set_type_params(&mut self, decl: DeclId, params: &[DeclId]) {
        self.p.decls.set_type_params(decl, params);
        let d = self.p.decls.get(decl);
        if matches!(d.kind, DeclKind::Class | DeclKind::Interface) && !params.is_empty() {
            let args: Vec<TypeId> = params.iter().map(|&t| self.p.decls.get(t).ty).collect();
            let ty = self.p.types.applied(decl, &args);
            self.p.decls.get_mut(decl).ty = ty;
        }
    }

    /// Sets a type parameter's bounds, once the types they mention exist.
    pub fn set_bounds(&mut self, param: DeclId, bounds: &[TypeId]) {
        self.p.decls.set_supers(param, bounds);
    }

    /// `T?`, or `ty` itself if it's nullable already.
    pub fn nullable(&mut self, ty: TypeId) -> TypeId {
        self.p.types.nullable(ty)
    }

    /// A library this one uses; external declarations are owned by it.
    pub fn uses(&mut self, library: &str, version: &str) -> DeclId {
        let decl = self.declare(DeclKind::Library, library, DeclId::NONE, TypeId::NONE);
        self.add_uses(decl, version);
        decl
    }

    pub(crate) fn add_uses(&mut self, decl: DeclId, version: &str) {
        self.p.uses.push(Uses {
            decl,
            version: version.into(),
        });
    }

    /// A new module, whose init block is set later.
    pub fn module(&mut self, path: &str) -> DeclId {
        let decl = self.declare(DeclKind::Module, path, DeclId::NONE, TypeId::NONE);
        self.add_module(decl);
        decl
    }

    /// Adds a module for a declared module.
    pub(crate) fn add_module(&mut self, decl: DeclId) {
        self.p.modules.push(Module {
            decl,
            path: String::new(),
            items: Vec::new(),
            init: NodeId::NONE,
            tests: Vec::new(),
        });
    }

    /// Another library's module, by its path there (empty for its root),
    /// owned by that library's `uses` declaration.
    pub fn external_module(&mut self, library: DeclId, path: &str) -> DeclId {
        self.declare(DeclKind::Module, path, library, TypeId::NONE)
    }

    /// Declares something with a source name.
    pub fn declare(&mut self, kind: DeclKind, name: &str, owner: DeclId, ty: TypeId) -> DeclId {
        let id = DeclId(self.p.decls.len() as u32);
        self.define(id, kind, &[name], owner, ty);
        id
    }

    /// Declares a class or interface with its own type: for core's class
    /// of a scalar, as `Int32` in `temper-core`, the scalar; otherwise its
    /// nominal type.
    pub fn declare_type(&mut self, kind: DeclKind, name: &str, owner: DeclId) -> DeclId {
        let decl = self.declare(kind, name, owner, TypeId::NONE);
        self.give_type(decl);
        decl
    }

    /// Gives a class or interface its own type, as [`declare_type`]
    /// does.
    ///
    /// [`declare_type`]: Builder::declare_type
    pub(crate) fn give_type(&mut self, decl: DeclId) {
        let p = &self.p;
        let d = p.decls.get(decl);
        let in_core = p.decls.contains(d.owner)
            && p.decls.get(d.owner).kind == DeclKind::Library
            && qname_library(p.name(d.owner)) == "core";
        let scalar = (d.kind == DeclKind::Class && in_core)
            .then(|| core_scalar(p.name(decl)))
            .flatten();
        let ty = match scalar {
            Some(ty) => ty,
            None => self.p.types.nominal(decl),
        };
        self.p.decls.get_mut(decl).ty = ty;
    }

    /// Declares something the compiler made, named by hint candidates in
    /// preference order.
    pub fn declare_made(
        &mut self,
        kind: DeclKind,
        hints: &[&str],
        owner: DeclId,
        ty: TypeId,
    ) -> DeclId {
        let id = DeclId(self.p.decls.len() as u32);
        self.define(id, kind, hints, owner, ty);
        self.add_facts(id, Facts::COMPILER_MADE);
        id
    }

    /// Defines a declaration with a given id, as text gives them.
    pub(crate) fn define(
        &mut self,
        id: DeclId,
        kind: DeclKind,
        names: &[&str],
        owner: DeclId,
        ty: TypeId,
    ) {
        let names: Vec<_> = names.iter().map(|n| self.p.strings.intern(n)).collect();
        self.p.decls.define(id, kind, &names, owner, ty);
    }

    pub fn add_facts(&mut self, decl: DeclId, facts: Facts) {
        let d = self.p.decls.get_mut(decl);
        d.facts = d.facts | facts;
    }

    pub fn set_doc(&mut self, decl: DeclId, doc: &str) {
        self.p.decls.get_mut(decl).doc = self.p.strings.intern(doc);
    }

    /// Overrides a declaration's derived qname with what follows the
    /// module's path, as `f().x=#1`.
    pub fn set_qname(&mut self, decl: DeclId, qname: &str) {
        self.p.decls.get_mut(decl).qname = self.p.strings.intern(qname);
    }

    /// Marks a declaration connected; `backends` lists those with host
    /// code, the rest using its body (empty: every backend).
    pub fn set_connected(&mut self, decl: DeclId, backends: &[&str]) {
        self.add_facts(decl, Facts::CONNECTED);
        let backends: Vec<_> = backends.iter().map(|b| self.p.strings.intern(b)).collect();
        self.p.decls.set_backends(decl, &backends);
    }

    /// Names an external function's, method's, or getter's parameters.
    pub fn set_param_names(&mut self, decl: DeclId, names: &[&str]) {
        let names: Vec<_> = names.iter().map(|n| self.p.strings.intern(n)).collect();
        self.p.decls.set_param_names(decl, &names);
    }

    // Modules.

    /// Sets a module's directory relative to the library's root.
    pub fn set_module_path(&mut self, module: DeclId, path: &str) {
        self.module_mut(module).path = path.into();
    }

    pub fn add_item(&mut self, module: DeclId, item: NodeId) {
        self.module_mut(module).items.push(item);
    }

    pub fn set_init(&mut self, module: DeclId, block: NodeId) {
        self.module_mut(module).init = block;
    }

    fn module_mut(&mut self, module: DeclId) -> &mut Module {
        let modules = self.p.modules.iter_mut();
        modules
            .rev()
            .find(|m| m.decl == module)
            .expect("not a module")
    }

    // Spans.

    /// Adds a source file that spans point into; a program with files
    /// carries spans.
    pub fn add_file(&mut self, path: &str, hash: [u8; 16]) -> FileId {
        self.p.has_spans = true;
        self.p.files.push(SourceFile {
            path: path.into(),
            hash,
            line_starts: Vec::new(),
            wide_lines: Vec::new(),
        });
        FileId(self.p.files.len() as u32 - 1)
    }

    /// Gives a source file its line table from its text, so its spans
    /// have lines and columns, as source maps need.
    pub fn set_file_text(&mut self, file: FileId, text: &str) {
        self.p.files[file.index()].set_text(text);
    }

    /// Sets a node's span; `derived` if it points at what caused the node
    /// rather than at its own source.
    pub fn set_span(&mut self, node: NodeId, span: Span, derived: bool) {
        self.p.nodes.set_span(node, span);
        if derived {
            self.p.nodes.add_flags(node, Flags::DERIVED);
        }
    }

    /// Sets a declaration's spans: the whole declaration, and its name.
    pub fn set_decl_spans(&mut self, decl: DeclId, span: Span, name_span: Span) {
        let d = self.p.decls.get_mut(decl);
        d.span = span;
        d.name_span = name_span;
    }

    // Nodes, unchecked.

    /// Adds any node as given, unchecked: for passes that copy nodes, and
    /// for tests of the validator.
    pub fn raw(
        &mut self,
        kind: Kind,
        ty: TypeId,
        slots: (u32, u32),
        flags: Flags,
        kids: &[NodeId],
    ) -> NodeId {
        self.p.nodes.push(kind, ty, slots, flags, kids)
    }

    fn node(&mut self, kind: Kind, slots: (u32, u32), kids: &[NodeId]) -> NodeId {
        self.raw(kind, TypeId::NONE, slots, Flags::NONE, kids)
    }

    fn expr(&mut self, kind: Kind, ty: TypeId, slots: (u32, u32), kids: &[NodeId]) -> NodeId {
        self.raw(kind, ty, slots, Flags::NONE, kids)
    }

    fn decl_type(&self, decl: DeclId) -> TypeId {
        if self.p.decls.contains(decl) {
            self.p.decls.get(decl).ty
        } else {
            TypeId::NONE
        }
    }

    // Declaration nodes.

    pub fn function(&mut self, decl: DeclId, params: &[NodeId], body: NodeId) -> NodeId {
        let kids: Vec<_> = params.iter().copied().chain([body]).collect();
        self.node(Kind::Function, (decl.0, 0), &kids)
    }

    /// A test, named `name` in reports, whose declaration is a function
    /// taking std's `Test`.
    pub fn test(&mut self, decl: DeclId, name: &str, params: &[NodeId], body: NodeId) -> NodeId {
        let name = self.p.strings.intern(name);
        let kids: Vec<_> = params.iter().copied().chain([body]).collect();
        self.node(Kind::Test, (decl.0, name.0), &kids)
    }

    /// Adds a test to a module, after those already there.
    pub fn add_test(&mut self, module: DeclId, test: NodeId) {
        self.module_mut(module).tests.push(test);
    }

    /// A function declared without a body (connected).
    pub fn bodiless_function(&mut self, decl: DeclId, params: &[NodeId]) -> NodeId {
        self.node(Kind::BodilessFunction, (decl.0, 0), params)
    }

    pub fn param(&mut self, decl: DeclId) -> NodeId {
        self.node(Kind::Param, (decl.0, 0), &[])
    }

    pub fn module_let(&mut self, decl: DeclId) -> NodeId {
        self.node(Kind::ModuleLet, (decl.0, 0), &[])
    }

    /// Sets a class's or interface's supertypes.
    pub fn set_supers(&mut self, decl: DeclId, supers: &[TypeId]) {
        self.p.decls.set_supers(decl, supers);
    }

    /// A class, or an interface, by its declaration's kind, with its
    /// members.
    pub fn class(&mut self, decl: DeclId, members: &[NodeId]) -> NodeId {
        let kind = match self
            .p
            .decls
            .contains(decl)
            .then(|| self.p.decls.get(decl).kind)
        {
            Some(DeclKind::Interface) => Kind::Interface,
            _ => Kind::Class,
        };
        self.node(kind, (decl.0, 0), members)
    }

    pub fn field(&mut self, decl: DeclId) -> NodeId {
        self.node(Kind::Field, (decl.0, 0), &[])
    }

    pub fn static_field(&mut self, decl: DeclId) -> NodeId {
        self.node(Kind::StaticField, (decl.0, 0), &[])
    }

    /// A method, getter, setter, or constructor, with its body.
    pub fn method(&mut self, decl: DeclId, params: &[NodeId], body: NodeId) -> NodeId {
        let kids: Vec<_> = params.iter().copied().chain([body]).collect();
        self.node(Kind::Method, (decl.0, 0), &kids)
    }

    /// An interface's method, getter, or setter without a body.
    pub fn abstract_method(&mut self, decl: DeclId, params: &[NodeId]) -> NodeId {
        self.node(Kind::AbstractMethod, (decl.0, 0), params)
    }

    // Statements.

    pub fn block(&mut self, stmts: &[NodeId]) -> NodeId {
        self.node(Kind::Block, (0, 0), stmts)
    }

    pub fn let_stmt(&mut self, local: DeclId, init: Option<NodeId>) -> NodeId {
        let kids: Vec<_> = init.into_iter().collect();
        self.node(Kind::Let, (local.0, 0), &kids)
    }

    pub fn assign(&mut self, local: DeclId, value: NodeId) -> NodeId {
        self.node(Kind::Assign, (local.0, 0), &[value])
    }

    pub fn set_module_let(&mut self, target: DeclId, value: NodeId) -> NodeId {
        self.node(Kind::SetModuleLet, (target.0, 0), &[value])
    }

    pub fn init_let(&mut self, target: DeclId, value: NodeId) -> NodeId {
        self.node(Kind::InitLet, (target.0, 0), &[value])
    }

    pub fn if_stmt(&mut self, cond: NodeId, then: NodeId, els: Option<NodeId>) -> NodeId {
        let kids: Vec<_> = [cond, then].into_iter().chain(els).collect();
        self.node(Kind::If, (0, 0), &kids)
    }

    pub fn while_loop(&mut self, label: DeclId, cond: NodeId, body: NodeId) -> NodeId {
        self.node(Kind::While, (label.0, 0), &[cond, body])
    }

    pub fn do_while(&mut self, label: DeclId, body: NodeId, cond: NodeId) -> NodeId {
        self.node(Kind::DoWhile, (label.0, 0), &[body, cond])
    }

    pub fn break_loop(&mut self, label: DeclId) -> NodeId {
        self.node(Kind::Break, (label.0, 0), &[])
    }

    pub fn continue_loop(&mut self, label: DeclId) -> NodeId {
        self.node(Kind::Continue, (label.0, 0), &[])
    }

    pub fn return_stmt(&mut self, value: Option<NodeId>) -> NodeId {
        let kids: Vec<_> = value.into_iter().collect();
        self.node(Kind::Return, (0, 0), &kids)
    }

    pub fn expr_statement(&mut self, expr: NodeId) -> NodeId {
        self.node(Kind::ExprStatement, (0, 0), &[expr])
    }

    pub fn set_field(&mut self, field: DeclId, object: NodeId, value: NodeId) -> NodeId {
        self.node(Kind::SetField, (field.0, 0), &[object, value])
    }

    pub fn init_static(&mut self, target: DeclId, value: NodeId) -> NodeId {
        self.node(Kind::InitStatic, (target.0, 0), &[value])
    }

    // Expressions.

    pub fn bool_lit(&mut self, value: bool) -> NodeId {
        self.expr(Kind::BoolLit, TypeId::BOOLEAN, (value as u32, 0), &[])
    }

    pub fn int32(&mut self, value: i32) -> NodeId {
        self.expr(Kind::Int32Lit, TypeId::INT32, (value as u32, 0), &[])
    }

    pub fn int64(&mut self, value: i64) -> NodeId {
        self.expr(Kind::Int64Lit, TypeId::INT64, split_bits(value as u64), &[])
    }

    /// A `Float64`; every NaN becomes the one text writes as `nan`.
    pub fn float64(&mut self, value: f64) -> NodeId {
        let value = if value.is_nan() { f64::NAN } else { value };
        let slots = split_bits(value.to_bits());
        self.expr(Kind::Float64Lit, TypeId::FLOAT64, slots, &[])
    }

    pub fn string(&mut self, value: &str) -> NodeId {
        let id = self.p.strings.intern(value);
        self.expr(Kind::StringLit, TypeId::STRING, (id.0, 0), &[])
    }

    pub fn void(&mut self) -> NodeId {
        self.expr(Kind::VoidLit, TypeId::VOID, (0, 0), &[])
    }

    /// Reads a local or parameter.
    pub fn local(&mut self, local: DeclId) -> NodeId {
        let ty = self.decl_type(local);
        self.expr(Kind::Local, ty, (local.0, 0), &[])
    }

    pub fn get_module_let(&mut self, target: DeclId) -> NodeId {
        let ty = self.decl_type(target);
        self.expr(Kind::GetModuleLet, ty, (target.0, 0), &[])
    }

    /// A call's result and flags, from its callee's signature, with the
    /// type parameters it binds put in.
    fn call_type(
        &mut self,
        callee: DeclId,
        receiver: Option<NodeId>,
        type_args: &[TypeId],
    ) -> (TypeId, Flags) {
        let (result, flags) = match self.p.types.signature(self.decl_type(callee)) {
            Some(sig) if sig.bubbles => (sig.result, Flags::MAY_BUBBLE),
            Some(sig) => (sig.result, Flags::NONE),
            None => return (TypeId::NONE, Flags::NONE),
        };
        let receiver = receiver.map(|r| self.p.nodes.ty(r));
        let (from, to) = crate::analysis::bindings(&self.p, callee, receiver, type_args);
        (self.p.types.substitute(result, &from, &to), flags)
    }

    /// Calls a function, marked if it may bubble.
    pub fn call(&mut self, callee: DeclId, args: &[NodeId]) -> NodeId {
        self.call_generic(callee, &[], args)
    }

    /// Calls a generic function with its type arguments, marked if it may
    /// bubble.
    pub fn call_generic(
        &mut self,
        callee: DeclId,
        type_args: &[TypeId],
        args: &[NodeId],
    ) -> NodeId {
        let (result, flags) = self.call_type(callee, None, type_args);
        let ta = self.p.types.type_args(type_args);
        self.raw(Kind::CallFunction, result, (callee.0, ta.0), flags, args)
    }

    /// Calls a method or getter on a receiver, marked if it may bubble.
    pub fn call_method(&mut self, method: DeclId, receiver: NodeId, args: &[NodeId]) -> NodeId {
        self.call_method_generic(method, &[], receiver, args)
    }

    /// Calls a method on a receiver with the method's own type arguments,
    /// marked if it may bubble.
    pub fn call_method_generic(
        &mut self,
        method: DeclId,
        type_args: &[TypeId],
        receiver: NodeId,
        args: &[NodeId],
    ) -> NodeId {
        let (result, flags) = self.call_type(method, Some(receiver), type_args);
        let ta = self.p.types.type_args(type_args);
        let kids: Vec<_> = [receiver].into_iter().chain(args.iter().copied()).collect();
        self.raw(Kind::CallMethod, result, (method.0, ta.0), flags, &kids)
    }

    /// `this`, typed as its class or interface.
    pub fn this(&mut self, ty: TypeId) -> NodeId {
        self.expr(Kind::This, ty, (0, 0), &[])
    }

    /// Reads a field, typed with its class's type parameters as the
    /// object's type gives them.
    pub fn get_field(&mut self, field: DeclId, object: NodeId) -> NodeId {
        let ty = self.decl_type(field);
        let receiver = Some(self.p.nodes.ty(object));
        let (from, to) = crate::analysis::bindings(&self.p, field, receiver, &[]);
        let ty = self.p.types.substitute(ty, &from, &to);
        self.expr(Kind::GetField, ty, (field.0, 0), &[object])
    }

    pub fn get_static(&mut self, target: DeclId) -> NodeId {
        let ty = self.decl_type(target);
        self.expr(Kind::GetStatic, ty, (target.0, 0), &[])
    }

    /// Calls a constructor, marked if it may bubble.
    pub fn new_object(&mut self, ctor: DeclId, args: &[NodeId]) -> NodeId {
        self.new_generic(ctor, &[], args)
    }

    /// Calls a generic class's constructor with the class's type
    /// arguments, marked if it may bubble.
    pub fn new_generic(&mut self, ctor: DeclId, type_args: &[TypeId], args: &[NodeId]) -> NodeId {
        let (result, flags) = self.call_type(ctor, None, type_args);
        let ta = self.p.types.type_args(type_args);
        self.raw(Kind::New, result, (ctor.0, ta.0), flags, args)
    }

    /// A list's literal, typed `ty`, a `List<T>`.
    pub fn list_lit(&mut self, ty: TypeId, items: &[NodeId]) -> NodeId {
        self.expr(Kind::ListLit, ty, (0, 0), items)
    }

    pub fn upcast(&mut self, ty: TypeId, value: NodeId) -> NodeId {
        self.expr(Kind::Upcast, ty, (0, 0), &[value])
    }

    pub fn downcast(&mut self, ty: TypeId, value: NodeId) -> NodeId {
        self.expr(Kind::Downcast, ty, (0, 0), &[value])
    }

    /// `value as ty`, checked, bubbling if it fails.
    pub fn as_cast(&mut self, ty: TypeId, value: NodeId) -> NodeId {
        self.raw(Kind::As, ty, (0, 0), Flags::MAY_BUBBLE, &[value])
    }

    /// Whether `value` is of type `ty`, a class or interface.
    pub fn is(&mut self, ty: TypeId, value: NodeId) -> NodeId {
        self.expr(Kind::Is, TypeId::BOOLEAN, (ty.0, 0), &[value])
    }

    pub fn identical(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.expr(Kind::Identical, TypeId::BOOLEAN, (0, 0), &[left, right])
    }

    /// The argument for an optional parameter left out, typed as the
    /// parameter.
    pub fn omitted(&mut self, ty: TypeId) -> NodeId {
        self.expr(Kind::Omitted, ty, (0, 0), &[])
    }

    /// Gives a node typed by its place (`Null`, `Bubble`, `Panic`, or
    /// `Error`) its type, once the place is known; other nodes keep theirs.
    pub fn retype(&mut self, node: NodeId, ty: TypeId) {
        let kind = self.p.nodes.kind(node);
        if matches!(kind, Kind::Null | Kind::Bubble | Kind::Panic | Kind::Error) {
            self.p.nodes.set_ty(node, ty);
        }
    }

    /// `null`, typed as the nullable type its place wants.
    pub fn null(&mut self, ty: TypeId) -> NodeId {
        self.expr(Kind::Null, ty, (0, 0), &[])
    }

    pub fn is_null(&mut self, value: NodeId) -> NodeId {
        self.expr(Kind::IsNull, TypeId::BOOLEAN, (0, 0), &[value])
    }

    /// A nullable value as its non-null type, where flow facts prove it.
    pub fn non_null(&mut self, value: NodeId) -> NodeId {
        let ty = self.p.nodes.ty(value);
        let ty = self.p.types.non_null(ty);
        self.expr(Kind::NonNull, ty, (0, 0), &[value])
    }

    /// A value as its type's nullable type.
    pub fn as_nullable(&mut self, value: NodeId) -> NodeId {
        let ty = self.p.nodes.ty(value);
        let ty = self.p.types.nullable(ty);
        self.expr(Kind::AsNullable, ty, (0, 0), &[value])
    }

    /// `left == right` where either may be null, with `op` the equality
    /// of their non-null values.
    pub fn eq_nullable(&mut self, op: Op, left: NodeId, right: NodeId) -> NodeId {
        self.expr(Kind::EqNullable, TypeId::BOOLEAN, (op.0, 0), &[left, right])
    }

    pub fn or_else(&mut self, ty: TypeId, body: NodeId, fallback: NodeId) -> NodeId {
        self.expr(Kind::OrElse, ty, (0, 0), &[body, fallback])
    }

    /// `bubble()`, typed as its place wants.
    pub fn bubble(&mut self, ty: TypeId) -> NodeId {
        self.raw(Kind::Bubble, ty, (0, 0), Flags::MAY_BUBBLE, &[])
    }

    /// `panic()`, typed as its place wants.
    pub fn panic(&mut self, ty: TypeId) -> NodeId {
        self.expr(Kind::Panic, ty, (0, 0), &[])
    }

    /// Calls an operator, marked if it may bubble.
    pub fn op(&mut self, op: Op, args: &[NodeId]) -> NodeId {
        let (result, flags) = match op.info() {
            Some(info) if info.bubbles => (info.result, Flags::MAY_BUBBLE),
            Some(info) => (info.result, Flags::NONE),
            None => (TypeId::NONE, Flags::NONE),
        };
        self.raw(Kind::Op, result, (op.0, 0), flags, args)
    }

    pub fn and(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.expr(Kind::And, TypeId::BOOLEAN, (0, 0), &[left, right])
    }

    pub fn or(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.expr(Kind::Or, TypeId::BOOLEAN, (0, 0), &[left, right])
    }

    pub fn if_value(&mut self, ty: TypeId, cond: NodeId, then: NodeId, els: NodeId) -> NodeId {
        self.expr(Kind::IfValue, ty, (0, 0), &[cond, then, els])
    }

    pub fn block_value(&mut self, ty: TypeId, stmts: &[NodeId], value: NodeId) -> NodeId {
        let kids: Vec<_> = stmts.iter().copied().chain([value]).collect();
        self.expr(Kind::BlockValue, ty, (0, 0), &kids)
    }

    /// Code the frontend couldn't settle, typed as its place wants.
    pub fn error(&mut self, ty: TypeId, message: &str) -> NodeId {
        let id = self.p.strings.intern(message);
        self.expr(Kind::Error, ty, (id.0, 0), &[])
    }
}
