//! Members after `.`: what a receiver's type has, with a generic class's
//! type arguments put in for its type parameters (`push(item: String)`
//! on a `Stack<String>`), builtin types' members from core's declarations
//! ([`builtin_members`]), and statics after a type's name.

use super::*;
use temper_frontend::builtins::TYPES;
use temper_frontend::typing::{BuiltinMember, builtin_members};

/// Type parameters' names as their arguments show, by the parameters'
/// symbols.
pub(crate) type Subst = HashMap<SymbolId, String>;

/// What a value has members of.
pub(crate) enum Receiver {
    /// A class's or interface's instance, by its declaration.
    Class { decl: NodeId, subst: Subst },
    /// A builtin type's value, as `List<String>`: its name and arguments.
    Builtin {
        name: &'static str,
        args: Vec<String>,
    },
}

impl Analysis<'_> {
    /// What a value of type `t` has members of: its class's or builtin
    /// type's, through `?` and a type parameter's bound.
    pub(crate) fn receiver(&self, t: TypeId) -> Option<Receiver> {
        let types = &self.settled.typing.types;
        let mut t = t;
        for _ in 0..8 {
            match types.get(t) {
                TypeData::Nullable(inner) => t = *inner,
                TypeData::Param(p) => t = types.bound_of(*p)?,
                _ => break,
            }
        }
        let (base, args) = match types.get(t) {
            TypeData::Applied { base, args } => (*base, args.as_slice()),
            _ => (t, &[][..]),
        };
        match types.get(base) {
            TypeData::Class(symbol) => {
                let decl = self.settled.scopes.symbol(*symbol).decl?;
                let mut subst = Subst::new();
                if let Some(info) = types.class_info(*symbol) {
                    for (&formal, &arg) in info.formals.iter().zip(args) {
                        subst.insert(formal, self.show(arg));
                    }
                }
                Some(Receiver::Class { decl, subst })
            }
            TypeData::Builtin(i) => Some(Receiver::Builtin {
                name: TYPES[*i as usize],
                args: args.iter().map(|&a| self.show(a)).collect(),
            }),
            _ => None,
        }
    }

    /// The members a receiver offers after `.`, instance members only.
    pub(crate) fn members_of(&self, receiver: &Receiver) -> Vec<Completion> {
        let mut out = Vec::new();
        match receiver {
            Receiver::Class { decl, subst } => {
                self.class_members(*decl, subst, false, &mut out, &mut Vec::new())
            }
            Receiver::Builtin { name, args } => {
                for m in builtin_members(name, args)
                    .into_iter()
                    .filter(|m| !m.is_static)
                {
                    out.push(builtin_completion(m));
                }
            }
        }
        out
    }

    /// A class's public members, its own then its supertypes', with
    /// `subst` for its type parameters: instance members, or with
    /// `statics`, static ones.
    pub(crate) fn class_members(
        &self,
        class: NodeId,
        subst: &Subst,
        statics: bool,
        out: &mut Vec<Completion>,
        seen: &mut Vec<NodeId>,
    ) {
        let tree = &self.settled.library.tree;
        if tree.kind(class) != NodeKind::Class || seen.contains(&class) {
            return;
        }
        seen.push(class);
        let (_, supers, members) = core::class_parts(tree, class);
        for &m in members {
            let data = tree.data(m);
            let role = data & ROLE_MASK;
            if data & core::MEMBER_PRIVATE != 0
                || (data & core::MEMBER_STATIC != 0) != statics
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
            let detail = self.decl_type_in(m, &text, m, subst);
            out.push(Completion {
                name: text,
                kind: self.decl_kind(m, None),
                detail,
            });
        }
        if statics {
            return;
        }
        // Supertypes, with their type parameters as this class names them.
        let types = &self.settled.typing.types;
        let info = (self.decl_symbols().get(&class)).and_then(|&s| types.class_info(s));
        if let Some(info) = info {
            for &sup in &info.supers {
                let (base, args) = match types.get(sup) {
                    TypeData::Applied { base, args } => (*base, args.as_slice()),
                    _ => (sup, &[][..]),
                };
                let TypeData::Class(symbol) = *types.get(base) else {
                    continue;
                };
                let Some(decl) = self.settled.scopes.symbol(symbol).decl else {
                    continue;
                };
                let mut inner = Subst::new();
                if let Some(sup_info) = types.class_info(symbol) {
                    for (&formal, &arg) in sup_info.formals.iter().zip(args) {
                        inner.insert(formal, self.show_in(arg, subst));
                    }
                }
                self.class_members(decl, &inner, false, out, seen);
            }
            return;
        }
        for &s in supers {
            let s = match tree.kind(s) {
                NodeKind::Call => tree.children(s).first().copied().unwrap_or(s),
                _ => s,
            };
            if let Some(symbol) = self.settled.scopes.resolved(s) {
                if let Some(decl) = self.settled.scopes.symbol(symbol).decl {
                    self.class_members(decl, &Subst::new(), false, out, seen);
                }
            }
        }
    }

    /// A class member's declaration by name, its own or inherited, with
    /// the substitution its declaring class needs.
    pub(crate) fn find_member(&self, receiver: &Receiver, name: &str) -> Option<(NodeId, Subst)> {
        let Receiver::Class { decl, subst } = receiver else {
            return None;
        };
        let tree = &self.settled.library.tree;
        let mut todo = vec![(*decl, subst.clone())];
        let mut seen = Vec::new();
        while let Some((class, subst)) = todo.pop() {
            if seen.contains(&class) || tree.kind(class) != NodeKind::Class {
                continue;
            }
            seen.push(class);
            let (_, _, members) = core::class_parts(tree, class);
            for &m in members {
                let named = self.settled.library.name(tree.children(m)[0]);
                if named.is_some_and(|n| self.settled.library.names.get(n) == name)
                    && tree.data(m) & ROLE_MASK != ROLE_TYPE_PARAM
                {
                    return Some((m, subst));
                }
            }
            let types = &self.settled.typing.types;
            let info = (self.decl_symbols().get(&class)).and_then(|&s| types.class_info(s));
            for &sup in info.map_or(&[][..], |i| i.supers.as_slice()) {
                let (base, args) = match types.get(sup) {
                    TypeData::Applied { base, args } => (*base, args.as_slice()),
                    _ => (sup, &[][..]),
                };
                if let TypeData::Class(symbol) = *types.get(base) {
                    let mut inner = Subst::new();
                    if let Some(sup_info) = types.class_info(symbol) {
                        for (&formal, &arg) in sup_info.formals.iter().zip(args) {
                            inner.insert(formal, self.show_in(arg, &subst));
                        }
                    }
                    if let Some(d) = self.settled.scopes.symbol(symbol).decl {
                        todo.push((d, inner));
                    }
                }
            }
        }
        None
    }

    /// A builtin type's member, by name, as its receiver shows it.
    pub(crate) fn builtin_member(&self, receiver: &Receiver, name: &str) -> Option<BuiltinMember> {
        let Receiver::Builtin { name: ty, args } = receiver else {
            return None;
        };
        builtin_members(ty, args)
            .into_iter()
            .find(|m| m.name == name)
    }

    /// The receiver whose expression ends at byte `end` of `file`, before
    /// a `.`: the largest typed expression ending there, or a type's name.
    pub(crate) fn receiver_node(&self, file: FileId, end: u32) -> Option<NodeId> {
        let tree = &self.settled.library.tree;
        let root = self.settled.library.files.get(file.0 as usize)?.root;
        let pos = self.global(file, end);
        let mut best: Option<(u32, NodeId)> = None;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let span = tree.span(node);
            if span.start > pos || span.end < pos {
                continue;
            }
            stack.extend(tree.children(node));
            if span.end != pos || span.start == span.end {
                continue;
            }
            let typed = (self.settled.typing.expr(node)).is_some_and(|t| t != types::UNKNOWN)
                && !matches!(tree.kind(node), NodeKind::Let | NodeKind::Block);
            let a_type = self.type_named(node).is_some() || self.is_console(node);
            let width = span.end - span.start;
            if (typed || a_type) && best.is_none_or(|(w, _)| width > w) {
                best = Some((width, node));
            }
        }
        best.map(|(_, n)| n)
    }

    /// The members after `receiver.`: a type's statics after its name, or
    /// else the members of the receiver's type.
    pub(crate) fn members_after(&self, receiver: NodeId) -> Option<Vec<Completion>> {
        if let Some(named) = self.type_named(receiver) {
            let mut out = Vec::new();
            match named {
                Named::Class(class) => {
                    self.class_members(class, &Subst::new(), true, &mut out, &mut Vec::new())
                }
                Named::Builtin(name) => {
                    for m in builtin_members(name, &[])
                        .into_iter()
                        .filter(|m| m.is_static)
                    {
                        out.push(builtin_completion(m));
                    }
                }
            }
            return Some(out);
        }
        if self.is_console(receiver) {
            return Some(self.members_of(&CONSOLE));
        }
        let t = self.settled.typing.expr(receiver)?;
        Some(match self.receiver(t) {
            Some(r) => self.members_of(&r),
            None => Vec::new(),
        })
    }

    /// Whether a node is the builtin `console`, which typing leaves
    /// untyped; core declares it a `Console`.
    pub(crate) fn is_console(&self, node: NodeId) -> bool {
        use temper_frontend::builtins::{Builtin, Constant};
        self.settled.scopes.resolved_builtin(node) == Some(Builtin::Value(Constant::Console))
    }

    /// The type a name names, if it names one: a class or a builtin type.
    pub(crate) fn type_named(&self, node: NodeId) -> Option<Named> {
        let tree = &self.settled.library.tree;
        if tree.kind(node) != NodeKind::RightName {
            return None;
        }
        let symbol = self
            .settled
            .scopes
            .symbol(self.settled.scopes.resolved(node)?);
        match (symbol.kind, symbol.decl) {
            (_, Some(decl)) if tree.kind(decl) == NodeKind::Class => Some(Named::Class(decl)),
            (SymbolKind::Builtin(_), None) => {
                let name = self.settled.library.names.get(symbol.name);
                let name = TYPES.iter().find(|&&t| t == name)?;
                (!builtin_members(name, &[]).is_empty()).then_some(Named::Builtin(name))
            }
            _ => None,
        }
    }

    /// A type as messages write it, with type parameters in `subst` shown
    /// as their arguments.
    pub(crate) fn show_in(&self, t: TypeId, subst: &Subst) -> String {
        let class_name = |b: SymbolId| match subst.get(&b) {
            Some(shown) => shown.clone(),
            None => {
                let name = self.settled.scopes.symbol(b).name;
                self.settled.library.names.get(name).to_string()
            }
        };
        self.settled.typing.types.display(t, &class_name)
    }
}

/// The builtin `console`'s members, as core declares them.
pub(crate) const CONSOLE: Receiver = Receiver::Builtin {
    name: "Console",
    args: Vec::new(),
};

/// A type that a name names.
pub(crate) enum Named {
    Class(NodeId),
    Builtin(&'static str),
}

/// A builtin type's member as a completion.
fn builtin_completion(m: BuiltinMember) -> Completion {
    Completion {
        name: m.name,
        kind: match m.is_method {
            true => DeclKind::Method,
            false => DeclKind::Field,
        },
        detail: Some(m.detail),
    }
}
