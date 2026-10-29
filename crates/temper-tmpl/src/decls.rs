//! Declarations and interned strings (notes/tmpl/names.md, Level 3).

use crate::ids::{DeclId, Span, StrId, TypeId};
use std::collections::HashMap;

/// Interned strings: names, string literals, messages, and docs.
#[derive(Clone, Debug, Default)]
pub struct Strings {
    list: Vec<String>,
    ids: HashMap<String, StrId>,
}

impl Strings {
    pub fn intern(&mut self, text: &str) -> StrId {
        if let Some(&id) = self.ids.get(text) {
            return id;
        }
        let id = StrId(self.list.len() as u32);
        self.list.push(text.to_string());
        self.ids.insert(text.to_string(), id);
        id
    }

    pub fn get(&self, id: StrId) -> &str {
        &self.list[id.index()]
    }

    pub fn contains(&self, id: StrId) -> bool {
        id.index() < self.list.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeclKind {
    Module,
    /// Another library, which a `uses` section names; it owns that
    /// section's external declarations.
    Library,
    /// A class: from a used library, so far (subset B adds the program's
    /// own).
    Class,
    /// An interface: from a used library, so far.
    Interface,
    /// A top-level function, or a static one owned by a class or
    /// interface.
    Function,
    /// A method of a class or interface, called with a receiver.
    Method,
    /// A getter of a class or interface, called with a receiver and no
    /// arguments.
    Getter,
    /// A module `let`, `var` if marked.
    ModuleLet,
    Param,
    /// A local `let`, `var` if marked; owned by its function, or by its
    /// module in an init block.
    Local,
    /// A loop's label, owned like a local.
    Label,
    /// An id that nothing defines: a pass's tombstone, or a gap in text.
    Removed,
}

impl DeclKind {
    /// Its name in messages.
    pub fn name(self) -> &'static str {
        match self {
            DeclKind::Module => "module",
            DeclKind::Library => "library",
            DeclKind::Class => "class",
            DeclKind::Interface => "interface",
            DeclKind::Function => "function",
            DeclKind::Method => "method",
            DeclKind::Getter => "getter",
            DeclKind::ModuleLet => "module let",
            DeclKind::Param => "parameter",
            DeclKind::Local => "local",
            DeclKind::Label => "label",
            DeclKind::Removed => "removed declaration",
        }
    }
}

/// Facts about a declaration, read once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Facts(pub u16);

impl Facts {
    pub const NONE: Facts = Facts(0);
    /// May be assigned again: a `var`.
    pub const VAR: Facts = Facts(1);
    pub const EXPORTED: Facts = Facts(2);
    /// Made by the compiler: its name and alternatives are hints.
    pub const COMPILER_MADE: Facts = Facts(4);
    /// A parameter with a default, which a call may pass as `Omitted`;
    /// the signature says so too.
    pub const OPTIONAL: Facts = Facts(8);
    /// Marked `@keep`: its name is claimed first and never changed.
    pub const KEEP: Facts = Facts(16);
    /// Marked `@connected` (or core's equivalent). How it resolves follows
    /// from its library ([`resolves_by_qname`](crate::qname::resolves_by_qname));
    /// a body, if any, is its pure Temper fallback.
    pub const CONNECTED: Facts = Facts(32);

    pub fn contains(self, other: Facts) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Facts {
    type Output = Facts;
    fn bitor(self, other: Facts) -> Facts {
        Facts(self.0 | other.0)
    }
}

/// One declared thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decl {
    pub kind: DeclKind,
    /// The source name, or a compiler-made declaration's first hint.
    pub name: StrId,
    /// Its function, or its module; [`DeclId::NONE`] for a module.
    pub owner: DeclId,
    /// A function's is its signature; [`TypeId::NONE`] for modules and
    /// labels.
    pub ty: TypeId,
    pub facts: Facts,
    /// Its doc comment, or [`StrId::NONE`].
    pub doc: StrId,
    /// An override of its derived qname: what follows the module's path,
    /// as `f().x=#1`; [`StrId::NONE`] almost always
    /// (notes/tmpl/names.md, "Qnames").
    pub qname: StrId,
    /// The whole declaration.
    pub span: Span,
    /// Its name.
    pub name_span: Span,
    /// Hint candidates after the name, for a compiler-made declaration; or
    /// parameter names, for an external function, method, or getter, which
    /// has no parameter declarations.
    alts: (u32, u32),
    /// A connected declaration's backends with host code; empty for all.
    backends: (u32, u32),
}

/// Every declaration, local or not, by [`DeclId`].
#[derive(Clone, Debug, Default)]
pub struct DeclTable {
    decls: Vec<Decl>,
    /// Lists of strings that declarations refer to by range: hint
    /// candidates, external parameters' names, and backends.
    alts: Vec<StrId>,
}

impl DeclTable {
    pub fn len(&self) -> usize {
        self.decls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.decls.is_empty()
    }

    /// Whether `id` names a declaration that isn't removed.
    pub fn contains(&self, id: DeclId) -> bool {
        self.decls
            .get(id.index())
            .is_some_and(|d| d.kind != DeclKind::Removed)
    }

    pub fn get(&self, id: DeclId) -> &Decl {
        &self.decls[id.index()]
    }

    pub fn ids(&self) -> impl Iterator<Item = DeclId> + use<> {
        (0..self.decls.len() as u32).map(DeclId)
    }

    /// A compiler-made declaration's hint candidates after its name.
    pub fn alts(&self, id: DeclId) -> &[StrId] {
        let (start, len) = self.decls[id.index()].alts;
        &self.alts[start as usize..(start + len) as usize]
    }

    /// An external function's, method's, or getter's parameter names.
    pub fn param_names(&self, id: DeclId) -> &[StrId] {
        self.alts(id)
    }

    /// The backends a connected declaration has host code for; empty
    /// when every backend needs it.
    pub fn backends(&self, id: DeclId) -> &[StrId] {
        let (start, len) = self.decls[id.index()].backends;
        &self.alts[start as usize..(start + len) as usize]
    }

    pub(crate) fn set_param_names(&mut self, id: DeclId, names: &[StrId]) {
        let range = self.push_list(names);
        self.decls[id.index()].alts = range;
    }

    pub(crate) fn set_backends(&mut self, id: DeclId, backends: &[StrId]) {
        let range = self.push_list(backends);
        self.decls[id.index()].backends = range;
    }

    fn push_list(&mut self, list: &[StrId]) -> (u32, u32) {
        let start = self.alts.len() as u32;
        self.alts.extend_from_slice(list);
        (start, list.len() as u32)
    }

    pub(crate) fn get_mut(&mut self, id: DeclId) -> &mut Decl {
        &mut self.decls[id.index()]
    }

    /// Defines `id`, growing the table with tombstones as needed.
    pub(crate) fn define(
        &mut self,
        id: DeclId,
        kind: DeclKind,
        names: &[StrId],
        owner: DeclId,
        ty: TypeId,
    ) {
        let removed = Decl {
            kind: DeclKind::Removed,
            name: StrId::NONE,
            owner: DeclId::NONE,
            ty: TypeId::NONE,
            facts: Facts::NONE,
            doc: StrId::NONE,
            qname: StrId::NONE,
            span: Span::UNKNOWN,
            name_span: Span::UNKNOWN,
            alts: (0, 0),
            backends: (0, 0),
        };
        if self.decls.len() <= id.index() {
            self.decls.resize(id.index() + 1, removed);
        }
        let start = self.alts.len() as u32;
        self.alts.extend_from_slice(names.get(1..).unwrap_or(&[]));
        self.decls[id.index()] = Decl {
            kind,
            name: names.first().copied().unwrap_or(StrId::NONE),
            owner,
            ty,
            alts: (start, names.len().saturating_sub(1) as u32),
            ..removed
        };
    }
}
