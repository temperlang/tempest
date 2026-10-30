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

    pub(crate) fn program_mut(&mut self) -> &mut Program {
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
        });
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

    /// A call's result and flags, from its callee's signature.
    fn call_type(&self, callee: DeclId) -> (TypeId, Flags) {
        match self.p.types.signature(self.decl_type(callee)) {
            Some(sig) if sig.bubbles => (sig.result, Flags::MAY_BUBBLE),
            Some(sig) => (sig.result, Flags::NONE),
            None => (TypeId::NONE, Flags::NONE),
        }
    }

    /// Calls a function, marked if it may bubble.
    pub fn call(&mut self, callee: DeclId, args: &[NodeId]) -> NodeId {
        let (result, flags) = self.call_type(callee);
        self.raw(Kind::CallFunction, result, (callee.0, 0), flags, args)
    }

    /// Calls a method or getter on a receiver, marked if it may bubble.
    pub fn call_method(&mut self, method: DeclId, receiver: NodeId, args: &[NodeId]) -> NodeId {
        let (result, flags) = self.call_type(method);
        let kids: Vec<_> = [receiver].into_iter().chain(args.iter().copied()).collect();
        self.raw(Kind::CallMethod, result, (method.0, 0), flags, &kids)
    }

    /// The argument for an optional parameter left out, typed as the
    /// parameter.
    pub fn omitted(&mut self, ty: TypeId) -> NodeId {
        self.expr(Kind::Omitted, ty, (0, 0), &[])
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
