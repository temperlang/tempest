//! tmpl, Temper's backend IR: a settled, typed, fully explicit program
//! that a backend can translate without asking anything of the frontend
//! (notes/tmpl/README.md).
//!
//! This crate has no dependencies. So far it covers subset A (functions,
//! locals, `if`, loops, returns, calls, builtin operators, module init,
//! and core's members as external declarations called by `CallFunction`
//! and `CallMethod`), modules, imports, and tests, and classes and
//! interfaces (subset B's first steps):
//!
//! - [`Program`], with its tables: [`Nodes`], [`TypeTable`],
//!   [`DeclTable`], and [`Strings`];
//! - [`schema`], one table declaring every node kind, from which
//!   [`Kind`] and [`Node`] are generated;
//! - [`Builder`], for constructing programs;
//! - [`text`], the canonical text form, which reads back exactly;
//! - [`validate()`](mod@validate), the validator's rules, each problem
//!   with its code;
//! - [`ops`], the operator table;
//! - [`qname`], qualified names in Kotlin's format, and how connected
//!   declarations resolve;
//! - [`naming`], the naming pass, which gives each declaration a pretty
//!   output name by a target's [`naming::NamingRules`];
//! - [`analysis`], analyses that passes and backends share;
//! - [`host`], a library's host code, and which connected declarations
//!   each backend takes from it;
//! - [`source_map`], Source Map v3 files from spans, for backends.

pub mod analysis;
pub mod builder;
pub mod coroutines;
mod decls;
pub mod host;
mod ids;
pub mod naming;
mod nodes;
pub mod ops;
mod profile;
pub mod qname;
mod same;
pub mod schema;
pub mod source_map;
pub mod text;
mod types;
pub mod validate;

pub use builder::Builder;
pub use decls::{Decl, DeclKind, DeclTable, Facts, Strings};
pub use ids::{DeclId, FileId, NodeId, SourceFile, Span, StrId, TypeId};
pub use nodes::{Flags, Kids, Nodes};
pub use profile::{Features, Profile};
pub use qname::{QName, QNames};
pub use same::difference;
pub use schema::{Kind, Node};
pub use types::{Scalar, Signature, TypeArgs, TypeData, TypeList, TypeTable};
pub use validate::{Problem, Rule, validate};

/// The library a program is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub version: String,
}

/// A version, or Kotlin's default, `0.0.1`, for `0.0.0` or none, which
/// mean a library sets none.
pub fn version_or_default(version: &str) -> &str {
    match version {
        "0.0.0" | "" => "0.0.1",
        v => v,
    }
}

/// A library this one uses: its declaration, of kind
/// [`DeclKind::Library`], which owns the external declarations used from
/// it, and its version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uses {
    pub decl: DeclId,
    pub version: String,
}

/// One module: its declarations and its init block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub decl: DeclId,
    /// Its directory relative to the library's root, as `foo/bar`; empty
    /// at the root. Qnames start with it.
    pub path: String,
    /// `Function`, `BodilessFunction`, and `ModuleLet` nodes, in source
    /// order.
    pub items: Vec<NodeId>,
    /// A `Block`: `InitLet`s and top-level statements, in source order.
    pub init: NodeId,
    /// `Test` nodes, in source order, run after init. What a module
    /// imports isn't stored: it's the other modules' declarations its
    /// nodes and types name ([`analysis::imports`]).
    pub tests: Vec<NodeId>,
}

/// One library's program: modules plus the tables they share.
///
/// Programs compare equal when they mean the same: ids of nodes, types,
/// and strings may differ, but declarations keep theirs.
#[derive(Clone, Debug)]
pub struct Program {
    pub library: Library,
    /// The profile it's lowered for; `None` for starting tmpl.
    pub profile: Option<String>,
    /// The libraries it uses, core among them, with what it uses of each.
    pub uses: Vec<Uses>,
    /// In dependency order, then source order.
    pub modules: Vec<Module>,
    pub nodes: Nodes,
    pub types: TypeTable,
    pub decls: DeclTable,
    pub strings: Strings,
    /// The files that spans point into.
    pub files: Vec<SourceFile>,
    /// Whether nodes and declarations carry spans; `false` for a program
    /// read from text without its spans section.
    pub has_spans: bool,
    /// The library's host code and how to use it, given by the build; not
    /// in tmpl's text, and not compared ([`host`]).
    pub host: host::Host,
    /// Its dependencies' naming, for the backend it's translated for,
    /// given by the build: what the naming pass names their declarations
    /// ([`naming::Interface`]). Not in tmpl's text, and not compared.
    pub interfaces: Vec<naming::Interface>,
}

impl Program {
    /// The libraries it uses other than the runtime, `temper-core`: each
    /// one's name and version, sorted by name, for package files'
    /// dependencies. A version of `0.0.0` means the library sets none.
    pub fn dependencies(&self) -> Vec<(&str, &str)> {
        let mut found: Vec<(&str, &str)> = (self.uses.iter())
            .map(|u| (self.name(u.decl), u.version.as_str()))
            .filter(|(name, _)| *name != "temper-core")
            .collect();
        found.sort();
        found.dedup();
        found
    }

    /// The library's version, or Kotlin's default, `0.0.1`, where it sets
    /// none (`0.0.0`, as the frontend gives it), for the package files
    /// that always name one.
    pub fn version_or_default(&self) -> &str {
        version_or_default(&self.library.version)
    }

    /// A declaration's name.
    pub fn name(&self, decl: DeclId) -> &str {
        let name = self.decls.get(decl).name;
        if name.is_none() {
            ""
        } else {
            self.strings.get(name)
        }
    }

    /// A function's, method's, or getter's signature.
    pub fn signature(&self, decl: DeclId) -> Option<Signature<'_>> {
        self.types.signature(self.decls.get(decl).ty)
    }

    /// A type as text writes it: a nominal type as its declaration,
    /// `Console#9`.
    pub fn show_type(&self, ty: TypeId) -> String {
        self.types.show_with(ty, &|d| {
            if !self.decls.contains(d) {
                return format!("#{}", d.0);
            }
            let made = self.decls.get(d).facts.contains(Facts::COMPILER_MADE);
            let tilde = if made { "~" } else { "" };
            format!("{tilde}{}#{}", text::name(self.name(d)), d.0)
        })
    }

    /// Whether a call of this method is virtual: exactly when its owner is
    /// an interface.
    pub fn is_virtual(&self, method: DeclId) -> bool {
        let owner = self.decls.get(method).owner;
        self.decls.contains(owner) && self.decls.get(owner).kind == DeclKind::Interface
    }
}

impl PartialEq for Program {
    fn eq(&self, other: &Program) -> bool {
        same::difference(self, other).is_none()
    }
}
