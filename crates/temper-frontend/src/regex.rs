//! Regexes: std's `regex` data model as plain data ([`Node`]), Temper's
//! regex dialect parsed into it, a backtracking matcher, and regex
//! literals.
//!
//! - **Parsing** ([`parse`]) follows temper-regex-parser's `parser.temper`
//!   (0.3.0), which Kotlin Temper's `rgx` macro uses: `(?name=…)` captures,
//!   `(?:…)` and `(…)` groups, `[…]` sets, `\d`, `\s`, `\w`, `\b` and code
//!   escapes, `?`, `*`, `+` and `{m,n}`, each maybe reluctant. One
//!   difference: `[a-]` keeps the `a`, which the parser drops.
//! - **Literals:** `/a+b/` and `rgx"a${s}"` are `rgx(list(literal parts),
//!   list(values))`. Settling parses the pattern and rewrites the call to
//!   build std's objects, `new Sequence([new Repeat(new CodePoints("a"), 1,
//!   null, false), …]).compiled()`, with names resolved to `std/regex`'s
//!   exports, as Kotlin's `RegexLiteralMacro` does. An interpolated value
//!   becomes code points, `new CodePoints(s)`, joined with literal text
//!   around it (`new CodePoints(cat("a", s))`), at run time, where Kotlin
//!   needs it constant.
//! - **Matching** ([`Regex`]) is what the interpreter's connected `Regex`
//!   methods use, compiled from std's objects rather than their formatted
//!   text. It's leftmost-first backtracking over code points, as the JVM's
//!   and JS's engines are, following std's docs and formatter for the
//!   rest: `.` matches all but `\n`, `^` and `$` only the text's ends, `\s`
//!   is `[ \t\n\x0B\f\r]`, `\w` and `\b` are ASCII, an empty `Or` matches
//!   the empty string and an empty set nothing (negated, anything). A
//!   capture in a repeat keeps its last pass. Deep inputs recurse deeply:
//!   each pass of a repeat takes a few stack frames.

use crate::builtins::{Builtin, Function};
use crate::library::Message;
use crate::scope::{ScopeId, SymbolId};
use crate::settle::{Fact, Settle};
use temper_syntax::ast::{Builtin as SyntaxBuiltin, Value};
use temper_syntax::tree::{NodeId, NodeKind, Span};

/// std's empty special classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    Begin,
    Dot,
    End,
    WordBoundary,
    Digit,
    Space,
    Word,
}

const SPECIALS: [(Special, &str); 7] = [
    (Special::Begin, "Begin"),
    (Special::Dot, "Dot"),
    (Special::End, "End"),
    (Special::WordBoundary, "WordBoundary"),
    (Special::Digit, "Digit"),
    (Special::Space, "Space"),
    (Special::Word, "Word"),
];

impl Special {
    pub fn named(name: &str) -> Option<Special> {
        SPECIALS.iter().find(|(_, n)| *n == name).map(|&(s, _)| s)
    }

    pub fn name(self) -> &'static str {
        SPECIALS.iter().find(|(s, _)| *s == self).unwrap().1
    }
}

/// A `RegexNode`, by std's class names.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// `CodePoints`: in sequence, or in a set, any of them.
    Chars(String),
    /// `CodeRange`, inclusive.
    Range(u32, u32),
    /// `CodeSet`: its items and whether it's negated.
    Set(Vec<Node>, bool),
    Or(Vec<Node>),
    Sequence(Vec<Node>),
    /// `Repeat`: item, min, max (none for no limit), reluctant.
    Repeat(Box<Node>, u32, Option<u32>, bool),
    /// `Capture`, with its place among the pattern's captures, which
    /// [`Regex::new`] sets.
    Capture {
        name: String,
        index: usize,
        item: Box<Node>,
    },
    Special(Special),
    /// While parsing a literal, an interpolated value, by its index.
    Slot(usize),
}

/// Parses Temper's regex dialect, where `(?$argN)`, for `N` below
/// `slots`, stands for the `N`th interpolated value.
pub fn parse(pattern: &str, slots: usize) -> Result<Node, String> {
    let mut p = Parser {
        chars: pattern.chars().collect(),
        pos: 0,
        slots,
    };
    let node = p.or()?;
    if !p.done() {
        return Err(p.error("unexpected ')'; unmatched closing parenthesis"));
    }
    Ok(node)
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    slots: usize,
}

impl Parser {
    fn done(&self) -> bool {
        self.pos >= self.chars.len()
    }

    /// The next char, or NUL at the end, as `parser.temper`'s `peek`.
    fn peek(&self) -> char {
        self.chars.get(self.pos).copied().unwrap_or('\0')
    }

    fn read(&mut self) -> char {
        let c = self.peek();
        self.pos = (self.pos + 1).min(self.chars.len());
        c
    }

    fn eat(&mut self, c: char) -> bool {
        let found = !self.done() && self.peek() == c;
        if found {
            self.pos += 1;
        }
        found
    }

    fn error(&self, message: &str) -> String {
        format!("at position {}: {message}", self.pos)
    }

    fn or(&mut self) -> Result<Node, String> {
        let mut options = Vec::new();
        while !self.done() && self.peek() != ')' {
            if !options.is_empty() && !self.eat('|') {
                return Err(self.error("or"));
            }
            options.push(self.sequence()?);
        }
        Ok(if options.len() == 1 {
            options.pop().unwrap()
        } else {
            Node::Or(options)
        })
    }

    fn sequence(&mut self) -> Result<Node, String> {
        let mut items = Vec::new();
        while !self.done() && self.peek() != '|' && self.peek() != ')' {
            let single = self.single()?;
            let item = self.postfix(single)?;
            // Adjacent code points join.
            match (items.last_mut(), item) {
                (Some(Node::Chars(text)), Node::Chars(more)) => text.push_str(&more),
                (_, item) => items.push(item),
            }
        }
        Ok(if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Sequence(items)
        })
    }

    fn postfix(&mut self, item: Node) -> Result<Node, String> {
        let (min, max) = if self.eat('+') {
            (1, None)
        } else if self.eat('*') {
            (0, None)
        } else if self.eat('?') {
            (0, Some(1))
        } else if let Some(bounds) = self.braces() {
            bounds
        } else {
            return Ok(item);
        };
        let reluctant = self.eat('?');
        if max.is_some_and(|max| min > max) {
            return Err(self.error("quantifier has min greater than max"));
        }
        Ok(Node::Repeat(Box::new(item), min, max, reluctant))
    }

    /// `{n}`, `{n,}` or `{n,m}`, or nothing read.
    fn braces(&mut self) -> Option<(u32, Option<u32>)> {
        let start = self.pos;
        let bounds = (|| {
            if !self.eat('{') {
                return None;
            }
            let min = self.number()?;
            if self.eat('}') {
                return Some((min, Some(min)));
            }
            if !self.eat(',') {
                return None;
            }
            let max = self.number();
            self.eat('}').then_some((min, max))
        })();
        if bounds.is_none() {
            self.pos = start;
        }
        bounds
    }

    fn number(&mut self) -> Option<u32> {
        let mut n: Option<u32> = None;
        while let Some(d) = self.peek().to_digit(10).filter(|_| !self.done()) {
            self.pos += 1;
            n = Some(n.unwrap_or(0).saturating_mul(10).saturating_add(d));
        }
        n
    }

    fn single(&mut self) -> Result<Node, String> {
        if self.eat('[') {
            return self.set();
        }
        if self.eat('(') {
            return self.group();
        }
        if self.eat('^') {
            return Ok(Node::Special(Special::Begin));
        }
        if self.eat('$') {
            return Ok(Node::Special(Special::End));
        }
        if self.eat('.') {
            return Ok(Node::Special(Special::Dot));
        }
        if self.eat('\\') {
            return self.escape();
        }
        if matches!(self.peek(), '+' | '*' | '?') {
            return Err(self.error("invalid repeat"));
        }
        Ok(Node::Chars(self.read().to_string()))
    }

    fn group(&mut self) -> Result<Node, String> {
        // `(…)` and `(?:…)` don't capture.
        let node = if !self.eat('?') || self.eat(':') {
            self.or()?
        } else if self.eat('$') {
            let mut name = String::new();
            while !self.eat(')') {
                if self.done() {
                    return Err(self.error("missing closing paren in (?$...)"));
                }
                name.push(self.read());
            }
            let slot = name
                .strip_prefix("arg")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|&n| n < self.slots);
            return slot
                .map(Node::Slot)
                .ok_or_else(|| self.error(&format!("no such slot: {name}")));
        } else if matches!(self.peek(), '=' | '!' | '<') {
            return Err(
                self.error("lookaround and (?<name>...) aren't supported; use (?name=pattern)")
            );
        } else {
            if self.chars.get(self.pos + 1) == Some(&':') {
                return Err(self.error("inline flag groups aren't supported"));
            }
            // A named capture, `(?name=pattern)`, ignoring spaces in the name.
            let mut name = String::new();
            while self.eat(' ') {}
            while !self.eat('=') {
                if self.done() {
                    return Err(self.error(&format!("missing = in (?{name}=...)")));
                }
                name.push(self.read());
                while self.eat(' ') {}
            }
            let item = self.or()?;
            Node::Capture {
                name,
                index: 0,
                item: Box::new(item),
            }
        };
        if !self.eat(')') {
            return Err(self.error("closing paren"));
        }
        Ok(node)
    }

    fn set(&mut self) -> Result<Node, String> {
        let negated = self.eat('^');
        let mut items = Vec::new();
        while !self.eat(']') {
            if self.done() {
                return Err(self.error("missing ]"));
            }
            let first = self.set_unit()?;
            if !self.eat('-') {
                items.push(first);
                continue;
            }
            if self.eat(']') {
                items.push(first);
                items.push(Node::Chars("-".into()));
                break;
            }
            let second = self.set_unit()?;
            let code = |node: &Node| match node {
                Node::Chars(text) => text.chars().next().map(|c| c as u32),
                _ => None,
            };
            match (code(&first), code(&second)) {
                (Some(min), Some(max)) => items.push(Node::Range(min, max)),
                _ => return Err(self.error("invalid range edge")),
            }
        }
        Ok(Node::Set(items, negated))
    }

    fn set_unit(&mut self) -> Result<Node, String> {
        if !self.eat('\\') {
            return Ok(Node::Chars(self.read().to_string()));
        }
        // In a set, `\b` is a backspace.
        if self.eat('b') {
            return Ok(Node::Chars('\u{8}'.to_string()));
        }
        match self.escape()? {
            node @ (Node::Chars(_)
            | Node::Special(Special::Digit | Special::Space | Special::Word)) => Ok(node),
            _ => Err(self.error("invalid code part")),
        }
    }

    fn escape(&mut self) -> Result<Node, String> {
        if self.done() {
            return Err(self.error("trailing '\\' at end of pattern"));
        }
        let c = self.read();
        let code = |n: u32| char::from_u32(n).map(|c| Node::Chars(c.to_string()));
        let negated = |special| Node::Set(vec![Node::Special(special)], true);
        let node = match c {
            '0' => code(0),
            'a' => code(7),
            'f' => code(12),
            'v' => code(11),
            'n' => code(10),
            'r' => code(13),
            't' => code(9),
            'b' => Some(Node::Special(Special::WordBoundary)),
            'd' => Some(Node::Special(Special::Digit)),
            's' => Some(Node::Special(Special::Space)),
            'w' => Some(Node::Special(Special::Word)),
            'D' => Some(negated(Special::Digit)),
            'S' => Some(negated(Special::Space)),
            'W' => Some(negated(Special::Word)),
            'x' => return self.hex(2, false),
            'u' => return self.hex(4, true),
            '(' | ')' | '[' | ']' | '{' | '}' | '.' | '\\' | '-' | '/' | '^' | '$' | '|' | '+'
            | '*' | '?' => code(c as u32),
            _ => None,
        };
        node.ok_or_else(|| self.error(&format!("unsupported escape \\{c}")))
    }

    /// `\xNN`, `\uNNNN` or `\u{N…}`, after the letter.
    fn hex(&mut self, digits: usize, braced: bool) -> Result<Node, String> {
        let braced = braced && self.eat('{');
        let mut value = 0u32;
        let mut count = 0;
        while if braced {
            !self.eat('}')
        } else {
            count < digits
        } {
            let digit = self.read().to_digit(16).filter(|_| count < 6);
            let Some(digit) = digit else {
                return Err(self.error("invalid hex escape"));
            };
            value = value * 16 + digit;
            count += 1;
        }
        match char::from_u32(value).filter(|_| count > 0) {
            Some(c) => Ok(Node::Chars(c.to_string())),
            None => Err(self.error("invalid code point")),
        }
    }
}

/// A capture's span, as byte offsets, if it took part in a match.
pub type Captures = Vec<Option<(usize, usize)>>;

/// A compiled regex: its pattern and its captures' names, in order.
#[derive(Debug)]
pub struct Regex {
    pub root: Node,
    pub names: Vec<String>,
}

/// A match: its span, as byte offsets, and its captures'.
#[derive(Debug)]
pub struct Found {
    pub begin: usize,
    pub end: usize,
    pub captures: Captures,
}

impl Regex {
    /// Numbers a pattern's captures in order, and names them.
    pub fn new(mut root: Node) -> Regex {
        fn number(node: &mut Node, names: &mut Vec<String>) {
            match node {
                Node::Capture { name, index, item } => {
                    *index = names.len();
                    names.push(name.clone());
                    number(item, names);
                }
                Node::Set(items, _) | Node::Or(items) | Node::Sequence(items) => {
                    items.iter_mut().for_each(|i| number(i, names))
                }
                Node::Repeat(item, ..) => number(item, names),
                _ => {}
            }
        }
        let mut names = Vec::new();
        number(&mut root, &mut names);
        Regex { root, names }
    }

    /// The leftmost match at or after `begin`, a char boundary.
    pub fn find(&self, text: &str, begin: usize) -> Option<Found> {
        let mut start = begin.min(text.len());
        loop {
            let mut captures = vec![None; self.names.len()];
            let mut end = 0;
            if step(&self.root, text, start, &mut captures, &mut |e, _| {
                end = e;
                true
            }) {
                return Some(Found {
                    begin: start,
                    end,
                    captures,
                });
            }
            start += text[start..].chars().next()?.len_utf8();
        }
    }

    /// Every match, each search starting where the last match ended, or a
    /// char further after an empty one, as Kotlin's `Regex.findAll`.
    pub fn find_all(&self, text: &str) -> Vec<Found> {
        let mut all = Vec::new();
        let mut at = 0;
        while let Some(found) = self.find(text, at) {
            at = found.end;
            let empty = found.begin == found.end;
            all.push(found);
            if empty {
                match text[at..].chars().next() {
                    Some(c) => at += c.len_utf8(),
                    None => break,
                }
            }
        }
        all
    }
}

/// Matches `node` at `pos`, then the rest through `k`, backtracking
/// into `node` while `k` fails.
fn step(
    node: &Node,
    text: &str,
    pos: usize,
    caps: &mut Captures,
    k: &mut dyn FnMut(usize, &mut Captures) -> bool,
) -> bool {
    match node {
        Node::Chars(chars) => text[pos..].starts_with(chars.as_str()) && k(pos + chars.len(), caps),
        Node::Sequence(items) => sequence(items, text, pos, caps, k),
        Node::Or(items) if items.is_empty() => k(pos, caps),
        Node::Or(items) => items.iter().any(|item| step(item, text, pos, caps, k)),
        Node::Repeat(..) => repeat(node, 0, text, pos, caps, k),
        Node::Capture { index, item, .. } => {
            let index = *index;
            step(item, text, pos, caps, &mut |end, caps| {
                let saved = caps[index].replace((pos, end));
                k(end, caps) || {
                    caps[index] = saved;
                    false
                }
            })
        }
        Node::Special(Special::Begin) => pos == 0 && k(pos, caps),
        Node::Special(Special::End) => pos == text.len() && k(pos, caps),
        Node::Special(Special::WordBoundary) => {
            let before = text[..pos].chars().next_back().is_some_and(is_word);
            let after = text[pos..].chars().next().is_some_and(is_word);
            before != after && k(pos, caps)
        }
        Node::Slot(_) => false,
        single => match text[pos..].chars().next() {
            Some(c) if matches_char(single, c) => k(pos + c.len_utf8(), caps),
            _ => false,
        },
    }
}

fn sequence(
    items: &[Node],
    text: &str,
    pos: usize,
    caps: &mut Captures,
    k: &mut dyn FnMut(usize, &mut Captures) -> bool,
) -> bool {
    match items.split_first() {
        None => k(pos, caps),
        Some((first, rest)) => step(first, text, pos, caps, &mut |p, caps| {
            sequence(rest, text, p, caps, k)
        }),
    }
}

/// A `Repeat` after `count` passes.
// Newer clippy sees `a || b` and `b || a` as the same, but `||` runs left to
// right: reluctant tries stopping first, greedy another pass.
#[allow(clippy::if_same_then_else)]
fn repeat(
    node: &Node,
    count: u32,
    text: &str,
    pos: usize,
    caps: &mut Captures,
    k: &mut dyn FnMut(usize, &mut Captures) -> bool,
) -> bool {
    let &Node::Repeat(ref item, min, max, reluctant) = node else {
        unreachable!()
    };
    let again = |caps: &mut Captures, k: &mut dyn FnMut(usize, &mut Captures) -> bool| {
        max.is_none_or(|max| count < max)
            && step(item, text, pos, caps, &mut |end, caps| {
                // An empty pass past the minimum would go on forever.
                (end != pos || count < min) && repeat(node, count + 1, text, end, caps, k)
            })
    };
    if count < min {
        again(caps, k)
    } else if reluctant {
        k(pos, caps) || again(caps, k)
    } else {
        again(caps, k) || k(pos, caps)
    }
}

/// Whether a one-char node, or a set's item, matches `c`.
fn matches_char(node: &Node, c: char) -> bool {
    match node {
        Node::Chars(chars) => chars.contains(c),
        &Node::Range(min, max) => (min..=max).contains(&(c as u32)),
        Node::Set(items, negated) => items.iter().any(|item| matches_char(item, c)) != *negated,
        Node::Special(Special::Dot) => c != '\n',
        Node::Special(Special::Digit) => c.is_ascii_digit(),
        Node::Special(Special::Space) => matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r'),
        Node::Special(Special::Word) => is_word(c),
        _ => false,
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// What a literal's pattern is built from: `std/regex`'s scope, and the
/// interpolated values.
struct Building {
    std: ScopeId,
    values: Vec<NodeId>,
    span: Span,
    like: NodeId,
}

impl Settle {
    /// `rgx(list(parts), list(values))`, a regex literal: rewrites it to
    /// build its pattern from std's classes and compile it, returning true,
    /// or waits for `std/regex`'s scope.
    pub(crate) fn regex_literal(&mut self, call: NodeId, forced: bool) -> Result<bool, Fact> {
        let (literals, values) = match *self.lib.tree.children(call) {
            [_, literals, values] => (self.list_items(literals), self.list_items(values)),
            _ => (None, None),
        };
        let text = |&l: &NodeId| match self.lib.value(l) {
            Some(Value::String(text)) => Some(text.to_string()),
            _ => None,
        };
        let texts: Option<Vec<String>> = literals.and_then(|ls| ls.iter().map(text).collect());
        let (Some(texts), Some(values)) = (texts, values) else {
            self.error(call, Message::MalformedSpecial("rgx".into()));
            return Ok(true);
        };
        if texts.len() != values.len() + 1 {
            self.error(call, Message::MalformedSpecial("rgx".into()));
            return Ok(true);
        }
        let span = self.lib.tree.span(call);
        let module = self.lib.files[self.lib.locate(span.start).0.0 as usize].module;
        let std = self
            .lib
            .import_dir(module, "std/regex")
            .map(|dir| self.lib.module_at(&dir));
        let std = match std {
            Some(Some(m)) if m != module => m,
            Some(_) => {
                self.error(call, Message::ImportFailed("std/regex".into()));
                return Ok(true);
            }
            None => {
                self.error(
                    call,
                    Message::Unsupported("regex literals without std".into()),
                );
                return Ok(true);
            }
        };
        let std_scope = self.modules[std.0 as usize];
        if !self.scopes.get(std_scope).complete {
            if !forced {
                return Err(Fact::ScopeComplete(std_scope));
            }
            self.error(call, Message::ImportFailed("std/regex".into()));
            return Ok(true);
        }
        let mut pattern = texts[0].clone();
        for (i, text) in texts[1..].iter().enumerate() {
            pattern.push_str(&format!("(?$arg{i}){text}"));
        }
        let node = match parse(&pattern, values.len()) {
            Ok(node) => node,
            Err(why) => {
                self.error(call, Message::InvalidRegex(why));
                return Ok(true);
            }
        };
        let b = Building {
            std: std_scope,
            values,
            span,
            like: call,
        };
        let Some(data) = self.build_regex(&node, &b) else {
            self.error(call, Message::ImportFailed("std/regex".into()));
            return Ok(true);
        };
        self.imports.push((module, std));
        let dot = self.lib.names.intern(".");
        let dot = self.lib.add_name(NodeKind::BuiltinName, dot, span, call);
        let compiled = Value::Symbol(self.lib.names.intern("compiled"));
        let compiled = self.lib.add_value(compiled, span, call);
        let method = self
            .lib
            .add(NodeKind::Call, span, 0, &[dot, data, compiled], call);
        self.rewrite(call, NodeKind::Call, 0, &[method]);
        Ok(true)
    }

    /// The items of a `list(…)` call, as the grammar or a tagged string
    /// makes it.
    fn list_items(&self, node: NodeId) -> Option<Vec<NodeId>> {
        let (&callee, items) = self.lib.tree.children(node).split_first()?;
        let is_list = self.lib.value(callee) == Some(&Value::Builtin(SyntaxBuiltin::List))
            || self.scopes.resolved(callee)
                == Some(self.scopes.hidden(Builtin::Function(Function::List)));
        (self.lib.tree.kind(node) == NodeKind::Call && is_list).then(|| items.to_vec())
    }

    /// An exported name of `std/regex`, resolved.
    fn std_regex_name(&mut self, name: &str, b: &Building) -> Option<NodeId> {
        let name = self.lib.names.intern(name);
        let names = &self.scopes.get(b.std).names;
        let &(_, symbol) = names
            .iter()
            .rev()
            .find(|&&(n, s)| n == name && self.scopes.symbol(s).exported)?;
        Some(self.builtin_name(symbol, b.span, b.like))
    }

    /// `new Class(args)`, of a `std/regex` class.
    fn new_regex(&mut self, class: &str, mut args: Vec<NodeId>, b: &Building) -> Option<NodeId> {
        let new: SymbolId = self.scopes.hidden(Builtin::Function(Function::New));
        let mut children = vec![
            self.builtin_name(new, b.span, b.like),
            self.std_regex_name(class, b)?,
        ];
        children.append(&mut args);
        Some(self.lib.add(NodeKind::Call, b.span, 0, &children, b.like))
    }

    /// `list(…)` of items, or for a sequence, of its runs ([`runs`]).
    fn build_list(&mut self, items: &[Node], sequence: bool, b: &Building) -> Option<NodeId> {
        let list = self.scopes.hidden(Builtin::Function(Function::List));
        let mut children = vec![self.builtin_name(list, b.span, b.like)];
        let runs = if sequence {
            runs(items)
        } else {
            items.iter().map(|i| vec![i.clone()]).collect()
        };
        for run in runs {
            children.push(self.build_run(&run, b)?);
        }
        Some(self.lib.add(NodeKind::Call, b.span, 0, &children, b.like))
    }

    /// A run from [`runs`]: one node, or `new CodePoints(…)` of literal
    /// text and interpolated values.
    fn build_run(&mut self, run: &[Node], b: &Building) -> Option<NodeId> {
        if let [node] = run {
            if !is_text(node) {
                return self.build_regex(node, b);
            }
        }
        let mut parts = Vec::new();
        for part in run {
            parts.push(match part {
                Node::Chars(text) => {
                    self.lib
                        .add_value(Value::String(text.clone()), b.span, b.like)
                }
                &Node::Slot(i) => b.values[i],
                _ => unreachable!("only code points and slots join"),
            });
        }
        let text = match parts[..] {
            [only] => only,
            _ => {
                let cat = self.scopes.hidden(Builtin::Function(Function::Cat));
                parts.insert(0, self.builtin_name(cat, b.span, b.like));
                self.lib.add(NodeKind::Call, b.span, 0, &parts, b.like)
            }
        };
        self.new_regex("CodePoints", vec![text], b)
    }

    fn build_regex(&mut self, node: &Node, b: &Building) -> Option<NodeId> {
        let (span, like) = (b.span, b.like);
        let int = |s: &mut Self, n: u32| s.lib.add_value(Value::Int32(n as i32), span, like);
        let bool = |s: &mut Self, v: bool| s.lib.add_value(Value::Bool(v), span, like);
        match node {
            Node::Chars(_) | Node::Slot(_) => self.build_run(std::slice::from_ref(node), b),
            &Node::Range(min, max) => {
                let args = vec![int(self, min), int(self, max)];
                self.new_regex("CodeRange", args, b)
            }
            Node::Set(items, negated) => {
                let args = vec![self.build_list(items, false, b)?, bool(self, *negated)];
                self.new_regex("CodeSet", args, b)
            }
            Node::Or(items) => {
                let args = vec![self.build_list(items, false, b)?];
                self.new_regex("Or", args, b)
            }
            Node::Sequence(items) => match &runs(items)[..] {
                [run] if run.iter().all(is_text) => self.build_run(run, b),
                _ => {
                    let args = vec![self.build_list(items, true, b)?];
                    self.new_regex("Sequence", args, b)
                }
            },
            Node::Repeat(item, min, max, reluctant) => {
                let item = self.build_regex(item, b)?;
                let max = match max {
                    Some(max) => int(self, *max),
                    None => self.lib.add_value(Value::Null, span, like),
                };
                let args = vec![item, int(self, *min), max, bool(self, *reluctant)];
                self.new_regex("Repeat", args, b)
            }
            Node::Capture { name, item, .. } => {
                let name = self.lib.add_value(Value::String(name.clone()), span, like);
                let args = vec![name, self.build_regex(item, b)?];
                self.new_regex("Capture", args, b)
            }
            Node::Special(special) => self.std_regex_name(special.name(), b),
        }
    }
}

/// Code points or an interpolated value, which join with neighbors like
/// them into one `CodePoints`.
fn is_text(node: &Node) -> bool {
    matches!(node, Node::Chars(_) | Node::Slot(_))
}

/// A sequence's items, with each run of text ([`is_text`]) together.
fn runs(items: &[Node]) -> Vec<Vec<Node>> {
    let mut out: Vec<Vec<Node>> = Vec::new();
    for item in items {
        match out.last_mut() {
            Some(run) if is_text(item) && run.iter().all(is_text) => run.push(item.clone()),
            _ => out.push(vec![item.clone()]),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regex(pattern: &str) -> Regex {
        Regex::new(parse(pattern, 0).unwrap())
    }

    fn found(pattern: &str, text: &str) -> Option<(String, Vec<Option<String>>)> {
        let found = regex(pattern).find(text, 0)?;
        let captures = found
            .captures
            .iter()
            .map(|c| c.map(|(b, e)| text[b..e].to_string()))
            .collect();
        Some((text[found.begin..found.end].to_string(), captures))
    }

    #[test]
    fn parses_the_dialect() {
        use Node::*;
        let chars = |s: &str| Chars(s.into());
        assert_eq!(
            parse("ab+c", 0),
            Ok(Sequence(vec![
                chars("a"),
                Repeat(Box::new(chars("b")), 1, None, false),
                chars("c")
            ]))
        );
        assert_eq!(
            parse("[^a-c\\d-]", 0),
            Ok(Set(
                vec![Range(97, 99), Special(super::Special::Digit), chars("-")],
                true
            ))
        );
        assert_eq!(
            parse("x{2,}?|", 0),
            Ok(Or(vec![
                Repeat(Box::new(chars("x")), 2, None, true),
                Sequence(vec![])
            ]))
        );
        assert_eq!(parse("a{,2}", 0), Ok(chars("a{,2}")));
        assert_eq!(
            parse("[a-]", 0),
            Ok(Set(vec![chars("a"), chars("-")], false))
        );
        assert_eq!(
            parse("\\u{1F30D}\\x41(?$arg0)", 1),
            Ok(Sequence(vec![chars("🌍A"), Slot(0)]))
        );
        assert!(
            parse("(?<n>a)", 0).is_err()
                && parse("a)", 0).is_err()
                && parse("*", 0).is_err()
                && parse("(?$arg0)", 0).is_err()
        );
    }

    #[test]
    fn matches_backtracking() {
        let text = |s: &str| Some(s.to_string());
        assert_eq!(
            found("a+(?b=b+)c", "xaabbc"),
            Some(("aabbc".into(), vec![text("bb")]))
        );
        // A capture in a repeat keeps its last pass; one that didn't take part has none.
        assert_eq!(
            found("((?d=d)|(?e=e))+", "cee"),
            Some(("ee".into(), vec![None, text("e")]))
        );
        assert_eq!(found("a.*?c", "abcbc").unwrap().0, "abc");
        assert_eq!(found("a.*c", "abcbc").unwrap().0, "abcbc");
        assert_eq!(
            found("\\b[a-z_][a-z0-9_]*\\b", "123_abc def_456")
                .unwrap()
                .0,
            "def_456"
        );
        assert_eq!(found("[^]+", "a\nb").unwrap().0, "a\nb");
        assert!(
            found("[]", "a").is_none()
                && found("^b", "ab").is_none()
                && found("a$", "ab").is_none()
        );
        assert_eq!(found("(?:)*x", "x").unwrap().0, "x");
        let r = regex("(^|,)\\s*");
        let spans: Vec<(usize, usize)> = r
            .find_all("a, b")
            .iter()
            .map(|f| (f.begin, f.end))
            .collect();
        assert_eq!(spans, [(0, 0), (1, 3)]);
    }
}
