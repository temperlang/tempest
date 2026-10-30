//! Temper's JavaScript backend: tmpl in, JavaScript out.
//!
//! Works from tmpl alone (notes/tmpl/overview.md, "Boundaries"). Its
//! variants share the translation and differ only in data
//! (notes/tmpl/profiles.md, "Variants and the first profiles"); the first,
//! [`JS`], is Kotlin-compatible: the same package and module layout
//! (ES modules, with each module's code in a `.internal.js` file that the
//! public one re-exports from), `@temperlang/core` runtime, and public
//! names as Kotlin Temper's JS backend, with prettier function bodies: no
//! numbered names, template literals, `const` where nothing reassigns,
//! and labels only where a jump needs one. JSDoc annotations type
//! parameters, results, and declarations without a value, so
//! `tsc --checkJs --strict` passes on the output.
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! JS's rules ([`names::RULES`]), then [`translate`] into JS's output tree
//! ([`tree`]), with operators spelled by [`ops`] and core's connected
//! declarations by [`connected`], then the printer, which can mark where
//! statements and functions' names come from for a source map
//! ([`translate_with_source_maps`]).
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
use temper_tmpl::source_map::source_map;
use temper_tmpl::{DeclId, Facts, Features, Node, Profile, Program};

/// A named set of translation choices for JavaScript.
pub struct Variant {
    /// Its name on the command line: `js`.
    pub name: &'static str,
    /// What it handles natively, which lowering and validation read.
    pub profile: Profile,
    pub rules: NamingRules,
    /// The `@temperlang/core` version its output depends on.
    pub runtime_version: &'static str,
}

/// The first variant: Kotlin-compatible API and names, pretty bodies.
/// Everything is native except statements inside expressions, which the
/// hoisting pass will lower (not built yet: a program that needs it is
/// [`NotYet`]).
pub const JS: Variant = Variant {
    name: "js",
    profile: Profile {
        native: Features::NONE,
    },
    rules: names::RULES,
    runtime_version: "0.6.0",
};

/// Every variant, by name.
pub const VARIANTS: &[&Variant] = &[&JS];

/// Something the translation doesn't cover yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub what: String,
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not yet in the JS backend: {}", self.what)
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

/// Translates a library, laid out as Kotlin's JS backend lays it out:
///
/// ```text
/// hello-world/package.json                ES modules, depending on @temperlang/core
/// hello-world/index.js                    export * from "./hello_world.js";
/// hello-world/hello_world.js              export { … } from "./hello_world.internal.js";
/// hello-world/hello_world.internal.js     the library's root module
/// ```
///
/// The package takes the library's name; the module's files take it with
/// `_` for `-`. Kotlin also copies `@temperlang/core` beside the package;
/// this doesn't, yet.
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    generate(p, variant, None)
}

/// [`translate`], with a source map for the `.internal.js` module, when
/// the program has spans: `hello_world.internal.js.map` after it, and a
/// `//# sourceMappingURL=` comment ending the module. The map names each
/// source by its path after `sources_prefix`, which leads from the
/// package's directory to the library's root (`../../src/`, say), and
/// leaves out the sources' text. It maps each statement and function, and
/// each function's name, with its source name (notes/tmpl/nodes.md,
/// "Source positions").
pub fn translate_with_source_maps(
    p: &Program,
    variant: &Variant,
    sources_prefix: &str,
) -> Result<Vec<OutFile>, NotYet> {
    generate(p, variant, Some(sources_prefix))
}

fn generate(p: &Program, variant: &Variant, maps: Option<&str>) -> Result<Vec<OutFile>, NotYet> {
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
    let file = names::module_file(package);
    let mut translator = translate::Translator::new(p, &names);
    let tree = translator.module(module)?;
    // In source order, as Kotlin lists them.
    let mut exported: Vec<DeclId> = module
        .items
        .iter()
        .filter_map(|&item| match p.nodes.get(item) {
            Node::Function { decl, .. } | Node::ModuleLet { decl } => Some(decl),
            _ => None,
        })
        .filter(|&d| p.decls.get(d).facts.contains(Facts::EXPORTED))
        .collect();
    exported.sort();
    let exports: Vec<String> = exported.iter().map(|&d| names.get(d).to_string()).collect();
    let package_json = format!(
        r#"{{
    "name": "{package}",
    "type": "module",
    "exports": {{
        "./{file}": "./{file}.js",
        ".": "./index.js"
    }},
    "dependencies": {{
        "@temperlang/core": "{runtime}"
    }}
}}
"#,
        runtime = variant.runtime_version,
    );
    let public = if exports.is_empty() {
        format!("export {{}} from \"./{file}.internal.js\";\n")
    } else {
        let one_line = format!(
            "export {{ {} }} from \"./{file}.internal.js\";",
            exports.join(", ")
        );
        if one_line.len() <= tree::WIDTH {
            format!("{one_line}\n")
        } else {
            let list: String = exports.iter().map(|e| ["  ", e, ",\n"].concat()).collect();
            format!("export {{\n{list}}} from \"./{file}.internal.js\";\n")
        }
    };
    let internal = format!("{file}.internal.js");
    let mapped = tree::module_mapped(&tree);
    let map = maps.and_then(|prefix| source_map(p, &internal, prefix, &mapped));
    let mut module = mapped.text;
    if map.is_some() {
        module.push_str(&format!("//# sourceMappingURL={internal}.map\n"));
    }
    let mut files = vec![
        OutFile {
            path: format!("{package}/package.json"),
            text: package_json,
        },
        OutFile {
            path: format!("{package}/index.js"),
            text: format!("export * from \"./{file}.js\";\n"),
        },
        OutFile {
            path: format!("{package}/{file}.js"),
            text: public,
        },
        OutFile {
            path: format!("{package}/{internal}"),
            text: module,
        },
    ];
    if let Some(map) = map {
        files.push(OutFile {
            path: format!("{package}/{internal}.map"),
            text: map.to_json(),
        });
    }
    Ok(files)
}
