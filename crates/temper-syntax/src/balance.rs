//! Bracket recovery, ours: Kotlin Temper has none. An adapter step, after
//! string fixing, that makes brackets balance before the parser sees them,
//! so one missing or extra bracket stays one local error instead of
//! swallowing the rest of the file.
//!
//! Tokens that already balance are left alone, so code that's well formed
//! parses exactly as in Kotlin. Otherwise:
//!
//! - An unclosed `(` or `[` gets a synthetic closer at the nearest
//!   plausible place among its own tokens: before a line indented at or
//!   left of the opener's line, before a `{` that looks like a body (in
//!   `f(x: Int: Int { ... }`), or before a `;` (except in `for (...)`).
//!   Failing those, it closes where the enclosing bracket does.
//! - A close bracket with no open bracket to match is dropped.
//! - A `}` starting a line left of its `{`'s line, when an outer `{` starts
//!   a line at that indent, closes the outer one: the inner `{` is missing
//!   its `}`, which goes before its first line at or left of its own.
//! - Brackets left open at the end close where they should, as above,
//!   replacing the lexer's closers at the end.
//! - Angle brackets are weaker: a `<` still open at a real close bracket,
//!   `;`, `{` or the end is less-than, and a `>` with no `<` is
//!   greater-than.
//!
//! Quotes and string holes are kept as the lexer paired them: `)` and `]`
//! never close past them.

use crate::adapt::PToken;
use crate::lex::TokenKind;
use crate::tree::Span;

/// A bracket problem, reported as a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BracketError {
    pub kind: BracketErrorKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BracketErrorKind {
    /// A close bracket is missing, for the open bracket on `line`.
    Unclosed {
        open: &'static str,
        close: &'static str,
        line: u32,
    },
    /// A close bracket matches no open bracket.
    Unmatched,
    /// A `<` with no `>`, and maybe what was likely meant, like `a < b`.
    UnclosedAngle(Option<String>),
}

/// What [`balance`] changes.
#[derive(Debug, Default)]
pub(crate) struct Fixes {
    /// Synthetic closers to put after the token at an index, in order.
    pub inserts: Vec<(usize, &'static str)>,
    pub deleted: Vec<bool>,
    /// `<` and `>` to treat as comparisons.
    pub demoted: Vec<bool>,
    pub errors: Vec<BracketError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Paren,
    Square,
    Curly,
    Angle,
    Quote,
}

#[derive(Clone, Copy, Debug)]
enum Class {
    /// An open bracket; strict ones (quotes, holes) are as the lexer paired them.
    Open(Kind, bool),
    Close(Kind),
    /// A closer the lexer added at the end, or marked as unmatched.
    EndCurly,
    Stray,
    Semi,
    Other,
}

/// An open bracket on the stack.
#[derive(Clone, Copy, Debug)]
struct Entry {
    kind: Kind,
    strict: bool,
    token: usize,
}

/// Fixes for `tokens`, or `None` when their brackets balance.
pub(crate) fn balance(text: &str, source_len: u32, tokens: &[PToken]) -> Option<Fixes> {
    let fixes = Balancer::new(text, source_len, tokens, false).run();
    if fixes.errors.is_empty() && !fixes.deleted.iter().any(|&d| d) {
        return None;
    }
    Some(Balancer::new(text, source_len, tokens, true).run())
}

struct Balancer<'a> {
    text: &'a str,
    source_len: u32,
    tokens: &'a [PToken],
    /// Whether a `}` may close an outer `{` by indentation.
    realign: bool,
    stack: Vec<Entry>,
    lines: std::cell::OnceCell<Vec<u32>>,
    /// For open brackets, the index just past their pair: after the close
    /// token, or the token a synthetic closer goes before.
    after: Vec<usize>,
    fixes: Fixes,
}

impl<'a> Balancer<'a> {
    fn new(text: &'a str, source_len: u32, tokens: &'a [PToken], realign: bool) -> Self {
        Balancer {
            text,
            source_len,
            tokens,
            realign,
            stack: Vec::new(),
            lines: std::cell::OnceCell::new(),
            after: vec![0; tokens.len()],
            fixes: Fixes {
                inserts: Vec::new(),
                deleted: vec![false; tokens.len()],
                demoted: vec![false; tokens.len()],
                errors: Vec::new(),
            },
        }
    }

    fn text_of(&self, i: usize) -> &'a str {
        let t = self.tokens[i];
        &self.text[t.text_start as usize..t.text_end as usize]
    }

    fn skip(&self, i: usize) -> bool {
        self.tokens[i].kind.is_ignorable() || self.fixes.deleted[i]
    }

    fn class(&self, i: usize) -> Class {
        let token = self.tokens[i];
        let text = self.text_of(i);
        match token.kind {
            TokenKind::LeftDelimiter => Class::Open(Kind::Quote, true),
            TokenKind::RightDelimiter => Class::Close(Kind::Quote),
            TokenKind::Error if text == "}" => Class::Stray,
            TokenKind::Punctuation => match text {
                "(" | "\\(" => Class::Open(Kind::Paren, false),
                "[" => Class::Open(Kind::Square, false),
                "{" => Class::Open(Kind::Curly, false),
                "${" | "\\{" | "\\u{" => Class::Open(Kind::Curly, true),
                "<" if token.may_bracket => Class::Open(Kind::Angle, false),
                ")" => Class::Close(Kind::Paren),
                "]" => Class::Close(Kind::Square),
                "}" if token.synthetic && token.start == self.source_len => Class::EndCurly,
                "}" => Class::Close(Kind::Curly),
                ">" if token.may_bracket => Class::Close(Kind::Angle),
                ";" => Class::Semi,
                _ => Class::Other,
            },
            _ => Class::Other,
        }
    }

    /// Where each line starts, found once, when first needed.
    fn lines(&self) -> &[u32] {
        self.lines.get_or_init(|| {
            let source = &self.text.as_bytes()[..self.source_len as usize];
            let breaks = source.iter().enumerate().filter(|&(_, &b)| b == b'\n');
            std::iter::once(0)
                .chain(breaks.map(|(n, _)| n as u32 + 1))
                .collect()
        })
    }

    /// The line `pos` is on, from 0, and where that line starts.
    fn line(&self, pos: u32) -> (usize, usize) {
        let lines = self.lines();
        let line = lines.partition_point(|&start| start <= pos) - 1;
        (line, lines[line] as usize)
    }

    /// Whether token `i` starts its line, and if so, its indent.
    fn line_start(&self, i: usize) -> Option<u32> {
        let pos = self.tokens[i].start.min(self.source_len);
        let (_, line) = self.line(pos);
        let before = &self.text.as_bytes()[line..pos as usize];
        (before.iter().all(|&b| b == b' ' || b == b'\t')).then_some(before.len() as u32)
    }

    /// The indent of the line token `i` is on.
    fn line_indent(&self, i: usize) -> u32 {
        let pos = self.tokens[i].start.min(self.source_len);
        let (_, line) = self.line(pos);
        let before = &self.text.as_bytes()[line..pos as usize];
        before
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count() as u32
    }

    fn line_of(&self, pos: u32) -> u32 {
        self.line(pos.min(self.source_len)).0 as u32 + 1
    }

    /// The last token before `i` that the parser sees.
    fn prev(&self, i: usize) -> Option<usize> {
        (0..i).rev().find(|&j| !self.skip(j))
    }

    fn next(&self, i: usize) -> Option<usize> {
        (i..self.tokens.len()).find(|&j| !self.skip(j))
    }

    fn run(mut self) -> Fixes {
        for i in 0..self.tokens.len() {
            if self.tokens[i].kind.is_ignorable() {
                continue;
            }
            match self.class(i) {
                Class::Open(Kind::Curly, strict) => {
                    self.close_angles();
                    self.push(Kind::Curly, strict, i);
                }
                Class::Open(kind, strict) => self.push(kind, strict, i),
                Class::Close(Kind::Angle) => {
                    if self.stack.last().is_some_and(|e| e.kind == Kind::Angle) {
                        let e = self.stack.pop().unwrap();
                        self.after[e.token] = i + 1;
                    } else {
                        self.fixes.demoted[i] = true;
                    }
                }
                Class::Close(kind) => self.close(kind, i),
                Class::Semi => self.close_angles(),
                // Closing a multi-quoted string's scriptlet blocks or its
                // synthetic `{`, or else missing.
                Class::EndCurly if !self.missing_at_end() => self.close(Kind::Curly, i),
                Class::EndCurly => self.fixes.deleted[i] = true,
                Class::Stray => self.unmatched(i),
                Class::Other => {}
            }
        }
        // Angles still open at the end swallow nothing, so they stay, as
        // in Kotlin.
        let end = self.tokens.len();
        while let Some(e) = self.stack.pop() {
            self.unclosed(e, end);
        }
        // Stable, so inner closers stay first.
        self.fixes.inserts.sort_by_key(|&(after, _)| after);
        self.fixes
    }

    /// Whether a `}` the lexer added at the end is for a missing one, or
    /// one recovery already closed: not in a string, and not for a
    /// string's synthetic `{`.
    fn missing_at_end(&self) -> bool {
        !self.stack.iter().any(|e| e.kind == Kind::Quote)
            && (self.stack.iter().rev())
                .find(|e| e.kind == Kind::Curly)
                .is_none_or(|e| !self.tokens[e.token].synthetic)
    }

    fn push(&mut self, kind: Kind, strict: bool, token: usize) {
        self.stack.push(Entry {
            kind,
            strict,
            token,
        });
    }

    /// Treats `<` still open on top as less-than.
    fn close_angles(&mut self) {
        while self.stack.last().is_some_and(|e| e.kind == Kind::Angle) {
            let e = self.stack.pop().unwrap();
            self.fixes.demoted[e.token] = true;
            let hint = self.comparison_hint(e.token);
            let t = self.tokens[e.token];
            self.fixes.errors.push(BracketError {
                kind: BracketErrorKind::UnclosedAngle(hint),
                span: Span {
                    start: t.start,
                    end: t.end,
                },
            });
        }
    }

    /// `a < b` for `a<b` between a value and an operand, with no spaces.
    fn comparison_hint(&self, lt: usize) -> Option<String> {
        let (before, after) = (self.prev(lt)?, self.next(lt + 1)?);
        let (b, a) = (self.tokens[before], self.tokens[after]);
        let t = self.tokens[lt];
        if b.end != t.start
            || t.end != a.start
            || a.kind != TokenKind::Word && a.kind != TokenKind::Number
        {
            return None;
        }
        let left = self.text_of(before);
        // `List<Int` is more likely a type than a comparison.
        let value = left.starts_with(|c: char| c.is_lowercase() || c.is_ascii_digit() || c == '_')
            || left == ")";
        value.then(|| format!("{left} < {}", self.text_of(after)))
    }

    fn unmatched(&mut self, i: usize) {
        self.fixes.deleted[i] = true;
        let t = self.tokens[i];
        self.fixes.errors.push(BracketError {
            kind: BracketErrorKind::Unmatched,
            span: Span {
                start: t.start,
                end: t.end,
            },
        });
    }

    /// A close bracket of `kind` at token `i`.
    fn close(&mut self, kind: Kind, i: usize) {
        self.close_angles();
        let mut target = None;
        for (j, e) in self.stack.iter().enumerate().rev() {
            if e.kind == kind {
                target = Some(j);
                break;
            }
            // `}` closes past `(` and `[`, as the lexer paired curlies; `)`
            // and `]` close past neither curlies nor quotes.
            let barrier = match kind {
                Kind::Curly | Kind::Quote => e.kind == Kind::Quote,
                _ => matches!(e.kind, Kind::Curly | Kind::Quote),
            };
            if barrier {
                break;
            }
        }
        let Some(mut target) = target else {
            self.unmatched(i);
            return;
        };
        if self.realign && kind == Kind::Curly && !self.stack[target].strict {
            if let Some(indent) = self.line_start(i) {
                if self.line_indent(self.stack[target].token) > indent {
                    let outer = (0..target).rev().find(|&j| {
                        let e = self.stack[j];
                        e.kind == Kind::Curly && !e.strict && self.line_indent(e.token) == indent
                    });
                    let crosses = |j: usize| {
                        self.stack[j..target]
                            .iter()
                            .any(|e| e.kind == Kind::Quote || e.strict)
                    };
                    if let Some(outer) = outer.filter(|&j| !crosses(j)) {
                        target = outer;
                    }
                }
            }
        }
        while self.stack.len() > target + 1 {
            let e = self.stack.pop().unwrap();
            self.unclosed(e, i);
        }
        let e = self.stack.pop().unwrap();
        self.after[e.token] = i + 1;
    }

    /// Closes `e`, whose close bracket is missing, at the best place
    /// before token `end`.
    fn unclosed(&mut self, e: Entry, end: usize) {
        let (open, close) = match (e.kind, self.text_of(e.token)) {
            (Kind::Paren, "(") => ("(", ")"),
            (Kind::Paren, _) => ("\\(", ")"),
            (Kind::Square, _) => ("[", "]"),
            (Kind::Curly, "{") => ("{", "}"),
            (Kind::Curly, "${") => ("${", "}"),
            (Kind::Curly, "\\{") => ("\\{", "}"),
            (Kind::Curly, _) => ("\\u{", "}"),
            // The lexer closes every quote, and an angle open at the end
            // stays one, as in Kotlin.
            (Kind::Quote | Kind::Angle, _) => {
                self.after[e.token] = end;
                return;
            }
        };
        let spot = if e.strict { end } else { self.spot(e, end) };
        self.after[e.token] = spot;
        let prev = self.prev(spot).unwrap_or(e.token).max(e.token);
        let start = self.tokens[prev].end;
        let stop = self.next(spot).filter(|&j| j < self.tokens.len());
        let stop = stop.map_or(start, |j| self.tokens[j].start.max(start));
        // Stay on the line of the gap.
        let gap = &self.text.as_bytes()[(start as usize).min(self.source_len as usize)
            ..(stop as usize).min(self.source_len as usize)];
        let stop = gap
            .iter()
            .position(|&b| b == b'\n')
            .map_or(stop, |n| start + n as u32);
        self.fixes.inserts.push((prev, close));
        self.fixes.errors.push(BracketError {
            kind: BracketErrorKind::Unclosed {
                open,
                close,
                line: self.line_of(self.tokens[e.token].start),
            },
            span: Span { start, end: stop },
        });
    }

    /// Where the closer for `e` goes: the token it goes before.
    fn spot(&self, e: Entry, end: usize) -> usize {
        let opener_indent = self.line_indent(e.token);
        let in_for =
            e.kind == Kind::Paren && self.prev(e.token).is_some_and(|p| self.text_of(p) == "for");
        // `:`s among the bracket's own tokens: the last, and how many.
        let (mut colon, mut colons) = (None, 0);
        let mut t = e.token + 1;
        while t < end {
            if self.skip(t) {
                t += 1;
                continue;
            }
            if self
                .line_start(t)
                .is_some_and(|indent| indent <= opener_indent)
            {
                return t;
            }
            if matches!(e.kind, Kind::Paren | Kind::Square) {
                let text = self.text_of(t);
                if text == "{" && self.tokens[t].kind == TokenKind::Punctuation && self.body(t, end)
                {
                    // In `f(x: Int: Int {`, the last `:` starts a return
                    // type, as does one right after the `(`.
                    return match colon {
                        Some(c) if e.kind == Kind::Paren && (colons >= 2 || c == e.token + 1) => c,
                        _ => t,
                    };
                }
                if text == ":" {
                    colon = Some(t);
                    colons += 1;
                }
                if text == ";" && !in_for {
                    return t;
                }
            }
            t = if self.after[t] > t {
                self.after[t]
            } else {
                t + 1
            };
        }
        end
    }

    /// Whether the `{` at `t`, inside an unclosed `(` or `[`, looks like a
    /// body after it rather than a block lambda in it.
    fn body(&self, t: usize, end: usize) -> bool {
        let before = self.prev(t).map(|p| self.text_of(p));
        if matches!(
            before,
            Some("(" | "," | "=>" | "fn" | "else" | "do" | "=" | "[")
        ) {
            return false;
        }
        let after = self.after[t];
        match self.next(after).filter(|&j| j < end) {
            Some(j) => !matches!(
                self.text_of(j),
                "," | ")" | "]" | "." | "?." | "else" | "orelse" | "catch"
            ),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lex::SourceKind;
    use crate::parse::parse;

    fn errors(source: &str) -> Vec<String> {
        let parsed = parse(source, SourceKind::Standalone);
        (parsed.bracket_errors().iter())
            .map(|e| format!("{:?} at {}..{}", e.kind, e.span.start, e.span.end))
            .collect()
    }

    #[test]
    fn balanced_brackets_are_left_alone() {
        assert!(errors("let f(a: List<Int>): Int { g(a[0], fn { 1 }) }").is_empty());
        // As in Kotlin, which makes an angle of it at the end.
        assert!(errors("a<b").is_empty());
    }

    #[test]
    fn missing_paren_closes_before_the_return_type() {
        let source = "let f(x: Int: Int { x }";
        let errors = errors(source);
        assert_eq!(
            errors,
            [r#"Unclosed { open: "(", close: ")", line: 1 } at 12..12"#]
        );
        assert_eq!(&source[..12], "let f(x: Int");
    }

    #[test]
    fn stray_closers_and_angles() {
        assert_eq!(errors("f(1));"), ["Unmatched at 4..5"]);
        assert_eq!(
            errors("if (a<b) { c }"),
            [r#"UnclosedAngle(Some("a < b")) at 5..6"#]
        );
    }
}
