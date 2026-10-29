//! tmpl, Temper's backend IR: a settled, typed, fully explicit program
//! that a backend can translate without asking anything of the frontend
//! (notes/tmpl/README.md).
//!
//! This crate has no dependencies. So far it covers subset A (functions,
//! locals, `if`, loops, returns, calls, builtin operators and members,
//! and module init):
//!
//! - [`Program`], with its tables: [`Nodes`], [`TypeTable`],
//!   [`DeclTable`], and [`Strings`];
//! - [`schema`], one table declaring every node kind, from which
//!   [`Kind`] and [`Node`] are generated;
//! - [`Builder`], for constructing programs;
//! - [`text`], the canonical text form, which reads back exactly;
//! - [`validate()`](mod@validate), the validator's rules, each problem
//!   with its code;
//! - [`builtins`], the key table.

pub mod builder;
pub mod builtins;
mod decls;
mod ids;
mod nodes;
mod profile;
mod same;
pub mod schema;
pub mod text;
mod types;
pub mod validate;

pub use builder::Builder;
pub use decls::{Decl, DeclKind, DeclTable, Facts, Strings};
pub use ids::{DeclId, FileId, NodeId, SourceFile, Span, StrId, TypeId};
pub use nodes::{Flags, Kids, Nodes};
pub use profile::{Features, Profile};
pub use same::difference;
pub use schema::{Kind, Node};
pub use types::{Scalar, TypeData, TypeList, TypeTable};
pub use validate::{Problem, Rule, validate};

/// The library a program is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub version: String,
}

/// One module: its declarations and its init block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    /// Named by its path in the library.
    pub decl: DeclId,
    /// `Function` and `ModuleLet` nodes, in source order.
    pub items: Vec<NodeId>,
    /// A `Block`: `InitLet`s and top-level statements, in source order.
    pub init: NodeId,
    // Subset B: imports and tests.
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
}

impl Program {
    /// A declaration's name.
    pub fn name(&self, decl: DeclId) -> &str {
        let name = self.decls.get(decl).name;
        if name.is_none() {
            ""
        } else {
            self.strings.get(name)
        }
    }

    /// A function declaration's signature: parameter types, result, and
    /// whether it may bubble.
    pub fn signature(&self, decl: DeclId) -> Option<(&[TypeId], TypeId, bool)> {
        self.types.signature(self.decls.get(decl).ty)
    }
}

impl PartialEq for Program {
    fn eq(&self, other: &Program) -> bool {
        same::difference(self, other).is_none()
    }
}
