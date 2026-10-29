//! Members of builtin types, with their signatures read from Kotlin
//! Temper's `core.temper` (vendored in `vendor/temper/core`): the final
//! ones (`Boolean`, `Int32`, `Int64`, `Float64` and `String`), as
//! `s.toInt32()`, `x.toString()`, `s.isEmpty` or `String.fromCodePoint(c)`,
//! and the generic ones, as `ls.length`, `ls.get(i)` or `m.keys()`, whose
//! signatures name their type parameters.
//!
//! Tempest doesn't load core as a library yet, so this reads only those
//! types' declarations, each parsed once, when typing first needs it: each
//! public member's name, whether it's static, and its type, a field's or
//! getter's or a method's signature. A type in these signatures is a
//! builtin type, maybe given arguments, nullable, or a function type, or
//! one of the type's own type parameters or a method's own (`O` in
//! `map<O>`, which has no bound); anything else is unknown, as is a
//! parameter that isn't written `name: Type`.

use crate::builtins::TYPES;
use std::sync::OnceLock;
use temper_syntax::ast::{Ast, Builtin as AstBuiltin, Value, build};
use temper_syntax::lex::SourceKind;
use temper_syntax::parse::parse;
use temper_syntax::tree::{NodeId, NodeKind};

const CORE: &str = include_str!("../../../../vendor/temper/core/core.temper");

/// The builtin types whose members are read: first the final ones, which
/// no other type extends, then the generic ones, then the rest whose
/// members are used.
const READ_TYPES: [&str; 25] = [
    "Boolean",
    "Int32",
    "Int64",
    "Float64",
    "String",
    "Deque",
    "DoneResult",
    "Generator",
    "GeneratorResult",
    "List",
    "ListBuilder",
    "Listed",
    "Map",
    "MapBuilder",
    "Mapped",
    "Pair",
    "Promise",
    "PromiseBuilder",
    "SafeGenerator",
    "ValueResult",
    "StringBuilder",
    "StringIndex",
    "StringIndexOption",
    "NoStringIndex",
    "DenseBitVector",
];

/// A type in a member's signature.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum CoreType {
    /// Anything typing doesn't read.
    Unknown,
    /// A builtin type, by its index in [`TYPES`].
    Builtin(u8),
    /// The declaring type's own type parameter, by index, as `T` in
    /// `Listed<T>`.
    Param(u8),
    /// A method's own type parameter, by index, as `O` in `map<O>`.
    Own(u8),
    /// A builtin generic type given arguments, as `List<T>`.
    Applied(u8, Vec<CoreType>),
    /// `T?`.
    Nullable(Box<CoreType>),
    /// `fn (A, B): R`.
    Fn(Vec<CoreType>, Box<CoreType>),
}

#[derive(Debug, PartialEq)]
pub(super) enum CoreMemberKind {
    /// A field's or getter's value.
    Value(CoreType),
    /// A method, with its own type parameters' names, each parameter's
    /// type and whether it has a default.
    Method {
        type_params: Vec<String>,
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
    /// The operator it implements, if marked so, as `==` for
    /// `@operator("==")`.
    pub operator: Option<String>,
}

/// A builtin type's declaration: how many type parameters it has, and its
/// public members.
#[derive(Debug, Default)]
pub(super) struct CoreClass {
    pub type_params: usize,
    /// Its type parameters' names, as `K` and `V`.
    pub type_param_names: Vec<String>,
    pub members: Vec<CoreMember>,
}

/// A builtin type's declaration, by its name in [`TYPES`], read the first
/// time it's wanted; empty for a type whose members aren't read.
pub(super) fn core_class(type_name: &str) -> &'static CoreClass {
    static CLASSES: [OnceLock<CoreClass>; READ_TYPES.len()] =
        [const { OnceLock::new() }; READ_TYPES.len()];
    static EMPTY: CoreClass = CoreClass {
        type_params: 0,
        type_param_names: Vec::new(),
        members: Vec::new(),
    };
    match READ_TYPES.iter().position(|&t| t == type_name) {
        Some(i) => CLASSES[i].get_or_init(|| read_class(type_name)),
        None => &EMPTY,
    }
}

/// Reads one type's declaration, from the line `export class Name` or
/// `export sealed interface Name<…>` to the `}` that ends a line on its
/// own, or that line alone if it ends with `{}`, parsed alone.
fn read_class(name: &str) -> CoreClass {
    let declares = |line: &str| {
        let rest = line.strip_prefix("export ").unwrap_or("");
        let rest = (rest.strip_prefix("sealed "))
            .or_else(|| rest.strip_prefix("/*sealed*/ "))
            .unwrap_or(rest);
        let rest = (rest.strip_prefix("class "))
            .or_else(|| rest.strip_prefix("interface "))
            .unwrap_or("");
        rest.strip_prefix(name)
            .is_some_and(|r| r.starts_with([' ', '<']))
    };
    let mut offset = 0;
    let mut found = None;
    for line in CORE.split_inclusive('\n') {
        if declares(line) {
            found = Some((offset, line));
            break;
        }
        offset += line.len();
    }
    let Some((start, line)) = found else {
        return CoreClass::default();
    };
    let end = match line.trim_end().ends_with("{}") {
        true => start + line.len(),
        false => (CORE[start..].find("\n}\n")).map_or(CORE.len(), |e| start + e + 2),
    };
    let ast = build(&parse(&CORE[start..end], SourceKind::Standalone));
    let mut reader = Reader {
        ast: &ast,
        type_params: Vec::new(),
        own_params: Vec::new(),
    };
    let mut members = Vec::new();
    if let Some(body) = reader.class_body(ast.root()) {
        for &m in ast.tree().children(body) {
            reader.member(m, false, &mut members);
        }
    }
    CoreClass {
        type_params: reader.type_params.len(),
        type_param_names: reader.type_params,
        members,
    }
}

struct Reader<'a> {
    ast: &'a Ast,
    /// The type's own type parameters' names, in order.
    type_params: Vec<String>,
    /// The type parameters of the method being read, in order, and
    /// whether each has a bound, which isn't read, so it's unknown.
    own_params: Vec<(String, bool)>,
}

impl<'a> Reader<'a> {
    fn text(&self, node: NodeId) -> Option<&'a str> {
        Some(self.ast.names().get(self.ast.name(node)?))
    }

    fn symbol(&self, node: NodeId) -> Option<&'a str> {
        match self.ast.value(node)? {
            Value::Symbol(n) => Some(self.ast.names().get(*n)),
            _ => None,
        }
    }

    fn call_parts(&self, node: NodeId) -> Option<(Option<&'a str>, &'a [NodeId])> {
        let tree = self.ast.tree();
        let (&callee, args) =
            (tree.kind(node) == NodeKind::Call).then(|| tree.children(node).split_first())??;
        Some((self.text(callee), args))
    }

    /// `(Call class \word ^Name \typeArg T … (Fun (Block members)))`, or
    /// `interface`, maybe in a block or under decorations; notes its type
    /// parameters' names.
    fn class_body(&mut self, node: NodeId) -> Option<NodeId> {
        let tree = self.ast.tree();
        match (tree.kind(node), self.call_parts(node)) {
            (NodeKind::Block, _) => tree.children(node).iter().find_map(|&c| self.class_body(c)),
            (_, Some((Some("@"), [_, target]))) => self.class_body(*target),
            (_, Some((Some("class" | "interface"), args))) => {
                for pair in args.windows(2) {
                    if self.symbol(pair[0]) == Some("typeArg") {
                        let name = self.type_param_name(pair[1]).unwrap_or_default();
                        self.type_params.push(name.to_string());
                    }
                }
                let fun = *args.last()?;
                let block = *tree.children(fun).last()?;
                (tree.kind(fun) == NodeKind::Fun && tree.kind(block) == NodeKind::Block)
                    .then_some(block)
            }
            _ => None,
        }
    }

    /// A type parameter's name: `T`, `@out T`, or `K extends MapKey<K>`
    /// (`(Block \typeArg K \super …)`).
    fn type_param_name(&self, node: NodeId) -> Option<&'a str> {
        let tree = self.ast.tree();
        match (tree.kind(node), self.call_parts(node)) {
            (NodeKind::RightName | NodeKind::LeftName, _) => self.text(node),
            (_, Some((Some("@"), [_, target]))) => self.type_param_name(*target),
            (NodeKind::Block, _) => self.type_param_name(*tree.children(node).get(1)?),
            _ => None,
        }
    }

    /// Adds a member if it's public, reading its decorations.
    fn member(&mut self, node: NodeId, is_static: bool, out: &mut Vec<CoreMember>) {
        let tree = self.ast.tree();
        // `let begin: StringIndex`, `(Decl ^begin \type StringIndex)`.
        if tree.kind(node) == NodeKind::Decl {
            let parts = tree.children(node);
            let ty = (parts.windows(2)).find(|pair| self.symbol(pair[0]) == Some("type"));
            if let (Some(&name), Some(pair)) = (parts.first(), ty) {
                let ty = self.type_of(pair[1]);
                self.push(name, is_static, CoreMemberKind::Value(ty), out);
            }
            return;
        }
        let Some((callee, args)) = self.call_parts(node) else {
            return;
        };
        match (callee, args) {
            (Some("@"), [decorator, target]) => match self.text(*decorator) {
                Some("private" | "protected") => {}
                Some("static") => self.member(*target, true, out),
                _ => {
                    let before = out.len();
                    self.member(*target, is_static, out);
                    // `@operator("==")`.
                    if let Some((Some("operator"), [op])) = self.call_parts(*decorator) {
                        if let Some(Value::String(op)) = self.ast.value(*op) {
                            for m in &mut out[before..] {
                                m.operator = Some(op.to_string());
                            }
                        }
                    }
                }
            },
            // `get \word ^name \outType T { … }`
            (Some("get"), [word, name, rest @ ..]) if self.symbol(*word) == Some("word") => {
                let ty = self.out_type(rest).0;
                self.push(*name, is_static, CoreMemberKind::Value(ty), out);
            }
            // `set \word ^name …`, not a method named `set`.
            (Some("set"), [word, ..]) if self.symbol(*word) == Some("word") => {}
            // `key: K`
            (Some(":"), [name, ty]) => {
                let ty = self.type_of(*ty);
                self.push(*name, is_static, CoreMemberKind::Value(ty), out);
            }
            // `static e = 2.718…`
            (None, [op, target, value]) if self.text(*op) == Some("=") => {
                let ty = match self.ast.value(*value) {
                    Some(Value::Float64(_)) => builtin("Float64"),
                    Some(Value::Int32(_)) => builtin("Int32"),
                    Some(Value::String(_)) => builtin("String"),
                    _ => CoreType::Unknown,
                };
                self.push(*target, is_static, CoreMemberKind::Value(ty), out);
            }
            // `name \typeArg O (Block param) … \outType R`.
            (Some(_), parts) => {
                self.own_params = (parts.windows(2))
                    .filter(|pair| self.symbol(pair[0]) == Some("typeArg"))
                    .map(|pair| {
                        let name = self.type_param_name(pair[1]).unwrap_or_default();
                        let bounded = self.ast.tree().kind(pair[1]) == NodeKind::Block;
                        (name.to_string(), bounded)
                    })
                    .collect();
                let mut params = Vec::new();
                let mut rest = parts;
                while let [p, after @ ..] = rest {
                    match self.symbol(*p) {
                        Some("typeArg") => rest = after.get(1..).unwrap_or(&[]),
                        Some(_) => break,
                        None => {
                            if tree.kind(*p) != NodeKind::Fun {
                                params.push(self.param(*p));
                            }
                            rest = after;
                        }
                    }
                }
                let (ret, throws) = self.out_type(parts);
                let own_params = std::mem::take(&mut self.own_params);
                let kind = CoreMemberKind::Method {
                    type_params: own_params.into_iter().map(|(name, _)| name).collect(),
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
                operator: None,
            });
        }
    }

    /// `(Block \_complexArg_ name \type T \default d)`: its type, and
    /// whether it has a default.
    /// Also a decorated one, as `@inlineUnrealizedGoal let body: fn (T):
    /// Void`, `(Call @ d (Block \_complexArg_ (Decl body \type …)))`.
    fn param(&self, node: NodeId) -> (CoreType, bool) {
        let tree = self.ast.tree();
        if let Some((Some("@"), [_, target])) = self.call_parts(node) {
            return self.param(*target);
        }
        let parts = tree.children(node);
        if !matches!(tree.kind(node), NodeKind::Block | NodeKind::Decl) {
            return (CoreType::Unknown, false);
        }
        if let Some(&decl) = (parts.iter()).find(|&&p| tree.kind(p) == NodeKind::Decl) {
            return self.param(decl);
        }
        let after = |key: &str| {
            (parts.windows(2))
                .find(|pair| self.symbol(pair[0]) == Some(key))
                .map(|pair| pair[1])
        };
        (
            after("type").map_or(CoreType::Unknown, |t| self.type_of(t)),
            after("default").is_some(),
        )
    }

    /// What follows `\outType`: the type, and whether it `throws Bubble`.
    fn out_type(&self, parts: &[NodeId]) -> (CoreType, bool) {
        let Some(i) = parts
            .iter()
            .position(|&p| self.symbol(p) == Some("outType"))
        else {
            return (CoreType::Unknown, false);
        };
        let Some(&ty) = parts.get(i + 1) else {
            return (CoreType::Unknown, false);
        };
        match self.call_parts(ty) {
            Some((Some("throws"), [pass, ..])) => (self.type_of(*pass), true),
            _ => (self.type_of(ty), false),
        }
    }

    /// A type: a builtin type named as `String` or `builtins.String`
    /// (`%String`), maybe given arguments, as `List<T>`; the method's own
    /// type parameter, or else the type's; `T?`; or `fn (A): R`.
    fn type_of(&self, node: NodeId) -> CoreType {
        let tree = self.ast.tree();
        match tree.kind(node) {
            NodeKind::RightName | NodeKind::BuiltinName => {
                let Some(name) = self.text(node) else {
                    return CoreType::Unknown;
                };
                let own = self.own_params.iter().position(|(p, _)| p == name);
                match (own, self.type_params.iter().position(|p| p == name)) {
                    (Some(i), _) if self.own_params[i].1 => CoreType::Unknown,
                    (Some(i), _) => CoreType::Own(i as u8),
                    (None, Some(i)) => CoreType::Param(i as u8),
                    (None, None) => builtin(name),
                }
            }
            NodeKind::Call => {
                let children = tree.children(node);
                let angle = self.ast.value(children[0]) == Some(&Value::Builtin(AstBuiltin::Angle));
                match (angle, self.call_parts(node)) {
                    (true, Some((_, [base, args @ ..]))) => match self.type_of(*base) {
                        CoreType::Builtin(b) => {
                            CoreType::Applied(b, args.iter().map(|&a| self.type_of(a)).collect())
                        }
                        _ => CoreType::Unknown,
                    },
                    (false, Some((Some("?"), [inner]))) => {
                        CoreType::Nullable(Box::new(self.type_of(*inner)))
                    }
                    (false, Some((Some("fn"), parts))) => {
                        let out = parts
                            .iter()
                            .position(|&p| self.symbol(p) == Some("outType"));
                        let (Some(out), Some(&ret)) = (out, out.and_then(|i| parts.get(i + 1)))
                        else {
                            return CoreType::Unknown;
                        };
                        let params = parts[..out].iter().map(|&p| self.type_of(p)).collect();
                        CoreType::Fn(params, Box::new(self.type_of(ret)))
                    }
                    _ => CoreType::Unknown,
                }
            }
            _ => CoreType::Unknown,
        }
    }
}

/// A builtin type by name, or unknown.
fn builtin(name: &str) -> CoreType {
    match TYPES.iter().position(|&t| t == name) {
        Some(i) => CoreType::Builtin(i as u8),
        None => CoreType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(ty: &str, name: &str) -> &'static CoreMember {
        (core_class(ty).members.iter())
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("{ty}.{name}"))
    }

    #[test]
    fn reads_core_signatures() {
        let int = builtin("Int");
        let string = builtin("String");
        let to_int = find("String", "toInt32");
        assert_eq!(
            to_int.kind,
            CoreMemberKind::Method {
                type_params: Vec::new(),
                params: vec![(int.clone(), true)],
                ret: int,
                throws: true,
            }
        );
        assert!(find("String", "fromCodePoint").is_static);
        let eq = find("StringIndexOption", "eq");
        assert_eq!(eq.operator.as_deref(), Some("=="));
        assert_eq!(
            find("String", "isEmpty").kind,
            CoreMemberKind::Value(builtin("Boolean"))
        );
        assert_eq!(
            find("Float64", "pi").kind,
            CoreMemberKind::Value(builtin("Float64"))
        );
        let CoreMemberKind::Method { ret, .. } = &find("Int32", "toString").kind else {
            panic!("a method");
        };
        assert_eq!(*ret, string);
        let CoreMemberKind::Method { ret, .. } = &find("String", "split").kind else {
            panic!("a method");
        };
        let CoreType::Builtin(list) = builtin("List") else {
            panic!("List");
        };
        assert_eq!(*ret, CoreType::Applied(list, vec![string]));
        // Private fields aren't members to callers.
        assert!(
            core_class("Boolean")
                .members
                .iter()
                .all(|m| m.name != "content")
        );
        // Each type is found, though some declare no members.
        let empty = ["DoneResult", "GeneratorResult", "Promise", "NoStringIndex"];
        // The final types come first, the generic ones after, then the
        // rest.
        for (i, ty) in READ_TYPES.into_iter().enumerate() {
            let class = core_class(ty);
            assert_eq!(class.members.is_empty(), empty.contains(&ty), "{ty}");
            assert_eq!(class.type_params > 0, (5..20).contains(&i), "{ty}");
        }
    }

    #[test]
    fn reads_generic_signatures() {
        let t = CoreType::Param(0);
        let int = builtin("Int");
        assert_eq!(core_class("Listed").type_params, 1);
        assert_eq!(core_class("Map").type_params, 2);
        assert_eq!(core_class("DoneResult").type_params, 1);
        assert_eq!(
            find("Listed", "length").kind,
            CoreMemberKind::Value(int.clone())
        );
        assert_eq!(
            find("Listed", "getOr").kind,
            CoreMemberKind::Method {
                type_params: Vec::new(),
                params: vec![(int.clone(), false), (t.clone(), false)],
                ret: t.clone(),
                throws: false,
            }
        );
        // A method's own type parameter, which may shadow the type's.
        let o = CoreType::Own(0);
        let list = |x: &CoreType| match builtin("List") {
            CoreType::Builtin(b) => CoreType::Applied(b, vec![x.clone()]),
            _ => panic!("List"),
        };
        assert_eq!(
            find("Listed", "map").kind,
            CoreMemberKind::Method {
                type_params: vec!["O".to_string()],
                params: vec![(CoreType::Fn(vec![t.clone()], Box::new(o.clone())), false)],
                ret: list(&o),
                throws: false,
            }
        );
        let CoreMemberKind::Method {
            type_params, ret, ..
        } = &find("Mapped", "toListWith").kind
        else {
            panic!("a method");
        };
        assert_eq!((&type_params[..], ret), (&["T".to_string()][..], &list(&o)));
        assert_eq!(
            find("Mapped", "get").kind,
            CoreMemberKind::Method {
                type_params: Vec::new(),
                params: vec![(t.clone(), false)],
                ret: CoreType::Param(1),
                throws: true,
            }
        );
        assert_eq!(find("Pair", "key").kind, CoreMemberKind::Value(t.clone()));
        let CoreMemberKind::Method { params, .. } = &find("ListBuilder", "add").kind else {
            panic!("a method");
        };
        assert_eq!(params[..], [(t, false), (int, true)]);
    }
}
