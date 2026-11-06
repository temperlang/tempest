//! Editor queries over one settled library: diagnostics, hover, go to
//! definition, and completion, for the playground now and a language
//! server and `watch` later (docs/internals/08-playground.md).
//!
//! An [`Analysis`] owns one [`Settled`] library and answers from what the
//! settle loop already worked out: each name's symbol ([`Scopes`]), each
//! declaration's and expression's type ([`temper_frontend::typing::Typing`]), and
//! each node's span. Nothing is settled again for a query.
//!
//! Positions here are byte offsets in one file. Editors count UTF-16 code
//! units (CodeMirror and LSP alike), so callers convert once, at their
//! boundary, with [`Position`] and [`byte_offset`].
//!
//! This crate depends on `temper-frontend` only, not tmpl or backends.
//! Backend-aware queries (per-declaration output previews, backend
//! diagnostics, and naming hints) may come later, lazily computed, behind
//! an optional cargo feature.

use std::collections::HashMap;
use temper_frontend::core::{
    self, CLASS_INTERFACE, LET_VAR, ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK, ROLE_METHOD,
    ROLE_PARAM, ROLE_SETTER, ROLE_TYPE_PARAM,
};
use temper_frontend::library::{FileId, Level};
use temper_frontend::scope::{BUILTINS, ScopeId, Scopes, SymbolId, SymbolKind};
use temper_frontend::settle::Settled;
use temper_frontend::types::{TypeData, TypeId, show_result};
use temper_frontend::typing::Resolved;
use temper_syntax::adapt::CommentKind;
use temper_syntax::ast::Value;
use temper_syntax::tree::{NodeId, NodeKind, Span};

/// What a declaration is, as hover shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
    Let,
    Var,
    Function,
    Parameter,
    TypeParameter,
    Class,
    Interface,
    Field,
    Method,
    Getter,
    Setter,
    Constructor,
    /// One of Temper's builtins, as `console` or `Int`.
    Builtin,
}

impl DeclKind {
    /// As people say it: `let`, `type parameter`, `method`.
    pub fn label(self) -> &'static str {
        match self {
            DeclKind::Let => "let",
            DeclKind::Var => "var",
            DeclKind::Function => "function",
            DeclKind::Parameter => "parameter",
            DeclKind::TypeParameter => "type parameter",
            DeclKind::Class => "class",
            DeclKind::Interface => "interface",
            DeclKind::Field => "field",
            DeclKind::Method => "method",
            DeclKind::Getter => "getter",
            DeclKind::Setter => "setter",
            DeclKind::Constructor => "constructor",
            DeclKind::Builtin => "builtin",
        }
    }
}

/// A problem, at a span in a file.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub level: Level,
    pub message: String,
    pub file: FileId,
    /// Byte offsets in its file.
    pub span: Span,
}

/// What hovering over a position shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hover {
    /// What it's about, in the file's bytes: a name, or the smallest
    /// typed expression around the position.
    pub span: Span,
    /// The name, if it's a name.
    pub name: Option<String>,
    /// What its declaration is, if it's a name.
    pub kind: Option<DeclKind>,
    /// Its type as messages write it, `Int32` or `fn (String): Boolean`;
    /// for a function, its signature with parameter names, as
    /// `greet(name: String): String`.
    pub ty: Option<String>,
    /// The declaration's doc comment, `/** … */`, without its markers.
    pub doc: Option<String>,
}

/// A place in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub file: FileId,
    /// Byte offsets in its file.
    pub span: Span,
}

/// A name that may go at a position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub name: String,
    pub kind: DeclKind,
    /// Its type, if known.
    pub detail: Option<String>,
}

/// Queries over one settled library.
pub struct Analysis {
    pub settled: Settled,
    /// Each node's parent, under the files' roots.
    parents: Vec<Option<NodeId>>,
    /// Each declaration's symbol, by its `Let` or `Class`.
    decl_symbols: HashMap<NodeId, SymbolId>,
}

impl Analysis {
    pub fn new(settled: Settled) -> Analysis {
        let tree = &settled.library.tree;
        let mut parents = vec![None; tree.len()];
        let mut stack: Vec<NodeId> = settled.library.files.iter().map(|f| f.root).collect();
        let mut seen = vec![false; tree.len()];
        while let Some(node) = stack.pop() {
            if std::mem::replace(&mut seen[node.0 as usize], true) {
                continue;
            }
            for &child in tree.children(node) {
                if parents[child.0 as usize].is_none() {
                    parents[child.0 as usize] = Some(node);
                }
                stack.push(child);
            }
        }
        let decl_symbols = (settled.scopes.symbols.iter().enumerate())
            .filter_map(|(i, s)| Some((s.decl?, SymbolId(i as u32))))
            .collect();
        Analysis {
            settled,
            parents,
            decl_symbols,
        }
    }

    /// Every diagnostic, in the order found, with its span in its file.
    pub fn diagnostics(&self) -> Vec<Problem> {
        let library = &self.settled.library;
        (library.diagnostics.iter())
            .map(|d| {
                let (file, start) = library.locate(d.span.start);
                Problem {
                    level: d.level,
                    message: d.message.text(),
                    file,
                    span: Span {
                        start,
                        end: start + d.span.end.saturating_sub(d.span.start),
                    },
                }
            })
            .collect()
    }

    /// What's at byte `offset` of `file`: the name there, with its
    /// declaration's kind, type, and doc comment, or else the type of the
    /// smallest expression around it.
    pub fn hover(&self, file: FileId, offset: u32) -> Option<Hover> {
        let (node, target) = self.target(file, offset)?;
        let span = self.local(node);
        let Some(target) = target else {
            let t = self.settled.typing.expr(node)?;
            return Some(Hover {
                span,
                name: None,
                kind: None,
                ty: Some(self.show(t)),
                doc: None,
            });
        };
        let (name, kind, ty, decl) = match target {
            Target::Symbol(symbol) => {
                let s = self.settled.scopes.symbol(symbol);
                let name = self.settled.library.names.get(s.name).to_string();
                let kind = self.kind(symbol);
                let ty = match s.decl {
                    Some(decl) => self.decl_type(decl, &name, node),
                    None => self.settled.typing.expr(node).map(|t| self.show(t)),
                };
                (name, kind, ty, s.decl)
            }
            Target::Member { call, key } => {
                let name = self.settled.library.names.get(key).to_string();
                match self.member(call) {
                    Some(Resolved::Decl(decl)) => {
                        let decl = *decl;
                        let kind = self.decl_kind(decl, None);
                        (
                            name.clone(),
                            kind,
                            self.decl_type(decl, &name, call),
                            Some(decl),
                        )
                    }
                    _ => {
                        let ty = self.settled.typing.expr(call).map(|t| self.show(t));
                        (name, DeclKind::Builtin, ty, None)
                    }
                }
            }
        };
        Some(Hover {
            span,
            name: Some(name),
            kind: Some(kind),
            ty,
            doc: decl.and_then(|d| self.doc(d)),
        })
    }

    /// Where the name at byte `offset` of `file` is declared: its
    /// declaration's name, in whichever file that is. None for builtins.
    pub fn definition(&self, file: FileId, offset: u32) -> Option<Location> {
        let decl = match self.target(file, offset)?.1? {
            Target::Symbol(symbol) => self.settled.scopes.symbol(symbol).decl?,
            Target::Member { call, .. } => match self.member(call)? {
                Resolved::Decl(decl) => *decl,
                _ => return None,
            },
        };
        let tree = &self.settled.library.tree;
        let name = match tree.kind(decl) {
            NodeKind::Let | NodeKind::Class => tree.children(decl)[0],
            _ => decl,
        };
        let (file, _) = self.settled.library.locate(tree.span(name).start);
        Some(Location {
            file,
            span: self.local(name),
        })
    }

    /// The names in scope at byte `offset` of `file`, innermost first,
    /// each once, with builtins last. After `.`, the members of what's
    /// before it, if its type is a class's.
    pub fn completions(&self, file: FileId, offset: u32) -> Vec<Completion> {
        let pos = self.global(file, offset);
        if let Some(members) = self.member_completions(file, offset) {
            return members;
        }
        let mut scope = self.scope_at(file, pos);
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let scopes = &self.settled.scopes;
        while let Some(id) = scope {
            let s = scopes.get(id);
            for &(name, symbol) in s.names.iter().rev() {
                let sym = scopes.symbol(symbol);
                let text = self.settled.library.names.get(name);
                let visible = sym.visible_from <= pos || id == BUILTINS;
                if !visible
                    || self.settled.library.names.is_temporary(name)
                    || text.is_empty()
                    || !text.starts_with(|c: char| c.is_alphabetic() || c == '_')
                    || !seen.insert(name)
                {
                    continue;
                }
                let kind = self.kind(symbol);
                let detail = sym.decl.and_then(|d| self.decl_type(d, text, d));
                out.push(Completion {
                    name: text.to_string(),
                    kind,
                    detail,
                });
            }
            scope = s.parent;
        }
        out
    }

    /// Members after `name.` (or `name.prefix`) at `offset`, when `name`
    /// is a declared local whose type is a class's.
    fn member_completions(&self, file: FileId, offset: u32) -> Option<Vec<Completion>> {
        let source = self
            .settled
            .library
            .files
            .get(file.0 as usize)?
            .parsed
            .source();
        let before = source.get(..offset as usize)?;
        let before = before.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_');
        let before = before.strip_suffix('.')?;
        let object_end = before.len() as u32;
        let object = before
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .filter(|o| !o.is_empty())?;
        let pos = self.global(file, object_end);
        let scope = self.scope_at(file, pos);
        let name = self.settled.library.names.find(object)?;
        let mut at = scope;
        let scopes = &self.settled.scopes;
        let symbol = loop {
            let s = scopes.get(at?);
            if let Some(&(_, symbol)) = s.names.iter().rev().find(|(n, _)| *n == name) {
                break symbol;
            }
            at = s.parent;
        };
        let decl = scopes.symbol(symbol).decl?;
        let t = *self.settled.typing.decls.get(&decl)?;
        let types = &self.settled.typing.types;
        let t = match types.get(t) {
            TypeData::Nullable(inner) => *inner,
            _ => t,
        };
        let base = match types.get(t) {
            TypeData::Applied { base, .. } => *base,
            _ => t,
        };
        let TypeData::Class(class) = *types.get(base) else {
            return Some(Vec::new());
        };
        let class = scopes.symbol(class).decl?;
        let mut out = Vec::new();
        self.class_members(class, &mut out, &mut Vec::new());
        Some(out)
    }

    /// A class's public instance members, then its supertypes'.
    fn class_members(&self, class: NodeId, out: &mut Vec<Completion>, seen: &mut Vec<NodeId>) {
        let tree = &self.settled.library.tree;
        if tree.kind(class) != NodeKind::Class || seen.contains(&class) {
            return;
        }
        seen.push(class);
        let (_, supers, members) = core::class_parts(tree, class);
        for &m in members {
            let data = tree.data(m);
            let role = data & ROLE_MASK;
            if data & (core::MEMBER_STATIC | core::MEMBER_PRIVATE) != 0
                || role == ROLE_CONSTRUCTOR
                || role == ROLE_TYPE_PARAM
            {
                continue;
            }
            let Some(name) = self.settled.library.name(tree.children(m)[0]) else {
                continue;
            };
            let text = self.settled.library.names.get(name).to_string();
            if out.iter().any(|c: &Completion| c.name == text) {
                continue;
            }
            let detail = self.decl_type(m, &text, m);
            out.push(Completion {
                name: text,
                kind: self.decl_kind(m, None),
                detail,
            });
        }
        for &s in supers {
            let s = match tree.kind(s) {
                NodeKind::Call => tree.children(s).first().copied().unwrap_or(s),
                _ => s,
            };
            if let Some(symbol) = self.settled.scopes.resolved(s) {
                if let Some(decl) = self.settled.scopes.symbol(symbol).decl {
                    self.class_members(decl, out, seen);
                }
            }
        }
    }

    /// The innermost scope around a library-wide position in `file`.
    fn scope_at(&self, file: FileId, pos: u32) -> Option<ScopeId> {
        let tree = &self.settled.library.tree;
        let root = self.settled.library.files.get(file.0 as usize)?.root;
        // The smallest node with a scope whose span holds the position.
        let mut best: Option<(u32, ScopeId)> = None;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let span = tree.span(node);
            let holds = span.start <= pos && pos <= span.end;
            if let Some(scope) = self.settled.scopes.of_node(node) {
                let width = span.end - span.start;
                if (holds || node == root) && best.is_none_or(|(w, _)| width <= w) {
                    best = Some((if node == root { u32::MAX } else { width }, scope));
                }
            }
            stack.extend(tree.children(node));
        }
        let module = self.settled.library.files[file.0 as usize].module;
        best.map(|(_, s)| s).or_else(|| {
            (self.settled.modules.iter())
                .find(|m| m.id == module)
                .map(|m| m.scope)
        })
    }

    /// The node at a position, and the name it's a use or declaration
    /// of, if it is one. Names win over other nodes, then smaller spans.
    fn target(&self, file: FileId, offset: u32) -> Option<(NodeId, Option<Target>)> {
        let library = &self.settled.library;
        let tree = &library.tree;
        let root = library.files.get(file.0 as usize)?.root;
        let pos = self.global(file, offset);
        let mut best: Option<(Rank, NodeId, Option<Target>)> = None;
        let mut stack = vec![root];
        let mut seen = std::collections::HashSet::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            stack.extend(tree.children(node));
            let span = tree.span(node);
            if !(span.start <= pos && pos <= span.end) || span.start == span.end {
                continue;
            }
            let target = self.name_target(node);
            let typed = target.is_some() || self.settled.typing.expr(node).is_some();
            if !typed || node == root {
                continue;
            }
            // Prefer a span the position is inside over one it ends,
            // then names, then smaller spans.
            let key = (
                pos < span.end,
                target.is_some(),
                u32::MAX - (span.end - span.start),
            );
            if best.as_ref().is_none_or(|b| b.0 < key) {
                best = Some((key, node, target));
            }
        }
        best.map(|(_, node, target)| (node, target))
    }

    /// The name a node is, if it's one: a use, a declaration's name, or
    /// a member's name after `.`.
    fn name_target(&self, node: NodeId) -> Option<Target> {
        let library = &self.settled.library;
        let tree = &library.tree;
        match tree.kind(node) {
            NodeKind::RightName => self.settled.scopes.resolved(node).map(Target::Symbol),
            NodeKind::LeftName => {
                let parent = self.parent(node)?;
                if library.names.is_temporary(library.name(node)?) {
                    return None;
                }
                (matches!(tree.kind(parent), NodeKind::Let | NodeKind::Class)
                    && tree.children(parent).first() == Some(&node))
                .then(|| self.decl_symbols.get(&parent).copied().map(Target::Symbol))
                .flatten()
            }
            NodeKind::Value => {
                let Some(Value::Symbol(key)) = library.value(node) else {
                    return None;
                };
                let call = self.parent(node)?;
                let children = tree.children(call);
                let is_dot = tree.kind(call) == NodeKind::Call
                    && children.len() == 3
                    && children[2] == node
                    && library
                        .name(children[0])
                        .is_some_and(|n| library.names.get(n) == ".");
                is_dot.then_some(Target::Member { call, key: *key })
            }
            _ => None,
        }
    }

    /// What a `.` call reads: its own resolution, or for a method call,
    /// the call around it's.
    fn member(&self, dot: NodeId) -> Option<&Resolved> {
        let typing = &self.settled.typing;
        let outer = self.parent(dot).filter(|&p| {
            let tree = &self.settled.library.tree;
            tree.kind(p) == NodeKind::Call && tree.children(p).first() == Some(&dot)
        });
        match typing.resolved(dot) {
            Some(r @ Resolved::Decl(_)) => Some(r),
            r => outer.and_then(|o| typing.resolved(o)).or(r),
        }
    }

    fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.parents.get(node.0 as usize).copied().flatten()
    }

    /// What a symbol's declaration is.
    fn kind(&self, symbol: SymbolId) -> DeclKind {
        let s = self.settled.scopes.symbol(symbol);
        match (s.kind, s.decl) {
            (SymbolKind::Builtin(_), _) | (_, None) => DeclKind::Builtin,
            (SymbolKind::TypeParam, _) => DeclKind::TypeParameter,
            (kind, Some(decl)) => self.decl_kind(decl, Some(kind)),
        }
    }

    /// What a `Let` or `Class` declares.
    fn decl_kind(&self, decl: NodeId, kind: Option<SymbolKind>) -> DeclKind {
        let tree = &self.settled.library.tree;
        let data = tree.data(decl);
        if tree.kind(decl) == NodeKind::Class {
            return match data & CLASS_INTERFACE {
                0 => DeclKind::Class,
                _ => DeclKind::Interface,
            };
        }
        if tree.kind(decl) != NodeKind::Let {
            return DeclKind::Let;
        }
        match data & ROLE_MASK {
            ROLE_PARAM => return DeclKind::Field,
            ROLE_METHOD => return DeclKind::Method,
            ROLE_GETTER => return DeclKind::Getter,
            ROLE_SETTER => return DeclKind::Setter,
            ROLE_CONSTRUCTOR => return DeclKind::Constructor,
            ROLE_TYPE_PARAM => return DeclKind::TypeParameter,
            _ => {}
        }
        if kind == Some(SymbolKind::Member) {
            return DeclKind::Field;
        }
        if kind == Some(SymbolKind::Parameter) {
            return DeclKind::Parameter;
        }
        let init = core::let_parts(tree, decl).2;
        if data & LET_VAR == 0 && init.is_some_and(|i| tree.kind(i) == NodeKind::Fn) {
            return DeclKind::Function;
        }
        match data & LET_VAR {
            0 => DeclKind::Let,
            _ => DeclKind::Var,
        }
    }

    /// A declaration's type as hover shows it: a function's or method's
    /// signature with its parameters' names, else its type, or else the
    /// type at `use_site`, which may be narrower (or the only one known).
    fn decl_type(&self, decl: NodeId, name: &str, use_site: NodeId) -> Option<String> {
        let tree = &self.settled.library.tree;
        let typing = &self.settled.typing;
        if tree.kind(decl) == NodeKind::Class {
            let params: Vec<String> = (core::type_params(tree, decl).iter())
                .filter_map(|&p| self.settled.library.name(tree.children(p)[0]))
                .map(|n| self.settled.library.names.get(n).to_string())
                .collect();
            return Some(match params.is_empty() {
                true => name.to_string(),
                false => format!("{name}<{}>", params.join(", ")),
            });
        }
        if tree.kind(decl) == NodeKind::Let {
            let init = core::let_parts(tree, decl).2;
            let role = tree.data(decl) & ROLE_MASK;
            if let Some(f) = init.filter(|&i| tree.kind(i) == NodeKind::Fn) {
                if role == ROLE_GETTER {
                    // A getter reads as a field of its result type.
                    let t = (typing.decls.get(&decl).copied()).or_else(|| typing.expr(f));
                    return match t.map(|t| (t, typing.types.get(t))) {
                        Some((_, TypeData::Fn { ret, .. })) => Some(self.show(*ret)),
                        Some((t, _)) => Some(self.show(t)),
                        None => typing.expr(use_site).map(|t| self.show(t)),
                    };
                }
                if tree.data(decl) & LET_VAR == 0 {
                    return Some(self.signature(f, name, typing.decls.get(&decl).copied()));
                }
            }
        }
        let t = typing
            .decls
            .get(&decl)
            .copied()
            .or_else(|| typing.expr(use_site))?;
        Some(self.show(t))
    }

    /// A function's signature, `name(a: Int32, b?: String): Boolean`.
    fn signature(&self, f: NodeId, name: &str, t: Option<TypeId>) -> String {
        let tree = &self.settled.library.tree;
        let typing = &self.settled.typing;
        let (params, _, _) = core::fn_parts(tree, f);
        let params: Vec<String> = (params.iter())
            .map(|&p| {
                let n = (self.settled.library.name(tree.children(p)[0]))
                    .map_or("_", |n| self.settled.library.names.get(n));
                let optional = core::let_parts(tree, p).2.is_some();
                let ty = typing.decls.get(&p).map_or("?".into(), |&t| self.show(t));
                format!("{n}{}: {ty}", if optional { "?" } else { "" })
            })
            .collect();
        let ret = match t.or_else(|| typing.expr(f)).map(|t| typing.types.get(t)) {
            Some(TypeData::Fn { ret, throws, .. }) => show_result(self.show(*ret), *throws),
            _ => "?".into(),
        };
        format!("{name}({}): {ret}", params.join(", "))
    }

    /// The doc comment just before a declaration, if any: `/** … */`
    /// with nothing but modifiers and decorators between.
    fn doc(&self, decl: NodeId) -> Option<String> {
        let library = &self.settled.library;
        let tree = &library.tree;
        let name = match tree.kind(decl) {
            NodeKind::Let | NodeKind::Class => tree.children(decl)[0],
            _ => return None,
        };
        let (file, start) = library.locate(tree.span(name).start);
        let parsed = &library.files.get(file.0 as usize)?.parsed;
        let source = parsed.source();
        let comment = (parsed.comments().iter())
            .rev()
            .find(|c| c.end <= start && c.kind == CommentKind::Block)?;
        let text = source.get(comment.start as usize..comment.end as usize)?;
        let between = source.get(comment.end as usize..start as usize)?;
        if !text.starts_with("/**") || between.contains(|c| ";{}()=,/".contains(c)) {
            return None;
        }
        let body = text.trim_start_matches("/**").trim_end_matches("*/");
        let lines: Vec<&str> = (body.lines())
            .map(|l| l.trim().trim_start_matches('*').trim())
            .collect();
        let doc = lines.join("\n").trim().to_string();
        (!doc.is_empty()).then_some(doc)
    }

    fn show(&self, t: TypeId) -> String {
        let class_name = |b: SymbolId| {
            let name = self.settled.scopes.symbol(b).name;
            self.settled.library.names.get(name).to_string()
        };
        self.settled.typing.types.display(t, &class_name)
    }

    /// A node's span in its file.
    fn local(&self, node: NodeId) -> Span {
        let span = self.settled.library.tree.span(node);
        let (_, start) = self.settled.library.locate(span.start);
        Span {
            start,
            end: start + span.end.saturating_sub(span.start),
        }
    }

    /// A byte offset in a file as a library-wide position.
    fn global(&self, file: FileId, offset: u32) -> u32 {
        let base = self
            .settled
            .library
            .files
            .get(file.0 as usize)
            .map_or(0, |f| f.base);
        base + offset
    }

    /// The scopes, for callers that need more than the queries give.
    pub fn scopes(&self) -> &Scopes {
        &self.settled.scopes
    }
}

/// How well a node fits a position: whether the position is inside it
/// (not at its end), whether it's a name, and how small it is.
type Rank = (bool, bool, u32);

/// What a name at a position names.
#[derive(Clone, Copy, Debug)]
enum Target {
    Symbol(SymbolId),
    /// A member read, `obj.key`, by its `.` call.
    Member {
        call: NodeId,
        key: temper_syntax::names::NameId,
    },
}

/// Where a byte offset is, as JavaScript and editors count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// UTF-16 code units before it.
    pub utf16: usize,
    /// Its line, from 1.
    pub line: usize,
    /// Its column in UTF-16 code units, from 1.
    pub column: usize,
}

impl Position {
    /// The position of a byte offset in `source`, clamped to its end and
    /// moved back to a character boundary.
    pub fn of(source: &str, offset: usize) -> Position {
        let mut offset = offset.min(source.len());
        while !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let before = &source[..offset];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        Position {
            utf16: before.encode_utf16().count(),
            line: before.matches('\n').count() + 1,
            column: before[line_start..].encode_utf16().count() + 1,
        }
    }
}

/// The byte offset of a UTF-16 offset in `source`, clamped to its end; an
/// offset inside a surrogate pair counts as its character's start.
pub fn byte_offset(source: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (i, c) in source.char_indices() {
        units += c.len_utf16();
        if units > utf16 {
            return i;
        }
    }
    source.len()
}
