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
//! output tree ([`tree`]), with operators spelled by [`ops`] and core's
//! connected declarations by [`connected`], then the printer.
//!
//! So far it covers tmpl's subset A, one module per library; anything
//! else is [`NotYet`].

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
/// Everything is native except statements inside expressions, which the
/// hoisting pass will lower (not built yet: a program that needs it is
/// [`NotYet`]).
pub const PY: Variant = Variant {
    name: "py",
    profile: Profile {
        native: Features::NONE,
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
/// fibonacci/pyproject.toml
/// fibonacci/fibonacci/__init__.py      from .fibonacci import *
/// fibonacci/fibonacci/fibonacci.py     the library's root module
/// ```
///
/// The project directory takes the library's name; the package and the
/// root module's file take it as a Python name (`hello-world` gives
/// `hello_world`).
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
    let package = names::package_name(library);
    let mut translator = translate::Translator::new(p, &names);
    let tree = translator.module(module)?;
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
    Ok(vec![
        OutFile {
            path: format!("{library}/pyproject.toml"),
            text: pyproject,
        },
        OutFile {
            path: format!("{library}/{package}/__init__.py"),
            text: format!("from .{package} import *\n"),
        },
        OutFile {
            path: format!("{library}/{package}/{package}.py"),
            text: tree::module(&tree),
        },
    ])
}
