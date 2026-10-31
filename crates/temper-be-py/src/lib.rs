//! Temper's Python backend: tmpl in, Python out.
//!
//! Works from tmpl alone (notes/tmpl/overview.md, "Boundaries"). Its
//! variants share the translation and differ only in data
//! (notes/tmpl/profiles.md, "Variants and the first profiles"); the first,
//! [`PY`], is Kotlin-compatible: the same package and module layout,
//! `temper_core` runtime, and public names as Kotlin Temper's Python
//! backend, with prettier function bodies (no numbered names, f-strings,
//! Python's own operators where they agree with Temper's).
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! Python's rules ([`names::RULES`]), then [`translate`] into Python's
//! output tree ([`tree`]), with operators spelled by [`ops`], core's
//! connected declarations by [`connected`], and classes and interfaces
//! by [`classes`], then the printer.
//!
//! So far it covers tmpl's subset A, with several modules per library,
//! imports between them and from other libraries, and tests; and from
//! subset B, classes and interfaces (fields, constructors, methods,
//! getters and setters, statics, default and abstract members, `is`,
//! `as`, `==` and `toString` protocols), and nullable types (`T | None`,
//! `is None` checks, `??` as a `:=` test), `orelse` as `try` and
//! `except Exception`, `bubble()` as `raise RuntimeError()`, and
//! `panic()` as `raise BaseException('panic')`, which no `orelse`
//! catches; and generics: module-level `TypeVar`s (invariant, as
//! Temper's type parameters are, where Kotlin's makes some covariant),
//! `Generic[…]` bases, and core's lists and maps as Kotlin's runtime
//! models them (`List` and `Listed` a `Sequence`, a literal a `tuple`,
//! `ListBuilder` a `MutableSequence` made as `[]`, `Map` a
//! `MappingProxyType`, `Mapped` a `Mapping`, and `MapBuilder` a
//! `dict`). What can't go inside a Python expression (an `orelse`, a
//! block's statements, an `if` or `&&` whose branch needs statements) is
//! hoisted into statements before it, with temporaries keeping what's
//! evaluated earlier in order ([`translate`]). Anything else is
//! [`NotYet`].

pub mod classes;
pub mod connected;
pub mod names;
pub mod ops;
pub mod runtime;
pub mod translate;
pub mod tree;

use std::fmt;
use temper_tmpl::naming::{NamingRules, assign};
use temper_tmpl::{Features, Profile, Program};

/// A named set of translation choices for Python.
pub struct Variant {
    /// Its name on the command line: `py`.
    pub name: &'static str,
    /// What it handles natively, which lowering and validation read.
    pub profile: Profile,
    pub rules: NamingRules,
    /// The `temper-core` version its output depends on.
    pub runtime_version: &'static str,
}

/// The first variant: Kotlin-compatible API and names, pretty bodies.
/// Everything is native; statements inside expressions are hoisted by
/// the translator itself.
pub const PY: Variant = Variant {
    name: "py",
    profile: Profile {
        native: Features::STATEMENT_EXPRESSIONS,
    },
    rules: names::RULES,
    runtime_version: "0.6.0",
};

/// Every variant, by name.
pub const VARIANTS: &[&Variant] = &[&PY];

/// Something the translation doesn't cover yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub what: String,
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not yet in the Python backend: {}", self.what)
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

/// `pyproject.toml`'s dependencies, as Kotlin writes them: the runtime
/// and each library used, by its name, with `==` its version where it
/// sets one, a quoted line each, sorted.
fn dependencies(p: &Program, variant: &Variant) -> String {
    let mut lines = vec![format!("temper-core=={}", variant.runtime_version)];
    for (name, version) in p.dependencies() {
        let name = translate::library_name(name);
        let name = name.as_str();
        lines.push(match version {
            "0.0.0" | "" => name.to_string(),
            v => format!("{name}=={v}"),
        });
    }
    lines.sort();
    let quoted: Vec<String> = lines.iter().map(|l| format!("    \"{l}\"")).collect();
    quoted.join(",\n")
}

/// Translates a library, laid out as Kotlin's Python backend lays it out:
///
/// ```text
/// mods/pyproject.toml
/// mods/mods/__init__.py            imports each module, then
///                                  from .mods import *
/// mods/mods/mods.py                the library's root module
/// mods/mods/helpers/__init__.py    a module with modules under it
/// mods/mods/helpers/deep.py        one without
/// mods/tests/__init__.py           empty
/// mods/tests/test_mods.py          the root module's tests
/// mods/tests/test_helpers_deep.py  another's
/// ```
///
/// The project directory takes the library's name; the package and the
/// root module's file take it as a Python name (`hello-world` gives
/// `hello_world`).
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    let library = &p.library.name;
    let package = names::library_package(library);
    let mut translator = translate::Translator::new(p, &names);
    let mut sources = Vec::new();
    let mut tests = Vec::new();
    let mut inits = Vec::new();
    for module in &p.modules {
        let path = &module.path;
        let parent =
            !path.is_empty() && (p.modules.iter()).any(|m| m.path.starts_with(&format!("{path}/")));
        let file = if path.is_empty() {
            format!("{package}/{package}.py")
        } else if parent {
            format!("{package}/{path}/__init__.py")
        } else {
            format!("{package}/{path}.py")
        };
        if file == format!("{package}/{package}.py") && !path.is_empty() {
            return Err(NotYet {
                what: format!("a module named as its library's root, {path}"),
            });
        }
        let tree = translator.module(module)?;
        sources.push(OutFile {
            path: format!("{library}/{file}"),
            text: tree::module(&tree),
        });
        let stem = match path.as_str() {
            "" => package.clone(),
            path => names::package_name(&path.replace('/', "_")),
        };
        let class = format!("Test{}", temper_tmpl::naming::pascal_case(&stem));
        if let Some(tree) = translator.tests(module, &class)? {
            tests.push(OutFile {
                path: format!("{library}/tests/test_{stem}.py"),
                text: tree::module(&tree),
            });
        }
        if !path.is_empty() {
            let (parent, leaf) = match path.rsplit_once('/') {
                Some((parent, leaf)) => {
                    let parts: Vec<String> = parent.split('/').map(names::package_name).collect();
                    (parts.join("."), leaf)
                }
                None => (String::new(), path.as_str()),
            };
            // `as` itself: an explicit re-export, which checkers don't call
            // an unused import.
            let leaf = names::package_name(leaf);
            inits.push(format!("from .{parent} import {leaf} as {leaf}\n"));
        }
    }
    if !tests.is_empty() {
        tests.insert(
            0,
            OutFile {
                path: format!("{library}/tests/__init__.py"),
                text: String::new(),
            },
        );
    }
    let init = format!("{}from .{package} import *\n", inits.concat());
    let pyproject = format!(
        r#"[project]
name = "{library}"
version = "{version}"
description = ""
readme = {{text = "n/a", content-type = "text/plain"}}
requires-python = ">=3.11"
license = {{ text = "LicenseRef-Proprietary" }}
authors = [  ]

dependencies = [
{dependencies}
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
"#,
        version = p.version_or_default(),
        dependencies = dependencies(p, variant),
    );
    Ok([
        OutFile {
            path: format!("{library}/pyproject.toml"),
            text: pyproject,
        },
        OutFile {
            path: format!("{library}/{package}/__init__.py"),
            text: init,
        },
    ]
    .into_iter()
    .chain(sources)
    .chain(tests)
    .collect())
}
