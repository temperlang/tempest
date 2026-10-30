//! Qualified names (qnames), in Kotlin Temper's format (its `QName.kt`;
//! notes/tmpl/names.md, "Qnames").
//!
//! A qname is a library's name, the module's directory relative to the
//! library's root, then one part per enclosing declaration, each kinded by
//! punctuation, and a disambiguation index only where it would otherwise be
//! ambiguous:
//!
//! ```text
//! my-library/foo/bar.f()           a function
//! my-library/foo/bar.f().(x)       its parameter
//! my-library/foo/bar.f().y=        a local in it, however deeply nested
//! my-library/foo/bar.x=#1          the second local `x` of an init block
//! core.type Int32.toString()       a method of core's `Int32`
//! core.type String.get isEmpty()   a getter
//! ```
//!
//! [`QName`] parses and prints the format exactly as Kotlin does, `\`
//! escapes included. [`QNames::derive`] gives every declaration of a
//! program its qname, from the library's name, the module's path, the
//! owner chain, the source names, and the declarations' kinds; a
//! declaration stores only an override, where Kotlin's would differ.

use crate::decls::{DeclKind, Facts};
use crate::ids::DeclId;
use crate::{Program, TypeId};
use std::collections::HashMap;
use std::fmt;

/// What a part of a qname names, which its punctuation shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// `type C`: a class or interface.
    Type,
    /// `<T>`: a type parameter.
    TypeFormal,
    /// `get x()`.
    Getter,
    /// `set x()`.
    Setter,
    /// `constructor x()`; Kotlin writes constructors as `constructor()`, a
    /// method named `constructor`, in practice.
    Constructor,
    /// `f()`: a function or method.
    FunctionOrMethod,
    /// `(x)`: a parameter.
    Input,
    /// `x`: a top-level declaration or a property.
    Decl,
    /// `x=`: a local.
    Local,
}

impl PartKind {
    const ALL: [PartKind; 9] = [
        PartKind::Type,
        PartKind::TypeFormal,
        PartKind::Getter,
        PartKind::Setter,
        PartKind::Constructor,
        PartKind::FunctionOrMethod,
        PartKind::Input,
        PartKind::Decl,
        PartKind::Local,
    ];

    /// The text before and after the name.
    fn affixes(self) -> (&'static str, &'static str) {
        match self {
            PartKind::Decl => ("", ""),
            PartKind::Local => ("", "="),
            PartKind::FunctionOrMethod => ("", "()"),
            PartKind::Input => ("(", ")"),
            PartKind::TypeFormal => ("<", ">"),
            PartKind::Constructor => ("constructor", "()"),
            PartKind::Getter => ("get", "()"),
            PartKind::Setter => ("set", "()"),
            PartKind::Type => ("type", ""),
        }
    }
}

/// One part of a qname: a name and its kind.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Part {
    pub name: String,
    pub kind: PartKind,
}

/// A qualified name, as Kotlin Temper's `QName` holds it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QName {
    /// `core` for core and `std` for std ([`qname_library`]).
    pub library: String,
    /// The module's directory, by segment; empty at the library's root.
    pub path: Vec<String>,
    pub parts: Vec<Part>,
    /// Where the qname would otherwise be ambiguous in its module: the
    /// declaration's place among those sharing it, in declaration order.
    pub index: Option<u32>,
}

/// Characters that delimit a qname's segments.
fn is_meta(c: char) -> bool {
    matches!(
        c,
        '/' | '.' | '<' | '>' | '(' | ')' | '=' | ' ' | '#' | '\\'
    )
}

/// The escaped form of a character, if it has one. As in Kotlin, `(` and
/// `)` have none, so a name holding them can't be written (a known gap).
fn escape_of(c: char) -> Option<&'static str> {
    Some(match c {
        '\0' => "\\0",
        '\t' => "\\t",
        '\n' => "\\n",
        '\r' => "\\r",
        ' ' => "\\ ",
        '"' => "\\\"",
        '#' => "\\#",
        '.' => "\\.",
        '/' => "\\/",
        '<' => "\\<",
        '=' => "\\=",
        '>' => "\\>",
        '\\' => "\\\\",
        _ => return None,
    })
}

fn unescape(c: char) -> Option<char> {
    Some(match c {
        '0' => '\0',
        't' => '\t',
        'n' => '\n',
        'r' => '\r',
        ' ' | '"' | '#' | '.' | '/' | '<' | '=' | '>' | '\\' => c,
        _ => return None,
    })
}

fn escape_onto(text: &str, out: &mut String) {
    for c in text.chars() {
        match escape_of(c) {
            Some(e) => out.push_str(e),
            None => out.push(c),
        }
    }
}

impl fmt::Display for QName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = String::new();
        escape_onto(&self.library, &mut out);
        for segment in &self.path {
            out.push('/');
            escape_onto(segment, &mut out);
        }
        for part in &self.parts {
            let (before, after) = part.kind.affixes();
            out.push('.');
            out.push_str(before);
            if before.ends_with(|c: char| c.is_ascii_lowercase()) {
                // Keywords are followed by spaces.
                out.push(' ');
            }
            escape_onto(&part.name, &mut out);
            out.push_str(after);
        }
        if let Some(index) = self.index {
            out.push('#');
            out.push_str(&index.to_string());
        }
        f.write_str(&out)
    }
}

/// A cursor over a qname's text, reading escapes as Kotlin's parser does.
struct Cursor<'a> {
    text: &'a str,
    at: usize,
    /// What messages show: the text as given, before any prefix added.
    shown: &'a str,
    /// How much was added before `shown`, to subtract from offsets.
    bias: usize,
}

/// A character read, and whether it was escaped.
#[derive(Clone, Copy)]
struct Read {
    c: char,
    escaped: bool,
    len: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Result<Option<Read>, String> {
        let mut chars = self.text[self.at..].chars();
        let Some(c) = chars.next() else {
            return Ok(None);
        };
        if c != '\\' {
            let len = c.len_utf8();
            return Ok(Some(Read {
                c,
                escaped: false,
                len,
            }));
        }
        let Some(next) = chars.next() else {
            return Err(format!("incomplete escape at the end of `{}`", self.shown));
        };
        let c = unescape(next).ok_or_else(|| {
            format!(
                "bad escape `\\{next}` at offset {} in `{}`",
                self.offset(),
                self.shown
            )
        })?;
        Ok(Some(Read {
            c,
            escaped: true,
            len: 1 + next.len_utf8(),
        }))
    }

    /// A run of meta-characters, or of others, up to a separator.
    fn drain(&mut self, want_meta: bool) -> Result<String, String> {
        let mut out = String::new();
        while let Some(r) = self.peek()? {
            if !r.escaped && matches!(r.c, '.' | '/' | '#') {
                break;
            }
            if want_meta != (!r.escaped && is_meta(r.c)) {
                break;
            }
            out.push(r.c);
            self.at += r.len;
        }
        Ok(out)
    }

    fn offset(&self) -> usize {
        self.at.saturating_sub(self.bias)
    }

    /// Whether the next character is `c`, unescaped.
    fn at_plain(&self, c: char) -> Result<bool, String> {
        Ok(self.peek()?.is_some_and(|r| !r.escaped && r.c == c))
    }
}

impl QName {
    /// Parses a qname's text, as Kotlin's `QName.fromString` does.
    pub fn parse(text: &str) -> Result<QName, String> {
        QName::parse_shown(text, text, 0)
    }

    fn parse_shown(full: &str, text: &str, bias: usize) -> Result<QName, String> {
        let mut cur = Cursor {
            text: full,
            at: 0,
            shown: text,
            bias,
        };
        let library = cur.drain(false)?;
        if library.is_empty() || library.starts_with('-') {
            return Err(format!("malformed library name in `{text}`"));
        }
        let mut path = Vec::new();
        while cur.at_plain('/')? {
            cur.at += 1;
            let segment = cur.drain(false)?;
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(format!("bad path segment `{segment}` in `{text}`"));
            }
            path.push(segment);
        }
        let mut parts = Vec::new();
        let mut index = None;
        while let Some(r) = cur.peek()? {
            if r.escaped {
                return Err(format!(
                    "unparsed content at offset {} in `{text}`",
                    cur.offset()
                ));
            }
            cur.at += r.len;
            match r.c {
                '.' => {
                    let start = cur.at;
                    let shown_start = cur.offset();
                    let mut prefix = cur.drain(true)?;
                    let mut name = cur.drain(false)?;
                    if prefix.is_empty() && cur.at_plain(' ')? {
                        // A keyword, as `type`, then its name.
                        prefix = name;
                        cur.at += 1;
                        name = cur.drain(false)?;
                    }
                    let suffix = cur.drain(true)?;
                    if name.is_empty() {
                        return Err(format!("empty name at offset {} in `{text}`", cur.offset()));
                    }
                    let kind = PartKind::ALL
                        .into_iter()
                        .find(|k| k.affixes() == (prefix.as_str(), suffix.as_str()))
                        .ok_or_else(|| {
                            format!(
                                "can't tell the kind of `{}` at offset {shown_start} in `{text}`",
                                &full[start..cur.at]
                            )
                        })?;
                    parts.push(Part { name, kind });
                }
                '#' => {
                    let digits = &full[cur.at..];
                    let n = digits
                        .parse::<u32>()
                        .ok()
                        .filter(|_| digits.bytes().all(|b| b.is_ascii_digit()))
                        .ok_or_else(|| {
                            format!(
                                "bad disambiguation index at offset {} in `{text}`",
                                cur.offset()
                            )
                        })?;
                    index = Some(n);
                    cur.at = full.len();
                }
                c => {
                    return Err(format!(
                        "expected `.` or `#`, not `{c}`, at offset {} in `{text}`",
                        cur.offset().saturating_sub(r.len)
                    ));
                }
            }
        }
        Ok(QName {
            library,
            path,
            parts,
            index,
        })
    }

    /// Parses the part of a qname after the module's path, as a stored
    /// override holds it: `f().x=#1`.
    pub fn parse_tail(tail: &str) -> Result<(Vec<Part>, Option<u32>), String> {
        let q = QName::parse_shown(&format!("l.{tail}"), tail, 2)?;
        if q.parts.is_empty() {
            return Err(format!("no parts in `{tail}`"));
        }
        Ok((q.parts, q.index))
    }
}

/// A library's name as qnames write it: `core` for `temper-core` and
/// `std` for `temper-std`, as in Kotlin; others as they are.
pub fn qname_library(name: &str) -> &str {
    match name {
        "temper-core" => "core",
        "temper-std" => "std",
        _ => name,
    }
}

/// The library a declaration belongs to, as qnames write it: the program's
/// own, or for an external declaration, its `uses` section's.
pub fn library_of(p: &Program, decl: DeclId) -> &str {
    let mut d = decl;
    for _ in 0..p.decls.len() {
        if !p.decls.contains(d) {
            break;
        }
        let entry = p.decls.get(d);
        match entry.kind {
            DeclKind::Library => return qname_library(p.name(d)),
            DeclKind::Module if entry.owner.is_none() => break,
            _ => d = entry.owner,
        }
    }
    qname_library(&p.library.name)
}

/// Whether a declaration comes from another library: owned, through its
/// owners, by a `uses` section's library.
pub fn is_external(p: &Program, decl: DeclId) -> bool {
    let mut d = decl;
    for _ in 0..p.decls.len() {
        if !p.decls.contains(d) {
            return false;
        }
        match p.decls.get(d).kind {
            DeclKind::Library => return d != decl,
            DeclKind::Module if p.decls.get(d).owner.is_none() => return false,
            _ => d = p.decls.get(d).owner,
        }
    }
    false
}

/// How a connected declaration resolves: core's and std's by qname,
/// through each variant's connection table; everything else by name, by
/// host code that defines the name the declaration is given
/// (notes/tmpl/names.md, "Connected declarations"). The one place that
/// decides, so moving std to user code's mechanism changes only this.
pub fn resolves_by_qname(p: &Program, decl: DeclId) -> bool {
    p.decls.contains(decl)
        && p.decls.get(decl).facts.contains(Facts::CONNECTED)
        && matches!(library_of(p, decl), "core" | "std")
}

/// The scalar type that core's class of this name stands for, as
/// `core.type Int32` stands for `Int32`.
pub fn core_scalar(name: &str) -> Option<TypeId> {
    crate::TypeTable::scalar_named(name).filter(|&t| t != TypeId::VOID)
}

/// Every declaration's qname, derived (notes/tmpl/names.md, "Qnames").
#[derive(Clone, Debug, Default)]
pub struct QNames {
    names: Vec<Option<QName>>,
    /// Declarations whose override doesn't parse, with why.
    pub bad_overrides: Vec<(DeclId, String)>,
}

impl QNames {
    /// A declaration's qname; `None` for compiler-made declarations,
    /// labels, modules, and used libraries, which Kotlin gives none, and
    /// for any whose owners don't lead to a module or a used library.
    pub fn get(&self, decl: DeclId) -> Option<&QName> {
        self.names.get(decl.index())?.as_ref()
    }

    /// Derives every declaration's qname. Each is built from its owner's
    /// parts (without the owner's index) and its own, by kind; a stored
    /// override replaces what follows the module's path. Derived qnames
    /// that coincide in a module all get `#n`, in `DeclId` order, which is
    /// declaration order.
    pub fn derive(p: &Program) -> QNames {
        let mut bases: Vec<Option<Option<QName>>> = vec![None; p.decls.len()];
        let mut bad = Vec::new();
        let mut overridden = vec![false; p.decls.len()];
        for id in p.decls.ids() {
            base(p, id, &mut bases, &mut bad, &mut overridden);
        }
        let mut names: Vec<Option<QName>> = bases.into_iter().map(Option::flatten).collect();
        // Modules and used libraries are only roots: Kotlin gives them none.
        for id in p.decls.ids() {
            if matches!(p.decls.get(id).kind, DeclKind::Module | DeclKind::Library) {
                names[id.index()] = None;
            }
        }
        let mut groups: HashMap<QName, Vec<DeclId>> = HashMap::new();
        for id in p.decls.ids() {
            if let Some(q) = &names[id.index()] {
                if !overridden[id.index()] {
                    groups.entry(q.clone()).or_default().push(id);
                }
            }
        }
        for ids in groups.into_values() {
            if ids.len() > 1 {
                for (i, id) in ids.into_iter().enumerate() {
                    if let Some(q) = &mut names[id.index()] {
                        q.index = Some(i as u32);
                    }
                }
            }
        }
        QNames {
            names,
            bad_overrides: bad,
        }
    }
}

/// A declaration's qname before disambiguation, memoized in `bases`; an
/// override keeps its own index.
fn base(
    p: &Program,
    id: DeclId,
    bases: &mut Vec<Option<Option<QName>>>,
    bad: &mut Vec<(DeclId, String)>,
    overridden: &mut [bool],
) -> Option<QName> {
    if let Some(done) = &bases[id.index()] {
        return done.clone();
    }
    // Guards against owner cycles (D2's problem, not this one's).
    bases[id.index()] = Some(None);
    let q = derive_one(p, id, bases, bad, overridden);
    bases[id.index()] = Some(q.clone());
    q
}

fn derive_one(
    p: &Program,
    id: DeclId,
    bases: &mut Vec<Option<Option<QName>>>,
    bad: &mut Vec<(DeclId, String)>,
    overridden: &mut [bool],
) -> Option<QName> {
    let d = p.decls.get(id);
    match d.kind {
        DeclKind::Removed | DeclKind::Label => return None,
        DeclKind::Module if !d.owner.is_none() => {
            // Another library's module, named by its path.
            let path = p.name(id).split('/').filter(|s| !s.is_empty());
            return Some(QName {
                library: qname_library(p.name(d.owner)).to_string(),
                path: path.map(String::from).collect(),
                parts: Vec::new(),
                index: None,
            });
        }
        DeclKind::Module => {
            let module = p.modules.iter().find(|m| m.decl == id)?;
            let path = module.path.split('/').filter(|s| !s.is_empty());
            return Some(QName {
                library: qname_library(&p.library.name).to_string(),
                path: path.map(String::from).collect(),
                parts: Vec::new(),
                index: None,
            });
        }
        DeclKind::Library => {
            return Some(QName {
                library: qname_library(p.name(id)).to_string(),
                path: Vec::new(),
                parts: Vec::new(),
                index: None,
            });
        }
        _ => {}
    }
    if d.facts.contains(Facts::COMPILER_MADE) || !p.decls.contains(d.owner) {
        return None;
    }
    let mut q = base(p, d.owner, bases, bad, overridden)?;
    q.index = None;
    if !d.qname.is_none() {
        match QName::parse_tail(p.strings.get(d.qname)) {
            Ok((parts, index)) => {
                overridden[id.index()] = true;
                q.parts = parts;
                q.index = index;
                return Some(q);
            }
            Err(e) => {
                bad.push((id, e));
                return None;
            }
        }
    }
    let name = p.name(id);
    if name.is_empty() {
        return None;
    }
    let kind = match d.kind {
        DeclKind::Class | DeclKind::Interface => PartKind::Type,
        // Kotlin's constructor is a method named `constructor`.
        DeclKind::Function | DeclKind::Method | DeclKind::Constructor => PartKind::FunctionOrMethod,
        DeclKind::Getter => PartKind::Getter,
        DeclKind::Setter => PartKind::Setter,
        DeclKind::Field | DeclKind::StaticField => PartKind::Decl,
        DeclKind::Param => PartKind::Input,
        DeclKind::Local => PartKind::Local,
        DeclKind::ModuleLet => PartKind::Decl,
        DeclKind::Module | DeclKind::Library | DeclKind::Label | DeclKind::Removed => {
            return None;
        }
    };
    q.parts.push(Part {
        name: name.to_string(),
        kind,
    });
    Some(q)
}
