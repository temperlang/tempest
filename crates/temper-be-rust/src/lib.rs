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
//! So far it covers tmpl's subset A, modules, classes and interfaces
//! (`translate::classes` says how they're written), and nullable types,
//! `orelse`, `bubble()`, and `panic()` (`translate::nullables`); anything
//! else is [`NotYet`].

pub mod connected;
pub mod names;
pub mod ops;
pub mod runtime;
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
/// fibonacci/src/helpers/mod.rs    another module, `helpers`
/// fibonacci/src/helpers/deep/mod.rs   and `helpers/deep`
/// ```
///
/// `lib.rs` declares the top modules, each the modules under it, and
/// `init` initializes the libraries used, then each module in order. A
/// library used only by tests, as std is for its `testing`, is a
/// dev-dependency, where Kotlin makes it a dependency.
///
/// The package takes the library's name; its crate, the name with `_`
/// for `-` (`hello-world` gives `hello_world`). Kotlin also copies
/// `temper-core` beside the package; `tempest build` does too ([`runtime`]).
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    let package = &p.library.name;
    let krate = names::crate_name(package);
    let mut translator = translate::Translator::new(p, &names);
    // Each module's file, by its path; parents without a module of their
    // own get a file declaring their children.
    let mut files: Vec<(String, Vec<tree::Item>)> = Vec::new();
    let mut root = None;
    let mut exports = false;
    for module in &p.modules {
        let items = translator.module(module)?;
        if module.path.is_empty() {
            exports = (module.items.iter()).any(|&item| {
                let decl = match p.nodes.get(item) {
                    temper_tmpl::Node::Function { decl, .. }
                    | temper_tmpl::Node::BodilessFunction { decl, .. }
                    | temper_tmpl::Node::ModuleLet { decl }
                    | temper_tmpl::Node::Class { decl, .. }
                    | temper_tmpl::Node::Interface { decl, .. } => decl,
                    _ => return false,
                };
                p.decls.get(decl).facts.contains(Facts::EXPORTED)
            });
            root = Some(items);
        } else {
            let parts: Vec<String> = (module.path.split('/'))
                .map(translate::module_name)
                .collect();
            files.push((parts.join("/"), items));
        }
    }
    let Some(root) = root else {
        return Err(NotYet {
            what: "a library without a root module".into(),
        });
    };
    // Every directory a module is in, and the modules under each.
    let mut dirs: Vec<String> = Vec::new();
    for (path, _) in &files {
        let mut prefix = String::new();
        for part in path.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if !dirs.contains(&prefix) {
                dirs.push(prefix.clone());
            }
        }
    }
    let children = |parent: &str| -> Vec<tree::Item> {
        let mut out: Vec<String> = (dirs.iter())
            .filter_map(|d| match parent {
                "" => (!d.contains('/')).then(|| d.clone()),
                parent => d
                    .strip_prefix(parent)
                    .and_then(|r| r.strip_prefix('/'))
                    .filter(|r| !r.contains('/'))
                    .map(str::to_string),
            })
            .map(|d| d.rsplit('/').next().unwrap_or(&d).to_string())
            .collect();
        out.sort();
        out.into_iter().map(tree::Item::Mod).collect()
    };
    let mut module_files = Vec::new();
    for dir in &dirs {
        let mut items = children(dir);
        if let Some((_, own)) = files.iter().find(|(path, _)| path == dir) {
            items.extend(own.iter().cloned());
        }
        module_files.push(OutFile {
            path: format!("{package}/src/{dir}/mod.rs"),
            text: tree::file(&items),
        });
    }
    // The libraries used outside tests, which `init` initializes, and
    // those used only by tests.
    let mut used = Vec::new();
    for module in &p.modules {
        for import in temper_tmpl::analysis::module_imports(p, module) {
            if let Some(library) = library_decl(p, import.from) {
                if !used.contains(&library) {
                    used.push(library);
                }
            }
        }
    }
    let dependency = |name: &str, version: &str| {
        let (package, path) = match name {
            "std" => ("temper-std", "std"),
            name => (name, name),
        };
        let version = temper_tmpl::version_or_default(version);
        format!("{package} = {{ path = \"../{path}\", version = \"{version}\" }}\n")
    };
    let (mut dependencies, mut dev_dependencies) = (String::new(), String::new());
    for (name, version) in p.dependencies() {
        if used.contains(&name) {
            dependencies.push_str(&dependency(name, version));
        } else {
            dev_dependencies.push_str(&dependency(name, version));
        }
    }
    if !dev_dependencies.is_empty() {
        dev_dependencies.insert_str(0, "\n[dev-dependencies]\n");
    }
    let cargo = format!(
        r#"[package]
name = "{package}"
version = "{version}"
edition = "2021"
rust-version = "{rust}"

[dependencies]
temper-core = {{ path = "../temper-core", version = "={runtime}" }}
{dependencies}{dev_dependencies}"#,
        version = p.version_or_default(),
        rust = variant.rust_version,
        runtime = variant.runtime_version,
    );
    let reexport = if exports { "\npub use r#mod::*;\n" } else { "" };
    let mods: String = (children("").iter())
        .map(|m| match m {
            tree::Item::Mod(name) => format!("pub mod {name};\n"),
            _ => String::new(),
        })
        .collect();
    let mut inits = String::new();
    for name in &used {
        let krate = translate::library_crate(name);
        inits.push_str(&format!(
            "    {krate}::init(Some(support::config().clone()))?;\n"
        ));
    }
    for module in &p.modules {
        let path = match translator.module_path(module.decl) {
            Some(path) => path.trim_start_matches("crate::").to_string(),
            None => continue,
        };
        inits.push_str(&format!("    {path}::init()?;\n"));
    }
    let lib = format!(
        r#"{mods}mod r#mod;
mod support;
{reexport}
/// Initializes the library, once, with `config` or the default
/// configuration, and gives the runner of its async tasks.
pub fn init(config: Option<temper_core::Config>) -> temper_core::Result<temper_core::AsyncRunner> {{
    support::CONFIG.get_or_init(|| config.unwrap_or_default());
{inits}    Ok(support::config().runner().clone())
}}
"#
    );
    let run = tree::Expr::path(format!("{krate}::init"))
        .call(vec![tree::Expr::path("None")])
        .method("unwrap", Vec::new())
        .method("run_all_blocking", Vec::new());
    let main = tree::file(&[tree::Item::Fn {
        vis: "",
        name: "main".into(),
        params: Vec::new(),
        result: None,
        body: tree::Block::new(vec![tree::Stmt::Expr(run)], None),
    }]);
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
            text: tree::file(&root),
        },
        OutFile {
            path: format!("{package}/src/support/mod.rs"),
            text: support.to_string(),
        },
    ]
    .into_iter()
    .chain(module_files)
    .collect())
}

/// The library a module of another library's is in, by its `uses`
/// declaration's name; `None` for the program's own and core's.
fn library_decl(p: &Program, from: temper_tmpl::DeclId) -> Option<&str> {
    let d = p.decls.get(from);
    let library = match d.kind {
        temper_tmpl::DeclKind::Library => from,
        _ if !d.owner.is_none() => d.owner,
        _ => return None,
    };
    let name = p.name(library);
    (name != "temper-core").then_some(name)
}
