//! Temper's Rust backend: tmpl in, Rust out.
//!
//! Works from tmpl alone (notes/tmpl/overview.md, "Boundaries"). Its
//! variants share the translation and differ only in data
//! (notes/tmpl/profiles.md, "Variants and the first profiles"); the first,
//! [`RUST`], is Kotlin-compatible: the same crate layout, `temper-core`
//! runtime, string representation (`Arc<String>`, with
//! `impl ToArcString` parameters), and public names and signatures as
//! Kotlin Temper's Rust backend, with prettier function bodies: no
//! numbered names, no `return` where a value ends a function, literals
//! inside format strings, only the `use`s needed, and laid out as
//! rustfmt lays it out.
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! Rust's rules ([`names::RULES`]), then [`translate`] into Rust's output
//! tree ([`tree`]), with operators spelled by [`ops`] and core's
//! connected declarations by [`connected`], then the printer. Module
//! state is safe across threads ([`translate`]'s documentation says how).
//!
//! So far it covers tmpl's subset A, one module per library; anything
//! else is [`NotYet`].

pub mod connected;
pub mod names;
pub mod ops;
pub mod translate;
pub mod tree;

use std::fmt;
use temper_tmpl::naming::{NamingRules, assign};
use temper_tmpl::{Facts, Features, Profile, Program};

/// A named set of translation choices for Rust.
pub struct Variant {
    /// Its name on the command line: `rust`.
    pub name: &'static str,
    /// What it handles natively, which lowering and validation read.
    pub profile: Profile,
    pub rules: NamingRules,
    /// The `temper-core` version its output depends on.
    pub runtime_version: &'static str,
    /// The `rust-version` its crates declare, as Kotlin's.
    pub rust_version: &'static str,
}

/// The first variant: Kotlin-compatible API and names, pretty bodies.
/// Everything subset A has is native in Rust, statements inside
/// expressions included (as blocks), so nothing is lowered.
pub const RUST: Variant = Variant {
    name: "rust",
    profile: Profile {
        native: Features::STATEMENT_EXPRESSIONS,
    },
    rules: names::RULES,
    runtime_version: "0.6.0",
    rust_version: "1.71.1",
};

/// Every variant, by name.
pub const VARIANTS: &[&Variant] = &[&RUST];

/// Something the translation doesn't cover yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub what: String,
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not yet in the Rust backend: {}", self.what)
    }
}

impl std::error::Error for NotYet {}

/// A generated file: its path relative to the output directory, and its
/// text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub text: String,
}

/// Translates a library, laid out as Kotlin's Rust backend lays it out:
///
/// ```text
/// fibonacci/Cargo.toml            depends on ../temper-core
/// fibonacci/src/lib.rs            `init`, and the root module's exports
/// fibonacci/src/main.rs           runs `init`
/// fibonacci/src/mod.rs            the library's root module
/// fibonacci/src/support/mod.rs    the crate's `Config`
/// ```
///
/// The package takes the library's name; its crate, the name with `_`
/// for `-` (`hello-world` gives `hello_world`). Kotlin also copies
/// `temper-core` beside the package; this doesn't, yet.
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    let [module] = p.modules.as_slice() else {
        return Err(NotYet {
            what: format!("a library of {} modules", p.modules.len()),
        });
    };
    if !module.path.is_empty() {
        return Err(NotYet {
            what: "a module that isn't at the library's root".into(),
        });
    }
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    let package = &p.library.name;
    let krate = names::crate_name(package);
    let mut translator = translate::Translator::new(p, &names);
    let items = translator.module(module)?;
    let exports = p
        .decls
        .ids()
        .any(|d| p.decls.contains(d) && p.decls.get(d).facts.contains(Facts::EXPORTED));
    let cargo = format!(
        r#"[package]
name = "{package}"
version = "{version}"
edition = "2021"
rust-version = "{rust}"

[dependencies]
temper-core = {{ path = "../temper-core", version = "={runtime}" }}
"#,
        version = p.library.version,
        rust = variant.rust_version,
        runtime = variant.runtime_version,
    );
    let reexport = if exports { "\npub use r#mod::*;\n" } else { "" };
    let lib = format!(
        r#"mod r#mod;
mod support;
{reexport}
/// Initializes the library, once, with `config` or the default
/// configuration, and gives the runner of its async tasks.
pub fn init(config: Option<temper_core::Config>) -> temper_core::Result<temper_core::AsyncRunner> {{
    support::CONFIG.get_or_init(|| config.unwrap_or_default());
    r#mod::init()?;
    Ok(support::config().runner().clone())
}}
"#
    );
    let main = format!("fn main() {{\n    {krate}::init(None).unwrap().run_all_blocking();\n}}\n");
    let support = r#"use std::sync::OnceLock;

pub(crate) static CONFIG: OnceLock<temper_core::Config> = OnceLock::new();

pub(crate) fn config() -> &'static temper_core::Config {
    CONFIG.get().unwrap()
}
"#;
    Ok(vec![
        OutFile {
            path: format!("{package}/Cargo.toml"),
            text: cargo,
        },
        OutFile {
            path: format!("{package}/src/lib.rs"),
            text: lib,
        },
        OutFile {
            path: format!("{package}/src/main.rs"),
            text: main,
        },
        OutFile {
            path: format!("{package}/src/mod.rs"),
            text: tree::file(&items),
        },
        OutFile {
            path: format!("{package}/src/support/mod.rs"),
            text: support.to_string(),
        },
    ])
}
