//! Members of the final builtin types (`Boolean`, `Int32`, `Int64`,
//! `Float64` and `String`), with their signatures read from Kotlin Temper's
//! `core.temper` (vendored in `vendor/temper/core`), as `s.toInt32()`,
//! `x.toString()`, `s.isEmpty` or `String.fromCodePoint(c)`.
//!
//! Tempest doesn't load core as a library yet, so this reads only those
//! classes' declarations, each parsed once, when typing first needs it: each
//! public member's name, whether it's static, and its type, a field's or
//! getter's or a method's signature. A type in these signatures that
//! isn't a builtin type's name, such as `List<String>`, is unknown, as
//! is a parameter that isn't written `name: Type`.

use crate::builtins::TYPES;
use std::sync::OnceLock;
use temper_syntax::ast::{Ast, Value, build};
use temper_syntax::lex::SourceKind;
use temper_syntax::parse::parse;
use temper_syntax::tree::{NodeId, NodeKind};

const CORE: &str = include_str!("../../../../vendor/temper/core/core.temper");

/// The builtin types whose members are read, which no other type extends.
const FINAL_TYPES: [&str; 5] = ["Boolean", "Int32", "Int64", "Float64", "String"];

/// A type in a member's signature: a builtin type, by its index in
/// [`TYPES`], or `None` if it's anything else.
pub(super) type CoreType = Option<u8>;

#[derive(Debug, PartialEq)]
pub(super) enum CoreMemberKind {
    /// A field's or getter's value.
    Value(CoreType),
    /// A method, with each parameter's type and whether it has a default.
    Method {
        params: Vec<(CoreType, bool)>,
        ret: CoreType,
        throws: bool,
    },
}

#[derive(Debug)]
pub(super) struct CoreMember {
    pub name: String,
    pub is_static: bool,
    pub kind: CoreMemberKind,
}

/// The public members of a final builtin type, by its name in [`TYPES`].
/// Each type's are read the first time they're wanted.
pub(super) fn core_members(type_name: &str) -> &'static [CoreMember] {
    static MEMBERS: [OnceLock<Vec<CoreMember>>; FINAL_TYPES.len()] =
        [const { OnceLock::new() }; FINAL_TYPES.len()];
    match FINAL_TYPES.iter().position(|&t| t == type_name) {
        Some(i) => MEMBERS[i].get_or_init(|| read_class(type_name)),
        None => &[],
    }
}

/// Reads one class's members: its declaration, from `export class Name `
/// to the `}` that ends a line on its own, is parsed alone.
fn read_class(name: &str) -> Vec<CoreMember> {
    let header = format!("\nexport class {name} ");
    let Some(start) = CORE.find(&header) else {
        return Vec::new();
    };
    let end = CORE[start..]
        .find("\n}\n")
        .map_or(CORE.len(), |e| start + e + 2);
    let ast = build(&parse(&CORE[start..end], SourceKind::Standalone));
    let reader = Reader { ast: &ast };
    let mut members = Vec::new();
    if let Some(body) = reader.class_body(ast.root()) {
        for &m in ast.tree().children(body) {
            reader.member(m, false, &mut members);
        }
    }
    members
}

struct Reader<'a> {
    ast: &'a Ast,
}

impl Reader<'_> {
    fn text(&self, node: NodeId) -> Option<&str> {
        Some(self.ast.names().get(self.ast.name(node)?))
    }

    fn symbol(&self, node: NodeId) -> Option<&str> {
        match self.ast.value(node)? {
            Value::Symbol(n) => Some(self.ast.names().get(*n)),
            _ => None,
        }
    }

    fn call_parts(&self, node: NodeId) -> Option<(Option<&str>, &[NodeId])> {
        let tree = self.ast.tree();
        let (&callee, args) =
            (tree.kind(node) == NodeKind::Call).then(|| tree.children(node).split_first())??;
        Some((self.text(callee), args))
    }

    /// `(Call class \word ^Name … (Fun (Block members)))`, maybe in a block
    /// or under decorations.
    fn class_body(&self, node: NodeId) -> Option<NodeId> {
        let tree = self.ast.tree();
        match (tree.kind(node), self.call_parts(node)) {
            (NodeKind::Block, _) => tree.children(node).iter().find_map(|&c| self.class_body(c)),
            (_, Some((Some("@"), [_, target]))) => self.class_body(*target),
            (_, Some((Some("class"), args))) => {
                let fun = *args.last()?;
                let block = *tree.children(fun).last()?;
                (tree.kind(fun) == NodeKind::Fun && tree.kind(block) == NodeKind::Block)
                    .then_some(block)
            }
            _ => None,
        }
    }

    /// Adds a member if it's public, reading its decorations.
    fn member(&self, node: NodeId, is_static: bool, out: &mut Vec<CoreMember>) {
        let tree = self.ast.tree();
        let Some((callee, args)) = self.call_parts(node) else {
            return;
        };
        match (callee, args) {
            (Some("@"), [decorator, target]) => match self.text(*decorator) {
                Some("private" | "protected") => {}
                Some("static") => self.member(*target, true, out),
                _ => self.member(*target, is_static, out),
            },
            // `get \word ^name \outType T { … }`
            (Some("get"), [word, name, rest @ ..]) if self.symbol(*word) == Some("word") => {
                let ty = self.out_type(rest).0;
                self.push(*name, is_static, CoreMemberKind::Value(ty), out);
            }
            (Some("set"), _) => {}
            // `static e = 2.718…`
            (None, [op, target, value]) if self.text(*op) == Some("=") => {
                let ty = match self.ast.value(*value) {
                    Some(Value::Float64(_)) => type_index("Float64"),
                    Some(Value::Int32(_)) => type_index("Int32"),
                    Some(Value::String(_)) => type_index("String"),
                    _ => None,
                };
                self.push(*target, is_static, CoreMemberKind::Value(ty), out);
            }
            (Some(_), parts) => {
                let mut params = Vec::new();
                for &p in parts.iter().take_while(|&&p| self.symbol(p).is_none()) {
                    if tree.kind(p) != NodeKind::Fun {
                        params.push(self.param(p));
                    }
                }
                let (ret, throws) = self.out_type(parts);
                let kind = CoreMemberKind::Method {
                    params,
                    ret,
                    throws,
                };
                self.push(tree.children(node)[0], is_static, kind, out);
            }
            _ => {}
        }
    }

    fn push(&self, name: NodeId, is_static: bool, kind: CoreMemberKind, out: &mut Vec<CoreMember>) {
        if let Some(name) = self.text(name) {
            out.push(CoreMember {
                name: name.to_string(),
                is_static,
                kind,
            });
        }
    }

    /// `(Block \_complexArg_ name \type T \default d)`: its type, and
    /// whether it has a default.
    fn param(&self, node: NodeId) -> (CoreType, bool) {
        let tree = self.ast.tree();
        let parts = tree.children(node);
        if tree.kind(node) != NodeKind::Block {
            return (None, false);
        }
        let after = |key: &str| {
            (parts.windows(2))
                .find(|pair| self.symbol(pair[0]) == Some(key))
                .map(|pair| pair[1])
        };
        (
            after("type").and_then(|t| self.type_of(t)),
            after("default").is_some(),
        )
    }

    /// What follows `\outType`: the type, and whether it `throws Bubble`.
    fn out_type(&self, parts: &[NodeId]) -> (CoreType, bool) {
        let Some(i) = parts
            .iter()
            .position(|&p| self.symbol(p) == Some("outType"))
        else {
            return (None, false);
        };
        let Some(&ty) = parts.get(i + 1) else {
            return (None, false);
        };
        match self.call_parts(ty) {
            Some((Some("throws"), [pass, ..])) => (self.type_of(*pass), true),
            _ => (self.type_of(ty), false),
        }
    }

    /// A builtin type named as `String` or `builtins.String` (`%String`).
    fn type_of(&self, node: NodeId) -> CoreType {
        match self.ast.tree().kind(node) {
            NodeKind::RightName | NodeKind::BuiltinName => type_index(self.text(node)?),
            _ => None,
        }
    }
}

fn type_index(name: &str) -> CoreType {
    TYPES.iter().position(|&t| t == name).map(|i| i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_core_signatures() {
        let find = |ty: &str, name: &str| {
            (core_members(ty).iter())
                .find(|m| m.name == name)
                .unwrap_or_else(|| panic!("{ty}.{name}"))
        };
        let int = type_index("Int");
        let string = type_index("String");
        let to_int = find("String", "toInt32");
        assert_eq!(
            to_int.kind,
            CoreMemberKind::Method {
                params: vec![(int, true)],
                ret: int,
                throws: true,
            }
        );
        assert!(find("String", "fromCodePoint").is_static);
        assert_eq!(
            find("String", "isEmpty").kind,
            CoreMemberKind::Value(type_index("Boolean"))
        );
        assert_eq!(
            find("Float64", "pi").kind,
            CoreMemberKind::Value(type_index("Float64"))
        );
        let CoreMemberKind::Method { ret, .. } = find("Int32", "toString").kind else {
            panic!("a method");
        };
        assert_eq!(ret, string);
        // Private fields aren't members to callers.
        assert!(core_members("Boolean").iter().all(|m| m.name != "content"));
        for ty in FINAL_TYPES {
            assert!(core_members(ty).len() > 1, "{ty}");
        }
    }
}
