//! Builtins' declarations in core (`temper_frontend::corelib`): where
//! core declares a builtin type, function, or value, or a builtin type's
//! member, for go to definition, and its doc comment, for hover. Core
//! isn't one of the library's files, so its places are in core's own
//! text, by path from core's root.

use super::*;
use members::Named;
use temper_frontend::builtins::{Builtin, Constant};
use temper_frontend::corelib;
use temper_frontend::typing::{core_declaration, core_member_declarations};

/// A place in one of core's files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreLocation {
    /// Its file's path from core's root, as `core.temper`, one of
    /// [`corelib::FILES`].
    pub path: &'static str,
    /// Its file's text.
    pub source: &'static str,
    /// Byte offsets in its file.
    pub span: Span,
}

impl Analysis<'_> {
    /// Where core declares the builtin named at byte `offset` of `file`:
    /// a builtin type, function, or value, or a builtin type's member,
    /// through generic receivers and supertypes. None for anything else,
    /// and for builtins core doesn't declare.
    pub fn core_definition(&self, file: FileId, offset: u32) -> Option<CoreLocation> {
        let (_, target) = self.target(file, offset)?;
        let (ats, name) = self.core_target(target?)?;
        let at = *ats.first()?;
        Some(CoreLocation {
            path: corelib::MAIN,
            source: corelib::TEXT,
            span: Span {
                start: at as u32,
                end: (at + name.len()) as u32,
            },
        })
    }

    /// Where core declares what a target names, if it's a builtin, with
    /// its name: a member's declarations nearest first, from its type's
    /// own through its supertypes'; never empty.
    pub(crate) fn core_target(&self, target: Target) -> Option<(Vec<usize>, String)> {
        match target {
            Target::Symbol(symbol) => {
                let s = self.settled.scopes.symbol(symbol);
                if s.decl.is_some() || !matches!(s.kind, SymbolKind::Builtin(_)) {
                    return None;
                }
                let name = self.settled.library.names.get(s.name);
                Some((vec![core_declaration(name)?], name.to_string()))
            }
            Target::Member { call, key } => {
                if let Some(Resolved::Decl(_)) = self.member(call) {
                    return None;
                }
                let name = self.settled.library.names.get(key);
                let owner = self.builtin_owner(call)?;
                let ats = core_member_declarations(owner, name);
                (!ats.is_empty()).then(|| (ats, name.to_string()))
            }
        }
    }

    /// The builtin type whose member a `.` call reads: its object's type,
    /// or the type its object names, for a static, or `Console` for the
    /// builtin `console`.
    fn builtin_owner(&self, call: NodeId) -> Option<&'static str> {
        let object = *self.settled.library.tree.children(call).get(1)?;
        if let Some(Named::Builtin(name)) = self.type_named(object) {
            return Some(name);
        }
        if self.settled.scopes.resolved_builtin(object) == Some(Builtin::Value(Constant::Console)) {
            return Some("Console");
        }
        match self.receiver_of(call)? {
            members::Receiver::Builtin { name, .. } => Some(name),
            members::Receiver::Class { .. } => None,
        }
    }
}

/// The doc comment, `/** … */`, right before a declaration whose name is
/// at byte `at` of core's text, past decorations' lines, as
/// [`doc_text`] gives it.
pub(crate) fn core_doc(at: usize) -> Option<String> {
    let text = corelib::TEXT;
    let line_start = text.get(..at)?.rfind('\n').map_or(0, |i| i + 1);
    if text[line_start..at].contains(|c| ";{}()=,".contains(c)) {
        return None;
    }
    let mut end = line_start;
    loop {
        let before = text[..end].trim_end();
        if before.ends_with("*/") {
            let start = before.rfind("/*")?;
            return doc_text(before.get(start..)?);
        }
        let line = before.rfind('\n').map_or(0, |i| i + 1);
        if !before[line..].trim_start().starts_with('@') || line == 0 {
            return None;
        }
        end = line;
    }
}
