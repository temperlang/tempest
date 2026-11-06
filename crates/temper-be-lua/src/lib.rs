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
//! Classes and interfaces are the runtime's types, `temper.type`, as
//! Kotlin's ([`classes`]): `r.area` reads a getter, `r:scaled(2)` calls a
//! method, and `Rect.unit` is a static.
//!
//! Null is the runtime's `temper.null`, as Kotlin's, since Lua's `nil`
//! can't be a table's value or tell an argument left out, checked by
//! `temper.is_null`, which takes `nil` too; nullable types are
//! `T|temper.Null` for LuaLS. `orelse` is Kotlin's `temper.pcall` of a
//! function, which an `if` then follows with the fallback (and redoes
//! a jump out of the body, which returns a signal saying which), and
//! `bubble()` the runtime's error. What Lua can't write inline (`orelse`,
//! statements before a value, as `??` makes, and `if` values that Lua's
//! `and` and `or` can't choose) goes before its statement
//! ([`translate`], "Hoisting").
//!
//! Generics are Lua's own: core's lists are tables (`{ 1, 2 }`), maps
//! and pairs the runtime's, as Kotlin's, with `---@generic` for a
//! generic function, `T[]`, `table<K, V>`, and `{ key: K, value: V }` for
//! LuaLS, and `any` for a generic class's type parameters, which LuaLS's
//! classes don't take.
//!
//! Where Kotlin's isn't right, it differs ([`translate`], [`classes`]):
//! exported `var`s that functions or methods write stay current in the
//! exports table, Lua's keyword `goto` is reserved, `Boolean`'s `<` is
//! logic, where Kotlin's compares booleans, which Lua can't, a type's
//! runtime name is qualified by its module, so `is` doesn't take one
//! module's `Point` for another's, a static can't replace the runtime's
//! keys of its type, and a plain `toString()` is also `__tostring`.
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! Lua's rules ([`names::RULES`]), then [`translate`] into Lua's output
//! tree ([`tree`]), with operators spelled by [`ops`] and core's connected
//! declarations by [`connected`], then the printer.
//!

pub mod classes;
pub mod connected;
pub mod names;
pub mod ops;
pub mod runtime;
pub mod translate;
pub mod tree;

use std::fmt;
use temper_tmpl::coroutines::CoroutineStrategy;
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
/// translator hoists itself until the hoisting pass is built.
pub const LUA: Variant = Variant {
    name: "lua",
    profile: Profile {
        native: Features::NONE,
        coroutines: CoroutineStrategy::TranslateToGenerator,
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

/// Translates a library, laid out as Kotlin's Lua backend lays it out:
///
/// ```text
/// fibonacci/init.lua                  the library's root module
/// fibonacci/helpers/deep.lua          another module, by its path
/// fibonacci/tests/init-test.lua       the root module's tests, for LuaUnit
/// fibonacci/tests/helpers-deep-test.lua  another's
/// fibonacci/_support.lua              host code's other files, copied
/// fibonacci/fibonacci-dev-1.rockspec  depending on temper-core 0.6.0
/// ```
///
/// Kotlin also zips the library beside the rockspec, for LuaRocks, whose
/// `source.url` names the zip by its absolute path; this names it
/// relative, as Kotlin does when it has no path, and writes no zip:
/// `luarocks make` in the library's directory needs none.
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    // Operands that run before an `orelse` or a block's value, which Lua
    // writes as statements before, held first (`temper_tmpl::spill`).
    let spilled = temper_tmpl::spill::spill(p);
    let p = spilled.as_ref().unwrap_or(p);
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    if let Some(missing) = names.missing().first() {
        return Err(NotYet {
            what: format!("another library's name: {missing}"),
        });
    }
    let library = &p.library.name;
    let mut files = Vec::new();
    let mut tests = Vec::new();
    for module in &p.modules {
        let mut translator = translate::Translator::new(p, &names, module);
        let access = translate::TestAccess::of(p, module);
        let chunk = translator.module(module, &access)?;
        let (file, test) = match module.path.as_str() {
            "" => ("init".to_string(), "init-test".to_string()),
            path => (path.to_string(), format!("{}-test", path.replace('/', "-"))),
        };
        files.push(OutFile {
            path: format!("{library}/{file}.lua"),
            text: tree::chunk(&chunk),
        });
        if let Some(chunk) = translator.tests(module)? {
            tests.push(OutFile {
                path: format!("{library}/tests/{test}.lua"),
                text: tree::chunk(&chunk),
            });
        }
    }
    // The root module first.
    files.sort_by_key(|f| !f.path.ends_with("/init.lua"));
    files.extend(tests);
    // Host code's other files, as `_support.lua`, where Kotlin copies them,
    // beside the modules; each `_connected.lua` is inlined in its module.
    files.extend(
        (temper_tmpl::host::files(p, &[".lua"]).into_iter())
            .filter(|f| f.path.rsplit('/').next() != Some("_connected.lua"))
            .map(|f| OutFile {
                path: format!("{library}/{}", f.path),
                text: f.text.clone(),
            }),
    );
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
{dependencies}}}
source = {{
    url = {url},
}}
",
        package = quoted(library),
        version_q = quoted(&version),
        lua = variant.lua_version,
        runtime = variant.runtime_version,
        // Each library used, as Kotlin writes them, with its version where
        // it sets one.
        dependencies = (p.dependencies().iter())
            .map(|(name, version)| match *version {
                "0.0.0" | "" => format!("    {},\n", quoted(name)),
                v => format!("    {},\n", quoted(&format!("{name} == {v}"))),
            })
            .collect::<Vec<_>>()
            .concat(),
        url = quoted(&format!("file://{library}.zip")),
    );
    files.push(OutFile {
        path: format!("{library}/{library}-{version}.rockspec"),
        text: rockspec,
    });
    Ok(files)
}
