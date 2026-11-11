//! What a REPL session's own functions need beyond the frontend
//! ([`temper_frontend::repl::ReplTools`]): hover's text for `help`, tmpl
//! for `describe(n, "tmpl")`, and this build's backends for `translate`.

use crate::{Registry, checked, translate, with_std};
use std::cell::OnceCell;
use temper_analysis::Analysis;
use temper_frontend::library::Library;
use temper_frontend::print::write_diagnostics;
use temper_frontend::scope::SymbolId;
use temper_frontend::settle::{Settled, settle};
use temper_frontend::sources::Sources;
use temper_tmpl::Program;

/// The library name a REPL module translates as.
const NAME: &str = "interactive";

/// The tools, with std as tmpl, settled the first time a translation
/// uses it.
pub struct ReplTools {
    registry: Registry,
    std: OnceCell<Result<Program, String>>,
}

impl Default for ReplTools {
    fn default() -> ReplTools {
        ReplTools {
            registry: Registry::built_in(),
            std: OnceCell::new(),
        }
    }
}

impl ReplTools {
    /// A library settled, as tmpl, or why not: its diagnostics, then what
    /// tmpl doesn't cover.
    fn program(library: Library) -> Result<Program, String> {
        let settled = settle(library);
        checked(&settled, NAME).map_err(|e| {
            let mut diagnostics = String::new();
            write_diagnostics(&settled, &mut diagnostics);
            format!("{diagnostics}{e}")
        })
    }
}

impl temper_frontend::repl::ReplTools for ReplTools {
    fn symbol_help(&self, s: &Settled, symbol: SymbolId) -> Option<String> {
        let hover = Analysis::of(s).symbol_hover(symbol);
        let mut text = hover.headline();
        if let Some(doc) = &hover.doc {
            text.push_str("\n\n");
            text.push_str(doc.trim_end());
        }
        Some(text)
    }

    /// Without its spans, which say where in the module's source each
    /// declaration and expression came from.
    fn tmpl(&self, library: Library, _sources: &Sources) -> Result<String, String> {
        let text = temper_tmpl::text::write(&ReplTools::program(library)?);
        Ok(match text.find("\nspans {") {
            Some(at) => text[..at].trim_end().to_string(),
            None => text,
        })
    }

    fn translate(
        &self,
        library: Library,
        sources: &Sources,
        language: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let backend = (self.registry.get(language))
            .ok_or_else(|| format!("built without the {language} backend"))?;
        let program = ReplTools::program(library)?;
        let std = || {
            let std = self.std.get_or_init(|| {
                temper_frontend::stdlib::program(sources).map_err(|e| format!("std: {e}"))
            });
            std.as_ref().map_err(String::clone)
        };
        let program = with_std(program, backend, std)?;
        let files = translate(&program, backend, None)?;
        Ok(files
            .into_iter()
            .filter(|(path, _)| own_code(path))
            .collect())
    }

    fn languages(&self) -> Vec<String> {
        (self.registry.all().iter())
            .map(|b| b.name.to_string())
            .collect()
    }
}

/// Whether a generated file holds the module's own code, rather than
/// what every library gets: a package's metadata and build files, its
/// entry points and re-exports, and support code.
fn own_code(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let source = [".py", ".js", ".lua", ".java", ".cs", ".cpp", ".hpp", ".rs"];
    let boilerplate = [
        "__init__.py",
        "index.js",
        "interactive.js",
        "main.cpp",
        "Logging.cs",
        "Program.cs",
        "InteractiveMain.java",
        "lib.rs",
        "main.rs",
    ];
    source.iter().any(|e| name.ends_with(e))
        && !boilerplate.contains(&name)
        && !path.contains("/support/")
        && !path.contains("/program/")
}
