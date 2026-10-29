//! Reading canonical text, through the [`Builder`].
//!
//! Reading takes two passes over the tokens: the first defines modules,
//! functions, parameters, and module `let`s from their headers, skipping
//! bodies, so a body may use any of them; the second reads bodies, where
//! locals and labels must be defined before they're used. The reader
//! checks what text alone decides: syntax, ids defined once, names
//! matching their declarations, written types matching the local rules,
//! and `^` marking exactly the calls that may bubble. Everything else is
//! the validator's.

use super::{FORMAT_VERSION, MODEL_VERSION, writing_order};
use crate::Program;
use crate::builder::Builder;
use crate::builtins::Key;
use crate::decls::{DeclKind, Facts};
use crate::ids::{DeclId, FileId, NodeId, Span, TypeId};
use crate::nodes::Flags;
use crate::types::TypeTable;
use std::fmt;

/// Why text couldn't be read, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ReadError {}

/// Reads canonical text, or hand-written text close to it.
pub fn read(text: &str) -> Result<Program, ReadError> {
    let tokens = lex(text).map_err(|(at, message)| error_at(text, at, message))?;
    let mut r = Reader {
        src: text.to_string(),
        tokens,
        pos: 0,
        b: Builder::new("", ""),
        defined: Vec::new(),
        owner: DeclId::NONE,
    };
    let modules = r.headers()?;
    r.bodies(modules)
}

fn error_at(src: &str, at: usize, message: String) -> ReadError {
    let before = &src[..at.min(src.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.len() - before.rfind('\n').map_or(0, |i| i + 1) + 1;
    ReadError {
        line,
        column,
        message,
    }
}

// Tokens.

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    /// A bare or backquoted word without an id: keywords, types, and
    /// names such as the library's; also versions and hashes.
    Word {
        text: String,
        quoted: bool,
    },
    /// `name#id`, or `~hint|alt#id` if compiler-made.
    Decl {
        names: Vec<String>,
        id: u32,
        made: bool,
    },
    /// `%Int32.add`, without the `%`.
    Key(String),
    Int {
        value: i128,
        i64: bool,
    },
    Float(f64),
    Str(String),
    /// A `///` line, without the slashes and one space.
    Doc(String),
    Punct(&'static str),
    End,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    start: usize,
    end: usize,
}

const PUNCT: &[&str] = &[
    "&&", "||", "{", "}", "(", ")", ",", ";", ":", "=", "^", "#", "@", "-", "~",
];

fn lex(src: &str) -> Result<Vec<Token>, (usize, String)> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    loop {
        // Whitespace and comments.
        while i < bytes.len() {
            if bytes[i].is_ascii_whitespace() {
                i += 1;
            } else if src[i..].starts_with("//") && !src[i..].starts_with("///") {
                i = src[i..].find('\n').map_or(src.len(), |n| i + n);
            } else {
                break;
            }
        }
        let start = i;
        if i == bytes.len() {
            tokens.push(Token {
                tok: Tok::End,
                start,
                end: start,
            });
            return Ok(tokens);
        }
        let c = bytes[i];
        let after_word = i > 0 && bytes[i - 1].is_ascii_alphanumeric();
        let tok = if src[i..].starts_with("///") {
            let end = src[i..].find('\n').map_or(src.len(), |n| i + n);
            let line = &src[i + 3..end];
            i = end;
            Tok::Doc(line.strip_prefix(' ').unwrap_or(line).to_string())
        } else if c.is_ascii_digit() || (c == b'-' && !after_word && next_is_number(&src[i + 1..]))
        {
            let (tok, end) = number(src, i)?;
            i = end;
            tok
        } else if c.is_ascii_alphabetic() || c == b'_' || c == b'`' {
            let (name, end) = name(src, i)?;
            i = end;
            match id_after(src, i)? {
                Some((id, end)) => {
                    i = end;
                    Tok::Decl {
                        names: vec![name],
                        id,
                        made: false,
                    }
                }
                None => Tok::Word {
                    text: name,
                    quoted: c == b'`',
                },
            }
        } else if c == b'~' && bytes.get(i + 1).is_some_and(|&c| is_name_start(c)) {
            let mut names = Vec::new();
            i += 1;
            loop {
                let (name, end) = name(src, i)?;
                names.push(name);
                i = end;
                if bytes.get(i) == Some(&b'|') {
                    i += 1;
                } else {
                    break;
                }
            }
            let Some((id, end)) = id_after(src, i)? else {
                return Err((
                    i,
                    "expected `#` and an id after a compiler-made name".into(),
                ));
            };
            i = end;
            Tok::Decl {
                names,
                id,
                made: true,
            }
        } else if c == b'%' {
            let mut end = i + 1;
            loop {
                let word = src[end..]
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .map_or(src.len(), |n| end + n);
                if word == end {
                    return Err((end, "expected a key after `%`".into()));
                }
                end = word;
                if src[end..].starts_with('.') {
                    end += 1;
                } else {
                    break;
                }
            }
            let key = src[i + 1..end].to_string();
            i = end;
            Tok::Key(key)
        } else if c == b'"' {
            let (text, end) = string(src, i)?;
            i = end;
            Tok::Str(text)
        } else if let Some(p) = PUNCT.iter().find(|p| src[i..].starts_with(**p)) {
            i += p.len();
            Tok::Punct(p)
        } else {
            let c = src[i..].chars().next().unwrap_or('?');
            return Err((i, format!("unexpected character {c:?}")));
        };
        tokens.push(Token { tok, start, end: i });
    }
}

fn is_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'`'
}

fn next_is_number(rest: &str) -> bool {
    rest.starts_with(|c: char| c.is_ascii_digit()) || rest.starts_with("inf")
}

/// A bare or backquoted name.
fn name(src: &str, start: usize) -> Result<(String, usize), (usize, String)> {
    let rest = &src[start..];
    if let Some(quoted) = rest.strip_prefix('`') {
        let mut text = String::new();
        let mut chars = quoted.char_indices();
        while let Some((n, c)) = chars.next() {
            match c {
                '`' => return Ok((text, start + 1 + n + 1)),
                '\\' => match chars.next() {
                    Some((_, c @ ('`' | '\\'))) => text.push(c),
                    _ => return Err((start + 1 + n, "bad escape in a name".into())),
                },
                '\n' => break,
                c => text.push(c),
            }
        }
        return Err((start, "unclosed name".into()));
    }
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    if end == 0 {
        return Err((start, "expected a name".into()));
    }
    Ok((rest[..end].to_string(), start + end))
}

/// `#` and digits right after a name, if there.
fn id_after(src: &str, at: usize) -> Result<Option<(u32, usize)>, (usize, String)> {
    let Some(rest) = src[at..].strip_prefix('#') else {
        return Ok(None);
    };
    let digits = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if digits == 0 {
        return Ok(None);
    }
    let id = rest[..digits]
        .parse::<u32>()
        .ok()
        .filter(|&id| id != u32::MAX)
        .ok_or((at, "id out of range".to_string()))?;
    Ok(Some((id, at + 1 + digits)))
}

/// A number: `Int32`, `Int64` (`i64`), or `Float64`; or, for versions and
/// hashes, a word that starts with a digit.
fn number(src: &str, start: usize) -> Result<(Tok, usize), (usize, String)> {
    let bytes = src.as_bytes();
    let digits_from = |i: usize| {
        (i..bytes.len())
            .find(|&j| !bytes[j].is_ascii_digit())
            .unwrap_or(bytes.len())
    };
    let mut i = start + usize::from(bytes[start] == b'-');
    if src[i..].starts_with("inf") {
        return Ok((Tok::Float(f64::NEG_INFINITY), i + 3));
    }
    i = digits_from(i);
    let mut float = false;
    if bytes.get(i) == Some(&b'.') && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
        float = true;
        i = digits_from(i + 1);
        if bytes.get(i) == Some(&b'.') {
            // A version, as `0.1.0-beta.2`.
            let end = (i..bytes.len())
                .find(|&j| !(bytes[j].is_ascii_alphanumeric() || b".-+".contains(&bytes[j])))
                .unwrap_or(bytes.len());
            return Ok((word(src, start, end), end));
        }
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        let sign = usize::from(bytes.get(i + 1) == Some(&b'-'));
        if bytes.get(i + 1 + sign).is_some_and(u8::is_ascii_digit) {
            float = true;
            i = digits_from(i + 1 + sign);
        }
    }
    let i64 = !float && src[i..].starts_with("i64");
    let end = if i64 { i + 3 } else { i };
    if bytes
        .get(end)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    {
        // A hash, say.
        let end = (end..bytes.len())
            .find(|&j| !(bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_'))
            .unwrap_or(bytes.len());
        return Ok((word(src, start, end), end));
    }
    let text = &src[start..i];
    let tok = if float {
        Tok::Float(
            text.parse()
                .map_err(|_| (start, "bad number".to_string()))?,
        )
    } else {
        let value = text
            .parse()
            .map_err(|_| (start, "number out of range".to_string()))?;
        Tok::Int { value, i64 }
    };
    Ok((tok, end))
}

fn word(src: &str, start: usize, end: usize) -> Tok {
    Tok::Word {
        text: src[start..end].to_string(),
        quoted: false,
    }
}

fn string(src: &str, start: usize) -> Result<(String, usize), (usize, String)> {
    let mut text = String::new();
    let mut chars = src[start + 1..].char_indices();
    let at = |n: usize| start + 1 + n;
    while let Some((n, c)) = chars.next() {
        match c {
            '"' => return Ok((text, at(n) + 1)),
            '\\' => match chars.next() {
                Some((_, '"')) => text.push('"'),
                Some((_, '\\')) => text.push('\\'),
                Some((_, 'n')) => text.push('\n'),
                Some((_, 'r')) => text.push('\r'),
                Some((_, 't')) => text.push('\t'),
                Some((m, 'u')) => {
                    let rest = &src[at(m) + 1..];
                    let close = rest.find('}').filter(|_| rest.starts_with('{'));
                    let code = close
                        .and_then(|close| u32::from_str_radix(&rest[1..close], 16).ok())
                        .and_then(char::from_u32)
                        .ok_or((at(n), "bad \\u{…} escape".to_string()))?;
                    text.push(code);
                    for _ in 0..close.unwrap_or(0) + 1 {
                        chars.next();
                    }
                }
                _ => return Err((at(n), "bad escape in a string".into())),
            },
            '\n' => break,
            c => text.push(c),
        }
    }
    Err((start, "unclosed string".into()))
}

// Reading.

/// A module's items and init block, found by the first pass.
struct ModuleHead {
    decl: DeclId,
    items: Vec<Item>,
    init: usize,
}

enum Item {
    /// A function, its parameters, and where its body starts.
    Function(DeclId, Vec<DeclId>, usize),
    ModuleLet(DeclId),
}

struct Reader {
    src: String,
    tokens: Vec<Token>,
    pos: usize,
    b: Builder,
    /// Which declaration ids are defined so far.
    defined: Vec<bool>,
    /// The function or module that owns locals and labels being read.
    owner: DeclId,
}

type Read<T> = Result<T, ReadError>;

impl Reader {
    // Token helpers.

    fn peek(&self) -> &Tok {
        &self.tokens[self.pos].tok
    }

    fn peek_at(&self, ahead: usize) -> &Tok {
        let i = (self.pos + ahead).min(self.tokens.len() - 1);
        &self.tokens[i].tok
    }

    fn next(&mut self) -> Tok {
        let tok = self.tokens[self.pos].tok.clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn error<T>(&self, message: impl Into<String>) -> Read<T> {
        Err(error_at(
            &self.src,
            self.tokens[self.pos].start,
            message.into(),
        ))
    }

    fn is_punct(&self, p: &str) -> bool {
        matches!(self.peek(), Tok::Punct(q) if *q == p)
    }

    fn is_word(&self, w: &str) -> bool {
        is_keyword(self.peek(), w)
    }

    fn eat(&mut self, p: &str) -> bool {
        let found = self.is_punct(p);
        if found {
            self.next();
        }
        found
    }

    fn eat_word(&mut self, w: &str) -> bool {
        let found = self.is_word(w);
        if found {
            self.next();
        }
        found
    }

    fn expect(&mut self, p: &str) -> Read<()> {
        if self.eat(p) {
            Ok(())
        } else {
            self.error(format!("expected `{p}`"))
        }
    }

    fn expect_word(&mut self, w: &str) -> Read<()> {
        if self.eat_word(w) {
            Ok(())
        } else {
            self.error(format!("expected `{w}`"))
        }
    }

    /// A word with no id: a library or profile name, or a version.
    fn plain_word(&mut self, what: &str) -> Read<String> {
        match self.next() {
            Tok::Word { text, .. } => Ok(text),
            _ => {
                self.pos -= 1;
                self.error(format!("expected {what}"))
            }
        }
    }

    fn int(&mut self) -> Read<i128> {
        match *self.peek() {
            Tok::Int { value, i64: false } => {
                self.next();
                Ok(value)
            }
            _ => self.error("expected a number"),
        }
    }

    fn u32(&mut self) -> Read<u32> {
        let n = self.int()?;
        u32::try_from(n).or_else(|_| {
            self.pos -= 1;
            self.error("number out of range")
        })
    }

    /// Skips a `{ … }` group.
    fn skip_braces(&mut self) -> Read<()> {
        self.expect("{")?;
        let mut depth = 1;
        while depth > 0 {
            match self.next() {
                Tok::Punct("{") => depth += 1,
                Tok::Punct("}") => depth -= 1,
                Tok::End => return self.error("unclosed `{`"),
                _ => {}
            }
        }
        Ok(())
    }

    // Declarations.

    /// Defines the declaration at the current token.
    fn define(&mut self, kind: DeclKind, owner: DeclId, ty: TypeId) -> Read<DeclId> {
        let Tok::Decl { names, id, made } = self.peek().clone() else {
            return self.error("expected a declaration, as `name#id`");
        };
        if self.defined.get(id as usize) == Some(&true) {
            return self.error(format!("#{id} is defined twice"));
        }
        self.next();
        if self.defined.len() <= id as usize {
            self.defined.resize(id as usize + 1, false);
        }
        self.defined[id as usize] = true;
        let id = DeclId(id);
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        self.b.define(id, kind, &names, owner, ty);
        if made {
            self.b.add_facts(id, Facts::COMPILER_MADE);
        }
        Ok(id)
    }

    /// A use of a declaration, checked against its definition.
    fn reference(&mut self) -> Read<DeclId> {
        let Tok::Decl { names, id, made } = self.peek().clone() else {
            return self.error("expected a reference, as `name#id`");
        };
        if names.len() > 1 {
            return self.error("a reference names only the first hint");
        }
        if self.defined.get(id as usize) != Some(&true) {
            return self.error(format!("#{id} is used before it's defined"));
        }
        let decl = DeclId(id);
        let p = self.b.program();
        let made_there = p.decls.get(decl).facts.contains(Facts::COMPILER_MADE);
        if p.name(decl) != names[0] || made != made_there {
            let there = if made_there { "~" } else { "" };
            return self.error(format!(
                "#{id} is named {there}{}#{id}",
                super::write::name(p.name(decl))
            ));
        }
        self.next();
        Ok(decl)
    }

    fn ty(&mut self) -> Read<TypeId> {
        if let Tok::Word {
            text,
            quoted: false,
        } = self.peek()
        {
            if let Some(ty) = TypeTable::scalar_named(text) {
                self.next();
                return Ok(ty);
            }
        }
        self.error("expected a type")
    }

    // The first pass: headers.

    fn headers(&mut self) -> Read<Vec<ModuleHead>> {
        self.expect_word("tmpl")?;
        let format = self.int()?;
        self.expect_word("model")?;
        let model = self.int()?;
        if format != FORMAT_VERSION as i128 || model != MODEL_VERSION as i128 {
            return self.error(format!(
                "this reads tmpl {FORMAT_VERSION} model {MODEL_VERSION} only"
            ));
        }
        self.expect_word("library")?;
        let name = self.plain_word("the library's name")?;
        let version = self.plain_word("the library's version")?;
        self.b = Builder::new(&name, &version);
        if self.eat_word("profile") {
            let profile = self.plain_word("a profile's name")?;
            self.b.set_profile(&profile);
        }
        let mut modules = Vec::new();
        while self.eat_word("module") {
            modules.push(self.module_head()?);
        }
        if self.is_word("spans") {
            self.next();
            self.skip_braces()?;
        }
        if *self.peek() != Tok::End {
            return self.error("expected `module`, `spans`, or the end");
        }
        Ok(modules)
    }

    fn module_head(&mut self) -> Read<ModuleHead> {
        let decl = self.define(DeclKind::Module, DeclId::NONE, TypeId::NONE)?;
        self.b.add_module(decl);
        self.expect("{")?;
        let mut items = Vec::new();
        loop {
            let mut doc = Vec::new();
            while let Tok::Doc(line) = self.peek() {
                doc.push(line.clone());
                self.next();
            }
            if doc.is_empty() && self.eat_word("init") {
                break;
            }
            let exported = self.eat_word("export");
            let item = if self.eat_word("fn") {
                self.function_head(decl)?
            } else if self.is_word("let") || self.is_word("var") {
                let var = self.eat_word("var");
                if !var {
                    self.next();
                }
                let d = self.define(DeclKind::ModuleLet, decl, TypeId::NONE)?;
                self.expect(":")?;
                self.b.program_mut().decls.get_mut(d).ty = self.ty()?;
                self.expect(";")?;
                if var {
                    self.b.add_facts(d, Facts::VAR);
                }
                Item::ModuleLet(d)
            } else {
                return self.error("expected `fn`, `let`, `var`, or `init`");
            };
            let d = match item {
                Item::Function(d, ..) | Item::ModuleLet(d) => d,
            };
            if exported {
                self.b.add_facts(d, Facts::EXPORTED);
            }
            if !doc.is_empty() {
                self.b.set_doc(d, &doc.join("\n"));
            }
            items.push(item);
        }
        let init = self.pos;
        self.skip_braces()?;
        self.expect("}")?;
        Ok(ModuleHead { decl, items, init })
    }

    fn function_head(&mut self, module: DeclId) -> Read<Item> {
        let decl = self.define(DeclKind::Function, module, TypeId::NONE)?;
        self.expect("(")?;
        let mut params = Vec::new();
        let mut types = Vec::new();
        while !self.eat(")") {
            if !params.is_empty() {
                self.expect(",")?;
            }
            let param = self.define(DeclKind::Param, decl, TypeId::NONE)?;
            self.expect(":")?;
            let ty = self.ty()?;
            self.b.program_mut().decls.get_mut(param).ty = ty;
            params.push(param);
            types.push(ty);
        }
        self.expect(":")?;
        let result = self.ty()?;
        let bubbles = self.eat_word("throws");
        if bubbles {
            self.expect_word("Bubble")?;
        }
        let sig = self.b.function_type(&types, result, bubbles);
        self.b.program_mut().decls.get_mut(decl).ty = sig;
        let body = self.pos;
        self.skip_braces()?;
        Ok(Item::Function(decl, params, body))
    }

    // The second pass: bodies.

    fn bodies(mut self, modules: Vec<ModuleHead>) -> Read<Program> {
        for module in modules {
            for item in module.items {
                let node = match item {
                    Item::Function(decl, params, body) => {
                        let params: Vec<_> = params.iter().map(|&d| self.b.param(d)).collect();
                        self.pos = body;
                        self.owner = decl;
                        let body = self.block()?;
                        self.b.function(decl, &params, body)
                    }
                    Item::ModuleLet(decl) => self.b.module_let(decl),
                };
                self.b.add_item(module.decl, node);
            }
            self.pos = module.init;
            self.owner = module.decl;
            let init = self.block()?;
            self.b.set_init(module.decl, init);
        }
        while !matches!(self.peek(), Tok::End) && !self.is_word("spans") {
            self.next();
        }
        if self.eat_word("spans") {
            self.spans()?;
        }
        Ok(self.b.finish())
    }

    fn block(&mut self) -> Read<NodeId> {
        self.expect("{")?;
        let mut stmts = Vec::new();
        while !self.eat("}") {
            stmts.push(self.stmt()?);
        }
        Ok(self.b.block(&stmts))
    }

    /// Whether a statement starts here, rather than an expression.
    fn at_stmt(&self) -> bool {
        let keyword = ["let", "var", "init", "if", "break", "continue", "return"]
            .iter()
            .any(|w| self.is_word(w));
        keyword
            || self.is_punct("{")
            || (matches!(self.peek(), Tok::Decl { .. })
                && matches!(self.peek_at(1), Tok::Punct(":" | "=")))
    }

    fn stmt(&mut self) -> Read<NodeId> {
        if self.is_word("let") || self.is_word("var") {
            let var = self.is_word("var");
            self.next();
            let local = self.define(DeclKind::Local, self.owner, TypeId::NONE)?;
            if var {
                self.b.add_facts(local, Facts::VAR);
            }
            self.expect(":")?;
            self.b.program_mut().decls.get_mut(local).ty = self.ty()?;
            let init = if self.eat("=") {
                Some(self.expr()?)
            } else {
                None
            };
            self.expect(";")?;
            return Ok(self.b.let_stmt(local, init));
        }
        if self.eat_word("init") {
            let target = self.reference()?;
            self.expect("=")?;
            let value = self.expr()?;
            self.expect(";")?;
            return Ok(self.b.init_let(target, value));
        }
        if self.is_word("if") {
            return self.if_stmt();
        }
        if self.is_word("break") || self.is_word("continue") {
            let is_break = self.is_word("break");
            self.next();
            let label = self.reference()?;
            self.expect(";")?;
            return Ok(if is_break {
                self.b.break_loop(label)
            } else {
                self.b.continue_loop(label)
            });
        }
        if self.eat_word("return") {
            let value = if self.is_punct(";") {
                None
            } else {
                Some(self.expr()?)
            };
            self.expect(";")?;
            return Ok(self.b.return_stmt(value));
        }
        if self.is_punct("{") {
            return self.block();
        }
        if matches!(self.peek(), Tok::Decl { .. }) && *self.peek_at(1) == Tok::Punct(":") {
            return self.labeled_loop();
        }
        if matches!(self.peek(), Tok::Decl { .. }) && *self.peek_at(1) == Tok::Punct("=") {
            let target = self.reference()?;
            self.next();
            let value = self.expr()?;
            self.expect(";")?;
            let module_let = self.b.program().decls.get(target).kind == DeclKind::ModuleLet;
            return Ok(if module_let {
                self.b.set_module_let(target, value)
            } else {
                self.b.assign(target, value)
            });
        }
        if matches!(self.peek(), Tok::Doc(_)) {
            return self.error("a doc comment goes only before a module's item");
        }
        let expr = self.expr()?;
        self.expect(";")?;
        Ok(self.b.expr_statement(expr))
    }

    fn if_stmt(&mut self) -> Read<NodeId> {
        self.expect_word("if")?;
        self.expect("(")?;
        let cond = self.expr()?;
        self.expect(")")?;
        let then = self.block()?;
        let els = if self.eat_word("else") {
            if self.is_word("if") {
                let inner = self.if_stmt()?;
                Some(self.b.block(&[inner]))
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(self.b.if_stmt(cond, then, els))
    }

    fn labeled_loop(&mut self) -> Read<NodeId> {
        let label = self.define(DeclKind::Label, self.owner, TypeId::NONE)?;
        self.expect(":")?;
        if self.eat_word("while") {
            self.expect("(")?;
            let cond = self.expr()?;
            self.expect(")")?;
            let body = self.block()?;
            Ok(self.b.while_loop(label, cond, body))
        } else if self.eat_word("do") {
            let body = self.block()?;
            self.expect_word("while")?;
            self.expect("(")?;
            let cond = self.expr()?;
            self.expect(")")?;
            self.expect(";")?;
            Ok(self.b.do_while(label, body, cond))
        } else {
            self.error("expected `while` or `do` after a label")
        }
    }

    fn expr(&mut self) -> Read<NodeId> {
        let mut left = self.and()?;
        while self.eat("||") {
            let right = self.and()?;
            left = self.b.or(left, right);
        }
        Ok(left)
    }

    fn and(&mut self) -> Read<NodeId> {
        let mut left = self.primary()?;
        while self.eat("&&") {
            let right = self.primary()?;
            left = self.b.and(left, right);
        }
        Ok(left)
    }

    fn primary(&mut self) -> Read<NodeId> {
        match self.peek().clone() {
            Tok::Int { value, i64: false } => match i32::try_from(value) {
                Ok(value) => {
                    self.next();
                    Ok(self.b.int32(value))
                }
                Err(_) => self.error("Int32 out of range"),
            },
            Tok::Int { value, i64: true } => match i64::try_from(value) {
                Ok(value) => {
                    self.next();
                    Ok(self.b.int64(value))
                }
                Err(_) => self.error("Int64 out of range"),
            },
            Tok::Float(value) => {
                self.next();
                Ok(self.b.float64(value))
            }
            Tok::Str(text) => {
                self.next();
                Ok(self.b.string(&text))
            }
            Tok::Word {
                text,
                quoted: false,
            } => {
                let node = match text.as_str() {
                    "true" => self.b.bool_lit(true),
                    "false" => self.b.bool_lit(false),
                    "void" => self.b.void(),
                    "nan" => self.b.float64(f64::NAN),
                    "inf" => self.b.float64(f64::INFINITY),
                    _ => return self.error("expected an expression"),
                };
                self.next();
                Ok(node)
            }
            Tok::Key(name) => {
                let Some(key) = Key::named(&name) else {
                    return self.error(format!("no key %{name}"));
                };
                self.next();
                let args = self.args()?;
                let call = self.b.builtin(key, &args);
                self.caret(call)?;
                Ok(call)
            }
            Tok::Decl { .. } => {
                let decl = self.reference()?;
                let kind = self.b.program().decls.get(decl).kind;
                if self.is_punct("(") {
                    if kind != DeclKind::Function {
                        self.pos -= 1;
                        return self.error("only a function can be called");
                    }
                    let args = self.args()?;
                    let call = self.b.call(decl, &args);
                    self.caret(call)?;
                    return Ok(call);
                }
                match kind {
                    DeclKind::Local | DeclKind::Param => Ok(self.b.local(decl)),
                    DeclKind::ModuleLet => Ok(self.b.get_module_let(decl)),
                    _ => {
                        self.pos -= 1;
                        self.error(format!("a {} isn't a value", kind.name()))
                    }
                }
            }
            Tok::Punct("(") => self.parenthesized(),
            _ => self.error("expected an expression"),
        }
    }

    fn args(&mut self) -> Read<Vec<NodeId>> {
        self.expect("(")?;
        let mut args = Vec::new();
        while !self.eat(")") {
            if !args.is_empty() {
                self.expect(",")?;
            }
            args.push(self.expr()?);
        }
        Ok(args)
    }

    /// Checks that `^` follows exactly the calls that may bubble.
    fn caret(&mut self, call: NodeId) -> Read<()> {
        let marked = self.eat("^");
        let bubbles = self
            .b
            .program()
            .nodes
            .flags(call)
            .contains(Flags::MAY_BUBBLE);
        match (marked, bubbles) {
            (true, false) => {
                self.pos -= 1;
                self.error("`^` marks a call that can't bubble")
            }
            (false, true) => self.error("a call that may bubble needs `^`"),
            _ => Ok(()),
        }
    }

    /// `(e)`, `(e): T`, or a form whose type is always written.
    fn parenthesized(&mut self) -> Read<NodeId> {
        self.expect("(")?;
        if self.eat_word("if") {
            self.expect("(")?;
            let cond = self.expr()?;
            self.expect(")")?;
            let then = self.expr()?;
            self.expect_word("else")?;
            let els = self.expr()?;
            let ty = self.written_type()?;
            return Ok(self.b.if_value(ty, cond, then, els));
        }
        if self.eat_word("value") {
            self.expect("{")?;
            let mut stmts = Vec::new();
            let value = loop {
                if self.at_stmt() {
                    stmts.push(self.stmt()?);
                    continue;
                }
                let expr = self.expr()?;
                if self.eat(";") {
                    stmts.push(self.b.expr_statement(expr));
                } else {
                    self.expect("}")?;
                    break expr;
                }
            };
            let ty = self.written_type()?;
            return Ok(self.b.block_value(ty, &stmts, value));
        }
        if self.eat_word("error") {
            self.expect("(")?;
            let Tok::Str(message) = self.next() else {
                self.pos -= 1;
                return self.error("expected the error's message");
            };
            self.expect(")")?;
            let ty = self.written_type()?;
            return Ok(self.b.error(ty, &message));
        }
        let expr = self.expr()?;
        self.expect(")")?;
        if self.eat(":") {
            let at = self.pos;
            let ty = self.ty()?;
            let p = self.b.program();
            let rule = p.nodes.ty(expr);
            if ty != rule {
                self.pos = at;
                let rule = p.types.show(rule);
                return self.error(format!("the type here is {rule}, by the local rules"));
            }
        }
        Ok(expr)
    }

    /// `): T`, closing a form whose type is written.
    fn written_type(&mut self) -> Read<TypeId> {
        self.expect(")")?;
        self.expect(":")?;
        self.ty()
    }

    fn spans(&mut self) -> Read<()> {
        self.b.program_mut().has_spans = true;
        self.expect("{")?;
        while self.eat_word("file") {
            let id = self.u32()?;
            if id as usize != self.b.program().files.len() {
                return self.error("files are numbered in order from 0");
            }
            let Tok::Str(path) = self.next() else {
                self.pos -= 1;
                return self.error("expected the file's path");
            };
            let hash = self.hash()?;
            self.b.add_file(&path, hash);
            self.expect(";")?;
        }
        let order = writing_order(self.b.program());
        loop {
            if self.eat("#") {
                let id = self.u32()?;
                if self.defined.get(id as usize) != Some(&true) {
                    return self.error(format!("no declaration #{id}"));
                }
                let span = self.maybe_span()?;
                let name_span = if self.eat_word("name") {
                    self.span()?
                } else {
                    Span::UNKNOWN
                };
                self.b.set_decl_spans(DeclId(id), span, name_span);
            } else if self.eat("@") {
                let n = self.u32()?;
                let Some(&node) = order.get(n as usize) else {
                    return self.error(format!("no node @{n}"));
                };
                let span = self.maybe_span()?;
                let derived = self.eat("~");
                self.b.set_span(node, span, derived);
            } else {
                break;
            }
            self.expect(";")?;
        }
        self.expect("}")
    }

    fn hash(&mut self) -> Read<[u8; 16]> {
        let token = &self.tokens[self.pos];
        let text = &self.src[token.start..token.end];
        let mut hash = [0; 16];
        let valid = text.len() == 32
            && (0..16).all(|i| match u8::from_str_radix(&text[2 * i..2 * i + 2], 16) {
                Ok(byte) => {
                    hash[i] = byte;
                    true
                }
                Err(_) => false,
            });
        if !valid {
            return self.error("expected a hash of 32 hex digits");
        }
        self.next();
        Ok(hash)
    }

    fn maybe_span(&mut self) -> Read<Span> {
        if matches!(self.peek(), Tok::Int { .. }) {
            self.span()
        } else {
            Ok(Span::UNKNOWN)
        }
    }

    /// `file:start-end`.
    fn span(&mut self) -> Read<Span> {
        let file = self.u32()?;
        if file as usize >= self.b.program().files.len() {
            self.pos -= 1;
            return self.error(format!("no file {file}"));
        }
        self.expect(":")?;
        let start = self.u32()?;
        self.expect("-")?;
        let end = self.u32()?;
        Ok(Span {
            file: FileId(file),
            start,
            end,
        })
    }
}

fn is_keyword(tok: &Tok, w: &str) -> bool {
    matches!(tok, Tok::Word { text, quoted: false } if text == w)
}

#[cfg(test)]
mod tests {
    use super::{Tok, lex};

    fn tokens(src: &str) -> Vec<Tok> {
        let mut tokens: Vec<Tok> = lex(src).unwrap().into_iter().map(|t| t.tok).collect();
        tokens.pop(); // End
        tokens
    }

    fn word(text: &str) -> Tok {
        Tok::Word {
            text: text.into(),
            quoted: false,
        }
    }

    #[test]
    fn numbers_versions_and_hashes() {
        assert_eq!(
            tokens("1 -2 3i64 1.5 -1.0e-3 -inf 0.1.0-beta.2 00ff 0:10-20"),
            [
                Tok::Int {
                    value: 1,
                    i64: false
                },
                Tok::Int {
                    value: -2,
                    i64: false
                },
                Tok::Int {
                    value: 3,
                    i64: true
                },
                Tok::Float(1.5),
                Tok::Float(-1.0e-3),
                Tok::Float(f64::NEG_INFINITY),
                word("0.1.0-beta.2"),
                word("00ff"),
                Tok::Int {
                    value: 0,
                    i64: false
                },
                Tok::Punct(":"),
                Tok::Int {
                    value: 10,
                    i64: false
                },
                Tok::Punct("-"),
                Tok::Int {
                    value: 20,
                    i64: false
                },
            ]
        );
    }

    #[test]
    fn names_and_keys() {
        let decl = |names: &[&str], id, made| Tok::Decl {
            names: names.iter().map(|n| n.to_string()).collect(),
            id,
            made,
        };
        assert_eq!(
            tokens("x#1 `a b`#2 ~t|u#3 %Int32.add fn `fn` /// doc\n// skipped\n~"),
            [
                decl(&["x"], 1, false),
                decl(&["a b"], 2, false),
                decl(&["t", "u"], 3, true),
                Tok::Key("Int32.add".into()),
                word("fn"),
                Tok::Word {
                    text: "fn".into(),
                    quoted: true
                },
                Tok::Doc("doc".into()),
                Tok::Punct("~"),
            ]
        );
    }
}
