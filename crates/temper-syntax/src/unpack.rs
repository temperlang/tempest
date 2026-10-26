//! Values from token text, as Kotlin Temper's `unpackValue`: numbers,
//! string content with escapes, quoted names like ``nym`a-b` ``, and the
//! content of doc comments.

use crate::lex::is_kotlin_whitespace;

/// A number literal's value, as Kotlin Temper reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Number {
    Int32(i32),
    Int64(i64),
    Float64(f64),
}

/// A warning about a number literal that still has a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberWarning {
    /// An `Int32` out of bounds, with its value in decimal, and whether it
    /// would fit an `Int64`.
    Int32OutOfBounds(String, bool),
    Int64OutOfBounds(String),
}

/// Reads a number token, or `None` if it's malformed. Underscores separate
/// digits; `0x`, `0b` and `0o` set the radix; `i32`, `i64` and `f64` the
/// type. Out-of-range integers wrap, with a warning.
pub fn unpack_number(token: &str) -> Option<(Number, Option<NumberWarning>)> {
    let cleaned: String = token.chars().filter(|&c| c != '_').collect();
    let mut text = cleaned.as_str();
    let lower = text.get(..2).map(str::to_ascii_lowercase);
    let radix = match lower.as_deref() {
        Some("0x") => 16,
        Some("0b") => 2,
        Some("0o") => 8,
        _ => 10,
    };
    let prefixed = radix != 10;
    if prefixed {
        text = &text[2..];
    }
    let ty = match numeric_suffix(text, radix) {
        Some((unsuffixed, ty)) => {
            text = unsuffixed;
            ty
        }
        None if radix == 10 && text.contains(['.', 'e', 'E']) => NumType::Float64,
        None => NumType::Int32,
    };
    if ty == NumType::Float64 {
        return parse_java_double(text).map(|f| (Number::Float64(f), None));
    }
    // Zero-padded decimals like `010` would surprise people.
    if !prefixed && text.starts_with('0') && text.contains(|c: char| ('1'..='9').contains(&c)) {
        return None;
    }
    let big = BigDigits::parse(text, radix)?;
    let hex = radix == 16;
    Some(if ty == NumType::Int32 {
        let fits = big.within(1 << 31, (1 << 31) - 1) || (hex && big.within(0, u32::MAX as u128));
        let fits64 = big.within(1 << 63, (1 << 63) - 1);
        let warning = (!fits).then(|| NumberWarning::Int32OutOfBounds(big.decimal(), fits64));
        (Number::Int32(big.low as i32), warning)
    } else {
        let fits = big.within(1 << 63, (1 << 63) - 1) || (hex && big.within(0, u64::MAX as u128));
        let warning = (!fits).then(|| NumberWarning::Int64OutOfBounds(big.decimal()));
        (Number::Int64(big.low as i64), warning)
    })
}

#[derive(PartialEq)]
enum NumType {
    Int32,
    Int64,
    Float64,
}

/// A type suffix like `i64` among the last three characters, and the text
/// before it, as Kotlin's `findNumericTypeSuffix`.
fn numeric_suffix(text: &str, radix: u32) -> Option<(&str, NumType)> {
    let bytes = text.as_bytes();
    let lowest = bytes.len().saturating_sub(3);
    for i in (lowest..bytes.len()).rev() {
        let rest = &text[i + 1..];
        let suffix = match bytes[i] {
            b'f' | b'F' if radix == 16 => None,
            b'f' | b'F' => match rest {
                "64" => Some(NumType::Float64),
                _ => return None,
            },
            b'i' | b'I' => match rest {
                "32" => Some(NumType::Int32),
                "64" => Some(NumType::Int64),
                _ => return None,
            },
            b'0'..=b'9' => None,
            _ => return None,
        };
        if let Some(suffix) = suffix {
            return Some((&text[..i], suffix));
        }
    }
    None
}

/// As Java's `Double.parseDouble` on number tokens, which allows a trailing
/// `d` or `f`.
fn parse_java_double(text: &str) -> Option<f64> {
    let text = text.trim_matches(|c: char| c <= ' ');
    let digits = text.strip_suffix(['d', 'D', 'f', 'F']).unwrap_or(text);
    let plain = digits
        .bytes()
        .all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b));
    if !plain || !digits.bytes().any(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// An integer's sign, digits and radix, with its low 64 bits and its
/// magnitude if it fits in a `u128`.
struct BigDigits {
    negative: bool,
    digits: Vec<u8>,
    radix: u32,
    low: u64,
    magnitude: Option<u128>,
}

impl BigDigits {
    /// As Java's `BigInteger(text, radix)`.
    fn parse(text: &str, radix: u32) -> Option<BigDigits> {
        let (negative, body) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        if body.is_empty() {
            return None;
        }
        let mut digits = Vec::with_capacity(body.len());
        let (mut low, mut magnitude) = (0u64, Some(0u128));
        for c in body.chars() {
            let d = c.to_digit(radix)?;
            digits.push(d as u8);
            low = low.wrapping_mul(radix as u64).wrapping_add(d as u64);
            magnitude = magnitude
                .and_then(|m| m.checked_mul(radix as u128))
                .and_then(|m| m.checked_add(d as u128));
        }
        if negative {
            low = low.wrapping_neg();
        }
        Some(BigDigits {
            negative,
            digits,
            radix,
            low,
            magnitude,
        })
    }

    /// Whether the value is in `-min..=max`.
    fn within(&self, min: u128, max: u128) -> bool {
        match self.magnitude {
            Some(m) if self.negative => m <= min,
            Some(m) => m <= max,
            None => false,
        }
    }

    fn decimal(&self) -> String {
        // Base 10^9 limbs, least significant first.
        let mut limbs: Vec<u32> = vec![0];
        for &d in &self.digits {
            let mut carry = d as u64;
            for limb in &mut limbs {
                let v = *limb as u64 * self.radix as u64 + carry;
                *limb = (v % 1_000_000_000) as u32;
                carry = v / 1_000_000_000;
            }
            if carry > 0 {
                limbs.push(carry as u32);
            }
        }
        let mut out = String::new();
        if self.negative && limbs.iter().any(|&l| l != 0) {
            out.push('-');
        }
        let mut first = true;
        for &limb in limbs.iter().rev() {
            let text = limb.to_string();
            if !first {
                out.extend(std::iter::repeat_n('0', 9 - text.len()));
            }
            out.push_str(&text);
            first = false;
        }
        out
    }
}

/// String content with escapes decoded, or `None` for a bad escape, as
/// Kotlin's `unpackQuotedString`. With `delimited`, the first and last
/// characters are quotes to skip.
pub fn unpack_string(text: &str, delimited: bool) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let (mut i, limit) = if delimited {
        if chars.len() < 2 {
            return None;
        }
        (1, chars.len() - 1)
    } else {
        (0, chars.len())
    };
    let mut out = String::with_capacity(text.len());
    let hex = |from: usize, to: usize| -> Option<u32> {
        chars[from..to]
            .iter()
            .try_fold(0u32, |n, c| Some((n << 4) | c.to_digit(16)?))
    };
    while i < limit {
        let c = chars[i];
        i += 1;
        if c != '\\' {
            out.push(c);
            continue;
        }
        let next = *chars.get(i).filter(|_| i < limit)?;
        i += 1;
        match next {
            '\\' | '"' | '\'' | '`' | '$' | '{' | '}' | '/' => out.push(next),
            '0' => out.push('\0'),
            'b' => out.push('\u{8}'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            'v' => out.push('\u{b}'),
            'f' => out.push('\u{c}'),
            'r' => out.push('\r'),
            'u' if chars.get(i) == Some(&'{') && i < limit => {
                // `\u{...}`: comma-separated code points.
                let end = hex_comma_run(&chars[..limit], i + 1, &mut out)?;
                if end >= limit || chars[end] != '}' {
                    return None;
                }
                i = end + 1;
            }
            'u' | 'x' => {
                let count = if next == 'u' { 4 } else { 2 };
                if i + count > limit {
                    return None;
                }
                let code = hex(i, i + count)?;
                out.push(char::from_u32(code)?);
                i += count;
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Reads `hex(,hex)*` code points from `i`, returning where it stopped.
fn hex_comma_run(chars: &[char], mut i: usize, out: &mut String) -> Option<usize> {
    let mut code = 0u32;
    let mut digit = false;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        let value = match c {
            ',' if !digit => return None,
            ',' => {
                out.push(char::from_u32(code)?);
                (code, digit) = (0, false);
                continue;
            }
            _ => match c.to_digit(16).filter(|_| c.is_ascii()) {
                Some(value) => value,
                None => {
                    if digit {
                        out.push(char::from_u32(code)?);
                    }
                    return Some(i - 1);
                }
            },
        };
        code = (code << 4) | value;
        digit = true;
        if code > 0x10FFFF || (0xD800..=0xDFFF).contains(&code) {
            return None;
        }
    }
    Some(i)
}

/// A word's name: itself, or for ``nym`...` `` the quoted text, which must
/// be in Unicode NFKC, as Kotlin's `decodeName`.
pub fn decode_name(text: &str) -> Option<String> {
    match text.strip_prefix("nym") {
        Some(quoted) if quoted.starts_with('`') => unpack_string(quoted, true)
            .filter(|name| !name.is_empty() && crate::nfkc::is_nfkc(name)),
        _ => Some(text.to_string()),
    }
}

/// The text of a `/* */` comment without its delimiters and line prefixes,
/// as Kotlin's `blockCommentContent`.
pub fn block_comment_content(text: &str) -> String {
    unwrap_comment(text, b'*')
}

/// The text of a run of `//` comments, as Kotlin's `lineCommentContent`.
pub fn line_comment_content(text: &str) -> String {
    unwrap_comment(text, b'/')
}

/// The content of a paragraph's synthetic `/**? ... ?*/` comment, as
/// Kotlin's `unMassagedSemilitParagraphContent`.
pub fn paragraph_comment_content(text: &str) -> Option<String> {
    let lines = kotlin_lines(text);
    if lines.len() <= 2 || lines[0] != "/**?" || *lines.last().unwrap() != "?*/" {
        return None;
    }
    let inner: Vec<&str> = lines[1..lines.len() - 1]
        .iter()
        .map(|&line| match line {
            " *" => "",
            _ => line.strip_prefix(" * ").unwrap_or(line),
        })
        .collect();
    Some(inner.join("\n"))
}

/// Lines split at `\r\n`, `\n` or `\r`, as Kotlin's `lines()`.
fn kotlin_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&text[start..i]);
                start = i + 1;
            }
            b'\r' => {
                lines.push(&text[start..i]);
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    lines.push(&text[start..]);
    lines
}

/// Kotlin's `unwrapCommentContent`, for `/* */` comments when `mark` is
/// `*`, and `//` comments when it's `/`. The patterns it uses there are
/// written out as loops.
fn unwrap_comment(text: &str, mark: u8) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = normalized.split('\n').map(str::to_string).collect();
    let space = |c: char| c == ' ' || c == '\t';
    let mark_char = mark as char;
    // The first line's opening: `/[*]+ ?` or `//+ ?`.
    let first = &lines[0];
    if let Some(rest) = first.strip_prefix('/') {
        let after = rest.trim_start_matches(mark_char);
        if after.len() < rest.len() {
            lines[0] = after.strip_prefix(' ').unwrap_or(after).to_string();
        }
    }
    // The last line's closing: `[*]*/$` or `/+$`.
    let last = lines.len() - 1;
    let line = &lines[last];
    lines[last] = if mark == b'*' {
        match line.strip_suffix('/') {
            Some(rest) => rest.trim_end_matches('*').to_string(),
            None => line.clone(),
        }
    } else {
        line.trim_end_matches('/').to_string()
    };
    if lines.len() == 1 {
        return lines[0].trim_end_matches(space).to_string();
    }
    // Strip a common prefix of space and marks from all but the first line.
    let middle: Vec<&String> = lines[1..last]
        .iter()
        .filter(|line| line.chars().any(|c| !space(c)))
        .collect();
    let common = common_prefix(&middle);
    let prefix = allowed_prefix(&common, mark_char).to_string();
    if !prefix.is_empty() {
        for line in &mut lines[1..] {
            if let Some(rest) = line.strip_prefix(prefix.as_str()) {
                *line = rest.to_string();
            }
        }
    }
    if prefix.ends_with(' ') {
        if lines[0].starts_with(' ') {
            lines[0].remove(0);
        }
    } else if lines[1..]
        .iter()
        .all(|line| is_blank(line) || line.starts_with(' '))
    {
        for line in &mut lines {
            if line.starts_with(' ') {
                line.remove(0);
            }
        }
    }
    for line in &mut lines {
        line.truncate(line.trim_end_matches(space).len());
    }
    // A common run of marks at the ends of lines, as in a box of stars.
    let suffix = common_suffix(&lines[1..last]);
    let marks = suffix.len() - suffix.trim_end_matches(mark_char).len();
    if marks > 0 {
        let suffix = &suffix[suffix.len() - marks..];
        for line in &mut lines[..last] {
            if line.ends_with(suffix) {
                line.truncate(line.len() - suffix.len());
            }
        }
        for line in &mut lines {
            line.truncate(line.trim_end_matches(space).len());
        }
    }
    let begin = lines
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !l.is_empty())
        .map_or(begin, |i| i + 1);
    lines[begin..end].join("\n")
}

fn is_blank(text: &str) -> bool {
    text.chars().all(is_kotlin_whitespace)
}

/// The longest common prefix of the lines, empty for none.
fn common_prefix(lines: &[&String]) -> String {
    let Some(first) = lines.first() else {
        return String::new();
    };
    let mut len = first.len();
    for line in &lines[1..] {
        len = first
            .bytes()
            .zip(line.bytes())
            .take(len)
            .take_while(|(a, b)| a == b)
            .count();
    }
    while !first.is_char_boundary(len) {
        len -= 1;
    }
    first[..len].to_string()
}

fn common_suffix(lines: &[String]) -> String {
    let Some(first) = lines.first() else {
        return String::new();
    };
    let mut len = first.len();
    for line in &lines[1..] {
        len = first
            .bytes()
            .rev()
            .zip(line.bytes().rev())
            .take(len)
            .take_while(|(a, b)| a == b)
            .count();
    }
    let mut start = first.len() - len;
    while !first.is_char_boundary(start) {
        start += 1;
    }
    first[start..].to_string()
}

/// The part of a common prefix that may be stripped: `^[ \t]*(?:M+ ?)?`
/// for the mark `M`.
fn allowed_prefix(common: &str, mark: char) -> &str {
    let after_space = common.trim_start_matches([' ', '\t']);
    let after_marks = after_space.trim_start_matches(mark);
    if after_marks.len() == after_space.len() {
        return &common[..common.len() - after_space.len()];
    }
    let after = after_marks.strip_prefix(' ').unwrap_or(after_marks);
    &common[..common.len() - after.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(unpack_number("1_000"), Some((Number::Int32(1000), None)));
        assert_eq!(unpack_number("0x10"), Some((Number::Int32(16), None)));
        assert_eq!(unpack_number("0xFFFFFFFF"), Some((Number::Int32(-1), None)));
        assert_eq!(unpack_number("010"), None);
        assert_eq!(unpack_number("1.5"), Some((Number::Float64(1.5), None)));
        assert_eq!(unpack_number("2i64"), Some((Number::Int64(2), None)));
        assert_eq!(
            unpack_number("99999999999"),
            Some((
                Number::Int32(1215752191),
                Some(NumberWarning::Int32OutOfBounds("99999999999".into(), true))
            ))
        );
        assert_eq!(
            unpack_number("0x1_0000_0000_0000_0000_0000_0000_0000_0000").map(|n| n.1),
            Some(Some(NumberWarning::Int32OutOfBounds(
                "340282366920938463463374607431768211456".into(),
                false
            )))
        );
    }

    #[test]
    fn strings() {
        assert_eq!(
            unpack_string(r"a\nb\u{41,42}\x43", false).as_deref(),
            Some("a\nbABC")
        );
        assert_eq!(unpack_string(r"\q", false), None);
        assert_eq!(unpack_string(r"\uD800", false), None);
        assert_eq!(decode_name("nym`a-b`").as_deref(), Some("a-b"));
        assert_eq!(decode_name("x").as_deref(), Some("x"));
        // Not in NFKC.
        assert_eq!(decode_name("nym`\u{fb01}`"), None);
    }

    #[test]
    fn comments() {
        assert_eq!(block_comment_content("/** doc */"), "doc");
        assert_eq!(
            block_comment_content("/**\n * a\n *\n *   b\n */"),
            "a\n\n  b"
        );
        // A group of line comments includes the line break after the last.
        assert_eq!(line_comment_content("// a\n  // b\n"), "a\nb");
        assert_eq!(
            paragraph_comment_content("/**?\n * a\n *\n?*/").as_deref(),
            Some("a\n")
        );
    }
}
