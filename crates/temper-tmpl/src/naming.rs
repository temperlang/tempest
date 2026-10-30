//! The naming pass: every declaration of the program's own gets an output
//! name, pretty and numbered only to resolve a real conflict
//! (notes/tmpl/names.md, "The naming pass"; design.md section 8).
//!
//! As built for subset A and Python's first variant, function-scoped
//! targets: a function's parameters and locals share one scope, as
//! Python's do, and two of them may share a name when neither is alive
//! while the other is. For block-scoped targets without shadowing (JS's
//! `let` and `const`, [`NamingRules::block_scoped`]), two may share a name
//! only when declared in blocks that don't nest. The init block's locals live in the module's
//! scope, as Python's module-level code does. Labels and other
//! libraries' declarations get no name: labels aren't printed by
//! function-scoped targets so far, and external declarations are named by
//! their own library or their variant's connection table.

use crate::analysis::{module_imports, module_refs, test_imports, walk};
use crate::decls::{DeclKind, Facts};
use crate::ids::{DeclId, NodeId};
use crate::qname::{is_external, library_of};
use crate::schema::Node;
use crate::{Module, Program};
use std::collections::{HashMap, HashSet};

/// A target's case for a kind of name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    /// As the source has it.
    Keep,
    /// `snake_case`, by Kotlin Temper's `IdentStyle.Camel` to
    /// `IdentStyle.Snake` conversion: `halfAndHalf` is `half_and_half`,
    /// `HTMLParser` `html_parser`, and `a1b` `a1_b`.
    Snake,
    /// `PascalCase`, by Kotlin Temper's `IdentStyle.Camel` to
    /// `IdentStyle.Pascal` conversion ([`pascal_case`]): `halfAndHalf` is
    /// `HalfAndHalf`, `HTMLParser` `HtmlParser`, and `a98Rgb` `A98_Rgb`.
    Pascal,
}

/// A target's naming conventions, as data (notes/tmpl/names.md,
/// "`NamingRules`"). Only what's honored so far; block scoping, class
/// bodies, and shadowing rules come with the targets that need them.
pub struct NamingRules {
    /// Lists of keywords, and of names the translator prints (builtins,
    /// runtime helpers), which no declaration may take.
    pub reserved: &'static [&'static [&'static str]],
    /// The case for a kind of declaration, exported or not.
    pub case: fn(DeclKind, bool) -> Case,
    /// Put before module-level names that aren't exported: `_` in Python.
    pub private_prefix: &'static str,
    /// Put after a name that's reserved: `_` in Python (`from_`).
    pub escape_suffix: &'static str,
    /// Keywords escaped by [`raw_prefix`](Self::raw_prefix) rather than
    /// reserved: Rust's, as raw identifiers (`r#type`). Empty in Python.
    pub raw_keywords: &'static [&'static str],
    /// Put before a raw keyword: `r#` in Rust.
    pub raw_prefix: &'static str,
    /// Whether a character may be in a name, first or not; others become
    /// `_` and their hex code.
    pub identifier: fn(char, bool) -> bool,
    /// Whether a function's locals are block-scoped and may not shadow one
    /// another, as JS's `let` and `const`: two may share a name only when
    /// declared in blocks that don't nest, since a name declared twice in
    /// one block is an error, and an inner declaration would shadow an
    /// outer one from the start of its block. Otherwise (Python, Rust)
    /// two may share a name when neither is alive while the other is.
    pub block_scoped: bool,
}

/// Output names, by [`DeclId`].
#[derive(Clone, Debug, Default)]
pub struct NameMap {
    names: Vec<String>,
    suffixed: Vec<DeclId>,
    clashes: Vec<String>,
}

impl NameMap {
    /// A declaration's name; empty for one the pass doesn't name.
    pub fn get(&self, decl: DeclId) -> &str {
        self.names.get(decl.index()).map_or("", String::as_str)
    }

    /// The declarations that took a numbered suffix, which only a real
    /// conflict causes.
    pub fn suffixed(&self) -> &[DeclId] {
        &self.suffixed
    }

    /// Exports, connected, and `@keep` declarations whose names clash:
    /// errors, since those names can't change. Each is still given a
    /// suffixed name, so output stays valid.
    pub fn clashes(&self) -> &[String] {
        &self.clashes
    }
}

/// A word's kind, as Kotlin Temper's `Tok` in its identifier styles.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tok {
    Lower,
    Upper,
    Caseless,
    Digit,
    Delimiter,
    Other,
    Nil,
}

impl Tok {
    fn of(c: char) -> Tok {
        match c {
            '_' | '-' | '\u{b7}' | '\u{2027}' | '\u{318d}' | '\u{30fb}' | '\u{ff65}' => {
                Tok::Delimiter
            }
            _ if c.is_lowercase() => Tok::Lower,
            _ if c.is_uppercase() => Tok::Upper,
            _ if c.is_alphabetic() => Tok::Caseless,
            _ if c.is_numeric() => Tok::Digit,
            _ => Tok::Other,
        }
    }

    fn letters(self) -> bool {
        matches!(self, Tok::Lower | Tok::Upper | Tok::Caseless)
    }

    fn category(self) -> u8 {
        match self {
            Tok::Lower | Tok::Upper | Tok::Caseless => 0,
            Tok::Digit => 1,
            Tok::Delimiter | Tok::Other => 2,
            Tok::Nil => 3,
        }
    }
}

/// A camelCase name's words, as Kotlin's `breakChimeric` breaks them: at
/// case changes, delimiters (`_` and `-`, which are dropped), and digits;
/// each with the kind of its last character.
fn words(name: &str) -> Vec<(Tok, String)> {
    let mut words: Vec<(Tok, String)> = Vec::new();
    let (mut word_tok, mut word) = (Tok::Nil, String::new());
    for c in name.chars() {
        let t = Tok::of(c);
        if t == Tok::Delimiter {
            if !word.is_empty() {
                words.push((word_tok, std::mem::take(&mut word)));
            }
            word_tok = Tok::Nil;
            continue;
        }
        if t != word_tok {
            if word_tok == Tok::Upper && t == Tok::Lower {
                let count = word.chars().count();
                if count > 1 {
                    let last = word.pop().unwrap_or_default();
                    words.push((word_tok, std::mem::take(&mut word)));
                    word.push(last);
                }
            } else if !word.is_empty() {
                words.push((word_tok, std::mem::take(&mut word)));
            }
        }
        word.push(c);
        word_tok = t;
    }
    if !word.is_empty() {
        words.push((word_tok, word));
    }
    words
}

/// Converts a camelCase name to `snake_case` as Kotlin Temper does
/// (`IdentStyle.Camel.convertTo(IdentStyle.Snake, …)`): words break at
/// case changes, delimiters (`_` and `-`), and digits, are lowercased,
/// and are joined with `_` where they'd otherwise run together. A leading
/// digit or other non-letter gets a leading `_`.
pub fn snake_case(name: &str) -> String {
    // Join, as Kotlin's `mapLower` and `delimitUnderscore`.
    let mut out = String::new();
    let mut prior = Tok::Nil;
    for (t, w) in words(name) {
        let middle = prior != Tok::Nil;
        if middle && prior == Tok::Caseless && t == Tok::Caseless {
            out.push('\u{b7}');
        } else if (middle
            && (prior.category() == t.category() || (prior == Tok::Digit && t.letters())))
            || (!middle && !t.letters())
        {
            out.push('_');
        }
        if t.letters() {
            out.extend(w.chars().flat_map(char::to_lowercase));
        } else {
            out.push_str(&w);
        }
        prior = t;
    }
    out
}

/// Converts a camelCase name to `PascalCase` as Kotlin Temper does
/// (`IdentStyle.Camel.convertTo(IdentStyle.Pascal, …)`), as its C#
/// backend names functions and exported values: words break as
/// [`snake_case`]'s do, each is lowercased but for its first letter, and
/// `_` joins two words only where they'd otherwise run together: digits
/// before letters (`a98Rgb` is `A98_Rgb`, Kotlin's), or two runs of the
/// same uncased kind. A leading digit or other non-letter gets a leading
/// `_`.
pub fn pascal_case(name: &str) -> String {
    // Join, as Kotlin's `mapPascal` and `delimitChimeric`.
    let mut out = String::new();
    let mut prior = Tok::Nil;
    for (t, w) in words(name) {
        let middle = prior != Tok::Nil;
        let cased = matches!(t, Tok::Lower | Tok::Upper);
        if middle && prior == Tok::Caseless && t == Tok::Caseless {
            out.push('\u{b7}');
        } else if (middle && ((!cased && prior == t) || (prior == Tok::Digit && t.letters())))
            || (!middle && !t.letters())
        {
            out.push('_');
        }
        if t.letters() {
            let lower: String = w.chars().flat_map(char::to_lowercase).collect();
            let mut chars = lower.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        } else {
            out.push_str(&w);
        }
        prior = t;
    }
    out
}

/// Candidate names for an expression a pass or backend stores in a
/// temporary, best first (notes/tmpl/analyses.md, "Naming hints"): the
/// name of the declaration it reads or calls, a callee's with a leading
/// `get` or `to` dropped (`toInt32` gives `int32`); its type's name in
/// lower camel case (`int32`, `string`); then `value`. Duplicates are
/// dropped. Not yet: the parameter it feeds.
pub fn hints(p: &Program, expr: NodeId) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |name: String| {
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
    };
    let lower_first = |name: &str| {
        let mut chars = name.chars();
        chars.next().map_or(String::new(), |first| {
            first.to_lowercase().chain(chars).collect::<String>()
        })
    };
    match p.nodes.get(expr) {
        Node::Local { local: d } | Node::GetModuleLet { target: d } => push(p.name(d).into()),
        Node::CallFunction { callee: d, .. } | Node::CallMethod { method: d, .. } => {
            let name = p.name(d);
            let name = name.strip_prefix("get ").unwrap_or(name);
            let dropped = ["get", "to"].iter().find_map(|prefix| {
                name.strip_prefix(prefix)
                    .filter(|rest| rest.starts_with(|c: char| c.is_uppercase()))
            });
            push(match dropped {
                Some(rest) => lower_first(rest),
                None => name.to_string(),
            });
        }
        _ => {}
    }
    let ty = p.nodes.ty(expr);
    let type_name = match p.types.get(ty) {
        crate::TypeData::Scalar(s) => s.name().to_string(),
        crate::TypeData::Nominal { decl } => p.name(decl).to_string(),
        crate::TypeData::Function { .. } => String::new(),
    };
    push(lower_first(&type_name));
    push("value".into());
    out
}

/// Gives every declaration of the program's own a name, by `rules`.
pub fn assign(p: &Program, rules: &NamingRules) -> NameMap {
    let mut map = NameMap {
        names: vec![String::new(); p.decls.len()],
        ..NameMap::default()
    };
    let reserved: HashSet<&str> = rules
        .reserved
        .iter()
        .flat_map(|l| l.iter().copied())
        .collect();
    // Other libraries' declarations, but core's, which backends reach
    // through their runtimes: as their own library names its exports.
    for d in p.decls.ids() {
        let decl = p.decls.get(d);
        let named = matches!(
            decl.kind,
            DeclKind::Function
                | DeclKind::ModuleLet
                | DeclKind::Class
                | DeclKind::Interface
                | DeclKind::Method
                | DeclKind::Getter
        );
        if named && is_external(p, d) && library_of(p, d) != "core" {
            let cands = candidates(p, rules, d, true);
            map.names[d.index()] = escape(rules, &reserved, cands[0].clone());
        }
    }
    for module in &p.modules {
        name_module(p, rules, &reserved, module, &mut map);
    }
    map
}

/// The module-level declarations a function refers to, as
/// [`module_refs`] gives them, and other libraries' but core's.
fn referred(p: &Program, function: NodeId) -> Vec<DeclId> {
    let mut found = module_refs(p, function);
    walk(p, function, &mut |id| {
        let decl = match p.nodes.get(id) {
            Node::CallFunction { callee, .. } => callee,
            Node::GetModuleLet { target } | Node::SetModuleLet { target, .. } => target,
            _ => return,
        };
        if is_external(p, decl) && library_of(p, decl) != "core" && !found.contains(&decl) {
            found.push(decl);
        }
    });
    found
}

/// A declaration's candidates, styled for the target: its name, then a
/// compiler-made declaration's further hints.
fn candidates(p: &Program, rules: &NamingRules, decl: DeclId, module_level: bool) -> Vec<String> {
    let d = p.decls.get(decl);
    // Another library's declarations are its exports.
    let exported = d.facts.contains(Facts::EXPORTED) || is_external(p, decl);
    let mut raw: Vec<&str> = vec![p.name(decl)];
    raw.extend(p.decls.alts(decl).iter().map(|&s| p.strings.get(s)));
    raw.into_iter()
        .filter(|r| !r.is_empty())
        .map(|r| {
            let styled = match (rules.case)(d.kind, exported) {
                Case::Keep => r.to_string(),
                Case::Snake => snake_case(r),
                Case::Pascal => pascal_case(r),
            };
            let mut name = String::new();
            for (i, c) in styled.chars().enumerate() {
                if (rules.identifier)(c, i == 0) {
                    name.push(c);
                } else {
                    name.push_str(&format!("_{:x}", c as u32));
                }
            }
            if name.is_empty() {
                name.push('_');
            }
            if module_level && !exported {
                name.insert_str(0, rules.private_prefix);
            }
            name
        })
        .collect()
}

fn escape(rules: &NamingRules, reserved: &HashSet<&str>, mut name: String) -> String {
    if rules.raw_keywords.contains(&name.as_str()) {
        return format!("{}{name}", rules.raw_prefix);
    }
    while reserved.contains(name.as_str()) {
        name.push_str(rules.escape_suffix);
    }
    name
}

/// The first of `candidates` that `free` accepts, else the first with the
/// shortest numbered suffix that it accepts, and whether it's suffixed.
fn choose(
    rules: &NamingRules,
    reserved: &HashSet<&str>,
    candidates: &[String],
    free: impl Fn(&str) -> bool,
) -> (String, bool) {
    for c in candidates {
        let c = escape(rules, reserved, c.clone());
        if free(&c) {
            return (c, false);
        }
    }
    let first = escape(rules, reserved, candidates[0].clone());
    let sep = if first.ends_with(|c: char| c.is_ascii_digit()) {
        "_"
    } else {
        ""
    };
    (2..)
        .map(|n| escape(rules, reserved, format!("{first}{sep}{n}")))
        .find(|c| free(c))
        .map(|c| (c, true))
        .unwrap_or_default()
}

fn name_module(
    p: &Program,
    rules: &NamingRules,
    reserved: &HashSet<&str>,
    module: &Module,
    map: &mut NameMap,
) {
    // Module-level declarations: the items, then the init block's locals,
    // in the claiming order (exported, connected, and kept first, then
    // source names, then compiler-made ones).
    let mut level: Vec<DeclId> = module
        .items
        .iter()
        .filter_map(|&item| match p.nodes.get(item) {
            Node::Function { decl, .. }
            | Node::BodilessFunction { decl, .. }
            | Node::ModuleLet { decl } => Some(decl),
            _ => None,
        })
        .collect();
    for &test in &module.tests {
        if let Node::Test { decl, .. } = p.nodes.get(test) {
            level.push(decl);
        }
    }
    walk(p, module.init, &mut |id| {
        if let Node::Let { local, .. } = p.nodes.get(id) {
            level.push(local);
        }
    });
    let rank = |d: DeclId| {
        let facts = p.decls.get(d).facts;
        if facts.contains(Facts::EXPORTED)
            || facts.contains(Facts::CONNECTED)
            || facts.contains(Facts::KEEP)
        {
            0
        } else if facts.contains(Facts::COMPILER_MADE) {
            2
        } else {
            1
        }
    };
    level.sort_by_key(|&d| (rank(d), d));
    // What the module imports keeps its name here, so its own
    // declarations don't take it.
    let mut taken: HashSet<String> = HashSet::new();
    for import in module_imports(p, module)
        .into_iter()
        .chain(test_imports(p, module))
    {
        for d in import.decls {
            taken.insert(map.names[d.index()].clone());
        }
    }
    for d in level {
        let cands = candidates(p, rules, d, true);
        let (name, suffixed) = choose(rules, reserved, &cands, |c| !taken.contains(c));
        if suffixed {
            map.suffixed.push(d);
            if rank(d) == 0 {
                map.clashes.push(format!(
                    "`{}` can't be named `{}`, which another declaration has",
                    p.name(d),
                    escape(rules, reserved, cands[0].clone())
                ));
            }
        }
        taken.insert(name.clone());
        map.names[d.index()] = name;
    }

    for &item in module.items.iter().chain(&module.tests) {
        if let Node::Function { .. } | Node::Test { .. } = p.nodes.get(item) {
            name_function(p, rules, reserved, item, map);
        }
    }
}

/// When each local or parameter is alive, as positions in a walk of its
/// function: from its definition to its last use, stretched over any loop
/// that uses it without defining it, since the next iteration may read it.
#[derive(Default)]
struct Live {
    pos: u32,
    start: HashMap<DeclId, u32>,
    end: HashMap<DeclId, u32>,
    uses: HashMap<DeclId, Vec<u32>>,
    loops: Vec<(u32, u32)>,
}

impl Live {
    fn define(&mut self, d: DeclId) {
        self.start.insert(d, self.pos);
        self.end.insert(d, self.pos);
    }

    fn touch(&mut self, d: DeclId) {
        let end = self.end.entry(d).or_insert(self.pos);
        *end = (*end).max(self.pos);
        self.uses.entry(d).or_default().push(self.pos);
    }

    fn visit(&mut self, p: &Program, id: NodeId) {
        self.pos += 1;
        match p.nodes.get(id) {
            Node::Let { local, init } => {
                if let Some(init) = init {
                    self.visit(p, init);
                }
                self.pos += 1;
                self.define(local);
            }
            Node::Param { decl } => {
                self.define(decl);
            }
            Node::Local { local } => self.touch(local),
            Node::Assign { local, value } => {
                self.visit(p, value);
                self.pos += 1;
                self.touch(local);
            }
            Node::While { .. } | Node::DoWhile { .. } => {
                let start = self.pos;
                for &kid in p.nodes.kids(id) {
                    self.visit(p, kid);
                }
                self.pos += 1;
                self.loops.push((start, self.pos));
            }
            _ => {
                for &kid in p.nodes.kids(id) {
                    self.visit(p, kid);
                }
            }
        }
    }

    /// Each declaration's live range, after stretching over loops.
    fn ranges(&self) -> HashMap<DeclId, (u32, u32)> {
        let mut ranges = HashMap::new();
        for (&d, &start) in &self.start {
            let mut end = self.end.get(&d).copied().unwrap_or(start);
            let uses = self.uses.get(&d).map_or(&[][..], Vec::as_slice);
            for &(ls, le) in &self.loops {
                if start < ls && uses.iter().any(|&u| ls < u && u < le) {
                    end = end.max(le);
                }
            }
            ranges.insert(d, (start, end));
        }
        ranges
    }
}

fn name_function(
    p: &Program,
    rules: &NamingRules,
    reserved: &HashSet<&str>,
    function: NodeId,
    map: &mut NameMap,
) {
    // Python decides locality per function: a local can't take the name
    // of a module-level declaration the function refers to anywhere.
    let outer: HashSet<String> = referred(p, function)
        .into_iter()
        .map(|d| map.get(d).to_string())
        .collect();
    let mut live = Live::default();
    live.visit(p, function);
    let ranges = live.ranges();
    let mut decls: Vec<DeclId> = Vec::new();
    walk(p, function, &mut |id| match p.nodes.get(id) {
        Node::Param { decl } => decls.push(decl),
        Node::Let { local, .. } => decls.push(local),
        _ => {}
    });
    let scopes = Scopes::of(p, function);
    // Parameters first, then source names, then compiler-made ones.
    let made = |d: DeclId| p.decls.get(d).facts.contains(Facts::COMPILER_MADE);
    let param = |d: DeclId| p.decls.get(d).kind == DeclKind::Param;
    decls.sort_by_key(|&d| (!param(d), made(d), d));
    let mut named: Vec<(DeclId, String)> = Vec::new();
    for d in decls {
        if is_external(p, d) {
            continue;
        }
        let range = ranges.get(&d).copied().unwrap_or((0, u32::MAX));
        let cands = candidates(p, rules, d, false);
        let (name, suffixed) = choose(rules, reserved, &cands, |c| {
            !outer.contains(c)
                && !named.iter().any(|(other, n)| {
                    if n != c {
                        false
                    } else if rules.block_scoped {
                        scopes.nest(d, *other)
                    } else {
                        let (s, e) = ranges.get(other).copied().unwrap_or((0, u32::MAX));
                        s <= range.1 && range.0 <= e
                    }
                })
        });
        if suffixed {
            map.suffixed.push(d);
        }
        map.names[d.index()] = name.clone();
        named.push((d, name));
    }
}

/// Where each of a function's parameters and locals is declared, for
/// block-scoped targets: the block holding its `let`, or the function
/// itself for a parameter; and each block's enclosing one.
struct Scopes {
    of: HashMap<DeclId, NodeId>,
    parent: HashMap<NodeId, NodeId>,
}

impl Scopes {
    fn of(p: &Program, function: NodeId) -> Scopes {
        fn visit(p: &Program, id: NodeId, scope: NodeId, scopes: &mut Scopes) {
            let mut scope = scope;
            match p.nodes.get(id) {
                Node::Param { decl } => {
                    scopes.of.insert(decl, scope);
                }
                Node::Let { local, .. } => {
                    scopes.of.insert(local, scope);
                }
                Node::Block { .. } => {
                    scopes.parent.insert(id, scope);
                    scope = id;
                }
                _ => {}
            }
            for &kid in p.nodes.kids(id) {
                visit(p, kid, scope, scopes);
            }
        }
        let mut scopes = Scopes {
            of: HashMap::new(),
            parent: HashMap::new(),
        };
        visit(p, function, function, &mut scopes);
        scopes
    }

    /// Whether `inner` is `outer` or inside it.
    fn within(&self, mut inner: NodeId, outer: NodeId) -> bool {
        loop {
            if inner == outer {
                return true;
            }
            match self.parent.get(&inner) {
                Some(&up) => inner = up,
                None => return false,
            }
        }
    }

    /// Whether two declarations' blocks are the same, or one holds the
    /// other.
    fn nest(&self, a: DeclId, b: DeclId) -> bool {
        match (self.of.get(&a), self.of.get(&b)) {
            (Some(&x), Some(&y)) => self.within(x, y) || self.within(y, x),
            _ => true,
        }
    }
}
