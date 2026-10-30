//! Source maps (Source Map v3) from tmpl's spans, for backends
//! (notes/tmpl/nodes.md, "Source positions").
//!
//! A backend's printer writes into a [`MappedText`], marking where output
//! that comes from a span starts ([`MappedText::mark`]), and where a
//! declaration's name starts, with its source name
//! ([`MappedText::mark_name`]). [`source_map`] then turns the marks into
//! a map's JSON. Generated and source columns both count UTF-16 units, as
//! browsers and Node do; source positions come from each file's line
//! table ([`SourceFile::position`]). Maps leave out `sourcesContent`,
//! since tmpl doesn't carry its sources' text, and name each source by
//! its path, after a prefix that leads from the map to the library's
//! root.

use crate::{FileId, Program, SourceFile, Span};
use std::fmt::{self, Write};

/// Output text that records where output from each span starts.
#[derive(Clone, Debug, Default)]
pub struct MappedText {
    pub text: String,
    /// In the order written, so by offset.
    pub marks: Vec<Mark>,
}

/// Where output from a span starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    /// A byte offset into the text.
    pub offset: usize,
    pub span: Span,
    /// For a declaration's name, its name in the source.
    pub name: Option<String>,
}

impl MappedText {
    pub fn new() -> MappedText {
        MappedText::default()
    }

    pub fn push_str(&mut self, s: &str) {
        self.text.push_str(s);
    }

    pub fn push(&mut self, c: char) {
        self.text.push(c);
    }

    /// Marks what's written next as coming from `span`'s start, until the
    /// next mark. Unknown spans mark nothing.
    pub fn mark(&mut self, span: Span) {
        self.add(span, None);
    }

    /// Marks what's written next as a declaration's name, `name` in the
    /// source, at `span`.
    pub fn mark_name(&mut self, span: Span, name: &str) {
        self.add(span, Some(name.to_string()));
    }

    fn add(&mut self, span: Span, name: Option<String>) {
        if !span.is_known() {
            return;
        }
        let offset = self.text.len();
        let mark = Mark { offset, span, name };
        // A later mark at the same place is the more specific one.
        match self.marks.last_mut() {
            Some(last) if last.offset == offset => *last = mark,
            _ => self.marks.push(mark),
        }
    }
}

impl Write for MappedText {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.text.push_str(s);
        Ok(())
    }
}

impl SourceFile {
    /// Builds the line table from the file's text.
    pub(crate) fn set_text(&mut self, text: &str) {
        self.line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i as u32 + 1))
            .collect();
        self.wide_lines = (text.split('\n').enumerate())
            .filter(|(_, line)| !line.is_ascii())
            .map(|(i, line)| (i as u32, line.to_string()))
            .collect();
    }

    /// A byte offset's line and column, both counted from 0, the column
    /// in UTF-16 units; `None` without a line table.
    pub fn position(&self, offset: u32) -> Option<(u32, u32)> {
        let line = self.line_starts.partition_point(|&s| s <= offset);
        let line = line.checked_sub(1)?;
        let column = offset - self.line_starts[line];
        let line = line as u32;
        let column = match self.wide_lines.binary_search_by_key(&line, |(l, _)| *l) {
            Ok(i) => utf16_len(&self.wide_lines[i].1, column as usize),
            Err(_) => column,
        };
        Some((line, column))
    }
}

/// The UTF-16 length of the characters that start before byte `end`.
fn utf16_len(text: &str, end: usize) -> u32 {
    (text.char_indices())
        .take_while(|&(i, _)| i < end)
        .map(|(_, c)| c.len_utf16() as u32)
        .sum()
}

/// One mapping: a generated column, and the source position it maps to,
/// all from 0, with indexes into the map's `sources` and `names`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub column: u32,
    pub source: u32,
    pub source_line: u32,
    pub source_column: u32,
    pub name: Option<u32>,
}

/// A source map's content, before it's written as JSON.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    /// The generated file's name.
    pub file: String,
    pub sources: Vec<String>,
    pub names: Vec<String>,
    /// Per generated line, its segments in column order.
    pub lines: Vec<Vec<Segment>>,
}

/// The map of a generated file's marks, for `file`, its name, with each
/// source named by `sources_prefix` and its path; `None` if no mark has
/// a known position, as in a program read without spans.
pub fn source_map(
    p: &Program,
    file: &str,
    sources_prefix: &str,
    out: &MappedText,
) -> Option<SourceMap> {
    let mut map = SourceMap {
        file: file.to_string(),
        ..SourceMap::default()
    };
    let mut sources: Vec<FileId> = Vec::new();
    let text = out.text.as_bytes();
    let (mut scanned, mut line_start) = (0, 0);
    let mut lines: Vec<Vec<Segment>> = vec![Vec::new()];
    for mark in &out.marks {
        for (i, &b) in text[scanned..mark.offset].iter().enumerate() {
            if b == b'\n' {
                lines.push(Vec::new());
                line_start = scanned + i + 1;
            }
        }
        scanned = mark.offset;
        let Some((source_line, source_column)) =
            (p.files.get(mark.span.file.index())).and_then(|f| f.position(mark.span.start))
        else {
            continue;
        };
        let source = match sources.iter().position(|&f| f == mark.span.file) {
            Some(i) => i,
            None => {
                sources.push(mark.span.file);
                sources.len() - 1
            }
        } as u32;
        let name = mark.name.as_ref().map(|name| {
            (map.names.iter().position(|n| n == name)).unwrap_or_else(|| {
                map.names.push(name.clone());
                map.names.len() - 1
            }) as u32
        });
        let generated = &out.text[line_start..mark.offset];
        let column = if generated.is_ascii() {
            generated.len() as u32
        } else {
            utf16_len(generated, generated.len())
        };
        let segment = Segment {
            column,
            source,
            source_line,
            source_column,
            name,
        };
        let line = lines.last_mut().expect("a line");
        // Nothing new: the same place again, without a name.
        let same = |s: &Segment| {
            (s.source, s.source_line, s.source_column) == (source, source_line, source_column)
        };
        if name.is_none() && line.last().is_some_and(|s| s.name.is_none() && same(s)) {
            continue;
        }
        line.push(segment);
    }
    if sources.is_empty() {
        return None;
    }
    // Every line, to the end: Node (24) reads a name into a last
    // segment that ends the mappings without one.
    let rest = text[scanned..].iter().filter(|&&b| b == b'\n').count();
    lines.extend(std::iter::repeat_with(Vec::new).take(rest));
    map.sources = (sources.iter())
        .map(|f| format!("{sources_prefix}{}", p.files[f.index()].path))
        .collect();
    map.lines = lines;
    Some(map)
}

impl SourceMap {
    /// The map as Source Map v3 JSON, on one line.
    pub fn to_json(&self) -> String {
        let list = |items: &[String]| {
            let items: Vec<String> = items.iter().map(|s| json_string(s)).collect();
            format!("[{}]", items.join(","))
        };
        format!(
            "{{\"version\":3,\"file\":{},\"sources\":{},\"names\":{},\"mappings\":\"{}\"}}\n",
            json_string(&self.file),
            list(&self.sources),
            list(&self.names),
            encode_mappings(&self.lines)
        )
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Appends a number as a Base64 VLQ: five bits per digit, least
/// significant first, with a continuation bit, and the sign in the
/// lowest bit of the first.
pub fn vlq(out: &mut String, n: i64) {
    let mut v = (n.unsigned_abs() << 1) | u64::from(n < 0);
    loop {
        let digit = (v & 31) as usize;
        v >>= 5;
        out.push(BASE64[digit | if v > 0 { 32 } else { 0 }] as char);
        if v == 0 {
            break;
        }
    }
}

/// Reads one Base64 VLQ, or `None` at the end or on a bad digit.
fn read_vlq(digits: &mut std::iter::Peekable<std::str::Bytes<'_>>) -> Option<i64> {
    let (mut v, mut shift) = (0u64, 0);
    loop {
        let byte = digits.next()?;
        let digit = BASE64.iter().position(|&b| b == byte)? as u64;
        v |= (digit & 31) << shift;
        shift += 5;
        if digit < 32 {
            break;
        }
        if shift > 60 {
            return None;
        }
    }
    let magnitude = (v >> 1) as i64;
    Some(if v & 1 == 1 { -magnitude } else { magnitude })
}

/// The `mappings` field: lines separated by `;`, segments by `,`, each
/// segment's fields relative to the previous segment's (the generated
/// column only within its line).
pub fn encode_mappings(lines: &[Vec<Segment>]) -> String {
    let mut out = String::new();
    let (mut source, mut line, mut column, mut name) = (0i64, 0i64, 0i64, 0i64);
    for (i, segments) in lines.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        let mut generated = 0i64;
        for (j, s) in segments.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            vlq(&mut out, i64::from(s.column) - generated);
            vlq(&mut out, i64::from(s.source) - source);
            vlq(&mut out, i64::from(s.source_line) - line);
            vlq(&mut out, i64::from(s.source_column) - column);
            (generated, source) = (s.column.into(), s.source.into());
            (line, column) = (s.source_line.into(), s.source_column.into());
            if let Some(n) = s.name {
                vlq(&mut out, i64::from(n) - name);
                name = n.into();
            }
        }
    }
    out
}

/// Reads a `mappings` field back, for tests and tools. Segments without
/// a source, which [`encode_mappings`] never writes, are an error.
pub fn decode_mappings(mappings: &str) -> Result<Vec<Vec<Segment>>, String> {
    let mut lines = Vec::new();
    let mut state = [0i64; 4]; // source, line, column, name
    for (i, text) in mappings.split(';').enumerate() {
        let mut segments = Vec::new();
        let mut generated = 0i64;
        for segment in text.split(',').filter(|s| !s.is_empty()) {
            let mut digits = segment.bytes().peekable();
            let mut fields = Vec::new();
            while digits.peek().is_some() {
                let n = read_vlq(&mut digits).ok_or(format!("line {i}: bad VLQ in {segment}"))?;
                fields.push(n);
            }
            if fields.len() != 4 && fields.len() != 5 {
                return Err(format!("line {i}: {} fields in {segment}", fields.len()));
            }
            generated += fields[0];
            for k in 0..fields.len() - 1 {
                state[k] += fields[k + 1];
            }
            let field =
                |n: i64| u32::try_from(n).map_err(|_| format!("line {i}: {n} in {segment}"));
            segments.push(Segment {
                column: field(generated)?,
                source: field(state[0])?,
                source_line: field(state[1])?,
                source_column: field(state[2])?,
                name: if fields.len() == 5 {
                    Some(field(state[3])?)
                } else {
                    None
                },
            });
        }
        lines.push(segments);
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vlqs(ns: &[i64]) -> String {
        let mut out = String::new();
        for &n in ns {
            vlq(&mut out, n);
        }
        out
    }

    /// Vectors from the Source Map v3 spec's examples and Mozilla's
    /// `source-map` library's tests.
    #[test]
    fn vlq_vectors() {
        assert_eq!(vlqs(&[0, 1, -1, 15, -15, 16, -16]), "ACDefgBhB");
        assert_eq!(vlqs(&[1000]), "w+B");
        assert_eq!(vlqs(&[-1000]), "x+B");
        assert_eq!(vlqs(&[1 << 30]), "ggggggC");
        assert_eq!(vlqs(&[i64::from(i32::MAX)]), "+/////D");
        assert_eq!(vlqs(&[-i64::from(i32::MAX)]), "//////D");
        for n in [
            0,
            1,
            -1,
            31,
            32,
            -33,
            1000,
            -123_456_789,
            i64::from(i32::MIN),
        ] {
            let text = vlqs(&[n]);
            assert_eq!(read_vlq(&mut text.bytes().peekable()), Some(n), "{text}");
        }
        assert_eq!(read_vlq(&mut "g".bytes().peekable()), None);
        assert_eq!(read_vlq(&mut "!".bytes().peekable()), None);
    }

    fn seg(column: u32, source: u32, line: u32, col: u32, name: Option<u32>) -> Segment {
        Segment {
            column,
            source,
            source_line: line,
            source_column: col,
            name,
        }
    }

    #[test]
    fn mappings_vectors() {
        let lines = vec![
            vec![seg(0, 0, 0, 0, None), seg(9, 0, 0, 4, Some(0))],
            vec![],
            vec![seg(2, 0, 1, 2, None), seg(4, 1, 0, 0, Some(1))],
            vec![seg(0, 0, 3, 0, Some(0))],
        ];
        let encoded = encode_mappings(&lines);
        // Generated columns restart each line; the rest carry on.
        assert_eq!(encoded, "AAAA,SAAIA;;EACF,ECDFC;ADGAD");
        assert_eq!(decode_mappings(&encoded).unwrap(), lines);
        assert_eq!(decode_mappings("").unwrap(), vec![Vec::<Segment>::new()]);
        assert!(decode_mappings("A").is_err());
        assert!(decode_mappings("AAA").is_err());
        assert!(decode_mappings("AAAD").is_err());
    }

    #[test]
    fn positions() {
        let mut file = SourceFile {
            path: "a.temper".into(),
            hash: [0; 16],
            line_starts: Vec::new(),
            wide_lines: Vec::new(),
        };
        assert_eq!(file.position(0), None);
        file.set_text("ab\nx = \"é😀\" + y\r\nend");
        assert_eq!(file.line_starts, [0, 3, 21]);
        assert_eq!(file.position(1), Some((0, 1)));
        assert_eq!(file.position(3), Some((1, 0)));
        // `é` is two bytes and one unit; `😀` four bytes and two units.
        let plus = 3 + "x = \"é😀\" ".len() as u32;
        assert_eq!(file.position(plus), Some((1, 10)));
        assert_eq!(file.position(21), Some((2, 0)));
        assert_eq!(file.position(99), Some((2, 78)));
    }

    #[test]
    fn json() {
        let map = SourceMap {
            file: "a.js".into(),
            sources: vec!["../src/a \"b\".temper".into()],
            names: vec!["f".into()],
            lines: vec![vec![seg(0, 0, 0, 0, None)], vec![seg(2, 0, 1, 2, Some(0))]],
        };
        assert_eq!(
            map.to_json(),
            "{\"version\":3,\"file\":\"a.js\",\"sources\":[\"../src/a \\\"b\\\".temper\"],\
             \"names\":[\"f\"],\"mappings\":\"AAAA;EACEA\"}\n"
        );
    }
}
