//! A file's outline: its declarations, top-level ones and classes'
//! members, for an editor's document symbols. Locals inside functions
//! are left out.

use super::*;

/// A declaration in a file's outline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocSymbol {
    pub name: String,
    pub kind: DeclKind,
    /// Its type or signature, as hover shows it.
    pub detail: Option<String>,
    /// The whole declaration, in its file's bytes.
    pub span: Span,
    /// Its name.
    pub selection: Span,
    /// A class's members.
    pub children: Vec<DocSymbol>,
}

impl Analysis<'_> {
    /// The declarations in `file`, in source order, each class with its
    /// members.
    pub fn document_symbols(&self, file: FileId) -> Vec<DocSymbol> {
        let Some(f) = self.settled.library.files.get(file.0 as usize) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.outline(f.root, &mut out, &mut seen);
        out
    }

    fn outline(
        &self,
        node: NodeId,
        out: &mut Vec<DocSymbol>,
        seen: &mut std::collections::HashSet<NodeId>,
    ) {
        let tree = &self.settled.library.tree;
        if !seen.insert(node) {
            return;
        }
        match tree.kind(node) {
            NodeKind::Class => {
                let Some(mut symbol) = self.doc_symbol(node) else {
                    return;
                };
                let (_, _, members) = core::class_parts(tree, node);
                for &m in members {
                    if tree.data(m) & ROLE_MASK == ROLE_TYPE_PARAM || !seen.insert(m) {
                        continue;
                    }
                    symbol.children.extend(self.doc_symbol(m));
                }
                symbol.children.sort_by_key(|s| s.selection.start);
                out.push(symbol);
                out.sort_by_key(|s| s.selection.start);
            }
            NodeKind::Let => {
                if let Some(symbol) = self.doc_symbol(node) {
                    out.push(symbol);
                    out.sort_by_key(|s| s.selection.start);
                }
                // Not into functions' bodies, but a class may be in an
                // initializer.
                if let Some(init) = core::let_parts(tree, node).2 {
                    if tree.kind(init) != NodeKind::Fn {
                        self.outline(init, out, seen);
                    }
                }
            }
            NodeKind::Fn => {}
            _ => {
                for &child in tree.children(node) {
                    self.outline(child, out, seen);
                }
            }
        }
    }

    /// A `Let`'s or `Class`'s entry, without children, if it declares a
    /// name written in the source.
    fn doc_symbol(&self, decl: NodeId) -> Option<DocSymbol> {
        let library = &self.settled.library;
        let tree = &library.tree;
        let name_node = *tree.children(decl).first()?;
        let name = library.name(name_node)?;
        let text = library.names.get(name);
        if library.names.is_temporary(name) || text.is_empty() {
            return None;
        }
        let selection = self.name_span(name_node);
        let span = self.local(decl);
        // Generated declarations have no text of their own.
        if self.text_of_span(decl, selection) != Some(text) {
            return None;
        }
        let symbol = self.decl_symbols().get(&decl).copied();
        let kind = match symbol {
            Some(s) => self.kind(s),
            None => self.decl_kind(decl, None),
        };
        Some(DocSymbol {
            name: text.to_string(),
            kind,
            detail: self.decl_type(decl, text, decl),
            span: Span {
                start: span.start.min(selection.start),
                end: span.end.max(selection.end),
            },
            selection,
            children: Vec::new(),
        })
    }

    /// The text at a span of the file a node is in.
    fn text_of_span(&self, node: NodeId, span: Span) -> Option<&str> {
        let (file, _) = self
            .settled
            .library
            .locate(self.settled.library.tree.span(node).start);
        self.source(file)?
            .get(span.start as usize..span.end as usize)
    }
}
