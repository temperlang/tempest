//! Token adapters between the lexer and the parser, as in Kotlin Temper's
//! `TokenSourceAdapter.kt`. In order:
//!
//! 1. **Comment grouping** collects comments for docs, joining adjacent line
//!    comments. It doesn't change the tokens.
//! 2. **String fixing** wraps each string in synthetic `(` `)`, so it's one
//!    operand. A multi-quoted string with `:` scriptlet lines instead gets
//!    `{` `}` and its parts become statements: `+++ "chunk";`, the
//!    scriptlet code, and `${hole};`. It also trims incidental space from
//!    multi-quoted strings.
//! 3. **Bracket recovery** ([`balance`](crate::balance), ours) closes
//!    brackets left open and drops unmatched closers, so one bad bracket
//!    stays a local error. Balanced tokens are left alone.
//! 4. **Semicolon insertion** drops ignorable tokens, inserting `;` after a
//!    `}` that ends a line and before a `{` that starts one, when the tokens
//!    around can't continue the expression (see Kotlin's `asi.md`).
//! 5. **Call joins** put a synthetic `callJoin:` operator between `}` and a
//!    word that isn't an infix operator, so `} else {` continues a call.
//! 6. **Word pairing** turns modifier words like `public` into `@public`
//!    when a word follows, and `var x` or `const x` into `@var let x`.

use crate::balance::{BracketError, balance};
use crate::lex::{Synthetic, Token, TokenKind, doc_comment};
use crate::operator::{OpType, Operator, is_close_bracket, is_open_bracket};

/// A token as the parser sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PToken {
    pub kind: TokenKind,
    /// Byte span in the source; zero-width at its place for synthetic tokens.
    pub start: u32,
    pub end: u32,
    /// Byte span of the text in [`Adapted::text`]: the source span itself,
    /// or text after the source for synthetic and trimmed tokens.
    pub text_start: u32,
    pub text_end: u32,
    pub synthetic: bool,
    pub may_bracket: bool,
    /// Whether the parser may take the token as a prefix or infix operator.
    pub may_prefix: bool,
    pub may_infix: bool,
}

/// A comment, for doc comments and the language server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comment {
    pub kind: CommentKind,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentKind {
    /// A run of `//` comments on adjacent lines.
    Line,
    /// `/* ... */`.
    Block,
    /// Prose in a literate file.
    Semilit,
    /// A paragraph of prose, as a synthetic `/**? ... ?*/` comment.
    SemilitParagraph,
}

/// The adapters' output.
#[derive(Clone, Debug, Default)]
pub struct Adapted {
    /// The source, then texts of synthetic and changed tokens.
    pub text: String,
    pub source_len: u32,
    pub tokens: Vec<PToken>,
    pub comments: Vec<Comment>,
    /// Brackets that didn't balance, which bracket recovery fixed.
    pub bracket_errors: Vec<BracketError>,
}

impl Adapted {
    pub fn token_text(&self, token: PToken) -> &str {
        &self.text[token.text_start as usize..token.text_end as usize]
    }
}

/// Runs all adapters over the lexer's tokens for `source`.
pub fn adapt(source: &str, tokens: &[Token]) -> Adapted {
    let mut a = Adapter {
        text: source.to_string(),
        fixed: Vec::new(),
    };
    let comments = group_comments(source, tokens);
    let tokens: Vec<PToken> = tokens.iter().map(|&t| a.lexer_token(t)).collect();
    let tokens = a.fix_strings(&tokens);
    let (tokens, bracket_errors) = a.fix_brackets(tokens, source.len() as u32);
    let tokens = a.insert_semicolons(&tokens);
    let tokens = a.insert_call_joins(&tokens);
    let tokens = a.pair_words(&tokens);
    Adapted {
        source_len: source.len() as u32,
        text: a.text,
        tokens,
        comments,
        bracket_errors,
    }
}

struct Adapter {
    text: String,
    /// Where fixed texts like `;` already are in `text`.
    fixed: Vec<(&'static str, u32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarginKind {
    /// `"`: the line ends with a line break.
    Terminated,
    /// `~`, or no margin yet: no line break.
    Unterminated,
    /// `:`: a scriptlet line.
    Fragment,
}

#[derive(Debug)]
struct MqString {
    start: usize,
    end: Option<usize>,
    fragments: Vec<(usize, usize)>,
    chunks: Vec<(MarginKind, usize)>,
    /// Holes `${...}`, from `${` to `}` inclusive.
    holes: Vec<(usize, usize)>,
}

/// A token in a multi-quoted string with what to put before and after it.
#[derive(Clone, Copy)]
struct Edit {
    before: Option<PToken>,
    token: PToken,
    after: Option<PToken>,
    /// Starts a statement: content or a hole in a string with scriptlets.
    starts: bool,
}

const TRIPLE_QUOTE: &str = "\"\"\"";

impl Adapter {
    fn text(&self, token: PToken) -> &str {
        &self.text[token.text_start as usize..token.text_end as usize]
    }

    /// Adds text after the source, returning its span.
    fn add_text(&mut self, text: &str) -> (u32, u32) {
        let start = self.text.len() as u32;
        self.text.push_str(text);
        (start, self.text.len() as u32)
    }

    fn fixed_text(&mut self, text: &'static str) -> (u32, u32) {
        if let Some(&(_, start)) = self.fixed.iter().find(|(t, _)| *t == text) {
            return (start, start + text.len() as u32);
        }
        let (start, end) = self.add_text(text);
        self.fixed.push((text, start));
        (start, end)
    }

    /// A synthetic token at `pos`.
    fn synthetic(&mut self, pos: u32, text: &'static str, kind: TokenKind) -> PToken {
        let (text_start, text_end) = self.fixed_text(text);
        PToken {
            kind,
            start: pos,
            end: pos,
            text_start,
            text_end,
            synthetic: true,
            may_bracket: false,
            may_prefix: true,
            may_infix: true,
        }
    }

    fn bracket(&mut self, pos: u32, text: &'static str, close: bool) -> PToken {
        let mut token = self.synthetic(pos, text, TokenKind::Punctuation);
        token.may_bracket = true;
        token.may_infix = !close;
        token
    }

    fn semicolon(&mut self, pos: u32, from_string: bool) -> PToken {
        let mut token = self.synthetic(pos, ";", TokenKind::Punctuation);
        token.may_prefix = !from_string;
        token
    }

    fn lexer_token(&mut self, token: Token) -> PToken {
        let (text_start, text_end) = match token.synthetic {
            None => (token.start, token.end),
            Some(Synthetic::Paragraph) => {
                let text = doc_comment(&self.text[token.range()]).unwrap_or_default();
                self.add_text(&text)
            }
            Some(synthetic) => self.fixed_text(synthetic.text()),
        };
        PToken {
            kind: token.kind,
            start: token.start,
            end: token.end,
            text_start,
            text_end,
            synthetic: token.synthetic.is_some(),
            may_bracket: token.may_bracket,
            may_prefix: true,
            may_infix: true,
        }
    }

    // String fixing.

    fn fix_strings(&mut self, tokens: &[PToken]) -> Vec<PToken> {
        let mut out = Vec::with_capacity(tokens.len() + tokens.len() / 8);
        let mut i = 0;
        while i < tokens.len() {
            let token = tokens[i];
            i += 1;
            if token.kind != TokenKind::LeftDelimiter || self.text(token) != TRIPLE_QUOTE {
                self.wrap_quote(token, &mut out);
                continue;
            }
            // Collect up to the matching close, wrapping single-line strings.
            let mut collected = vec![token];
            let mut depth = 1;
            while i < tokens.len() {
                let token = tokens[i];
                i += 1;
                if self.text(token) != TRIPLE_QUOTE {
                    self.wrap_quote(token, &mut collected);
                    continue;
                }
                collected.push(token);
                match token.kind {
                    TokenKind::LeftDelimiter => depth += 1,
                    TokenKind::RightDelimiter => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            self.fix_multi_quoted(&collected, &mut out);
        }
        out
    }

    /// Wraps quotes in parentheses.
    fn wrap_quote(&mut self, token: PToken, out: &mut Vec<PToken>) {
        match token.kind {
            TokenKind::LeftDelimiter => {
                out.push(self.bracket(token.start, "(", false));
                out.push(token);
            }
            TokenKind::RightDelimiter => {
                out.push(token);
                out.push(self.bracket(token.end, ")", true));
            }
            _ => out.push(token),
        }
    }

    fn fix_multi_quoted(&mut self, collected: &[PToken], out: &mut Vec<PToken>) {
        let strings = self.find_multi_quoted(collected);
        let mut edits: Vec<Edit> = collected
            .iter()
            .map(|&token| Edit {
                before: None,
                token,
                after: None,
                starts: false,
            })
            .collect();
        let drop = |edits: &mut Vec<Edit>, i: usize| edits[i].token.kind = TokenKind::Space;
        for string in &strings {
            let Some(end) = string.end else { continue };
            let start = string.start;
            self.trim_incidental_space(string, &mut edits);
            if string.fragments.is_empty() {
                edits[start].before = Some(self.bracket(collected[start].start, "(", false));
                edits[end].after = Some(self.bracket(collected[end].end, ")", true));
                continue;
            }
            edits[start].before = Some(self.bracket(collected[start].start, "{", false));
            edits[end].after = Some(self.bracket(collected[end].end, "}", true));
            for &(first, last) in &string.fragments {
                drop(&mut edits, first);
                // Only space. Kotlin drops the line's last token, meaning its
                // line break, even when it's code, as at the end of input.
                if edits[last].token.kind.is_ignorable() {
                    drop(&mut edits, last);
                }
            }
            for &(_, chunk) in &string.chunks {
                // Trimming may have emptied it.
                if edits[chunk].token.kind == TokenKind::QuotedString {
                    let mut emit =
                        self.synthetic(collected[chunk].start, "+++", TokenKind::Punctuation);
                    emit.may_infix = false;
                    edits[chunk].before = Some(emit);
                    edits[chunk].after = Some(self.semicolon(collected[chunk].end, true));
                    edits[chunk].starts = true;
                }
            }
            for &(first, last) in &string.holes {
                let empty = (first + 1..last).all(|i| edits[i].token.kind.is_ignorable());
                if empty {
                    drop(&mut edits, first);
                    drop(&mut edits, last);
                } else {
                    edits[last].after = Some(self.semicolon(edits[last].token.end, true));
                    edits[first].starts = true;
                }
            }
        }
        // The last significant token, to end a scriptlet's statement
        // before content.
        let mut last: Option<PToken> = None;
        for edit in edits {
            if edit.starts {
                let ended = last.is_some_and(|l| {
                    // After `}`, semicolon insertion decides.
                    l.kind == TokenKind::LeftDelimiter || matches!(self.text(l), "{" | ";" | "}")
                });
                if !ended {
                    let pos = edit.before.unwrap_or(edit.token).start;
                    out.push(self.semicolon(pos, true));
                }
            }
            for token in edit
                .before
                .into_iter()
                .chain([edit.token])
                .chain(edit.after)
            {
                out.push(token);
                if !token.kind.is_ignorable() {
                    last = Some(token);
                }
            }
        }
    }

    /// Finds the multi-quoted strings in `tokens`, in order of their first
    /// part, with their scriptlet lines, text chunks and holes.
    fn find_multi_quoted(&self, tokens: &[PToken]) -> Vec<MqString> {
        let mut strings: Vec<MqString> = Vec::new();
        fn string(strings: &mut Vec<MqString>, start: usize) -> &mut MqString {
            match strings.iter().position(|s| s.start == start) {
                Some(i) => &mut strings[i],
                None => {
                    strings.push(MqString {
                        start,
                        end: None,
                        fragments: Vec::new(),
                        chunks: Vec::new(),
                        holes: Vec::new(),
                    });
                    strings.last_mut().unwrap()
                }
            }
        }
        // The margin of the current line of each multi-quoted string.
        let mut margins: Vec<(usize, MarginKind)> = Vec::new();
        // Open holes: the index of their `${` and their string.
        let mut holes: Vec<(usize, usize)> = Vec::new();
        // Open quotes, with the index of each `"""`.
        let mut quotes: Vec<Option<usize>> = Vec::new();
        let mut fragments = Vec::new();
        // Open `{` and `${`: their index, and which string's hole they're
        // in, if any, directly or nested.
        let mut curlies: Vec<(usize, Option<usize>)> = Vec::new();
        for (i, &token) in tokens.iter().enumerate() {
            let text = self.text(token);
            let top = quotes.last().copied().flatten();
            match token.kind {
                TokenKind::Margin => {
                    if let Some(start) = fragments.pop() {
                        if let Some(top) = top {
                            string(&mut strings, top).fragments.push((start, i - 1));
                        }
                    }
                    if let Some(top) = top {
                        let margin = match text {
                            "\"" => MarginKind::Terminated,
                            ":" => MarginKind::Fragment,
                            _ => MarginKind::Unterminated,
                        };
                        match margins.iter_mut().find(|(start, _)| *start == top) {
                            Some(entry) => entry.1 = margin,
                            None => margins.push((top, margin)),
                        }
                    }
                    if text == ":" {
                        fragments.push(i);
                    }
                }
                TokenKind::Punctuation => match text {
                    "${" | "{" => {
                        let inside = curlies.last().and_then(|c| c.1);
                        // Holes are only on content lines, not in scriptlet
                        // code or code in a hole. Kotlin takes any `${`.
                        let hole = top.filter(|&top| {
                            let margin = margins.iter().find(|(start, _)| *start == top);
                            let fragment = margin.is_some_and(|m| m.1 == MarginKind::Fragment);
                            text == "${" && !fragment && inside != Some(top)
                        });
                        curlies.push((i, hole.or(inside)));
                        if let Some(top) = hole {
                            holes.push((i, top));
                        }
                    }
                    "}" => {
                        let Some((start, _)) = curlies.pop() else {
                            continue;
                        };
                        if let Some(&(_, top)) = holes.last().filter(|h| h.0 == start) {
                            holes.pop();
                            string(&mut strings, top).holes.push((start, i));
                        }
                    }
                    _ => {}
                },
                TokenKind::LeftDelimiter => quotes.push((text == TRIPLE_QUOTE).then_some(i)),
                TokenKind::RightDelimiter => {
                    if let Some(Some(top)) = quotes.pop() {
                        if let Some(start) = fragments.pop() {
                            string(&mut strings, top).fragments.push((start, i - 1));
                        }
                        string(&mut strings, top).end = Some(i);
                    }
                }
                TokenKind::QuotedString => {
                    if let Some(top) = top {
                        let margin = margins.iter().find(|(start, _)| *start == top);
                        let margin = margin.map_or(MarginKind::Unterminated, |m| m.1);
                        string(&mut strings, top).chunks.push((margin, i));
                    }
                }
                _ => {}
            }
        }
        strings
    }

    /// Removes incidental space from a multi-quoted string's chunks: spaces
    /// at the end of each line, the line break right after the open quote,
    /// and line breaks on lines without a `"` margin. Other line breaks
    /// become `\n`.
    fn trim_incidental_space(&mut self, string: &MqString, edits: &mut [Edit]) {
        let mut lines: Vec<(MarginKind, Vec<usize>)> = Vec::new();
        let mut line = Vec::new();
        let mut kind = MarginKind::Unterminated;
        for &(chunk_kind, i) in &string.chunks {
            let token = edits[i].token;
            if token.kind != TokenKind::QuotedString {
                continue;
            }
            line.push(i);
            kind = chunk_kind;
            if self.text(token).ends_with(['\n', '\r']) {
                lines.push((kind, std::mem::take(&mut line)));
            }
        }
        if !line.is_empty() {
            lines.push((kind, line));
        }
        for (kind, line) in lines {
            let last_index = *line.last().unwrap();
            let last = edits[last_index].token;
            let text = self.text(last);
            let after_open = line.len() == 1
                && last_index == string.start + 1
                && text
                    .bytes()
                    .all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'));
            let needs_newline = kind == MarginKind::Terminated && !after_open;
            let without_break = text.trim_end_matches(['\n', '\r']);
            let had_break = without_break.len() != text.len();
            let trimmed = without_break.trim_end_matches([' ', '\t']);
            if had_break && needs_newline {
                if trimmed.len() + 1 == text.len() && text.ends_with('\n') {
                    continue; // Already just right.
                }
                let new = format!("{trimmed}\n");
                let (start, end) = self.add_text(&new);
                edits[last_index].token.text_start = start;
                edits[last_index].token.text_end = end;
            } else if trimmed.is_empty() {
                edits[last_index].token.kind = TokenKind::Space;
            } else if trimmed.len() != text.len() {
                edits[last_index].token.text_end = last.text_start + trimmed.len() as u32;
            }
        }
    }

    // Bracket recovery.

    fn fix_brackets(
        &mut self,
        tokens: Vec<PToken>,
        source_len: u32,
    ) -> (Vec<PToken>, Vec<BracketError>) {
        let Some(fixes) = balance(&self.text, source_len, &tokens) else {
            return (tokens, Vec::new());
        };
        let mut inserts = fixes.inserts.into_iter().peekable();
        let mut out = Vec::with_capacity(tokens.len() + 4);
        for (i, mut token) in tokens.into_iter().enumerate() {
            if !fixes.deleted[i] {
                if fixes.demoted[i] {
                    token.may_bracket = false;
                }
                out.push(token);
            }
            while let Some((_, close)) = inserts.next_if(|&(after, _)| after == i) {
                out.push(self.bracket(token.end, close, true));
            }
        }
        (out, fixes.errors)
    }

    // Semicolon insertion.

    fn insert_semicolons(&mut self, tokens: &[PToken]) -> Vec<PToken> {
        let mut out = Vec::with_capacity(tokens.len());
        let mut last: Option<PToken> = None;
        let mut line_break = false;
        let mut depth = Depth::default();
        for &token in tokens {
            let text = self.text(token);
            if token.kind.is_ignorable() {
                line_break = line_break || text.contains(['\n', '\r']);
                continue;
            }
            depth.count(token.kind, text);
            let mut insert = false;
            if let (Some(last), true) = (last, line_break) {
                let last_text = self.text(last);
                // Before a `{` starting a line, unless the last token may
                // continue the expression.
                if token.kind == TokenKind::Punctuation
                    && text == "{"
                    && !is_open_bracket(last_text)
                {
                    let (prefix, infix, postfix) = positions(last_text, last.kind);
                    insert = postfix || !(prefix || infix);
                }
                // After a `}` ending a line, unless the next token may
                // continue the expression or the `}` closes a hole.
                if !insert && last.kind == TokenKind::Punctuation && last_text == "}" {
                    let (prefix, infix, postfix) = positions(text, token.kind);
                    insert = !is_close_bracket(text)
                        && (prefix || !(infix || postfix))
                        && !depth.in_hole();
                }
            }
            line_break = false;
            if let (Some(last), true) = (last, insert) {
                // Kotlin counts the token's brackets again. A bug, kept for
                // now so comparisons stay exact.
                depth.count(token.kind, self.text(token));
                out.push(self.semicolon(last.end, false));
            }
            out.push(token);
            last = Some(token);
        }
        out
    }

    // Call joins.

    fn insert_call_joins(&mut self, tokens: &[PToken]) -> Vec<PToken> {
        let mut out = Vec::with_capacity(tokens.len());
        let mut after_curly = false;
        let mut last: Option<PToken> = None;
        for &token in tokens {
            let text = self.text(token);
            if after_curly
                && token.kind == TokenKind::Word
                && Operator::matching(text, token.kind, OpType::Infix).is_empty()
            {
                let join = self.synthetic(token.start, "callJoin:", TokenKind::Word);
                out.push(join);
            }
            // Not after the `}` around a multi-quoted string with scriptlets,
            // which is one operand like a string in parentheses.
            let around_string =
                token.synthetic && last.is_some_and(|l| l.kind == TokenKind::RightDelimiter);
            after_curly =
                token.kind == TokenKind::Punctuation && self.text(token) == "}" && !around_string;
            out.push(token);
            last = Some(token);
        }
        out
    }

    // Word pairing.

    fn pair_words(&mut self, tokens: &[PToken]) -> Vec<PToken> {
        let mut out = Vec::with_capacity(tokens.len());
        let mut after_at = false;
        let mut after_dot = false;
        // The synthetic `let` to go after a modifier, if any.
        let mut pending_let: Option<PToken> = None;
        let mut i = 0;
        let mut modifier: Option<PToken> = None;
        loop {
            let token = if let Some(token) = modifier.take() {
                token
            } else if let Some(token) = pending_let.take() {
                token
            } else if i < tokens.len() {
                i += 1;
                tokens[i - 1]
            } else {
                break;
            };
            let (was_at, was_dot) = (after_at, after_dot);
            after_at = false;
            after_dot = false;
            let text = self.text(token);
            if was_dot && token.kind == TokenKind::Word {
                // A word after a dot is a member name, never an operator.
                out.push(PToken {
                    may_prefix: false,
                    may_infix: false,
                    ..token
                });
                continue;
            }
            if !was_at && MODIFIERS.contains(&text) {
                let words = tokens[i..]
                    .iter()
                    .take_while(|t| {
                        t.kind == TokenKind::Word
                            || (t.kind == TokenKind::Punctuation && self.text(**t) == "@")
                    })
                    .take(2)
                    .count();
                if words > 0 {
                    if (text == "const" || text == "var") && words == 1 {
                        pending_let = Some(self.synthetic(token.end, "let", TokenKind::Word));
                    }
                    modifier = Some(token);
                    after_at = true;
                    out.push(self.synthetic(token.start, "@", TokenKind::Punctuation));
                    continue;
                }
            }
            after_at = token.kind == TokenKind::Punctuation && text == "@";
            after_dot = token.kind == TokenKind::Punctuation && text == ".";
            out.push(token);
        }
        out
    }
}

/// Tracks `{` and `${` nesting for semicolon insertion.
#[derive(Default)]
struct Depth {
    depth: usize,
    /// Whether the `{` or `${` open at each depth is a `${`.
    holes: Vec<bool>,
}

impl Depth {
    fn count(&mut self, kind: TokenKind, text: &str) {
        if kind != TokenKind::Punctuation {
            return;
        }
        match text {
            "{" | "${" => {
                if self.holes.len() <= self.depth {
                    self.holes.resize(self.depth + 1, false);
                }
                self.holes[self.depth] = text == "${";
                self.depth += 1;
            }
            "}" => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
    }

    /// Whether the last `}` closed a `${`.
    fn in_hole(&self) -> bool {
        self.holes.get(self.depth).copied().unwrap_or(false)
    }
}

/// Words that become decorators before another word: `public x` is
/// `@public x`.
const MODIFIERS: [&str; 12] = [
    "abstract",
    "const",
    "export",
    "final",
    "native",
    "private",
    "protected",
    "public",
    "sealed",
    "static",
    "var",
    "volatile",
];

/// Whether a token may be a prefix, infix (or separator), or postfix operator.
fn positions(text: &str, kind: TokenKind) -> (bool, bool, bool) {
    let has = |op_type| !Operator::matching(text, kind, op_type).is_empty();
    (
        has(OpType::Prefix),
        has(OpType::Infix) || has(OpType::Separator),
        has(OpType::Postfix),
    )
}

fn comment_kind(text: &str, synthetic: bool) -> CommentKind {
    if synthetic && text.starts_with("/**?") && text.ends_with("?*/") {
        CommentKind::SemilitParagraph
    } else if text.starts_with("/*") && text.ends_with("*/") {
        CommentKind::Block
    } else if text.starts_with("//") {
        CommentKind::Line
    } else {
        CommentKind::Semilit
    }
}

/// Collects comments, joining line comments separated only by space with
/// at most one line break, along with that space.
fn group_comments(source: &str, tokens: &[Token]) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        i += 1;
        if token.kind != TokenKind::Comment {
            continue;
        }
        let kind = comment_kind(&token.text(source), token.synthetic.is_some());
        let mut end = token.end;
        if kind == CommentKind::Line {
            let mut last_was_break = false;
            while let Some(&next) = tokens.get(i) {
                match next.kind {
                    TokenKind::Comment => {
                        let text = next.text(source);
                        if comment_kind(&text, next.synthetic.is_some()) != CommentKind::Line {
                            break;
                        }
                    }
                    TokenKind::Space => {}
                    _ => break,
                }
                let breaks = line_breaks(&next.text(source));
                if (breaks != 0 && last_was_break) || breaks > 1 {
                    break;
                }
                last_was_break = breaks != 0;
                end = end.max(next.end);
                i += 1;
            }
        }
        comments.push(Comment {
            kind,
            start: token.start,
            end,
        });
    }
    comments
}

/// Line breaks in text, counting CRLF as one, up to 2.
fn line_breaks(text: &str) -> usize {
    let mut count = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && count < 2 {
        match bytes[i] {
            b'\n' => count += 1,
            b'\r' => {
                count += 1;
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    count
}
