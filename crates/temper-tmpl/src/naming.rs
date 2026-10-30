//! The naming pass: every declaration of the program's own gets an output
//! name, pretty and numbered only to resolve a real conflict
//! (notes/tmpl/names.md, "The naming pass"; design.md section 8).
//!
//! As built for subset A and Python's first variant, function-scoped
//! targets only: a function's parameters and locals share one scope, as
//! Python's do, and two of them may share a name when neither is alive
//! while the other is. The init block's locals live in the module's
//! scope, as Python's module-level code does. Labels and other
//! libraries' declarations get no name: labels aren't printed by
//! function-scoped targets so far, and external declarations are named by
//! their own library or their variant's connection table.

use crate::analysis::{module_refs, walk};
use crate::decls::{DeclKind, Facts};
use crate::ids::{DeclId, NodeId};
use crate::qname::is_external;
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
    /// Whether a character may be in a name, first or not; others become
    /// `_` and their hex code.
    pub identifier: fn(char, bool) -> bool,
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

/// Converts a camelCase name to `snake_case` as Kotlin Temper does
/// (`IdentStyle.Camel.convertTo(IdentStyle.Snake, …)`): words break at
/// case changes, delimiters (`_` and `-`), and digits, are lowercased,
/// and are joined with `_` where they'd otherwise run together. A leading
/// digit or other non-letter gets a leading `_`.
pub fn snake_case(name: &str) -> String {
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
    fn tok(c: char) -> Tok {
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
    let letters = |t: Tok| matches!(t, Tok::Lower | Tok::Upper | Tok::Caseless);
    let category = |t: Tok| match t {
        Tok::Lower | Tok::Upper | Tok::Caseless => 0,
        Tok::Digit => 1,
        Tok::Delimiter | Tok::Other => 2,
        Tok::Nil => 3,
    };
    // Break, as Kotlin's `breakChimeric`.
    let mut words: Vec<(Tok, String)> = Vec::new();
    let (mut word_tok, mut word) = (Tok::Nil, String::new());
    for c in name.chars() {
        let t = tok(c);
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
    // Join, as Kotlin's `mapLower` and `delimitUnderscore`.
    let mut out = String::new();
    let mut prior = Tok::Nil;
    for (t, w) in words {
        let middle = prior != Tok::Nil;
        if middle && prior == Tok::Caseless && t == Tok::Caseless {
            out.push('\u{b7}');
        } else if (middle
            && (category(prior) == category(t) || (prior == Tok::Digit && letters(t))))
            || (!middle && !letters(t))
        {
            out.push('_');
        }
        if letters(t) {
            out.extend(w.chars().flat_map(char::to_lowercase));
        } else {
            out.push_str(&w);
        }
        prior = t;
    }
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
    for module in &p.modules {
        name_module(p, rules, &reserved, module, &mut map);
    }
    map
}

/// A declaration's candidates, styled for the target: its name, then a
/// compiler-made declaration's further hints.
fn candidates(p: &Program, rules: &NamingRules, decl: DeclId, module_level: bool) -> Vec<String> {
    let d = p.decls.get(decl);
    let exported = d.facts.contains(Facts::EXPORTED);
    let mut raw: Vec<&str> = vec![p.name(decl)];
    raw.extend(p.decls.alts(decl).iter().map(|&s| p.strings.get(s)));
    raw.into_iter()
        .filter(|r| !r.is_empty())
        .map(|r| {
            let styled = match (rules.case)(d.kind, exported) {
                Case::Keep => r.to_string(),
                Case::Snake => snake_case(r),
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
    let mut taken: HashSet<String> = HashSet::new();
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

    for &item in &module.items {
        if let Node::Function { .. } = p.nodes.get(item) {
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
    let outer: HashSet<String> = module_refs(p, function)
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
                    let (s, e) = ranges.get(other).copied().unwrap_or((0, u32::MAX));
                    n == c && s <= range.1 && range.0 <= e
                })
        });
        if suffixed {
            map.suffixed.push(d);
        }
        map.names[d.index()] = name.clone();
        named.push((d, name));
    }
}
