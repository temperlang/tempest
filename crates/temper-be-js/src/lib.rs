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
//! So far it covers tmpl's subset A, with several modules per library,
//! imports between them and from other libraries, and tests, as Mocha
//! files; and from subset B, classes and interfaces as Kotlin's JS backend
//! writes them (JS classes over `@temperlang/core`'s `type` mixin, fields
//! as `#name` with getters and setters, and `C.new({ … })` factories),
//! with `is`, `as`, casts, and identity (`translate::classes`); and
//! nullable types (`T | null`, `null`, `== null` checks, `??` for its
//! tmpl form, and `===` or a null-safe helper call for `==` with a side
//! that may be null), `orelse` as `try { … } catch { … }`, `bubble()`,
//! and `panic()`, as Kotlin's. What tmpl has inside expressions that JS
//! writes as statements (an `orelse`, or a block value) the translator
//! hoists into temporaries before the statement (`t$0`), keeping earlier
//! operands in temporaries first so they still run first, and running
//! the hoisted statements only where the expression would (inside `&&`,
//! `||`, `if` values, and each time round a loop's condition). Anything
//! else is [`NotYet`].

pub mod connected;
pub mod names;
pub mod ops;
pub mod runtime;
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
/// translator hoists itself until the hoisting pass is built.
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
/// hello-world/index.js                    imports each other module, then
///                                         export * from "./hello_world.js";
/// hello-world/hello_world.js              export { … } from "./hello_world.internal.js";
/// hello-world/hello_world.internal.js     the library's root module
/// hello-world/helpers/deep.js             another module, by its path, and
/// hello-world/helpers/deep.internal.js    its code
/// hello-world/test/hello_world.js         the root module's tests, for Mocha
/// ```
///
/// The package takes the library's name; the root module's files take it
/// with `_` for `-`, and other modules' their paths. A module imports
/// another's exports from its public file, and its tests import from its
/// `.internal.js`, which exports what they use, and a setter, `NAME$set`,
/// for each `var` they assign. Kotlin also copies `@temperlang/core` beside the package;
/// `tempest build` does too ([`runtime`]).
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    generate(p, variant, None)
}

/// [`translate`], with a source map for each `.internal.js` module, when
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
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    let package = &p.library.name;
    let mut translator = translate::Translator::new(p, &names);
    let mut files = Vec::new();
    let mut entries = Vec::new();
    let mut index = String::new();
    let mut tested = false;
    for module in &p.modules {
        let file = translate::module_file(p, module.decl);
        let depth = file.matches('/').count();
        let access = translate::TestAccess::of(p, module);
        let tree = translator.module(module, &access)?;
        // In source order, as Kotlin lists them.
        let mut exported: Vec<DeclId> = module
            .items
            .iter()
            .filter_map(|&item| match p.nodes.get(item) {
                Node::Function { decl, .. }
                | Node::ModuleLet { decl }
                | Node::Class { decl, .. }
                | Node::Interface { decl, .. } => Some(decl),
                _ => None,
            })
            .filter(|&d| p.decls.get(d).facts.contains(Facts::EXPORTED))
            .collect();
        exported.sort();
        let exports: Vec<String> = exported.iter().map(|&d| names.get(d).to_string()).collect();
        let leaf = file.rsplit('/').next().unwrap_or(&file);
        let public = public_module(&exports, leaf);
        let internal = format!("{file}.internal.js");
        let internal_leaf = format!("{leaf}.internal.js");
        let mapped = tree::module_mapped(&tree);
        // Sources are in the module's directory under the library's root.
        let prefix = maps.map(|prefix| {
            let mut full = "../".repeat(depth);
            full.push_str(prefix);
            if !module.path.is_empty() {
                full.push_str(&module.path);
                full.push('/');
            }
            full
        });
        let map = prefix.and_then(|prefix| source_map(p, &internal_leaf, &prefix, &mapped));
        let mut text = mapped.text;
        if map.is_some() {
            text.push_str(&format!("//# sourceMappingURL={internal_leaf}.map\n"));
        }
        files.push(OutFile {
            path: format!("{package}/{file}.js"),
            text: public,
        });
        files.push(OutFile {
            path: format!("{package}/{internal}"),
            text,
        });
        if let Some(map) = map {
            files.push(OutFile {
                path: format!("{package}/{internal}.map"),
                text: map.to_json(),
            });
        }
        if let Some(tests) = translator.tests(module)? {
            tested = true;
            files.push(OutFile {
                path: format!("{package}/test/{file}.js"),
                text: tree::module(&tests),
            });
        }
        entries.push(format!("        \"./{file}\": \"./{file}.js\","));
        if module.path.is_empty() {
            index.push_str(&format!("export * from \"./{file}.js\";\n"));
        } else {
            index.push_str(&format!("import \"./{file}.js\";\n"));
        }
    }
    // The root module's `export *` goes last.
    let (imports, exports): (Vec<&str>, Vec<&str>) =
        index.lines().partition(|l| l.starts_with("import"));
    let mut index = String::new();
    for line in imports.iter().chain(&exports) {
        index.push_str(line);
        index.push('\n');
    }
    let tests = if tested {
        r#"
    "scripts": {
        "test": "mocha test --recursive --reporter mocha-junit-reporter"
    },"#
    } else {
        ""
    };
    let dev = if tested {
        r#",
    "devDependencies": {
        "mocha": "^10.0.0",
        "mocha-junit-reporter": "^2.0.2"
    }"#
    } else {
        ""
    };
    let package_json = format!(
        r#"{{
    "name": "{package}",
    "type": "module",
    "exports": {{
{entries}
        ".": "./index.js"
    }},{tests}
    "dependencies": {{
{dependencies}
    }}{dev}
}}
"#,
        entries = entries.join("\n"),
        dependencies = {
            // Sorted, as Kotlin's: `@temperlang/core` first.
            let mut lines = vec![format!(
                "        \"@temperlang/core\": \"{}\"",
                variant.runtime_version
            )];
            for (name, version) in p.dependencies() {
                let name = translate::package_of(name);
                let version = if version == "0.0.0" { "*" } else { version };
                lines.push(format!("        \"{name}\": \"{version}\""));
            }
            lines.join(",\n")
        },
    );
    files.insert(
        0,
        OutFile {
            path: format!("{package}/index.js"),
            text: index,
        },
    );
    files.insert(
        0,
        OutFile {
            path: format!("{package}/package.json"),
            text: package_json,
        },
    );
    Ok(files)
}

/// A module's public file, re-exporting its exports from its
/// `.internal.js`, beside it.
fn public_module(exports: &[String], leaf: &str) -> String {
    if exports.is_empty() {
        return format!("export {{}} from \"./{leaf}.internal.js\";\n");
    }
    let one_line = format!(
        "export {{ {} }} from \"./{leaf}.internal.js\";",
        exports.join(", ")
    );
    if one_line.len() <= tree::WIDTH {
        format!("{one_line}\n")
    } else {
        let list: String = exports.iter().map(|e| ["  ", e, ",\n"].concat()).collect();
        format!("export {{\n{list}}} from \"./{leaf}.internal.js\";\n")
    }
}
