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
//! (`translate::classes` says how they're written), nullable types,
//! `orelse`, `bubble()`, and `panic()` (`translate::nullables`), and
//! generics: type parameters bounded `Clone + Send + Sync + 'static` as
//! Kotlin's, an interface bound as its trait, a class bound as `Into`
//! the class (where Kotlin names a trait that doesn't exist), and core's
//! lists and maps as `temper_core`'s types, called through their traits
//! (`temper_core::ListedTrait::get(&ns, 0)`), with `List` and `Listed`
//! parameters taking `impl ToList` and `impl ToListed`, and a `Pair` a
//! tuple; anything else is [`NotYet`].
//!
//! A library's own connected functions and classes take host code from
//! `_connected.rs` beside their module, where Kotlin's choice says
//! (`temper_tmpl::host`), as Kotlin's backend writes them
//! ([`translate::host`]); without that file, [`NotYet`] names it.

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
/// fibonacci/src/_connected/mod.rs      the root module's host code, `_connected.rs`
/// fibonacci/src/_connected/more.rs     another `.rs` host file beside it
/// ```
///
/// Host code's module, `_connected`, is declared, with its warnings
/// allowed, where a module takes host code: the root module's in
/// `lib.rs`, so `super::` there is the crate root, which re-exports the
/// root module; another's in its own file. Host files in another
/// module's directory go in `_connected/` beside its `mod.rs`.
///
/// `lib.rs` declares the top modules, each the modules under it, and
/// `init` initializes the libraries used, then each module in order,
/// inside `temper_core::initializing`, as Kotlin's (#636). Where a
/// module-level value is read, it also has `init_on_first_use()`, which
/// a read calls where the value isn't set, so `init` is optional, and
/// `support` waits for the whole init on other threads (`SUPPORT_FIRST`,
/// where Kotlin's lets them read what's set so far). A
/// library used only by tests, as std is for its `testing`, is a
/// dev-dependency, where Kotlin makes it a dependency.
///
/// The package takes the library's name; its crate, the name with `_`
/// for `-` (`hello-world` gives `hello_world`); its binary too, but for
/// the names cargo forbids a binary ([`bin_name`]). Kotlin also copies
/// `temper-core` beside the package; `tempest build` does too ([`runtime`]).
/// The name of a library's binary, which runs its `init`: the package's
/// own, unless cargo forbids that for a binary (`build`, `deps`,
/// `examples`, and `incremental`, its target directory's names), then
/// `NAME-main`. Kotlin's names it the package, which cargo rejects.
pub fn bin_name(package: &str) -> String {
    if FORBIDDEN_BINS.contains(&package) {
        format!("{package}-main")
    } else {
        package.to_string()
    }
}

const FORBIDDEN_BINS: [&str; 4] = ["build", "deps", "examples", "incremental"];

/// A `[[bin]]` section naming the binary, where its package's name won't
/// do ([`bin_name`]).
fn bin_section(package: &str) -> String {
    if !FORBIDDEN_BINS.contains(&package) {
        return String::new();
    }
    format!(
        "\n[[bin]]\nname = \"{}\"\npath = \"src/main.rs\"\n",
        bin_name(package)
    )
}

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
    // Each module's host code's module, declared in its file.
    let host_dirs: Vec<String> = (translator.host_dirs.iter())
        .map(|d| {
            d.split('/')
                .map(translate::module_name)
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    let mut module_files = Vec::new();
    for dir in &dirs {
        let mut items = children(dir);
        if host_dirs.contains(dir) {
            items.push(tree::Item::HostMod);
        }
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
{dependencies}{dev_dependencies}{bin}"#,
        bin = bin_section(package),
        version = p.version_or_default(),
        rust = variant.rust_version,
        runtime = variant.runtime_version,
    );
    // The root module's host code, at the crate root, as Kotlin's, so
    // `super::` there names the root module's declarations, through its
    // re-export, which is private where the root module exports nothing.
    let root_host = translator.host_dirs.contains("");
    let reexport = match (exports, root_host) {
        (true, _) => "\npub use r#mod::*;\n",
        (false, true) => "\nuse r#mod::*;\n",
        (false, false) => "",
    };
    let host_mod = match root_host {
        true => format!("{}\nmod _connected;\n", tree::HOST_ALLOW),
        false => String::new(),
    };
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
            "        {krate}::init(Some(support::config().clone()))?;\n"
        ));
    }
    for module in &p.modules {
        let path = match translator.module_path(module.decl) {
            Some(path) => path.trim_start_matches("crate::").to_string(),
            None => continue,
        };
        inits.push_str(&format!("        {path}::init()?;\n"));
    }
    // Where module-level values initialize the library on first use,
    // `cfg` for those, empty or only for tests; else `None`.
    let cfg = match (translator.first_use, translator.first_use_in_tests) {
        (false, false) => None,
        (true, _) => Some(""),
        (false, true) => Some("#[cfg(test)]\n"),
    };
    // Marked as initializing, as Kotlin's, so a read of a value not set
    // yet panics rather than initializing again; and with first use,
    // marked ready after, so other threads' reads wait for it all.
    let wrap = match cfg {
        Some(_) => "support::initializing",
        None => "temper_core::initializing",
    };
    if !inits.is_empty() || cfg.is_some() {
        inits = format!(
            "    {wrap}(|| -> temper_core::Result<()> {{\n{inits}        Ok(())\n    }})?;\n"
        );
    }
    // For reads before any `init`, which give the default configuration.
    let first_use = match cfg {
        None => String::new(),
        Some(cfg) => format!(
            "
/// Initializes the library with the default configuration, where a
/// module-level value is read before any `init`.
{cfg}pub(crate) fn init_on_first_use() {{
    init(None).expect(\"initializing a Temper library on first use\");
}}
"
        ),
    };
    let lib = format!(
        r#"{mods}{host_mod}mod r#mod;
mod support;
{reexport}
/// Initializes the library, once, with `config` or the default
/// configuration, and gives the runner of its async tasks.
pub fn init(config: Option<temper_core::Config>) -> temper_core::Result<temper_core::AsyncRunner> {{
    support::CONFIG.get_or_init(|| config.unwrap_or_default());
{inits}    Ok(support::config().runner().clone())
}}
{first_use}"#
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
    let mut support = String::from(match cfg {
        Some(_) => SUPPORT_USES_FIRST,
        None => "use std::sync::OnceLock;\n",
    });
    support.push_str(SUPPORT_CONFIG);
    if let Some(cfg) = cfg {
        support.push_str(&SUPPORT_FIRST.replace("{cfg}", cfg));
    }
    // Host code's files, in `_connected/` beside each module's file, as
    // Kotlin copies them: `_connected.rs` as `mod.rs`, the others as they
    // are (`more.rs`, `_connected::more`).
    let host = (temper_tmpl::host::files(p, &[".rs"]).into_iter()).map(|f| {
        let (dir, name) = f.path.rsplit_once('/').unwrap_or(("", &f.path));
        let name = match name {
            "_connected.rs" => "mod.rs",
            name => name,
        };
        let dir: String = (dir.split('/').filter(|d| !d.is_empty()))
            .flat_map(|d| [translate::module_name(d), "/".to_string()])
            .collect();
        OutFile {
            path: format!("{package}/src/{dir}_connected/{name}"),
            text: f.text.clone(),
        }
    });
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
            text: support,
        },
    ]
    .into_iter()
    .chain(module_files)
    .chain(host)
    .collect())
}

const SUPPORT_USES_FIRST: &str = "use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
";

const SUPPORT_CONFIG: &str = "
pub(crate) static CONFIG: OnceLock<temper_core::Config> = OnceLock::new();

pub(crate) fn config() -> &'static temper_core::Config {
    CONFIG.get().unwrap()
}
";

/// What reads of module-level values need where they initialize the
/// library on first use: a library is ready once its init finishes, and
/// until then, only the thread running it reads values, as it sets them,
/// while other threads' reads wait for it, where Kotlin's read what's set
/// so far, racing with the rest of the init. `{cfg}` is `#[cfg(test)]`
/// where only tests read them.
const SUPPORT_FIRST: &str = "
/// Whether the library's init has finished.
static READY: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Whether this thread is running the library's init.
    static INITIALIZING: Cell<bool> = const { Cell::new(false) };
}

/// Runs the library's modules' inits, through
/// `temper_core::initializing`, with this thread marked as running them,
/// and then, if they succeed, marks the library ready.
pub(crate) fn initializing(
    inits: impl FnOnce() -> temper_core::Result<()>,
) -> temper_core::Result<()> {
    struct Running(bool);
    impl Drop for Running {
        fn drop(&mut self) {
            INITIALIZING.with(|running| running.set(self.0));
        }
    }
    let _running = Running(INITIALIZING.with(|running| running.replace(true)));
    temper_core::initializing(inits)?;
    READY.store(true, Ordering::Release);
    Ok(())
}

/// Reads a module-level value, through `temper_core::initialized`: at
/// once where the library's init has finished, or this thread is running
/// it; otherwise after initializing the library first, or waiting while
/// another thread does.
{cfg}pub(crate) fn initialized<T>(get: impl Fn() -> Option<T>) -> T {
    let ready = || READY.load(Ordering::Acquire) || INITIALIZING.with(Cell::get);
    temper_core::initialized(
        || if ready() { get() } else { None },
        crate::init_on_first_use,
    )
}
";

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
