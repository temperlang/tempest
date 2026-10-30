//! Temper's C++ backend: tmpl in, C++ out.
//!
//! Works from tmpl alone (notes/tmpl/overview.md, "Boundaries"). Its
//! variants share the translation and differ only in data
//! (notes/tmpl/profiles.md, "Variants and the first profiles"); the first,
//! [`CPP`], is Kotlin-compatible: the same layout (`LIBRARY/init.hpp` and
//! `LIBRARY/init.cpp` for the library's root module, and
//! `LIBRARY/main.cpp`, which runs it), the same namespace (the library's
//! name, [`names::namespace`]), names, and init function
//! (`global_init_init()`, which a program calls before anything else),
//! the `temper-core` runtime, included as `<temper-core/core.hpp>` from
//! the directory above the library's, and C++14, as Kotlin Temper's `cpp`
//! backend, with prettier bodies: no numbered names, the source's names
//! for parameters and locals, a `core` alias for `temper::core`, natural
//! control flow without Kotlin's extra blocks and labels, and blank lines
//! between functions. Like Kotlin's, it writes no build files: Kotlin
//! builds a library with `g++ -std=c++14 -I..` on its `.cpp` files and
//! its dependencies'.
//!
//! Where Kotlin's isn't safe, it differs ([`translate`]): its init
//! function runs once through `std::call_once`, its module `var`s that
//! functions write are read and written under a lock, it keeps Temper's
//! left-to-right order of evaluation where the order would show, and its
//! shifts are defined for every count. It also declares only exports in
//! the header, which Kotlin's fills with every function. It compiles
//! without warnings under `-Wall -Wextra -Wpedantic` and more (its tests
//! list them), and runs clean under AddressSanitizer,
//! UndefinedBehaviorSanitizer, and ThreadSanitizer.
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! C++'s rules ([`names::RULES`]), then [`translate`] into C++'s output
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

/// A named set of translation choices for C++.
pub struct Variant {
    /// Its name on the command line: `cpp`.
    pub name: &'static str,
    /// What it handles natively, which lowering and validation read.
    pub profile: Profile,
    pub rules: NamingRules,
    /// The C++ standard its output keeps to, as `g++ -std=` names it.
    pub standard: &'static str,
}

/// The first variant: Kotlin-compatible API and names, pretty bodies,
/// C++14, as Kotlin's (`-std=c++14`). Everything is native except
/// statements inside expressions, which the hoisting pass will lower (not
/// built yet: a program that needs it is [`NotYet`]), and left-to-right
/// order of evaluation, which the translator keeps with temporaries.
pub const CPP: Variant = Variant {
    name: "cpp",
    profile: Profile {
        native: Features::NONE,
    },
    rules: names::RULES,
    standard: "c++14",
};

/// Every variant, by name.
pub const VARIANTS: &[&Variant] = &[&CPP];

/// Something the translation doesn't cover yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub what: String,
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not yet in the C++ backend: {}", self.what)
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

/// Translates a library, laid out as Kotlin's C++ backend lays it out:
///
/// ```text
/// hello-world/init.hpp    the root module's header: exports and the init function
/// hello-world/init.cpp    its code, in namespace hello_world
/// hello-world/main.cpp    a program that runs it
/// ```
///
/// Kotlin also copies the runtime beside the library, as `temper-core/`,
/// which the files include from the directory above; this doesn't, yet.
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
    let namespace = names::namespace(library);
    let init = "global_init_init";
    let mut translator = translate::Translator::new(p, &names, module);
    let mut translated = translator.module(module)?;
    translated.source.includes = vec![format!("<{library}/init.hpp>"), "<mutex>".into()];
    translated.source.namespace = namespace.clone();
    translated.source.init = init.into();
    Ok(vec![
        OutFile {
            path: format!("{library}/init.hpp"),
            text: header(&namespace, &translated, init),
        },
        OutFile {
            path: format!("{library}/init.cpp"),
            text: tree::source(&translated.source),
        },
        OutFile {
            path: format!("{library}/main.cpp"),
            text: main(library, &namespace, init),
        },
    ])
}

/// The header: the runtime, then the module's exports and its init
/// function, in its namespace.
fn header(namespace: &str, t: &translate::Translated, init: &str) -> String {
    let mut out =
        format!("#pragma once\n\n#include <temper-core/core.hpp>\n\nnamespace {namespace} {{\n\n");
    for (ty, name, shared) in &t.exported_variables {
        if *shared {
            out.push_str(
                "// The module's functions write it under their lock, so read it only\n\
                 // while no other thread may be calling them.\n",
            );
        }
        out.push_str(&format!("extern {ty} {name};\n"));
    }
    if !t.exported_variables.is_empty() {
        out.push('\n');
    }
    for f in &t.exported_functions {
        out.push_str(&format!("{f};\n"));
    }
    if !t.exported_functions.is_empty() {
        out.push('\n');
    }
    out.push_str(&format!(
        "// Runs the module's top level, once; call it before anything else.\n\
         void {init}();\n\n}}  // namespace {namespace}\n"
    ));
    out
}

/// The program: runs the library's init, as Kotlin's `main.cpp` does.
fn main(library: &str, namespace: &str, init: &str) -> String {
    format!("#include \"{library}/init.hpp\"\n\nint main() {{\n  {namespace}::{init}();\n}}\n")
}
