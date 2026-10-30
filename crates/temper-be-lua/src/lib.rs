//! Temper's Lua backend: tmpl in, Lua out.
//!
//! Works from tmpl alone (notes/tmpl/overview.md, "Boundaries"). Its
//! variants share the translation and differ only in data
//! (notes/tmpl/profiles.md, "Variants and the first profiles"); the first,
//! [`LUA`], is Kotlin-compatible: the same layout (`LIBRARY/init.lua`, the
//! library's root module, which `require("LIBRARY")` loads with Kotlin's
//! `LUA_PATH` of `?.lua;?/init.lua`, and a LuaRocks rockspec beside it),
//! the same exports (a table of the library's exported names, which the
//! chunk returns), names, and `temper-core` runtime, required as
//! `temper-core`, as Kotlin Temper's `lua` backend, with prettier bodies:
//! no numbered names, local functions and locals declared where they're
//! first given a value, `..` for concatenation, `print` for logging,
//! natural control flow, with `goto` only for `continue` and jumps out of
//! inner loops, and blank lines between functions. LuaCATS annotations
//! (`---@param i integer`) type parameters, results, and locals declared
//! without a value, for LuaLS's checker.
//!
//! Where Kotlin's isn't right, it differs ([`translate`]): exported
//! `var`s that functions write stay current in the exports table, Lua's
//! keyword `goto` is reserved, and `Boolean`'s `<` is logic, where
//! Kotlin's compares booleans, which Lua can't.
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! Lua's rules ([`names::RULES`]), then [`translate`] into Lua's output
//! tree ([`tree`]), with operators spelled by [`ops`] and core's connected
//! declarations by [`connected`], then the printer.
//!
//! So far it covers tmpl's subset A, one module per library; anything
//! else is [`NotYet`]. No source maps yet.

pub mod connected;
pub mod names;
pub mod ops;
pub mod translate;
pub mod tree;

use std::fmt;
use temper_tmpl::naming::{NamingRules, assign};
use temper_tmpl::{Features, Profile, Program};

/// A named set of translation choices for Lua.
pub struct Variant {
    /// Its name on the command line: `lua`.
    pub name: &'static str,
    /// What it handles natively, which lowering and validation read.
    pub profile: Profile,
    pub rules: NamingRules,
    /// The `temper-core` version its output depends on.
    pub runtime_version: &'static str,
    /// The oldest Lua its rockspec accepts, as Kotlin's says (`lua >=
    /// 5.1`), though `goto` needs 5.2 or LuaJIT, and exact `Int64`s 5.3.
    pub lua_version: &'static str,
}

/// The first variant: Kotlin-compatible API and names, pretty bodies.
/// Everything is native except statements inside expressions, which the
/// hoisting pass will lower (not built yet: a program that needs it is
/// [`NotYet`]).
pub const LUA: Variant = Variant {
    name: "lua",
    profile: Profile {
        native: Features::NONE,
    },
    rules: names::RULES,
    runtime_version: "0.6.0",
    lua_version: "5.1",
};

/// Every variant, by name.
pub const VARIANTS: &[&Variant] = &[&LUA];

/// Something the translation doesn't cover yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub what: String,
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not yet in the Lua backend: {}", self.what)
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

/// The rockspec's version: the library's, or `dev` for one that doesn't
/// set it (the frontend gives it `0.0.0`), with rockspec revision 1, as
/// Kotlin's.
pub fn rock_version(p: &Program) -> String {
    let version = match p.library.version.as_str() {
        "0.0.0" | "" => "dev",
        v => v,
    };
    format!("{version}-1")
}

/// Translates a library, laid out as Kotlin's Lua backend lays out one
/// with a single module:
///
/// ```text
/// fibonacci/init.lua                  the library's root module
/// fibonacci/fibonacci-dev-1.rockspec  depending on temper-core 0.6.0
/// ```
///
/// Kotlin also zips the library beside the rockspec, for LuaRocks, whose
/// `source.url` names the zip by its absolute path; this names it
/// relative, as Kotlin does when it has no path, and writes no zip:
/// `luarocks make` in the library's directory needs none.
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
    let library = &p.library.name;
    let mut translator = translate::Translator::new(p, &names, module);
    let chunk = translator.module(module)?;
    let version = rock_version(p);
    let quoted = |s: &str| format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"));
    let rockspec = format!(
        "package = {package}
version = {version_q}
rockspec_format = '3.0'
description = {{
}}
build = {{
    type = 'builtin',
    copy_directories = {{'.'}},
    modules = {{}},
}}
dependencies = {{
    'lua >= {lua}',
    'temper-core == {runtime}',
}}
source = {{
    url = {url},
}}
",
        package = quoted(library),
        version_q = quoted(&version),
        lua = variant.lua_version,
        runtime = variant.runtime_version,
        url = quoted(&format!("file://{library}.zip")),
    );
    Ok(vec![
        OutFile {
            path: format!("{library}/init.lua"),
            text: tree::chunk(&chunk),
        },
        OutFile {
            path: format!("{library}/{library}-{version}.rockspec"),
            text: rockspec,
        },
    ])
}
